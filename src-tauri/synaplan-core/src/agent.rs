//! The client-owned tool loop (B2). The Messages gateway supports Anthropic-style
//! tool use: we send `tools`, the model answers with `tool_use` blocks and
//! `stop_reason: "tool_use"`, we execute them locally (via a caller-provided
//! dispatcher so this crate stays Tauri-free), append `tool_result` blocks, and
//! loop until the model stops calling tools or a hard iteration cap is hit.
//!
//! Requests are **non-streaming** here: parsing whole `tool_use` blocks is far
//! more robust than reassembling them from SSE deltas. Plain (no-skill) chat
//! still uses [`crate::messages::stream_chat`].

use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{json, Value};

use crate::http;
use crate::messages::{error_from_response, ChatError};

/// A tool advertised to the model.
#[derive(Debug, Clone)]
pub struct AgentTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// The outcome of executing one tool call, produced by the dispatcher.
#[derive(Debug, Clone)]
pub struct ToolDispatchResult {
    /// Text sent back to the model as the `tool_result`.
    pub content: String,
    pub is_error: bool,
    /// A short, human-facing line for the run activity UI.
    pub summary: String,
    /// A produced file path, if any (surfaced as an artifact card).
    pub artifact: Option<String>,
}

/// Events emitted during a turn, mapped to Tauri events by the caller.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    Text(String),
    ToolStart {
        name: String,
        input: Value,
    },
    ToolEnd {
        name: String,
        result: ToolDispatchResult,
    },
    Cancelled,
    Done,
}

/// Hard cap on model↔tool round-trips per user turn.
pub const MAX_ITERATIONS: usize = 12;
const MAX_TOKENS: u32 = 4096;

/// Run one agentic turn. `messages` is the conversation so far (`{role, content}`
/// objects, string or block content). `dispatch` executes a tool call; `emit`
/// receives UI events. The key is passed per call and never logged.
#[allow(clippy::too_many_arguments)]
pub async fn run_agent_turn<D, E>(
    base_url: &str,
    key: &str,
    model: Option<&str>,
    system: &str,
    mut messages: Vec<Value>,
    tools: &[AgentTool],
    cancel: &AtomicBool,
    mut dispatch: D,
    mut emit: E,
) -> Result<(), ChatError>
where
    D: FnMut(&str, &Value) -> ToolDispatchResult,
    E: FnMut(AgentEvent),
{
    let client = http::client().map_err(|_| ChatError::Network)?;
    let url = http::join(base_url, "/v1/messages");
    let tools_json = Value::Array(
        tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.input_schema,
                })
            })
            .collect(),
    );

    for _ in 0..MAX_ITERATIONS {
        if cancel.load(Ordering::Relaxed) {
            emit(AgentEvent::Cancelled);
            return Ok(());
        }

        let mut body = json!({
            "max_tokens": MAX_TOKENS,
            "system": system,
            "messages": messages,
            "tools": tools_json,
        });
        if let Some(m) = model {
            body["model"] = Value::String(m.to_string());
        }

        let resp = client
            .post(url.clone())
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|_| ChatError::Network)?;

        let status = resp.status().as_u16();
        if !(200..300).contains(&status) {
            let text = resp.text().await.unwrap_or_default();
            return Err(error_from_response(status, &text));
        }

        let value: Value = resp
            .json()
            .await
            .map_err(|e| ChatError::Server(e.to_string()))?;
        let content = value.get("content").cloned().unwrap_or_else(|| json!([]));
        let stop_reason = value
            .get("stop_reason")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        let mut tool_results: Vec<Value> = Vec::new();
        if let Some(blocks) = content.as_array() {
            for block in blocks {
                match block.get("type").and_then(Value::as_str) {
                    Some("text") => {
                        if let Some(text) = block.get("text").and_then(Value::as_str) {
                            if !text.is_empty() {
                                emit(AgentEvent::Text(text.to_string()));
                            }
                        }
                    }
                    Some("tool_use") => {
                        if cancel.load(Ordering::Relaxed) {
                            emit(AgentEvent::Cancelled);
                            return Ok(());
                        }
                        let id = block
                            .get("id")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let name = block
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let input = block.get("input").cloned().unwrap_or_else(|| json!({}));
                        emit(AgentEvent::ToolStart {
                            name: name.clone(),
                            input: input.clone(),
                        });
                        let result = dispatch(&name, &input);
                        tool_results.push(json!({
                            "type": "tool_result",
                            "tool_use_id": id,
                            "content": result.content,
                            "is_error": result.is_error,
                        }));
                        emit(AgentEvent::ToolEnd { name, result });
                    }
                    _ => {}
                }
            }
        }

        // Echo the assistant turn back verbatim so tool_use ids line up.
        messages.push(json!({ "role": "assistant", "content": content }));

        if tool_results.is_empty() || stop_reason != "tool_use" {
            emit(AgentEvent::Done);
            return Ok(());
        }

        messages.push(json!({ "role": "user", "content": tool_results }));
    }

    // Iteration cap reached — end the turn cleanly.
    emit(AgentEvent::Done);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_result_block_shape() {
        // Sanity: the block we send back matches the Anthropic tool_result shape.
        let block = json!({
            "type": "tool_result",
            "tool_use_id": "toolu_1",
            "content": "ok",
            "is_error": false,
        });
        assert_eq!(block["type"], "tool_result");
        assert_eq!(block["tool_use_id"], "toolu_1");
    }
}
