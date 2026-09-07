//! Upload a local artifact through `POST /api/v1/files/upload` (`desktop:files`).

use std::path::Path;

use serde_json::Value;
use thiserror::Error;

use crate::http;

#[derive(Debug, Error)]
pub enum FilesError {
    #[error("could not reach Synaplan")]
    Network,
    #[error("this computer was disconnected")]
    Unauthorized,
    #[error("{0}")]
    Server(String),
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
    if !status.is_success() {
        return Err(FilesError::Server(format!("upload HTTP {status}")));
    }
    let body: Value = resp.json().await.map_err(|_| FilesError::Network)?;
    extract_file_id(&body).ok_or_else(|| FilesError::Server("upload response had no id".into()))
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
}
