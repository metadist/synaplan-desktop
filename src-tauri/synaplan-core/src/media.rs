//! Generated image / video / speech over the desktop `/v1` machine API.

use std::time::Duration;

use serde_json::Value;
use thiserror::Error;

use crate::http;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MediaError {
    #[error("could not reach Synaplan")]
    Network,
    #[error("this computer was disconnected")]
    Unauthorized,
    #[error("{0}")]
    Server(String),
}

impl MediaError {
    pub fn code(&self) -> &'static str {
        match self {
            MediaError::Network => "network",
            MediaError::Unauthorized => "unauthorized",
            MediaError::Server(_) => "generation_failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedRemote {
    pub url: String,
    pub mime: String,
    pub kind: String,
    pub file_id: Option<i64>,
}

pub async fn generate_media(
    base_url: &str,
    key: &str,
    prompt: &str,
    kind: &str,
    model: &str,
) -> Result<GeneratedRemote, MediaError> {
    let body = serde_json::json!({
        "prompt": prompt,
        "type": kind,
        "model": model,
    });
    post_json(base_url, key, "/v1/media/generate", &body).await
}

pub async fn generate_speech(
    base_url: &str,
    key: &str,
    text: &str,
    model: &str,
) -> Result<GeneratedRemote, MediaError> {
    let body = serde_json::json!({
        "text": text,
        "model": model,
    });
    post_json(base_url, key, "/v1/audio/speech", &body).await
}

pub async fn download_bytes(base_url: &str, key: &str, url: &str) -> Result<Vec<u8>, MediaError> {
    let client = long_client()?;
    let absolute = if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        http::join(base_url, url)
    };
    let resp = client
        .get(absolute)
        .header("x-api-key", key)
        .send()
        .await
        .map_err(|_| MediaError::Network)?;
    match resp.status().as_u16() {
        200 => resp
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|_| MediaError::Network),
        401 => Err(MediaError::Unauthorized),
        other => Err(MediaError::Server(format!("download HTTP {other}"))),
    }
}

async fn post_json(
    base_url: &str,
    key: &str,
    path: &str,
    body: &Value,
) -> Result<GeneratedRemote, MediaError> {
    let client = long_client()?;
    let resp = client
        .post(http::join(base_url, path))
        .header("x-api-key", key)
        .json(body)
        .send()
        .await
        .map_err(|_| MediaError::Network)?;
    let status = resp.status().as_u16();
    let value: Value = resp.json().await.unwrap_or(Value::Null);
    match status {
        200 => parse_generated(&value)
            .ok_or_else(|| MediaError::Server("generation had no file".into())),
        401 => Err(MediaError::Unauthorized),
        _ => Err(MediaError::Server(error_message(&value, status))),
    }
}

fn parse_generated(value: &Value) -> Option<GeneratedRemote> {
    let file = value.get("file")?;
    let url = file.get("url")?.as_str()?.to_string();
    if url.is_empty() {
        return None;
    }
    Some(GeneratedRemote {
        url,
        mime: file
            .get("mimeType")
            .or_else(|| file.get("mime_type"))
            .and_then(Value::as_str)
            .unwrap_or("application/octet-stream")
            .to_string(),
        kind: file
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("file")
            .to_string(),
        file_id: file.get("id").and_then(Value::as_i64),
    })
}

fn error_message(body: &Value, status: u16) -> String {
    body.get("error")
        .and_then(|e| {
            e.as_str()
                .map(str::to_string)
                .or_else(|| e.get("message").and_then(Value::as_str).map(str::to_string))
        })
        .unwrap_or_else(|| format!("generation HTTP {status}"))
}

fn long_client() -> Result<reqwest::Client, MediaError> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(180))
        .user_agent(concat!("SynaplanDesktop/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|_| MediaError::Network)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_desktop_generate_shape() {
        let remote = parse_generated(&json!({
            "success": true,
            "file": {
                "url": "/api/v1/files/uploads/a.png",
                "type": "image",
                "mimeType": "image/png",
                "id": 9
            }
        }))
        .unwrap();
        assert_eq!(remote.url, "/api/v1/files/uploads/a.png");
        assert_eq!(remote.kind, "image");
        assert_eq!(remote.file_id, Some(9));
    }

    #[test]
    fn reads_nested_or_flat_errors() {
        assert_eq!(
            error_message(&json!({"error": "Unknown model"}), 400),
            "Unknown model"
        );
        assert_eq!(
            error_message(
                &json!({"error": {"message": "Authentication required"}}),
                401
            ),
            "Authentication required"
        );
    }
}
