//! The client-owned tool loop (B2). The Messages gateway supports Anthropic-style
//! tool use: we send `tools`, the model answers with `tool_use` blocks and
//! `stop_reason: "tool_use"`, we execute them locally (via a caller-provided
//! dispatcher so this crate stays Tauri-free), append `tool_result` blocks, and
//! loop until the model stops calling tools or a hard iteration cap is hit.
//!
//! Requests are **non-streaming** here: parsing whole `tool_use` blocks is far
//! more robust than reassembling them from SSE deltas. Plain (no-skill) chat
//! still uses [`crate::messages::stream_chat`].

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{json, Value};

use crate::http;
use crate::messages::{error_from_response, ChatError, TurnContext};

/// A tool advertised to the model.
///
/// Two kinds share this struct. A *client* tool carries `input_schema` and is
/// executed on this computer by the dispatcher. A *server* tool
/// (`server_type: Some(..)`) is a capability request the gateway answers
/// itself — the desktop never sees a call for it, only the final text.
#[derive(Debug, Clone)]
pub struct AgentTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    /// Anthropic server-tool type (e.g. `web_search_20250305`); `None` for a
    /// client tool.
    pub server_type: Option<String>,
}

impl AgentTool {
    /// A client tool the dispatcher executes locally.
    pub fn client(name: &str, description: &str, input_schema: Value) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            input_schema,
            server_type: None,
        }
    }

    /// The wire shape for the `tools` array.
    pub fn to_declaration(&self) -> Value {
        match &self.server_type {
            Some(kind) => json!({ "type": kind, "name": self.name }),
            None => json!({
                "name": self.name,
                "description": self.description,
                "input_schema": self.input_schema,
            }),
        }
    }
}

/// The web-search server tool. In `auto` mode the Synaplan gateway takes this
/// declaration over and runs the search with the workspace's provider; when
/// none is configured it is forwarded to an upstream that can honour it.
pub fn web_search_tool() -> AgentTool {
    AgentTool {
        name: "web_search".to_string(),
        description: String::new(),
        input_schema: Value::Null,
        server_type: Some("web_search_20250305".to_string()),
    }
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

/// Hard cap on model↔tool round-trips per user turn. A multi-file task (list a
/// folder, read a skill, convert three workbooks, read them back, write the
/// result) needs well over a dozen rounds; the tool timeout bounds each one.
pub const MAX_ITERATIONS: usize = 40;
/// Output budget per model round. A `write_file` carrying a whole document
/// easily needs several thousand tokens; every supported provider accepts this.
const MAX_TOKENS: u32 = 8192;
const WRAP_UP_MAX_TOKENS: u32 = 800;

/// Sent when a round ended with `stop_reason: max_tokens`: the reply (and any
/// tool call in it) was cut off, so nothing from it is executed.
pub const CONTINUE_AFTER_CUTOFF: &str = "Your previous reply was cut off by the output length limit before it was complete, so none of its tool calls were executed. Continue where you stopped. Keep each write_file under about 1,500 words and split larger documents into several files or several write_file calls.";

/// The instruction sent when the step budget is spent, so the turn still ends
/// with a truthful answer instead of silence.
pub const WRAP_UP_INSTRUCTION: &str = "You have used the maximum number of tool steps for this turn and cannot call any more tools now. In the user's language, tell them briefly what you completed (with file paths), what is still missing, and that they can reply \"continue\" to let you carry on.";

/// The conversation for the wrap-up call: the turn so far plus the instruction.
pub fn wrap_up_messages(mut messages: Vec<Value>) -> Vec<Value> {
    messages.push(json!({ "role": "user", "content": WRAP_UP_INSTRUCTION }));
    messages
}

/// Client-tool names declared on this request. Server tools are omitted: the
/// gateway owns those and they must not be replayed as `tool_use`.
pub fn declared_client_names(tools: &[AgentTool]) -> HashSet<String> {
    tools
        .iter()
        .filter(|t| t.server_type.is_none())
        .map(|t| t.name.clone())
        .collect()
}

/// The assistant blocks a client may replay: non-empty `text` and `tool_use`.
/// Falls back to one placeholder text block so the turn is never empty.
pub fn echo_blocks(content: &Value) -> Value {
    echo_declared_blocks(content, None)
}

/// Like [`echo_blocks`], but drops `tool_use` whose name is not in `allowed`.
/// Replaying an invented name (`repo_browser.open_file`) makes the next
/// `/v1/messages` fail with "not in request.tools".
pub fn echo_declared_blocks(content: &Value, allowed: Option<&HashSet<String>>) -> Value {
    let kept: Vec<Value> = content
        .as_array()
        .into_iter()
        .flatten()
        .filter(|b| match b.get("type").and_then(Value::as_str) {
            Some("tool_use") => match allowed {
                None => true,
                Some(names) => b
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|n| names.contains(n)),
            },
            Some("text") => b
                .get("text")
                .and_then(Value::as_str)
                .is_some_and(|t| !t.is_empty()),
            _ => false,
        })
        .cloned()
        .collect();
    if kept.is_empty() {
        json!([{ "type": "text", "text": "(no content)" }])
    } else {
        Value::Array(kept)
    }
}

/// User-side hint after the model invented a tool. Names only — no paths.
pub fn unknown_tool_nudge(unknown: &[String], available: &HashSet<String>) -> String {
    let mut names: Vec<&String> = available.iter().collect();
    names.sort();
    format!(
        "{} is not a tool you have. Call only: {}. There is no file browser other than list_files and read_file.",
        unknown.join(", "),
        names
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// The non-empty text blocks of an assistant `content` array.
pub fn text_only_blocks(content: &Value) -> Vec<String> {
    content
        .as_array()
        .into_iter()
        .flatten()
        .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|b| b.get("text").and_then(Value::as_str))
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect()
}

/// The gateway runs server tools (web search) itself, but when the model asks
/// for a server tool and a client tool in the *same* step it hands the whole
/// step back to us. We cannot answer the server tool, so this result tells the
/// model to call it alone next time — which the gateway then serves.
pub fn server_tool_result(name: &str) -> ToolDispatchResult {
    ToolDispatchResult {
        content: format!(
            "{name} runs on the Synaplan server and must be the only tool call in a step. Call {name} again by itself, then continue with the local tools."
        ),
        is_error: true,
        summary: format!("{name}: call it alone, then continue"),
        artifact: None,
    }
}

/// The JSON body of one (non-streaming) agent round-trip. Pure so a test can
/// check the project model lands in it.
pub fn agent_body(ctx: &TurnContext, system: &str, messages: &[Value], tools: &Value) -> Value {
    let mut body = json!({
        "max_tokens": MAX_TOKENS,
        "system": system,
        "messages": messages,
        "tools": tools,
    });
    ctx.apply_body(&mut body);
    body
}

/// Run one agentic turn. `messages` is the conversation so far (`{role, content}`
/// objects, string or block content). `dispatch` executes a tool call; `emit`
/// receives UI events. The key is passed per call and never logged.
#[allow(clippy::too_many_arguments)]
pub async fn run_agent_turn<D, E>(
    base_url: &str,
    key: &str,
    ctx: &TurnContext,
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
    let tools_json = Value::Array(tools.iter().map(AgentTool::to_declaration).collect());
    let server_tools: Vec<&str> = tools
        .iter()
        .filter(|t| t.server_type.is_some())
        .map(|t| t.name.as_str())
        .collect();
    let client_names = declared_client_names(tools);

    for _ in 0..MAX_ITERATIONS {
        if cancel.load(Ordering::Relaxed) {
            emit(AgentEvent::Cancelled);
            return Ok(());
        }

        let body = agent_body(ctx, system, &messages, &tools_json);

        let req = client
            .post(url.clone())
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01");
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

        // A round cut off by the output budget carries incomplete tool calls
        // (empty or half-written arguments). Executing them would write empty
        // or broken files, so the text is kept and the model is asked to go on.
        if stop_reason == "max_tokens" {
            let text_blocks = text_only_blocks(&content);
            for text in &text_blocks {
                emit(AgentEvent::Text(text.clone()));
            }
            messages.push(json!({
                "role": "assistant",
                "content": if text_blocks.is_empty() {
                    "(reply cut off)".to_string()
                } else {
                    text_blocks.join("\n")
                }
            }));
            messages.push(json!({ "role": "user", "content": CONTINUE_AFTER_CUTOFF }));
            continue;
        }

        let mut tool_results: Vec<Value> = Vec::new();
        let mut unknown: Vec<String> = Vec::new();
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
                        let declared =
                            client_names.contains(&name) || server_tools.contains(&name.as_str());
                        let result = if server_tools.contains(&name.as_str()) {
                            server_tool_result(&name)
                        } else {
                            dispatch(&name, &input)
                        };
                        if declared {
                            tool_results.push(json!({
                                "type": "tool_result",
                                "tool_use_id": id,
                                "content": result.content,
                                "is_error": result.is_error,
                            }));
                        } else if !unknown.contains(&name) {
                            unknown.push(name.clone());
                        }
                        emit(AgentEvent::ToolEnd { name, result });
                    }
                    _ => {}
                }
            }
        }

        // Echo the assistant turn back so tool_use ids line up — but only the
        // blocks every provider accepts on replay. Server-tool blocks
        // (`server_tool_use`, `web_*_tool_result`, `thinking`) are the
        // gateway's business and are rejected when a client sends them back.
        // Invented client names are dropped too: they are not in `request.tools`.
        messages.push(json!({
            "role": "assistant",
            "content": echo_declared_blocks(&content, Some(&client_names))
        }));

        let continue_tools =
            stop_reason == "tool_use" && (!tool_results.is_empty() || !unknown.is_empty());
        if !continue_tools {
            emit(AgentEvent::Done);
            return Ok(());
        }

        if unknown.is_empty() {
            messages.push(json!({ "role": "user", "content": tool_results }));
        } else {
            let nudge = unknown_tool_nudge(&unknown, &client_names);
            if tool_results.is_empty() {
                messages.push(json!({ "role": "user", "content": nudge }));
            } else {
                tool_results.push(json!({ "type": "text", "text": nudge }));
                messages.push(json!({ "role": "user", "content": tool_results }));
            }
        }
    }

    // Iteration cap reached — ask for an honest summary without tools so the
    // person learns what happened instead of seeing an empty answer.
    let mut body = agent_body(ctx, system, &wrap_up_messages(messages), &json!([]));
    body["max_tokens"] = json!(WRAP_UP_MAX_TOKENS);
    body.as_object_mut().map(|o| o.remove("tools"));
    let req = client
        .post(url)
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01");
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
    let value: Value = resp
        .json()
        .await
        .map_err(|e| ChatError::Server(e.to_string()))?;
    for block in value
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(text) = block.get("text").and_then(Value::as_str) {
            if !text.is_empty() {
                emit(AgentEvent::Text(text.to_string()));
            }
        }
    }
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

    #[test]
    fn echo_keeps_only_text_and_tool_use_blocks() {
        let content = json!([
            {"type": "server_tool_use", "id": "srv1", "name": "web_search", "input": {"query": "x"}},
            {"type": "web_search_tool_result", "tool_use_id": "srv1", "content": []},
            {"type": "text", "text": "Found it."},
            {"type": "tool_use", "id": "t1", "name": "write_file", "input": {"path": "a"}}
        ]);
        let kept = echo_blocks(&content);
        assert_eq!(kept.as_array().unwrap().len(), 2);
        assert_eq!(kept[0]["type"], "text");
        assert_eq!(kept[1]["type"], "tool_use");
        assert_eq!(echo_blocks(&json!([]))[0]["text"], "(no content)");
    }

    #[test]
    fn echo_drops_invented_tool_names_the_request_did_not_declare() {
        let allowed = HashSet::from(["list_files".to_string(), "read_file".to_string()]);
        let content = json!([
            {"type": "tool_use", "id": "t1", "name": "list_files", "input": {"path": "."}},
            {"type": "tool_use", "id": "t2", "name": "repo_browser.open_file", "input": {"path": "a.csv"}}
        ]);
        let kept = echo_declared_blocks(&content, Some(&allowed));
        let blocks = kept.as_array().unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0]["name"], "list_files");
        let nudge = unknown_tool_nudge(&["repo_browser.open_file".to_string()], &allowed);
        assert!(nudge.contains("repo_browser.open_file"));
        assert!(nudge.contains("list_files"));
        assert!(nudge.contains("read_file"));
    }

    #[test]
    fn text_only_blocks_drop_truncated_tool_calls() {
        let content = json!([
            {"type": "text", "text": "Writing the file now."},
            {"type": "tool_use", "id": "t1", "name": "write_file", "input": {}},
            {"type": "text", "text": ""}
        ]);
        assert_eq!(text_only_blocks(&content), vec!["Writing the file now."]);
        assert!(CONTINUE_AFTER_CUTOFF.contains("none of its tool calls were executed"));
    }

    #[test]
    fn a_server_tool_in_a_mixed_step_is_answered_with_a_retry_hint() {
        let r = server_tool_result("web_search");
        assert!(r.is_error);
        assert!(r.content.contains("only tool call in a step"));
        assert!(r.summary.starts_with("web_search"));
    }

    #[test]
    fn wrap_up_appends_the_instruction_as_the_last_user_message() {
        let msgs = vec![json!({ "role": "user", "content": "merge the files" })];
        let out = wrap_up_messages(msgs);
        assert_eq!(out.len(), 2);
        assert_eq!(out[1]["role"], "user");
        assert!(out[1]["content"]
            .as_str()
            .unwrap()
            .contains("cannot call any more tools"));
    }

    #[test]
    fn server_tool_declares_type_and_name_only() {
        let decl = web_search_tool().to_declaration();
        assert_eq!(decl["type"], "web_search_20250305");
        assert_eq!(decl["name"], "web_search");
        assert!(decl.get("input_schema").is_none());

        let client = AgentTool::client("read_file", "Read", json!({"type": "object"}));
        let decl = client.to_declaration();
        assert!(decl.get("type").is_none());
        assert_eq!(decl["input_schema"]["type"], "object");
    }

    #[test]
    fn agent_body_carries_the_project_model() {
        let msgs = vec![json!({ "role": "user", "content": "hi" })];
        let tools = json!([]);
        let body = agent_body(&TurnContext::model("llama3.2"), "sys", &msgs, &tools);
        assert_eq!(body["model"], "llama3.2");
        assert_eq!(body["system"], "sys");
        assert_eq!(body["max_tokens"], MAX_TOKENS);
        // Server-owned jobs (poll loop) still send no model.
        let body = agent_body(&TurnContext::default(), "sys", &msgs, &tools);
        assert!(body.get("model").is_none());
    }
}
