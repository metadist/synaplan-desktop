//! UC-07 dictation. A spoken fixture is produced by the workspace's own TTS,
//! decoded to the session PCM format, and then pushed through the **exact
//! protocol `useDictation.ts` speaks**: live session with the 40 s cap, phrase
//! commits after a ≥ 700 ms pause once ≥ 5 s of speech is buffered, a poll for
//! interim text, `commit_session` on stop, then the correction pass in 35 s
//! windows, and the one-shot `transcribe` fallback. Every phase is timed.

use std::time::Instant;

use serde_json::json;
use synaplan_core::dictation::{
    append_chunk, close_session, commit_session, open_session, session_text, transcribe,
    DictationContext, SAMPLE_RATE,
};
use synaplan_core::media::{download_bytes, generate_speech};

use crate::fixtures::{self, DICTATION_TEXT};
use crate::report::{snippet, Case, CaseResult};
use crate::Ctx;

// Mirrors of the constants in `src/composables/useDictation.ts`.
const FRAME_SAMPLES: usize = 4096;
const PAUSE_MS: f64 = 700.0;
const MIN_CHUNK_SAMPLES: usize = SAMPLE_RATE as usize * 5;
const MAX_CHUNK_SAMPLES: usize = SAMPLE_RATE as usize * 40;
const CORRECTION_WINDOW_SAMPLES: usize = SAMPLE_RATE as usize * 35;
const CORRECTION_TAIL_SAMPLES: usize = SAMPLE_RATE as usize * 2;
const SILENCE_RMS: f64 = 0.012;
const GENERIC_PROMPT: &str = "Notes dictated by a person; plain sentences, no formatting.";

const STOP_TO_FINAL_TARGET_MS: u64 = 5_000;
const STOP_TO_FINAL_CEILING_MS: u64 = 10_000;
const SIMILARITY_FLOOR: f64 = 0.75;

fn should_commit(samples: usize, paused_ms: f64) -> bool {
    (paused_ms >= PAUSE_MS && samples >= MIN_CHUNK_SAMPLES) || samples >= MAX_CHUNK_SAMPLES
}

/// The client's live phrase splitter over a whole take, offline: returns the
/// byte ranges it would have posted with `commit=true`.
fn live_phrases(pcm: &[u8]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let frame_bytes = FRAME_SAMPLES * 2;
    let mut chunk_start: Option<usize> = None;
    let mut chunk_samples = 0usize;
    let mut paused_ms = 0f64;
    let mut pos = 0usize;
    while pos < pcm.len() {
        let end = (pos + frame_bytes).min(pcm.len());
        let frame = &pcm[pos..end];
        let silent = fixtures::rms_pcm16(frame) < SILENCE_RMS;
        let frame_samples = frame.len() / 2;
        if silent && chunk_samples == 0 {
            pos = end;
            continue;
        }
        if chunk_start.is_none() {
            chunk_start = Some(pos);
        }
        chunk_samples += frame_samples;
        paused_ms = if silent {
            paused_ms + frame_samples as f64 / SAMPLE_RATE as f64 * 1000.0
        } else {
            0.0
        };
        if should_commit(chunk_samples, paused_ms) {
            out.push((chunk_start.unwrap_or(pos), end));
            chunk_start = None;
            chunk_samples = 0;
            paused_ms = 0.0;
        }
        pos = end;
    }
    if let Some(start) = chunk_start {
        if chunk_samples > 0 {
            out.push((start, pcm.len()));
        }
    }
    out
}

/// `correctionWindows` from the composable, over bytes.
fn correction_windows(pcm: &[u8]) -> Vec<(usize, usize)> {
    let window = CORRECTION_WINDOW_SAMPLES * 2;
    let tail = CORRECTION_TAIL_SAMPLES * 2;
    if pcm.is_empty() {
        return Vec::new();
    }
    if pcm.len() <= window {
        return vec![(0, pcm.len())];
    }
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut i = 0usize;
    while i < pcm.len() {
        let remaining = pcm.len() - i;
        if remaining <= window {
            if remaining < tail {
                if let Some(last) = out.last_mut() {
                    last.1 = pcm.len();
                } else {
                    out.push((i, pcm.len()));
                }
            } else {
                out.push((i, pcm.len()));
            }
            break;
        }
        out.push((i, i + window));
        i += window;
    }
    out
}

pub async fn uc07_dictation(ctx: &Ctx) -> CaseResult {
    let mut case = Case::new("UC-07", "Dictate a note");
    let Some(catalog) = &ctx.catalog else {
        case.skip("no catalog");
        return case.finish("");
    };
    let speak_default = catalog
        .defaults
        .get("TEXT2SOUND")
        .cloned()
        .unwrap_or_default();
    let voice_default = catalog
        .defaults
        .get("SOUND2TEXT")
        .cloned()
        .unwrap_or_default();
    let mut speak_candidates: Vec<String> = vec![speak_default.clone()];
    for id in ["openai:tts-1:text2sound", "openai:tts-1-hd:text2sound"] {
        if !speak_candidates.iter().any(|c| c == id) {
            speak_candidates.push(id.to_string());
        }
    }
    speak_candidates.retain(|c| !c.is_empty());
    if voice_default.is_empty() {
        case.check(
            "dictation model advertised",
            false,
            "catalog has no SOUND2TEXT default",
        );
        return case.finish("no dictation model");
    }
    case.metric("voice_model", voice_default.clone());

    // 1. The spoken fixture from the workspace TTS.
    let mut audio: Option<(Vec<u8>, String, String)> = None;
    for model in &speak_candidates {
        let started = Instant::now();
        match generate_speech(&ctx.base, &ctx.key, DICTATION_TEXT, model).await {
            Ok(remote) => match download_bytes(&ctx.base, &ctx.key, &remote.url).await {
                Ok(bytes) if !bytes.is_empty() => {
                    case.metric("tts_ms", started.elapsed().as_millis() as u64);
                    case.metric("tts_model", model.clone());
                    audio = Some((bytes, remote.mime.clone(), model.clone()));
                    break;
                }
                Ok(_) => ctx.log(&format!("{model}: empty audio")),
                Err(e) => ctx.log(&format!("{model}: download failed: {e}")),
            },
            Err(e) => ctx.log(&format!("{model}: speech failed: {e}")),
        }
    }
    let Some((audio_bytes, mime, _)) = audio else {
        case.check(
            "spoken fixture produced",
            false,
            "no SPEAK model produced audio",
        );
        return case.finish("no fixture");
    };
    case.check(
        "spoken fixture produced",
        true,
        format!("{} KB {mime}", audio_bytes.len() / 1024),
    );
    let pcm = match fixtures::decode_to_pcm16k(&audio_bytes) {
        Ok(p) => p,
        Err(e) => {
            case.check("fixture decoded to 16 kHz PCM", false, e);
            return case.finish("ffmpeg missing");
        }
    };
    let seconds = pcm.len() as f64 / (SAMPLE_RATE as f64 * 2.0);
    case.metric("take_seconds", format!("{seconds:.1}"));
    case.check(
        "fixture decoded to 16 kHz PCM",
        seconds > 10.0,
        format!("{seconds:.1} s of speech"),
    );

    let dictation = DictationContext {
        model: ctx.wire(&voice_default),
        language: Some("en".to_string()),
        prompt: GENERIC_PROMPT.to_string(),
    };

    // 2. Live session, as the composable drives it.
    let phrases = live_phrases(&pcm);
    case.metric("live_phrases", phrases.len() as u64);
    let stop_clock;
    let live_text;
    let mut first_interim_ms: Option<u64> = None;
    let mut commit_latencies: Vec<u64> = Vec::new();
    let session_started = Instant::now();
    match open_session(
        &ctx.base,
        &ctx.key,
        &dictation,
        &format!("usecases-{}", ctx.run_id),
    )
    .await
    {
        Ok(id) => {
            case.metric(
                "session_open_ms",
                session_started.elapsed().as_millis() as u64,
            );
            for (i, (start, end)) in phrases.iter().enumerate() {
                let t = Instant::now();
                match append_chunk(&ctx.base, &ctx.key, &id, pcm[*start..*end].to_vec(), true).await
                {
                    Ok(()) => commit_latencies.push(t.elapsed().as_millis() as u64),
                    Err(e) => {
                        case.check(
                            format!("live phrase {} accepted", i + 1),
                            false,
                            e.to_string(),
                        );
                    }
                }
                if first_interim_ms.is_none() {
                    if let Ok(text) = session_text(&ctx.base, &ctx.key, &id).await {
                        if !text.is_empty() {
                            first_interim_ms = Some(session_started.elapsed().as_millis() as u64);
                        }
                    }
                }
            }
            // Stop pressed: the composable commits what the session holds.
            stop_clock = Instant::now();
            live_text = match commit_session(&ctx.base, &ctx.key, &id).await {
                Ok(t) => t,
                Err(e) => {
                    case.check("live commit on stop", false, e.to_string());
                    String::new()
                }
            };
            close_session(&ctx.base, &ctx.key, &id).await;
        }
        Err(e) => {
            case.check(
                "dictation session opens",
                false,
                format!("{e} ({})", e.code()),
            );
            return case.finish("session refused");
        }
    }
    let live_commit_ms = stop_clock.elapsed().as_millis() as u64;
    case.metric("live_commit_ms", live_commit_ms);
    case.metric("live_phrase_commit_ms", json!(commit_latencies));
    case.metric(
        "first_interim_ms",
        first_interim_ms.map(|v| json!(v)).unwrap_or(json!(null)),
    );
    let live_sim = fixtures::similarity(DICTATION_TEXT, &live_text);
    case.metric("live_similarity", format!("{live_sim:.2}"));
    case.check(
        "live text arrives while dictating",
        !live_text.trim().is_empty(),
        format!("similarity {live_sim:.2}: \"{}\"", snippet(&live_text, 90)),
    );

    // 3. Correction pass: new session, 35 s windows, commit each, final commit.
    let correction_started = Instant::now();
    let mut corrected = String::new();
    let windows: Vec<(usize, usize)> = correction_windows(&pcm)
        .into_iter()
        .filter(|(s, e)| fixtures::rms_pcm16(&pcm[*s..*e]) >= SILENCE_RMS)
        .collect();
    case.metric("correction_windows", windows.len() as u64);
    match open_session(
        &ctx.base,
        &ctx.key,
        &dictation,
        &format!("usecases-fix-{}", ctx.run_id),
    )
    .await
    {
        Ok(id) => {
            for (start, end) in &windows {
                if let Err(e) =
                    append_chunk(&ctx.base, &ctx.key, &id, pcm[*start..*end].to_vec(), true).await
                {
                    case.check("correction window accepted", false, e.to_string());
                }
                let _ = session_text(&ctx.base, &ctx.key, &id).await;
            }
            corrected = commit_session(&ctx.base, &ctx.key, &id)
                .await
                .unwrap_or_default();
            close_session(&ctx.base, &ctx.key, &id).await;
        }
        Err(e) => {
            case.check("correction session opens", false, e.to_string());
        }
    }
    let correction_ms = correction_started.elapsed().as_millis() as u64;
    let stop_to_final_ms = stop_clock.elapsed().as_millis() as u64;
    case.metric("correction_pass_ms", correction_ms);
    case.metric("stop_to_final_ms", stop_to_final_ms);

    // 4. One-shot fallback with the original container.
    let one_shot_started = Instant::now();
    let one_shot = transcribe(&ctx.base, &ctx.key, &dictation, audio_bytes.clone(), &mime)
        .await
        .unwrap_or_default();
    let one_shot_ms = one_shot_started.elapsed().as_millis() as u64;
    case.metric("one_shot_ms", one_shot_ms);
    case.metric(
        "one_shot_similarity",
        format!("{:.2}", fixtures::similarity(DICTATION_TEXT, &one_shot)),
    );

    // 5. A two-minute take, timing only: the client's sequential correction
    //    pass (one 35 s window after another) against one upload of the same
    //    audio. This is the "dictation is very slow" number.
    let long_pcm: Vec<u8> = pcm.iter().copied().cycle().take(pcm.len() * 4).collect();
    let long_seconds = long_pcm.len() as f64 / (SAMPLE_RATE as f64 * 2.0);
    let long_windows = correction_windows(&long_pcm);
    let long_started = Instant::now();
    let mut long_windows_ok = 0usize;
    match open_session(
        &ctx.base,
        &ctx.key,
        &dictation,
        &format!("usecases-long-{}", ctx.run_id),
    )
    .await
    {
        Ok(id) => {
            for (start, end) in &long_windows {
                if append_chunk(
                    &ctx.base,
                    &ctx.key,
                    &id,
                    long_pcm[*start..*end].to_vec(),
                    true,
                )
                .await
                .is_ok()
                {
                    long_windows_ok += 1;
                }
                let _ = session_text(&ctx.base, &ctx.key, &id).await;
            }
            let _ = commit_session(&ctx.base, &ctx.key, &id).await;
            close_session(&ctx.base, &ctx.key, &id).await;
        }
        Err(e) => ctx.log(&format!("long take session refused: {e}")),
    }
    let long_correction_ms = long_started.elapsed().as_millis() as u64;
    let long_one_shot_started = Instant::now();
    let long_one_shot = transcribe(
        &ctx.base,
        &ctx.key,
        &dictation,
        fixtures::wav_from_pcm16k(&long_pcm),
        "audio/wav",
    )
    .await
    .unwrap_or_default();
    let long_one_shot_ms = long_one_shot_started.elapsed().as_millis() as u64;
    case.metric("long_take_seconds", format!("{long_seconds:.0}"));
    case.metric("long_correction_windows", long_windows.len() as u64);
    case.metric("long_correction_pass_ms", long_correction_ms);
    case.metric("long_one_shot_ms", long_one_shot_ms);
    case.check(
        format!(
            "two-minute take: stop → final within {} s",
            STOP_TO_FINAL_CEILING_MS / 1000
        ),
        long_correction_ms <= STOP_TO_FINAL_CEILING_MS && long_windows_ok == long_windows.len(),
        format!(
            "{long_correction_ms} ms for a {long_seconds:.0} s take over {} sequential window(s) ({long_windows_ok} accepted); one upload of the same audio took {long_one_shot_ms} ms and returned {} words",
            long_windows.len(),
            long_one_shot.split_whitespace().count()
        ),
    );

    // 6. What the note receives.
    let final_text = if corrected.trim().is_empty() {
        live_text.clone()
    } else {
        corrected.clone()
    };
    let sim = fixtures::similarity(DICTATION_TEXT, &final_text);
    case.metric("final_similarity", format!("{sim:.2}"));
    case.check(
        "final text matches what was said",
        sim >= SIMILARITY_FLOOR,
        format!(
            "similarity {sim:.2} (floor {SIMILARITY_FLOOR}): \"{}\"",
            snippet(&final_text, 120)
        ),
    );
    case.check(
        format!("stop → final within {} s", STOP_TO_FINAL_CEILING_MS / 1000),
        stop_to_final_ms <= STOP_TO_FINAL_CEILING_MS,
        format!(
            "{stop_to_final_ms} ms for a {seconds:.0} s take (target {} s): live commit {live_commit_ms} ms + correction pass {correction_ms} ms over {} window(s); one-shot alone would be {one_shot_ms} ms",
            STOP_TO_FINAL_TARGET_MS / 1000,
            windows.len()
        ),
    );
    case.finish(format!(
        "{seconds:.0} s take → text in {:.1} s after stop, similarity {sim:.2}",
        stop_to_final_ms as f64 / 1000.0
    ))
}
