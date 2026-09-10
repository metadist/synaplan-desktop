//! Files sent to Synaplan through `POST /api/v1/files/upload` (`desktop:files`).
//!
//! Two very different uploads share the route:
//!
//! - [`upload_store`] — a skill's artifact ("here is the pptx"); `process_level=store`,
//!   no knowledge folder.
//! - [`upload_project_file`] — a file that belongs to a project and should be
//!   searchable in that project's chat: `group_key = DESKTOP:{projectId}`,
//!   `process_level=vectorize`, and **always** the project's own index model as
//!   `vectorize_model` (plus `analyze_model` when the project set one). Sending
//!   the file without the hint would let the account default index it, which
//!   is exactly the leak the Models panel promises not to have (C16).

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::http;
use crate::projects::{ModelSlot, ProjectModels, KNOWLEDGE_FOLDER_PREFIX};

/// Files above this are refused before any bytes are read.
pub const MAX_UPLOAD_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FilesError {
    #[error("could not reach Synaplan")]
    Network,
    #[error("this computer was disconnected")]
    Unauthorized,
    #[error("Pick a model for indexing files before adding files to this project.")]
    EmbedUnset,
    #[error("the file could not be read: {0}")]
    Unreadable(String),
    #[error("the file is larger than the upload limit")]
    TooLarge,
    #[error("{0}")]
    Server(String),
}

impl FilesError {
    pub fn code(&self) -> &'static str {
        match self {
            FilesError::Network => "network",
            FilesError::Unauthorized => "unauthorized",
            FilesError::EmbedUnset => "embed_model_unset",
            FilesError::Unreadable(_) => "file_unreadable",
            FilesError::TooLarge => "file_too_large",
            FilesError::Server(_) => "server",
        }
    }
}

/// The model hints a project upload carries. Built only from the project's own
/// bindings; never from an account default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadHints {
    pub vectorize_model: String,
    pub analyze_model: Option<String>,
}

impl UploadHints {
    /// Refuses when the project has no index model: a vectorize upload without
    /// the hint would be indexed by whatever the account defaults to.
    pub fn from_models(models: &ProjectModels) -> Result<Self, FilesError> {
        let embed = models.get(ModelSlot::Embed);
        if embed.is_empty() {
            return Err(FilesError::EmbedUnset);
        }
        let docs = models.get(ModelSlot::Docs);
        Ok(Self {
            vectorize_model: embed.to_string(),
            analyze_model: if docs.is_empty() {
                None
            } else {
                Some(docs.to_string())
            },
        })
    }
}

/// The text fields of a project upload, in a shape a test can read.
pub fn project_upload_fields(project_id: &str, hints: &UploadHints) -> Vec<(&'static str, String)> {
    let mut fields = vec![
        ("process_level", "vectorize".to_string()),
        ("source", "api".to_string()),
        ("file_count", "1".to_string()),
        (
            "group_key",
            format!("{KNOWLEDGE_FOLDER_PREFIX}{project_id}"),
        ),
        ("vectorize_model", hints.vectorize_model.clone()),
    ];
    if let Some(analyze) = &hints.analyze_model {
        fields.push(("analyze_model", analyze.clone()));
    }
    fields
}

/// What the Files view shows per uploaded file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeFile {
    pub id: i64,
    pub name: String,
    pub size: u64,
    pub state: KnowledgeState,
    /// Plain-language reason when `state` is `Failed`, if the server gave one.
    pub detail: Option<String>,
    pub uploaded_at: String,
}

/// Plain-language lifecycle of a file in the knowledge folder (`07` §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeState {
    /// Uploaded, nothing processed yet.
    Sent,
    /// Text is being extracted.
    Reading,
    /// Being embedded with this project's index model.
    Indexing,
    /// Searchable in chat.
    Ready,
    /// The source changed since it was indexed.
    Stale,
    Failed,
}

/// Map the server's `status` + `vector_state` pair onto the UI lifecycle.
pub fn knowledge_state(status: &str, vector_state: &str) -> KnowledgeState {
    match (status, vector_state) {
        ("error", _) | (_, "failed") => KnowledgeState::Failed,
        (_, "stale") => KnowledgeState::Stale,
        (_, "vectorized") | ("vectorized", _) => KnowledgeState::Ready,
        ("vectorizing", _) | (_, "pending") => KnowledgeState::Indexing,
        ("extracting", _) => KnowledgeState::Reading,
        _ => KnowledgeState::Sent,
    }
}

/// Parse one row of `GET /api/v1/files`.
pub fn parse_knowledge_file(row: &Value) -> Option<KnowledgeFile> {
    let id = row.get("id").and_then(Value::as_i64)?;
    let pick = |k: &str| {
        row.get(k)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let name = pick("display_name")
        .or_else(|| pick("original_name"))
        .or_else(|| pick("filename"))
        .unwrap_or_else(|| format!("file-{id}"));
    let status = pick("status").unwrap_or_default();
    let vector_state = pick("vector_state").unwrap_or_default();
    let state = knowledge_state(&status, &vector_state);
    let uploaded_at = row
        .get("uploaded_at")
        .and_then(Value::as_u64)
        .map(crate::projects::ids::iso8601_from_unix)
        .or_else(|| pick("uploaded_date"))
        .unwrap_or_default();
    Some(KnowledgeFile {
        id,
        name,
        size: row.get("file_size").and_then(Value::as_u64).unwrap_or(0),
        state,
        detail: if state == KnowledgeState::Failed {
            pick("error").or_else(|| pick("error_message"))
        } else {
            None
        },
        uploaded_at,
    })
}

/// Parse the `GET /api/v1/files` body into rows, newest first.
pub fn parse_knowledge_list(body: &str) -> Result<Vec<KnowledgeFile>, FilesError> {
    let value: Value = serde_json::from_str(body).map_err(|e| FilesError::Server(e.to_string()))?;
    let rows = value
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| FilesError::Server("file list had no files".into()))?;
    let mut out: Vec<KnowledgeFile> = rows.iter().filter_map(parse_knowledge_file).collect();
    out.sort_by(|a, b| b.uploaded_at.cmp(&a.uploaded_at).then(b.id.cmp(&a.id)));
    Ok(out)
}

fn read_for_upload(path: &Path) -> Result<(Vec<u8>, String), FilesError> {
    let meta = std::fs::metadata(path).map_err(|e| FilesError::Unreadable(e.to_string()))?;
    if !meta.is_file() {
        return Err(FilesError::Unreadable("not a regular file".into()));
    }
    if meta.len() > MAX_UPLOAD_BYTES {
        return Err(FilesError::TooLarge);
    }
    let bytes = std::fs::read(path).map_err(|e| FilesError::Unreadable(e.to_string()))?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file.bin".into());
    Ok((bytes, name))
}

async fn post_form(
    base_url: &str,
    key: &str,
    form: reqwest::multipart::Form,
) -> Result<Value, FilesError> {
    let client = http::client().map_err(|_| FilesError::Network)?;
    let url = http::join(base_url, "/api/v1/files/upload");
    let resp = client
        .post(url)
        .header("x-api-key", key)
        .multipart(form)
        .send()
        .await
        .map_err(|_| FilesError::Network)?;
    let status = resp.status();
    if status.as_u16() == 401 {
        return Err(FilesError::Unauthorized);
    }
    let body: Value = resp.json().await.unwrap_or(Value::Null);
    // 206 = partial success; a single-file upload that failed lands here too.
    if !status.is_success() && status.as_u16() != 206 {
        let msg = body
            .get("error")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("upload HTTP {status}"));
        return Err(FilesError::Server(msg));
    }
    Ok(body)
}

/// Upload `path` as `process_level=store`. Returns the numeric file id.
pub async fn upload_store(base_url: &str, key: &str, path: &Path) -> Result<i64, FilesError> {
    let bytes = std::fs::read(path).map_err(|e| FilesError::Server(e.to_string()))?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "artifact.bin".into());
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(name)
        .mime_str("application/octet-stream")
        .map_err(|e| FilesError::Server(e.to_string()))?;
    let form = reqwest::multipart::Form::new()
        .part("files[]", part)
        .text("process_level", "store")
        .text("source", "api")
        .text("file_count", "1");

    let body = post_form(base_url, key, form).await?;
    extract_file_id(&body).ok_or_else(|| FilesError::Server("upload response had no id".into()))
}

/// Upload `path` into a project's knowledge folder, indexed with the project's
/// own models. Returns the row as the list would show it.
pub async fn upload_project_file(
    base_url: &str,
    key: &str,
    path: &Path,
    project_id: &str,
    hints: &UploadHints,
) -> Result<KnowledgeFile, FilesError> {
    let (bytes, name) = read_for_upload(path)?;
    let size = bytes.len() as u64;
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(name.clone())
        .mime_str("application/octet-stream")
        .map_err(|e| FilesError::Server(e.to_string()))?;
    let mut form = reqwest::multipart::Form::new().part("files[]", part);
    for (field, value) in project_upload_fields(project_id, hints) {
        form = form.text(field, value);
    }

    let body = post_form(base_url, key, form).await?;
    if let Some(err) = first_upload_error(&body) {
        return Err(FilesError::Server(err));
    }
    let id = extract_file_id(&body)
        .ok_or_else(|| FilesError::Server("upload response had no id".into()))?;
    Ok(KnowledgeFile {
        id,
        name,
        size,
        state: KnowledgeState::Sent,
        detail: None,
        uploaded_at: crate::projects::ids::now_iso8601(),
    })
}

/// List the files in a project's knowledge folder.
pub async fn list_project_files(
    base_url: &str,
    key: &str,
    project_id: &str,
) -> Result<Vec<KnowledgeFile>, FilesError> {
    let client = http::client().map_err(|_| FilesError::Network)?;
    let url = http::join(base_url, "/api/v1/files");
    let resp = client
        .get(url)
        .query(&[
            (
                "group_key",
                format!("{KNOWLEDGE_FOLDER_PREFIX}{project_id}"),
            ),
            ("limit", "100".to_string()),
        ])
        .header("x-api-key", key)
        .send()
        .await
        .map_err(|_| FilesError::Network)?;
    match resp.status().as_u16() {
        200 => {
            let body = resp.text().await.map_err(|_| FilesError::Network)?;
            parse_knowledge_list(&body)
        }
        401 => Err(FilesError::Unauthorized),
        other => Err(FilesError::Server(format!("file list HTTP {other}"))),
    }
}

/// Remove a file from the workspace (`DELETE /api/v1/files/{id}`).
pub async fn delete_project_file(base_url: &str, key: &str, id: i64) -> Result<(), FilesError> {
    let client = http::client().map_err(|_| FilesError::Network)?;
    let url = http::join(base_url, &format!("/api/v1/files/{id}"));
    let resp = client
        .delete(url)
        .header("x-api-key", key)
        .send()
        .await
        .map_err(|_| FilesError::Network)?;
    match resp.status().as_u16() {
        200 | 204 => Ok(()),
        401 => Err(FilesError::Unauthorized),
        404 => Ok(()),
        other => Err(FilesError::Server(format!("delete HTTP {other}"))),
    }
}

fn first_upload_error(body: &Value) -> Option<String> {
    let errors = body.get("errors").and_then(Value::as_array)?;
    let first = errors.first()?;
    first
        .get("error")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| Some("the file was not accepted".to_string()))
}

fn extract_file_id(body: &Value) -> Option<i64> {
    if let Some(id) = body.get("id").and_then(Value::as_i64) {
        return Some(id);
    }
    body.get("files")
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .and_then(|f| f.get("id").and_then(Value::as_i64))
        .or_else(|| {
            body.get("data")
                .and_then(Value::as_array)
                .and_then(|arr| arr.first())
                .and_then(|f| f.get("id").and_then(Value::as_i64))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_id_from_several_shapes() {
        assert_eq!(extract_file_id(&json!({"id": 42})), Some(42));
        assert_eq!(
            extract_file_id(&json!({"files": [{"id": 7, "filename": "a.pptx"}]})),
            Some(7)
        );
        assert_eq!(extract_file_id(&json!({"data": [{"id": 9}]})), Some(9));
        assert_eq!(extract_file_id(&json!({})), None);
    }

    fn models(embed: &str, docs: &str) -> ProjectModels {
        ProjectModels {
            embed: embed.to_string(),
            docs: docs.to_string(),
            ..ProjectModels::default()
        }
    }

    #[test]
    fn project_upload_carries_folder_level_and_the_project_index_model() {
        let hints = UploadHints::from_models(&models("ollama:bge-m3:vectorize", "")).unwrap();
        let fields = project_upload_fields("01J0PROJECT", &hints);
        let get = |k: &str| {
            fields
                .iter()
                .find(|(f, _)| *f == k)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(get("group_key"), Some("DESKTOP:01J0PROJECT"));
        assert_eq!(get("process_level"), Some("vectorize"));
        assert_eq!(get("vectorize_model"), Some("ollama:bge-m3:vectorize"));
        assert_eq!(get("source"), Some("api"));
        assert_eq!(
            get("analyze_model"),
            None,
            "unset DOCS is omitted, not defaulted"
        );
    }

    #[test]
    fn docs_binding_travels_as_the_analyze_hint() {
        let hints =
            UploadHints::from_models(&models("ollama:bge-m3:vectorize", "openai:gpt-4o:analyze"))
                .unwrap();
        let fields = project_upload_fields("p", &hints);
        assert!(fields.contains(&("analyze_model", "openai:gpt-4o:analyze".to_string())));
    }

    #[test]
    fn embed_unset_refuses_before_any_upload() {
        assert_eq!(
            UploadHints::from_models(&models("", "openai:gpt-4o:analyze")),
            Err(FilesError::EmbedUnset)
        );
        assert_eq!(FilesError::EmbedUnset.code(), "embed_model_unset");
    }

    #[test]
    fn server_states_become_plain_lifecycle_steps() {
        assert_eq!(knowledge_state("uploaded", "none"), KnowledgeState::Sent);
        assert_eq!(
            knowledge_state("extracting", "none"),
            KnowledgeState::Reading
        );
        assert_eq!(
            knowledge_state("extracted", "pending"),
            KnowledgeState::Indexing
        );
        assert_eq!(
            knowledge_state("vectorizing", "none"),
            KnowledgeState::Indexing
        );
        assert_eq!(
            knowledge_state("vectorized", "vectorized"),
            KnowledgeState::Ready
        );
        assert_eq!(
            knowledge_state("vectorized", "stale"),
            KnowledgeState::Stale
        );
        assert_eq!(knowledge_state("error", "none"), KnowledgeState::Failed);
        assert_eq!(
            knowledge_state("extracted", "failed"),
            KnowledgeState::Failed
        );
    }

    #[test]
    fn list_rows_are_parsed_and_sorted_newest_first() {
        let body = json!({
            "files": [
                {"id": 1, "filename": "a.pdf", "display_name": "Quarterly report", "file_size": 1200, "status": "vectorized", "vector_state": "vectorized", "uploaded_at": 1_789_000_000},
                {"id": 2, "filename": "b.txt", "file_size": 5, "status": "error", "vector_state": "failed", "uploaded_at": 1_789_000_100, "error": "Text extraction failed"},
                {"filename": "no-id.txt"}
            ],
            "pagination": {"page": 1}
        })
        .to_string();
        let rows = parse_knowledge_list(&body).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, 2);
        assert_eq!(rows[0].state, KnowledgeState::Failed);
        assert_eq!(rows[0].detail.as_deref(), Some("Text extraction failed"));
        assert_eq!(rows[1].name, "Quarterly report");
        assert_eq!(rows[1].state, KnowledgeState::Ready);
        assert_eq!(rows[1].uploaded_at, "2026-09-10T00:26:40Z");
        assert!(matches!(
            parse_knowledge_list("{}"),
            Err(FilesError::Server(_))
        ));
    }

    #[test]
    fn a_rejected_upload_surfaces_the_server_reason() {
        let body = json!({"success": false, "files": [], "errors": [{"filename": "x", "error": "Unsupported type"}]});
        assert_eq!(
            first_upload_error(&body).as_deref(),
            Some("Unsupported type")
        );
        assert_eq!(first_upload_error(&json!({"errors": []})), None);
    }
}
