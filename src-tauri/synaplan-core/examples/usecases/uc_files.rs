//! UC-05 one file → one question (plus the two timing traps), UC-06
//! vectorization en bloc. Uploads go through `files::upload_project_file`
//! (the desktop's fields: `process_level=vectorize`, `group_key=DESKTOP:{id}`),
//! readiness through `files::list_project_files` (the same poll the Files
//! panel runs), questions through `stream_chat` with `rag_group_key`.

use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::json;
use synaplan_core::files::{
    delete_project_file, list_project_files, upload_project_file, KnowledgeFile, KnowledgeState,
    UploadHints,
};
use synaplan_core::messages::{ChatMessage, TurnContext};
use synaplan_core::projects::KNOWLEDGE_FOLDER_PREFIX;

use crate::fixtures::{self, Fact};
use crate::raw::{self, assistant, user};
use crate::report::{normalize, snippet, Case, CaseResult};
use crate::uc_chat::client_turn;
use crate::Ctx;

const SMALL_READY_CEILING: Duration = Duration::from_secs(120);
const CORPUS_READY_CEILING: Duration = Duration::from_secs(600);
const POLL_EVERY: Duration = Duration::from_secs(3);
/// The Files panel re-reads every 5 s; we poll a little faster to time it.
const BIG_FILE_BYTES: usize = 3 * 1024 * 1024;
const MID_FILE_BYTES: usize = 500 * 1024;

fn group_key(ctx: &Ctx) -> String {
    format!("{KNOWLEDGE_FOLDER_PREFIX}{}", ctx.project_id)
}

fn rag_ctx(ctx: &Ctx, model: &str) -> TurnContext {
    TurnContext {
        model: Some(ctx.wire(model)),
        agent_id: None,
        rag_group_key: Some(group_key(ctx)),
    }
}

fn state_name(s: KnowledgeState) -> &'static str {
    match s {
        KnowledgeState::Sent => "sent",
        KnowledgeState::Reading => "reading",
        KnowledgeState::Indexing => "indexing",
        KnowledgeState::Ready => "ready",
        KnowledgeState::Stale => "stale",
        KnowledgeState::Failed => "failed",
    }
}

/// Upload one local file into the run's knowledge folder; returns the row
/// and the upload duration.
async fn upload(ctx: &Ctx, path: &Path) -> Result<(KnowledgeFile, Duration), String> {
    let started = Instant::now();
    let row = upload_project_file(
        &ctx.base,
        &ctx.key,
        path,
        &ctx.project_id,
        &UploadHints {
            analyze_model: None,
        },
    )
    .await
    .map_err(|e| format!("{e} ({})", e.code()))?;
    Ok((row, started.elapsed()))
}

/// Poll the folder until every id in `ids` is `Ready` or `Failed`, or the
/// ceiling passes. Returns (ready ids with time-to-ready, failed rows).
async fn wait_ready(
    ctx: &Ctx,
    ids: &[i64],
    ceiling: Duration,
) -> (Vec<(i64, Duration)>, Vec<KnowledgeFile>, Vec<KnowledgeFile>) {
    let started = Instant::now();
    let mut ready: Vec<(i64, Duration)> = Vec::new();
    let mut failed: Vec<KnowledgeFile> = Vec::new();
    let mut last: Vec<KnowledgeFile> = Vec::new();
    loop {
        if let Ok(list) = list_project_files(&ctx.base, &ctx.key, &ctx.project_id).await {
            for id in ids {
                if ready.iter().any(|(r, _)| r == id) || failed.iter().any(|f| f.id == *id) {
                    continue;
                }
                if let Some(row) = list.iter().find(|f| f.id == *id) {
                    match row.state {
                        KnowledgeState::Ready => ready.push((*id, started.elapsed())),
                        KnowledgeState::Failed => failed.push(row.clone()),
                        _ => {}
                    }
                }
            }
            last = list;
        }
        if ready.len() + failed.len() == ids.len() || started.elapsed() > ceiling {
            break;
        }
        tokio::time::sleep(POLL_EVERY).await;
    }
    (ready, failed, last)
}

/// Ask `question` in a fresh thread with the knowledge folder pinned and
/// return (answer, matched).
async fn ask(
    ctx: &Ctx,
    model: &str,
    messages: &[ChatMessage],
    expect: &str,
) -> (String, bool, Option<String>) {
    let t = client_turn(ctx, &rag_ctx(ctx, model), messages, None).await;
    let hit = normalize(&t.text).contains(&normalize(expect));
    (t.text, hit, t.error.map(|e| e.to_string()))
}

/// UC-05: one Markdown file, one question, three models — then the traps.
pub async fn uc05_single_file(ctx: &Ctx) -> CaseResult {
    let mut case = Case::new("UC-05", "Drop one file, ask about it");
    let facts = fixtures::facts(&ctx.run_id);
    let fact = &facts[0];
    let dir = ctx.work_dir.join("uc05");

    // UC-05d — the thread exists before the file does. The person asks, gets
    // "I don't have that", drops the file, and asks again in the same thread.
    // The gateway caches the folder lookup per thread (first user text) for
    // two hours, so the second answer is decided before the upload happens.
    let cache_model = crate::with_default_first(ctx, ctx.models.clone())
        .first()
        .cloned()
        .unwrap_or_default();
    let cache_fact = &facts[1];
    let (before_answer, before_hit, _) = ask(
        ctx,
        &cache_model,
        &[user(&cache_fact.question)],
        &cache_fact.expect,
    )
    .await;
    let md = format!(
        "# Nordlicht internal memo ({})\n\n{}\n\n{}\n\nThe memo was written for the desktop use-case runner.\n",
        ctx.run_id, fact.sentence, cache_fact.sentence
    );
    let path = match fixtures::write_file(&dir, &format!("fact-{}.md", ctx.run_id), md.as_bytes()) {
        Ok(p) => p,
        Err(e) => {
            case.check("fixture written", false, e);
            return case.finish("could not write fixture");
        }
    };
    let (row, took) = match upload(ctx, &path).await {
        Ok(v) => v,
        Err(e) => {
            case.check("upload accepted", false, e);
            return case.finish("upload failed");
        }
    };
    case.metric("upload_ms", took.as_millis() as u64);
    case.check(
        "upload accepted",
        true,
        format!("id {} in {} ms", row.id, took.as_millis()),
    );
    let (ready, failed, _) = wait_ready(ctx, &[row.id], SMALL_READY_CEILING).await;
    let ready_ms = ready.first().map(|(_, d)| d.as_millis() as u64);
    case.metric(
        "ready_ms",
        ready_ms.map(|v| json!(v)).unwrap_or(json!(null)),
    );
    if !case.check(
        "file becomes Ready",
        ready.len() == 1,
        match (ready.first(), failed.first()) {
            (Some((_, d)), _) => format!("Ready after {} ms", d.as_millis()),
            (None, Some(f)) => format!("Failed: {}", f.detail.clone().unwrap_or_default()),
            _ => format!("still not Ready after {} s", SMALL_READY_CEILING.as_secs()),
        },
    ) {
        return case.finish("file never became Ready");
    }

    let models: Vec<String> = crate::with_default_first(ctx, ctx.models.clone())
        .into_iter()
        .take(3)
        .collect();
    let mut good = 0usize;
    for model in &models {
        let (answer, hit, err) = ask(ctx, model, &[user(&fact.question)], &fact.expect).await;
        let ok = case.check(
            format!("{model}: answer names the fact"),
            hit && err.is_none(),
            match err {
                Some(e) => format!("error shown: \"{e}\""),
                None => format!("\"{}\"", snippet(&answer, 120)),
            },
        );
        if ok {
            good += 1;
        }
    }

    // UC-05d continued: same thread, file now Ready.
    let cache_thread = [
        user(&cache_fact.question),
        assistant(&before_answer),
        user("I have added the file to the project now. Please check my project files and answer the question."),
    ];
    let (after_answer, after_hit, err) =
        ask(ctx, &cache_model, &cache_thread, &cache_fact.expect).await;
    case.check(
        "UC-05d asked before the file existed, then again in the same thread once Ready",
        after_hit && err.is_none(),
        format!(
            "{cache_model}: before upload hit={before_hit} \"{}\" · after Ready hit={after_hit} \"{}\"",
            snippet(&before_answer, 60),
            snippet(&after_answer, 100)
        ),
    );

    // UC-05c — second-turn question after an unrelated first message. The
    // gateway looks the folder up with the *first* user text of the thread.
    let model = models.first().cloned().unwrap_or_default();
    let thread = [
        user("Hello, I added some files to this project."),
        assistant("Hi! I can see your project. What would you like to know?"),
        user(&fact.question),
    ];
    let (answer, hit, err) = ask(ctx, &model, &thread, &fact.expect).await;
    case.check(
        "UC-05c second-turn question in a thread that started with small talk",
        hit && err.is_none(),
        match err {
            Some(e) => format!("error shown: \"{e}\""),
            None => format!("{model}: \"{}\"", snippet(&answer, 120)),
        },
    );

    // UC-05b — ask before the file is indexed, then again in the same thread.
    let pdf_fact = &facts[3];
    let pdf = fixtures::minimal_pdf(&[
        format!("Nordlicht plant sheet {}", ctx.run_id),
        pdf_fact.sentence.clone(),
        "Generated for the desktop use-case runner.".to_string(),
    ]);
    match fixtures::write_file(&dir, &format!("plant-{}.pdf", ctx.run_id), &pdf) {
        Ok(pdf_path) => match upload(ctx, &pdf_path).await {
            Ok((pdf_row, _)) => {
                let early_thread = [user(&pdf_fact.question)];
                let (early_answer, early_hit, _) =
                    ask(ctx, &model, &early_thread, &pdf_fact.expect).await;
                let early_state = raw::file_row(&ctx.base, &ctx.key, &group_key(ctx), pdf_row.id)
                    .await
                    .and_then(|r| {
                        r.get("vector_state")
                            .and_then(|v| v.as_str())
                            .map(str::to_string)
                    })
                    .unwrap_or_default();
                let (ready, _, _) = wait_ready(ctx, &[pdf_row.id], SMALL_READY_CEILING).await;
                let later_thread = [
                    user(&pdf_fact.question),
                    assistant(&early_answer),
                    user("Please check my project files again and answer the question."),
                ];
                let (later_answer, later_hit, err) =
                    ask(ctx, &model, &later_thread, &pdf_fact.expect).await;
                case.metric("uc05b_pdf_ready", ready.len() as u64);
                case.check(
                    "UC-05b asked too early, then again in the same thread once Ready",
                    later_hit && err.is_none() && ready.len() == 1,
                    format!(
                        "early (vector_state={early_state}): hit={early_hit} \"{}\" · after Ready: hit={later_hit} \"{}\"",
                        snippet(&early_answer, 70),
                        snippet(&later_answer, 90)
                    ),
                );
            }
            Err(e) => {
                case.check("UC-05b pdf upload accepted", false, e);
            }
        },
        Err(e) => {
            case.check("UC-05b pdf fixture written", false, e);
        }
    }

    case.finish(format!(
        "{good}/{} models answered; ready in {} ms",
        models.len(),
        ready_ms.unwrap_or(0)
    ))
}

/// UC-06: a folder of ~5 MB in seven files, indexed en bloc, then three
/// fact questions (big markdown, docx, pdf).
pub async fn uc06_en_bloc(ctx: &Ctx) -> CaseResult {
    let mut case = Case::new("UC-06", "Index a whole folder (megabytes) and ask");
    let facts = fixtures::facts(&ctx.run_id);
    let dir = ctx.work_dir.join("uc06");
    let run = &ctx.run_id;

    // Corpus: one big + four mid Markdown, one docx, one pdf.
    let mut files: Vec<(String, Vec<u8>, Option<Fact>)> = Vec::new();
    let big = fixtures::big_markdown(
        &format!("Nordlicht operations handbook {run}"),
        BIG_FILE_BYTES,
        &facts[..2],
        1,
    );
    files.push((
        format!("handbook-{run}.md"),
        big.into_bytes(),
        Some(facts[0].clone()),
    ));
    for i in 0..4 {
        let md = fixtures::big_markdown(
            &format!("Weekly report {i} {run}"),
            MID_FILE_BYTES,
            &[],
            10 + i as u64,
        );
        files.push((format!("weekly-{i}-{run}.md"), md.into_bytes(), None));
    }
    match fixtures::minimal_docx(&[
        format!("Nordlicht finance note {run}"),
        facts[2].sentence.clone(),
        "This note was generated for the desktop use-case runner.".to_string(),
    ]) {
        Ok(docx) => files.push((format!("finance-{run}.docx"), docx, Some(facts[2].clone()))),
        Err(e) => {
            case.check("docx fixture built", false, e);
        }
    }
    let pdf = fixtures::minimal_pdf(&[
        format!("Nordlicht survey sheet {run}"),
        facts[4].sentence.clone(),
        "Generated for the desktop use-case runner.".to_string(),
    ]);
    files.push((format!("survey-{run}.pdf"), pdf, Some(facts[4].clone())));

    let total_bytes: usize = files.iter().map(|(_, b, _)| b.len()).sum();
    case.metric("corpus_files", files.len() as u64);
    case.metric(
        "corpus_mb",
        format!("{:.2}", total_bytes as f64 / 1_048_576.0),
    );

    // Upload sequentially, like `useKnowledgeFiles.add` does.
    let started = Instant::now();
    let mut ids: Vec<(i64, String, Option<Fact>)> = Vec::new();
    let mut upload_ms_total = 0u64;
    for (name, bytes, fact) in &files {
        let path = match fixtures::write_file(&dir, name, bytes) {
            Ok(p) => p,
            Err(e) => {
                case.check(format!("{name} written locally"), false, e);
                continue;
            }
        };
        match upload(ctx, &path).await {
            Ok((row, took)) => {
                upload_ms_total += took.as_millis() as u64;
                ctx.log(&format!(
                    "uploaded {name} ({} KB) in {} ms → id {}",
                    bytes.len() / 1024,
                    took.as_millis(),
                    row.id
                ));
                ids.push((row.id, name.clone(), fact.clone()));
            }
            Err(e) => {
                case.check(format!("{name} upload accepted"), false, e);
            }
        }
    }
    case.metric("upload_ms_total", upload_ms_total);
    case.check(
        "every file accepted",
        ids.len() == files.len(),
        format!(
            "{}/{} uploaded in {} ms",
            ids.len(),
            files.len(),
            upload_ms_total
        ),
    );

    let only_ids: Vec<i64> = ids.iter().map(|(id, _, _)| *id).collect();
    let (ready, failed, last) = wait_ready(ctx, &only_ids, CORPUS_READY_CEILING).await;
    let all_ready_ms = started.elapsed().as_millis() as u64;
    case.metric("all_ready_ms", all_ready_ms);
    let mb_per_min = total_bytes as f64 / 1_048_576.0 / (all_ready_ms as f64 / 60_000.0);
    case.metric("mb_per_min", format!("{mb_per_min:.2}"));
    for (id, name, _) in &ids {
        let ms = ready
            .iter()
            .find(|(r, _)| r == id)
            .map(|(_, d)| d.as_millis() as u64);
        case.metric(
            format!("ready_ms {name}"),
            ms.map(|v| json!(v)).unwrap_or(json!(null)),
        );
    }
    let stuck: Vec<String> = ids
        .iter()
        .filter(|(id, _, _)| {
            !ready.iter().any(|(r, _)| r == id) && !failed.iter().any(|f| f.id == *id)
        })
        .map(|(id, name, _)| {
            let state = last
                .iter()
                .find(|f| f.id == *id)
                .map(|f| state_name(f.state))
                .unwrap_or("missing");
            format!("{name}={state}")
        })
        .collect();
    case.check(
        "every file Ready within the ceiling",
        ready.len() == ids.len(),
        format!(
            "{} ready, {} failed [{}], {} still processing [{}] after {:.1} s ({mb_per_min:.2} MB/min)",
            ready.len(),
            failed.len(),
            failed
                .iter()
                .map(|f| format!("{}: {}", f.name, f.detail.clone().unwrap_or_default()))
                .collect::<Vec<_>>()
                .join("; "),
            stuck.len(),
            stuck.join(", "),
            all_ready_ms as f64 / 1000.0
        ),
    );
    case.check(
        "indexing throughput ≥ 1 MB/min",
        mb_per_min >= 1.0 || ready.len() < ids.len(),
        format!(
            "{mb_per_min:.2} MB/min for {:.2} MB",
            total_bytes as f64 / 1_048_576.0
        ),
    );

    // Three fact questions on the default model.
    let model = crate::with_default_first(ctx, ctx.models.clone())
        .first()
        .cloned()
        .unwrap_or_default();
    let mut hits = 0usize;
    let mut asked = 0usize;
    for (id, name, fact) in &ids {
        let Some(fact) = fact else {
            continue;
        };
        if !ready.iter().any(|(r, _)| r == id) {
            continue;
        }
        asked += 1;
        let (answer, hit, err) = ask(ctx, &model, &[user(&fact.question)], &fact.expect).await;
        if hit {
            hits += 1;
        }
        case.check(
            format!("fact from {name} answered"),
            hit && err.is_none(),
            match err {
                Some(e) => format!("error shown: \"{e}\""),
                None => format!("{model}: \"{}\"", snippet(&answer, 120)),
            },
        );
    }
    case.metric("facts_hit", format!("{hits}/{asked}"));
    case.finish(format!(
        "{} files ({:.2} MB) ready in {:.1} s, {hits}/{asked} facts answered",
        ready.len(),
        total_bytes as f64 / 1_048_576.0,
        all_ready_ms as f64 / 1000.0
    ))
}

/// Remove every file the run put into the workspace.
pub async fn cleanup(ctx: &Ctx) {
    let Ok(list) = list_project_files(&ctx.base, &ctx.key, &ctx.project_id).await else {
        return;
    };
    let mut removed = 0usize;
    for f in &list {
        if delete_project_file(&ctx.base, &ctx.key, f.id).await.is_ok() {
            removed += 1;
        }
    }
    if !list.is_empty() {
        println!(
            "\ncleanup: removed {removed}/{} harness files from the workspace",
            list.len()
        );
    }
    let _ = std::fs::remove_dir_all(&ctx.work_dir);
}
