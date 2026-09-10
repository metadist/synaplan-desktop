//! Streaming chat against the Anthropic-compatible Messages gateway
//! (`POST /v1/messages`, `stream: true`) and model discovery (`GET /v1/models`).
//! The account default model is used unless the caller picks one — never a
//! hardcoded `claude-*` id.

use std::sync::atomic::{AtomicBool, Ordering};

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
    #[error("Choose a model to chat with, then try again.")]
    ModelUnavailable,
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
            ChatError::ModelUnavailable => "model_unavailable",
            ChatError::Network => "network",
            ChatError::Server(_) => "server",
        }
    }
}

/// Header that pins a published Assistant (the recipe) for one call.
pub const HEADER_AGENT_ID: &str = "x-synaplan-agent-id";
/// Header that selects the knowledge folder (`DESKTOP:{projectId}`) for one call.
pub const HEADER_RAG_GROUP_KEY: &str = "x-synaplan-rag-group-key";

/// What every `/v1/messages` call carries for the project it runs in (C15).
///
/// The project's chat model always goes in the body — the server must never
/// substitute the account default or an Assistant's recipe model. The
/// Assistant pin and the knowledge folder ride in headers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TurnContext {
    /// Wire model id (`None` only for computer-level jobs the server owns).
    pub model: Option<String>,
    pub agent_id: Option<i64>,
    pub rag_group_key: Option<String>,
}

impl TurnContext {
    /// A context that only sets the model.
    pub fn model(model: impl Into<String>) -> Self {
        Self {
            model: Some(model.into()),
            ..Self::default()
        }
    }

    /// Put the model into a request body (no-op when `model` is `None`).
    pub fn apply_body(&self, body: &mut serde_json::Value) {
        if let Some(m) = &self.model {
            body["model"] = serde_json::Value::String(m.clone());
        }
    }

    /// The extra headers this context adds to a request.
    pub fn headers(&self) -> Vec<(&'static str, String)> {
        let mut out = Vec::new();
        if let Some(id) = self.agent_id {
            out.push((HEADER_AGENT_ID, id.to_string()));
        }
        if let Some(key) = self.rag_group_key.as_deref().filter(|k| !k.is_empty()) {
            out.push((HEADER_RAG_GROUP_KEY, key.to_string()));
        }
        out
    }

    /// Apply [`Self::headers`] to a request builder.
    pub fn apply_headers(&self, mut req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        for (name, value) in self.headers() {
            req = req.header(name, value);
        }
        req
    }
}

/// The JSON body of a streaming chat turn. Pure so a test can inspect it.
pub fn chat_body(
    ctx: &TurnContext,
    messages: &[ChatMessage],
    max_tokens: u32,
) -> serde_json::Value {
    let mut body = serde_json::json!({
        "max_tokens": max_tokens,
        "stream": true,
        "messages": messages,
    });
    ctx.apply_body(&mut body);
    body
}

/// Stream one assistant turn. `on_event` is called for every text token and once
/// with [`ChatEvent::Done`] (or [`ChatEvent::Error`]). The API key is passed
/// per-call and is never logged.
pub async fn stream_chat<F>(
    base_url: &str,
    key: &str,
    ctx: &TurnContext,
    messages: &[ChatMessage],
    max_tokens: u32,
    cancel: &AtomicBool,
    mut on_event: F,
) -> Result<(), ChatError>
where
    F: FnMut(ChatEvent),
{
    let client = http::client().map_err(|_| ChatError::Network)?;
    let url = http::join(base_url, "/v1/messages");
    let body = chat_body(ctx, messages, max_tokens);

    let req = client
        .post(url)
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .header("accept", "text/event-stream");
    let resp = ctx
        .apply_headers(req)
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
        // User pressed Stop: end the turn cleanly, keeping what streamed so far.
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let bytes = chunk.map_err(|_| ChatError::Network)?;
        pending.extend_from_slice(&bytes);
        let decoded = take_valid_utf8(&mut pending);
        if decoded.is_empty() {
            continue;
        }
        for event in parser.push(&decoded) {
            match event {
                ChatEvent::Error(msg) => return Err(ChatError::Server(msg)),
                ChatEvent::Done => {
                    on_event(ChatEvent::Done);
                    return Ok(());
                }
                token => on_event(token),
            }
        }
    }

    // Stream ended (or was cancelled) without an explicit stop — treat as done.
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

/// A model advertised by `/v1/models`, with its provider (`owned_by`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider: String,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    #[serde(default)]
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
    #[serde(default, rename = "owned_by")]
    owned_by: String,
}

/// List available models (id + provider) for the picker. Capability filtering
/// happens client-side because `/v1/models` carries no capability field.
pub async fn list_models(base_url: &str, key: &str) -> Result<Vec<ModelInfo>, ChatError> {
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
            Ok(parsed
                .data
                .into_iter()
                .map(|m| ModelInfo {
                    id: m.id,
                    provider: if m.owned_by.is_empty() {
                        "unknown".to_string()
                    } else {
                        m.owned_by
                    },
                })
                .collect())
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
        404 => {
            // On /v1/messages a 404 means the requested model is not resolvable
            // (e.g. none was chosen). It is NOT the desktop feature being off.
            let msg = extract_error_message(body).unwrap_or_default();
            if msg.to_lowercase().contains("model") {
                ChatError::ModelUnavailable
            } else {
                ChatError::FeatureDisabled
            }
        }
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
        let model_body = r#"{"error":{"type":"not_found_error","message":"The model `(none)` does not exist or is not available."}}"#;
        assert_eq!(
            error_from_response(404, model_body),
            ChatError::ModelUnavailable
        );
        assert_eq!(
            error_from_response(500, ""),
            ChatError::Server("The server returned status 500.".to_string())
        );
    }

    #[test]
    fn chat_body_carries_the_project_model_and_nothing_else_picks_it() {
        let msgs = vec![ChatMessage {
            role: "user".into(),
            content: "hi".into(),
        }];
        let body = chat_body(&TurnContext::model("llama3.2"), &msgs, 512);
        assert_eq!(body["model"], "llama3.2");
        assert_eq!(body["stream"], true);
        assert_eq!(body["max_tokens"], 512);
        assert_eq!(body["messages"][0]["content"], "hi");

        // No model → the key is absent, not an empty string.
        let body = chat_body(&TurnContext::default(), &msgs, 512);
        assert!(body.get("model").is_none());
    }

    #[test]
    fn turn_context_headers_pin_assistant_and_knowledge_folder() {
        let ctx = TurnContext {
            model: Some("m".into()),
            agent_id: Some(42),
            rag_group_key: Some("DESKTOP:01ARZ3NDEKTSV4RRFFQ69G5FAV".into()),
        };
        assert_eq!(
            ctx.headers(),
            vec![
                (HEADER_AGENT_ID, "42".to_string()),
                (
                    HEADER_RAG_GROUP_KEY,
                    "DESKTOP:01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string()
                ),
            ]
        );
        assert!(TurnContext::model("m").headers().is_empty());
        let empty_key = TurnContext {
            rag_group_key: Some(String::new()),
            ..TurnContext::default()
        };
        assert!(empty_key.headers().is_empty());
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
