//! Dictation over the workspace's `/v1/audio/*` routes.
//!
//! The webview captures audio; every HTTP call is made here so the API key never
//! reaches JavaScript. Two paths run per take: a live PCM session for interim
//! text (the client decides when a phrase is complete — the server's own 3 s
//! auto-commit would cut mid-word, so `commit_after_bytes` is raised to 15 s as
//! a cap) and a one-shot transcription of the whole recording, which the UI
//! prefers when it stops. Model, language and prompt are the **project's** on
//! every call (C15): the project's Dictation binding, its dictation language,
//! and a generic note-taking prompt the UI supplies from its locale files.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

use crate::http;
use crate::projects::{wire_model_id, Project};

/// PCM the session expects: 16 kHz, mono, 16-bit little-endian.
pub const SAMPLE_RATE: u32 = 16_000;
pub const CHANNELS: u32 = 1;
pub const ENCODING: &str = "pcm_s16le";
/// 16 kHz * 2 bytes * 15 s — the server may only auto-commit when a speaker
/// never pauses; phrase boundaries are decided by the client.
pub const COMMIT_AFTER_BYTES: u32 = SAMPLE_RATE * 2 * 15;

/// What every dictation call carries; built from the project, never from a
/// UI locale or an account default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictationContext {
    /// Wire model id of the project's Dictation (VOICE) binding.
    pub model: String,
    /// ISO language from the project's `dictation_language`.
    pub language: String,
    /// Generic note-taking prompt in the user's language (from i18n).
    pub prompt: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DictationError {
    #[error("No Dictation model is set for this project yet.")]
    VoiceUnset,
    #[error("This computer was disconnected. Pair again.")]
    Unauthorized,
    #[error("Could not reach Synaplan. Check your connection.")]
    Network,
    #[error("The workspace does not know the Dictation model \"{0}\".")]
    ModelUnknown(String),
    #[error("The dictation session is gone. Start again.")]
    SessionGone,
    #[error("{0}")]
    Server(String),
}

impl DictationError {
    pub fn code(&self) -> &'static str {
        match self {
            DictationError::VoiceUnset => "voice_model_unset",
            DictationError::Unauthorized => "unauthorized",
            DictationError::Network => "network",
            DictationError::ModelUnknown(_) => "voice_model_unknown",
            DictationError::SessionGone => "dictation_session_gone",
            DictationError::Server(_) => "server",
        }
    }
}

/// The project's dictation context, or `VoiceUnset` when the Dictation slot is
/// empty — the mic must not open then.
pub fn dictation_context(
    project: &Project,
    prompt: &str,
) -> Result<DictationContext, DictationError> {
    let model = wire_model_id(&project.models.voice, None).ok_or(DictationError::VoiceUnset)?;
    let language = project.dictation_language.trim();
    Ok(DictationContext {
        model,
        language: if language.is_empty() {
            "en".to_string()
        } else {
            language.to_string()
        },
        prompt: prompt.trim().to_string(),
    })
}

/// JSON body for `POST /v1/audio/transcriptions/sessions`.
pub fn session_body(ctx: &DictationContext, client_id: &str) -> Value {
    json!({
        "client_id": client_id,
        "model": ctx.model,
        "language": ctx.language,
        "prompt": ctx.prompt,
        "encoding": ENCODING,
        "sample_rate": SAMPLE_RATE,
        "channels": CHANNELS,
        "commit_after_bytes": COMMIT_AFTER_BYTES,
    })
}

/// Text fields of the one-shot `POST /v1/audio/transcriptions` form.
pub fn one_shot_fields(ctx: &DictationContext) -> Vec<(&'static str, String)> {
    vec![
        ("model", ctx.model.clone()),
        ("language", ctx.language.clone()),
        ("prompt", ctx.prompt.clone()),
        ("response_format", "json".to_string()),
    ]
}

/// Server answers carry the text under `text`; a new session reports its `id`.
/// Everything else is ignored.
#[derive(Debug, Deserialize)]
struct SessionResponse {
    #[serde(default)]
    id: String,
}

#[derive(Debug, Deserialize)]
struct TextResponse {
    #[serde(default)]
    text: String,
}

fn status_error(status: u16, body: &str, model: &str) -> DictationError {
    match status {
        401 => DictationError::Unauthorized,
        404 if body.contains("model") => DictationError::ModelUnknown(model.to_string()),
        404 | 410 => DictationError::SessionGone,
        _ => DictationError::Server(format!("status {status}")),
    }
}

fn request(
    client: &reqwest::Client,
    method: reqwest::Method,
    base_url: &str,
    key: &str,
    path: &str,
) -> reqwest::RequestBuilder {
    client
        .request(method, http::join(base_url, path))
        .header("x-api-key", key)
        .header("Accept", "application/json")
}

async fn read(resp: reqwest::Response) -> Result<(u16, String), DictationError> {
    let status = resp.status().as_u16();
    let body = resp.text().await.map_err(|_| DictationError::Network)?;
    Ok((status, body))
}

/// Open a live session; returns its id (an opaque handle, safe for the webview).
pub async fn open_session(
    base_url: &str,
    key: &str,
    ctx: &DictationContext,
    client_id: &str,
) -> Result<String, DictationError> {
    let client = http::client().map_err(|_| DictationError::Network)?;
    let resp = request(
        &client,
        reqwest::Method::POST,
        base_url,
        key,
        "/v1/audio/transcriptions/sessions",
    )
    .json(&session_body(ctx, client_id))
    .send()
    .await
    .map_err(|_| DictationError::Network)?;
    let (status, body) = read(resp).await?;
    if status != 200 && status != 201 {
        return Err(status_error(status, &body, &ctx.model));
    }
    let parsed: SessionResponse =
        serde_json::from_str(&body).map_err(|e| DictationError::Server(e.to_string()))?;
    if parsed.id.is_empty() {
        return Err(DictationError::Server("session without id".into()));
    }
    Ok(parsed.id)
}

/// Append raw PCM; `commit` asks the server to transcribe what it holds.
/// Empty chunks are never sent.
pub async fn append_chunk(
    base_url: &str,
    key: &str,
    session_id: &str,
    pcm: Vec<u8>,
    commit: bool,
) -> Result<(), DictationError> {
    if pcm.is_empty() {
        return Ok(());
    }
    let client = http::client().map_err(|_| DictationError::Network)?;
    let path = format!(
        "/v1/audio/transcriptions/sessions/{session_id}/audio?commit={}",
        if commit { "true" } else { "false" }
    );
    let resp = request(&client, reqwest::Method::POST, base_url, key, &path)
        .header("Content-Type", "application/octet-stream")
        .body(pcm)
        .send()
        .await
        .map_err(|_| DictationError::Network)?;
    let (status, body) = read(resp).await?;
    // 409 = a commit is already running; the audio was still appended.
    if (200..300).contains(&status) || status == 409 {
        return Ok(());
    }
    Err(status_error(status, &body, ""))
}

/// The text recognised so far.
pub async fn session_text(
    base_url: &str,
    key: &str,
    session_id: &str,
) -> Result<String, DictationError> {
    let client = http::client().map_err(|_| DictationError::Network)?;
    let path = format!("/v1/audio/transcriptions/sessions/{session_id}");
    let resp = request(&client, reqwest::Method::GET, base_url, key, &path)
        .send()
        .await
        .map_err(|_| DictationError::Network)?;
    let (status, body) = read(resp).await?;
    if status != 200 {
        return Err(status_error(status, &body, ""));
    }
    let parsed: TextResponse =
        serde_json::from_str(&body).map_err(|e| DictationError::Server(e.to_string()))?;
    Ok(parsed.text.trim().to_string())
}

/// Transcribe whatever the session still holds and return the full live text.
pub async fn commit_session(
    base_url: &str,
    key: &str,
    session_id: &str,
) -> Result<String, DictationError> {
    let client = http::client().map_err(|_| DictationError::Network)?;
    let path = format!("/v1/audio/transcriptions/sessions/{session_id}/commit");
    let resp = request(&client, reqwest::Method::POST, base_url, key, &path)
        .send()
        .await
        .map_err(|_| DictationError::Network)?;
    let (status, body) = read(resp).await?;
    if status == 200 {
        if let Ok(parsed) = serde_json::from_str::<TextResponse>(&body) {
            return Ok(parsed.text.trim().to_string());
        }
    }
    session_text(base_url, key, session_id).await
}

/// Best effort; a session that cannot be closed expires on the server.
pub async fn close_session(base_url: &str, key: &str, session_id: &str) {
    if let Ok(client) = http::client() {
        let path = format!("/v1/audio/transcriptions/sessions/{session_id}");
        let _ = request(&client, reqwest::Method::DELETE, base_url, key, &path)
            .send()
            .await;
    }
}

/// One-shot transcription of a whole recording (`audio/webm` or similar).
pub async fn transcribe(
    base_url: &str,
    key: &str,
    ctx: &DictationContext,
    audio: Vec<u8>,
    mime: &str,
) -> Result<String, DictationError> {
    if audio.is_empty() {
        return Ok(String::new());
    }
    let client = http::client().map_err(|_| DictationError::Network)?;
    let mime = if mime.trim().is_empty() {
        "audio/webm"
    } else {
        mime
    };
    let extension = mime.split('/').nth(1).unwrap_or("webm");
    let extension = extension.split(';').next().unwrap_or("webm");
    let part = reqwest::multipart::Part::bytes(audio)
        .file_name(format!("take.{extension}"))
        .mime_str(mime)
        .map_err(|e| DictationError::Server(e.to_string()))?;
    let mut form = reqwest::multipart::Form::new().part("file", part);
    for (field, value) in one_shot_fields(ctx) {
        form = form.text(field, value);
    }
    let resp = request(
        &client,
        reqwest::Method::POST,
        base_url,
        key,
        "/v1/audio/transcriptions",
    )
    .multipart(form)
    .send()
    .await
    .map_err(|_| DictationError::Network)?;
    let (status, body) = read(resp).await?;
    if status != 200 {
        return Err(status_error(status, &body, &ctx.model));
    }
    let parsed: TextResponse =
        serde_json::from_str(&body).map_err(|e| DictationError::Server(e.to_string()))?;
    Ok(parsed.text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projects::{ProjectKind, ProjectModels};

    fn project(voice: &str, language: &str) -> Project {
        Project {
            id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".into(),
            slug: "work".into(),
            name: "Work".into(),
            kind: ProjectKind::Project,
            created_at: String::new(),
            updated_at: String::new(),
            dictation_language: language.into(),
            default_assistant_id: None,
            assistant_ids: vec![],
            enabled_skills: vec![],
            models: ProjectModels {
                voice: voice.into(),
                ..Default::default()
            },
            knowledge_folder: "DESKTOP:01ARZ3NDEKTSV4RRFFQ69G5FAV".into(),
        }
    }

    #[test]
    fn session_body_carries_project_model_language_prompt_and_the_15s_cap() {
        let ctx =
            dictation_context(&project("openai:whisper-1:sound2text", "de"), "Notes.").unwrap();
        let body = session_body(&ctx, "desktop-take-1");
        assert_eq!(body["model"], "whisper-1");
        assert_eq!(body["language"], "de");
        assert_eq!(body["prompt"], "Notes.");
        assert_eq!(body["encoding"], "pcm_s16le");
        assert_eq!(body["sample_rate"], 16_000);
        assert_eq!(body["channels"], 1);
        assert_eq!(body["commit_after_bytes"], 480_000);
        assert_eq!(body["client_id"], "desktop-take-1");
    }

    #[test]
    fn one_shot_fields_match_the_session() {
        let ctx = dictation_context(&project("openai:whisper-1:sound2text", "fr"), "P").unwrap();
        let fields = one_shot_fields(&ctx);
        assert!(fields.contains(&("model", "whisper-1".to_string())));
        assert!(fields.contains(&("language", "fr".to_string())));
        assert!(fields.contains(&("prompt", "P".to_string())));
    }

    #[test]
    fn an_unset_voice_slot_refuses_before_any_request() {
        assert_eq!(
            dictation_context(&project("", "en"), "P").unwrap_err(),
            DictationError::VoiceUnset
        );
    }

    #[test]
    fn an_empty_language_falls_back_to_english_not_the_ui_locale() {
        let ctx = dictation_context(&project("ollama:whisper:sound2text", "  "), "P").unwrap();
        assert_eq!(ctx.language, "en");
    }

    #[test]
    fn errors_map_to_stable_codes_and_never_echo_the_key() {
        assert_eq!(status_error(401, "", "m").code(), "unauthorized");
        assert_eq!(
            status_error(404, r#"{"error":{"message":"model not found"}}"#, "m"),
            DictationError::ModelUnknown("m".into())
        );
        assert_eq!(status_error(404, "", "m"), DictationError::SessionGone);
        let msg = status_error(500, "sk_secret", "m").to_string();
        assert!(!msg.contains("sk_"));
    }
}
