//! Streaming chat against the Anthropic-compatible Messages gateway
//! (`POST /v1/messages`, `stream: true`) and model discovery (`GET /v1/models`).
//! The account default model is used unless the caller picks one — never a
//! hardcoded `claude-*` id.

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::http;
use crate::sse::{ChatEvent, SseParser};

/// A single chat message in the conversation sent to the gateway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ChatError {
    #[error("This computer was disconnected. Pair again.")]
    Unauthorized,
    #[error("Desktop access is turned off.")]
    FeatureDisabled,
    #[error("The AI gateway is turned off on this Synaplan instance.")]
    GatewayDisabled,
    #[error("Could not reach Synaplan. Check your connection.")]
    Network,
    #[error("{0}")]
    Server(String),
}

impl ChatError {
    pub fn code(&self) -> &'static str {
        match self {
            ChatError::Unauthorized => "unauthorized",
            ChatError::FeatureDisabled => "feature_disabled",
            ChatError::GatewayDisabled => "gateway_disabled",
            ChatError::Network => "network",
            ChatError::Server(_) => "server",
        }
    }
}

/// Stream one assistant turn. `on_event` is called for every text token and once
/// with [`ChatEvent::Done`] (or [`ChatEvent::Error`]). The API key is passed
/// per-call and is never logged.
pub async fn stream_chat<F>(
    base_url: &str,
    key: &str,
    model: Option<&str>,
    messages: &[ChatMessage],
    max_tokens: u32,
    mut on_event: F,
) -> Result<(), ChatError>
where
    F: FnMut(ChatEvent),
{
    let client = http::client().map_err(|_| ChatError::Network)?;
    let url = http::join(base_url, "/v1/messages");

    let mut body = serde_json::json!({
        "max_tokens": max_tokens,
        "stream": true,
        "messages": messages,
    });
    if let Some(m) = model {
        body["model"] = serde_json::Value::String(m.to_string());
    }

    let resp = client
        .post(url)
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .header("accept", "text/event-stream")
        .json(&body)
        .send()
        .await
        .map_err(|_| ChatError::Network)?;

    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        let text = resp.text().await.unwrap_or_default();
        return Err(error_from_response(status, &text));
    }

    let mut parser = SseParser::new();
    let mut stream = resp.bytes_stream();
    // Buffer bytes so a multi-byte UTF-8 sequence split across chunks decodes
    // correctly (a naive from_utf8_lossy per chunk would corrupt it).
    let mut pending: Vec<u8> = Vec::new();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|_| ChatError::Network)?;
        pending.extend_from_slice(&bytes);
        let decoded = take_valid_utf8(&mut pending);
        if decoded.is_empty() {
            continue;
        }
        for event in parser.push(&decoded) {
            let done = matches!(event, ChatEvent::Done | ChatEvent::Error(_));
            on_event(event);
            if done {
                return Ok(());
            }
        }
    }

    // Stream ended without an explicit stop — treat as done.
    on_event(ChatEvent::Done);
    Ok(())
}

/// Drain the longest valid UTF-8 prefix from `buf`, leaving any trailing partial
/// multi-byte sequence behind for the next chunk.
fn take_valid_utf8(buf: &mut Vec<u8>) -> String {
    match std::str::from_utf8(buf) {
        Ok(s) => {
            let out = s.to_string();
            buf.clear();
            out
        }
        Err(e) => {
            let valid = e.valid_up_to();
            let out = String::from_utf8_lossy(&buf[..valid]).to_string();
            buf.drain(..valid);
            out
        }
    }
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    #[serde(default)]
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
}

/// List available model ids for a simple picker.
pub async fn list_models(base_url: &str, key: &str) -> Result<Vec<String>, ChatError> {
    let client = http::client().map_err(|_| ChatError::Network)?;
    let url = http::join(base_url, "/v1/models");
    let resp = client
        .get(url)
        .header("x-api-key", key)
        .send()
        .await
        .map_err(|_| ChatError::Network)?;

    match resp.status().as_u16() {
        200 => {
            let parsed: ModelsResponse = resp
                .json()
                .await
                .map_err(|e| ChatError::Server(e.to_string()))?;
            Ok(parsed.data.into_iter().map(|m| m.id).collect())
        }
        401 | 403 => Err(ChatError::Unauthorized),
        404 => Err(ChatError::FeatureDisabled),
        other => Err(ChatError::Server(format!("status {other}"))),
    }
}

/// Extract the human-readable message from a gateway/provider JSON error body
/// (`{"error":{"message":"…"}}`), if present.
fn extract_error_message(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| {
            v.get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .map(str::to_string)
        })
}

fn is_gateway_disabled(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("gateway") && lower.contains("disab")
}

/// Map a non-2xx response to a [`ChatError`]. Only a genuine `401` is treated as
/// an auth failure of the desktop key; a `403` (e.g. the Messages gateway being
/// disabled, or a scope issue) is NOT — mapping it to `Unauthorized` would wrongly
/// wipe a perfectly valid stored key.
pub(crate) fn error_from_response(status: u16, body: &str) -> ChatError {
    match status {
        401 => ChatError::Unauthorized,
        404 => ChatError::FeatureDisabled,
        _ => {
            let msg = extract_error_message(body)
                .unwrap_or_else(|| format!("The server returned status {status}."));
            if is_gateway_disabled(&msg) {
                ChatError::GatewayDisabled
            } else {
                ChatError::Server(msg)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_disabled_403_is_not_unauthorized() {
        let body = r#"{"type":"error","error":{"type":"permission_error","message":"Messages gateway is disabled on this Synaplan instance."}}"#;
        assert_eq!(error_from_response(403, body), ChatError::GatewayDisabled);
    }

    #[test]
    fn only_401_is_unauthorized() {
        assert_eq!(error_from_response(401, ""), ChatError::Unauthorized);
        // A 403 that is not a gateway message is a server error, never a wipe.
        assert!(matches!(
            error_from_response(403, r#"{"error":{"message":"forbidden"}}"#),
            ChatError::Server(_)
        ));
    }

    #[test]
    fn maps_404_and_generic_errors() {
        assert_eq!(error_from_response(404, ""), ChatError::FeatureDisabled);
        assert_eq!(
            error_from_response(500, ""),
            ChatError::Server("The server returned status 500.".to_string())
        );
    }

    #[test]
    fn take_valid_utf8_keeps_partial_multibyte() {
        // "é" is 0xC3 0xA9. Feed only the first byte first.
        let mut buf = vec![b'h', b'i', 0xC3];
        let out = take_valid_utf8(&mut buf);
        assert_eq!(out, "hi");
        assert_eq!(buf, vec![0xC3]);
        buf.push(0xA9);
        let out2 = take_valid_utf8(&mut buf);
        assert_eq!(out2, "é");
        assert!(buf.is_empty());
    }
}
