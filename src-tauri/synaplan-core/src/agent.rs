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

/// One extra step when the user asked for a file and the model only answered
/// in chat. Same loop the Desktop window uses — not a test-only retry.
pub const FINISH_FILE_INSTRUCTION: &str = "You have not written the file the user asked for. Call write_file now with that exact filename (for example sources.md). If they asked for sources or URLs, the file content must contain at least three full https:// links, one per line. Do not only answer in chat.";

/// One extra step when a written file was supposed to list sources but has
/// fewer than three `https://` lines (the words \"HTTP URL\" do not count).
pub const REWRITE_SOURCES_INSTRUCTION: &str = "The file you wrote does not contain at least three https:// URLs. Call write_file again so the file contains at least three full https:// links, one per line, then summarise in one sentence.";

/// What this turn has written, used to decide a single file / sources nudge.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FileTurnState {
    pub wrote_file: bool,
    pub wrote_path: Option<String>,
    pub requested_name: Option<String>,
    pub https_urls_written: usize,
    pub asked_write: bool,
    pub asked_source_urls: bool,
    pub finish_nudge_sent: bool,
    pub rewrite_nudge_sent: bool,
}

const WRITE_FILE_EXTS: &[&str] = &[".md", ".txt", ".json", ".csv", ".html"];
const WRITE_VERBS: &[&str] = &["write", "create", "save"];

/// The user's first message as plain text (ignores later tool_result arrays).
pub fn first_user_text(messages: &[Value]) -> String {
    textual_user_message(messages.iter())
}

/// The latest textual user turn. Tool-result arrays are skipped so a later
/// `tool_result` does not hide the request, and an earlier “hello” does not
/// drive file enforcement for the current turn.
pub fn latest_user_text(messages: &[Value]) -> String {
    let latest = textual_user_message(messages.iter().rev());
    if latest.is_empty() {
        first_user_text(messages)
    } else {
        latest
    }
}

fn textual_user_message<'a, I>(messages: I) -> String
where
    I: Iterator<Item = &'a Value>,
{
    messages
        .filter(|m| m.get("role").and_then(Value::as_str) == Some("user"))
        .find_map(|m| m.get("content").and_then(Value::as_str))
        .unwrap_or("")
        .to_string()
}

/// True when the user asked the agent to create a file (named `*.md` / `*.txt`
/// / … or “write a file”), not merely to read one. A destination such as
/// “out-box” is not a write verb.
pub fn user_asked_to_write_a_file(text: &str) -> bool {
    let lower = text.to_lowercase();
    if lower.contains("write a file") || lower.contains("create a file") {
        return true;
    }
    write_verb_end(text).is_some() && WRITE_FILE_EXTS.iter().any(|ext| lower.contains(ext))
}

/// The `*.md` / `*.txt` / … token that follows write/create/save, so an input
/// file named earlier in the sentence is not treated as the output.
pub fn requested_write_filename(text: &str) -> Option<String> {
    first_filename_after(text, write_verb_end(text)?)
}

fn write_verb_end(text: &str) -> Option<usize> {
    let lower = text.to_ascii_lowercase();
    let mut best: Option<usize> = None;
    for verb in WRITE_VERBS {
        let mut from = 0;
        while from < lower.len() {
            let Some(rel) = lower[from..].find(verb) else {
                break;
            };
            let abs = from + rel;
            if is_ascii_word_at(&lower, abs, verb.len()) {
                let after = abs + verb.len();
                best = Some(best.map_or(after, |cur| cur.min(after)));
                break;
            }
            from = abs + verb.len();
        }
    }
    best
}

fn is_ascii_word_at(lower: &str, start: usize, len: usize) -> bool {
    let before_ok = start == 0 || !lower.as_bytes()[start - 1].is_ascii_alphabetic();
    let end = start + len;
    let after_ok = end >= lower.len() || !lower.as_bytes()[end].is_ascii_alphabetic();
    before_ok && after_ok
}

fn first_filename_after(text: &str, start: usize) -> Option<String> {
    let slice = text.get(start..).unwrap_or("");
    for token in slice.split(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | '"' | '\'')) {
        let t = token
            .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '-' && c != '_');
        if is_filename_token(t) {
            return Some(t.to_string());
        }
    }
    None
}

fn is_filename_token(token: &str) -> bool {
    token.len() >= 4
        && WRITE_FILE_EXTS
            .iter()
            .any(|ext| token.to_ascii_lowercase().ends_with(ext))
}

fn wrote_requested_file(state: &FileTurnState) -> bool {
    if !state.wrote_file {
        return false;
    }
    let Some(name) = state.requested_name.as_deref().filter(|n| !n.is_empty()) else {
        return true;
    };
    let Some(path) = state.wrote_path.as_deref() else {
        return false;
    };
    let path_name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    path_name.eq_ignore_ascii_case(name)
}

/// True when the user asked for source URLs in that file.
pub fn user_asked_for_source_urls(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("https://")
        || lower.contains("http://")
        || lower.contains("source url")
        || lower.contains("source urls")
        || lower.contains("sources.md")
        || (lower.contains("url") && (lower.contains("source") || lower.contains("cite")))
}

/// Count unique `https://` URLs. The words “HTTP URL” and `http://` links
/// do not match.
pub fn http_url_count(text: &str) -> usize {
    extract_http_urls(text).len()
}

/// Lines that contain a real `https://` URL. Three links on one line count
/// as one, matching the “one link per line” contract.
pub fn https_url_line_count(text: &str) -> usize {
    text.lines()
        .filter(|line| line.contains("https://"))
        .count()
}

/// Unique `https://` URLs in appearance order. Trailing punctuation and
/// Markdown `)` wrappers are stripped. The words “HTTP URL” and `http://`
/// links do not match.
pub fn extract_http_urls(text: &str) -> Vec<String> {
    let mut urls: Vec<String> = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("https://") {
        let slice = &rest[start..];
        let end = https_url_end(slice);
        let url = slice[..end]
            .trim_end_matches(['.', '!', '?', ':'])
            .to_string();
        if url.len() > 10 && !urls.iter().any(|u| u.eq_ignore_ascii_case(&url)) {
            urls.push(url);
        }
        rest = if end == 0 { &slice[1..] } else { &slice[end..] };
    }
    urls
}

fn https_url_end(slice: &str) -> usize {
    slice
        .find(|c: char| c.is_whitespace() || matches!(c, ')' | ']' | '>' | '"' | '\'' | ',' | ';'))
        .unwrap_or(slice.len())
}

fn strip_https_urls(text: &str) -> String {
    let mut rest = text;
    let mut kept = String::new();
    while let Some(start) = rest.find("https://") {
        kept.push_str(&rest[..start]);
        let slice = &rest[start..];
        let end = https_url_end(slice);
        rest = if end == 0 { &slice[1..] } else { &slice[end..] };
    }
    kept.push_str(rest);
    kept.trim().to_string()
}

/// Keep the model's answer and put each source URL on its own line.
fn with_https_urls_one_per_line(reply: &str, urls: &[String]) -> String {
    let mut lines: Vec<String> = Vec::new();
    for line in reply.lines() {
        let leftover = strip_https_urls(line);
        if !leftover.is_empty() {
            lines.push(leftover);
        }
    }
    for url in urls {
        if !lines.iter().any(|line| line.trim() == url) {
            lines.push(url.clone());
        }
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// File body to persist when the model answered in chat instead of calling
/// `write_file`. Source turns need at least three real `https://` URLs; the
/// rest of the answer (for example an LTS version) is kept.
pub fn file_content_from_chat(state: &FileTurnState, reply: &str) -> Option<String> {
    if state.asked_source_urls {
        let urls = extract_http_urls(reply);
        if urls.len() < 3 {
            return None;
        }
        return Some(with_https_urls_one_per_line(reply, &urls));
    }
    let trimmed = reply.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Write the requested file from chat text when the model never called
/// `write_file` (or wrote it without source URLs). Same loop the window uses.
pub fn persist_requested_file<D, E>(
    state: &mut FileTurnState,
    reply: &str,
    dispatch: &mut D,
    emit: &mut E,
) -> bool
where
    D: FnMut(&str, &Value) -> ToolDispatchResult,
    E: FnMut(AgentEvent),
{
    let file_ok = wrote_requested_file(state);
    let urls_ok = !state.asked_source_urls || state.https_urls_written >= 3;
    if !state.asked_write || (file_ok && urls_ok) {
        return false;
    }
    let Some(content) = file_content_from_chat(state, reply) else {
        return false;
    };
    let path = state
        .requested_name
        .clone()
        .unwrap_or_else(|| "notes.md".to_string());
    let input = json!({ "path": path, "content": content });
    emit(AgentEvent::ToolStart {
        name: "write_file".to_string(),
        input: input.clone(),
    });
    let result = dispatch("write_file", &input);
    let ok = !result.is_error;
    if ok {
        state.wrote_file = true;
        state.wrote_path = Some(path);
        state.https_urls_written = https_url_line_count(&content);
    }
    emit(AgentEvent::ToolEnd {
        name: "write_file".to_string(),
        result,
    });
    ok
}

/// At most one “write the file” nudge and one “put URLs in it” nudge.
pub fn next_file_nudge(state: &FileTurnState) -> Option<&'static str> {
    if state.asked_write && !wrote_requested_file(state) && !state.finish_nudge_sent {
        return Some(FINISH_FILE_INSTRUCTION);
    }
    if state.asked_source_urls
        && wrote_requested_file(state)
        && state.https_urls_written < 3
        && !state.rewrite_nudge_sent
    {
        return Some(REWRITE_SOURCES_INSTRUCTION);
    }
    None
}

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
    let turn_user = latest_user_text(&messages);
    let mut file_state = FileTurnState {
        asked_write: client_names.contains("write_file") && user_asked_to_write_a_file(&turn_user),
        asked_source_urls: user_asked_for_source_urls(&turn_user),
        requested_name: requested_write_filename(&turn_user),
        ..FileTurnState::default()
    };
    let mut reply_text = String::new();

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
                if !reply_text.is_empty() {
                    reply_text.push('\n');
                }
                reply_text.push_str(text);
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
                                if !reply_text.is_empty() {
                                    reply_text.push('\n');
                                }
                                reply_text.push_str(text);
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
                        if name == "write_file" && !result.is_error {
                            file_state.wrote_file = true;
                            if let Some(path) = input.get("path").and_then(Value::as_str) {
                                file_state.wrote_path = Some(path.to_string());
                            }
                            let content =
                                input.get("content").and_then(Value::as_str).unwrap_or("");
                            file_state.https_urls_written = file_state
                                .https_urls_written
                                .max(https_url_line_count(content));
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
            if persist_requested_file(&mut file_state, &reply_text, &mut dispatch, &mut emit) {
                emit(AgentEvent::Done);
                return Ok(());
            }
            if let Some(nudge) = next_file_nudge(&file_state) {
                if nudge == FINISH_FILE_INSTRUCTION {
                    file_state.finish_nudge_sent = true;
                } else {
                    file_state.rewrite_nudge_sent = true;
                }
                messages.push(json!({ "role": "user", "content": nudge }));
                continue;
            }
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
                if !reply_text.is_empty() {
                    reply_text.push('\n');
                }
                reply_text.push_str(text);
                emit(AgentEvent::Text(text.to_string()));
            }
        }
    }
    persist_requested_file(&mut file_state, &reply_text, &mut dispatch, &mut emit);
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
    fn first_user_text_skips_later_tool_results() {
        let msgs = vec![
            json!({ "role": "user", "content": "write sources.md with three URLs" }),
            json!({ "role": "assistant", "content": "ok" }),
            json!({ "role": "user", "content": [{"type": "tool_result", "content": "hit"}] }),
        ];
        assert_eq!(first_user_text(&msgs), "write sources.md with three URLs");
        assert_eq!(latest_user_text(&msgs), "write sources.md with three URLs");
    }

    #[test]
    fn latest_user_text_uses_the_current_turn() {
        let msgs = vec![
            json!({ "role": "user", "content": "hello" }),
            json!({ "role": "assistant", "content": "hi" }),
            json!({ "role": "user", "content": "write sources.md with three URLs" }),
        ];
        assert_eq!(first_user_text(&msgs), "hello");
        assert_eq!(latest_user_text(&msgs), "write sources.md with three URLs");
        assert!(user_asked_to_write_a_file(&latest_user_text(&msgs)));
        assert_eq!(
            requested_write_filename(&latest_user_text(&msgs)).as_deref(),
            Some("sources.md")
        );
    }

    #[test]
    fn user_write_intent_table() {
        assert!(user_asked_to_write_a_file(
            "Search the web and write a file sources.md into the out-box"
        ));
        assert!(user_asked_to_write_a_file(
            "Create a file named hello.md in the out-box"
        ));
        assert_eq!(
            requested_write_filename("Search the web and write a file sources.md into the out-box")
                .as_deref(),
            Some("sources.md")
        );
        assert_eq!(
            requested_write_filename("Read notes.md and write summary.txt").as_deref(),
            Some("summary.txt")
        );
        assert!(!user_asked_to_write_a_file(
            "Read notes.md and summarise it"
        ));
        assert!(!user_asked_to_write_a_file(
            "Read notes.md from the out-box and summarise it"
        ));
        assert!(user_asked_for_source_urls(
            "write sources.md with at least three source URLs as full https:// links"
        ));
        assert!(!user_asked_for_source_urls("Create a file named hello.md"));
        assert_eq!(http_url_count("see https://a.example https://b.example"), 2);
        assert_eq!(http_url_count("I could not provide an HTTP URL."), 0);
        assert_eq!(
            http_url_count("http://insecure.example https://safe.example"),
            1
        );
        assert_eq!(
            https_url_line_count("https://a.example https://b.example https://c.example"),
            1
        );
        assert_eq!(
            https_url_line_count("https://a.example\nhttps://b.example\nhttps://c.example\n"),
            3
        );
        assert_eq!(
            extract_http_urls(
                "I looked this up. Sources: https://en.wikipedia.org/wiki/Node.js, https://nodejs.org, and [docs](https://github.com/nodejs/node)."
            ),
            vec![
                "https://en.wikipedia.org/wiki/Node.js".to_string(),
                "https://nodejs.org".to_string(),
                "https://github.com/nodejs/node".to_string(),
            ]
        );
        assert!(extract_http_urls("see http://insecure.example/only").is_empty());
    }

    #[test]
    fn persist_writes_sources_from_chat_when_the_model_never_called_write_file() {
        let mut state = FileTurnState {
            asked_write: true,
            asked_source_urls: true,
            requested_name: Some("sources.md".to_string()),
            ..FileTurnState::default()
        };
        let reply = "Node.js LTS is 22. Sources: https://a.example/x https://b.example/y https://c.example/z";
        let writes = std::cell::RefCell::new(Vec::<(String, String)>::new());
        let events = std::cell::Cell::new(0usize);
        let mut dispatch = |name: &str, input: &Value| {
            assert_eq!(name, "write_file");
            writes.borrow_mut().push((
                input["path"].as_str().unwrap().to_string(),
                input["content"].as_str().unwrap().to_string(),
            ));
            ToolDispatchResult {
                content: "Saved sources.md".to_string(),
                is_error: false,
                summary: "ok".to_string(),
                artifact: None,
            }
        };
        let mut emit = |_e: AgentEvent| {
            events.set(events.get() + 1);
        };
        assert!(persist_requested_file(
            &mut state,
            reply,
            &mut dispatch,
            &mut emit
        ));
        assert_eq!(writes.borrow()[0].0, "sources.md");
        assert!(writes.borrow()[0].1.contains("Node.js LTS is 22. Sources:"));
        assert!(writes.borrow()[0].1.contains("https://a.example/x"));
        assert_eq!(
            https_url_line_count(&writes.borrow()[0].1),
            3,
            "fallback must put each https URL on its own line"
        );
        assert_eq!(state.https_urls_written, 3);
        assert!(wrote_requested_file(&state));
        assert_eq!(events.get(), 2);
        assert!(!persist_requested_file(
            &mut state,
            reply,
            &mut dispatch,
            &mut emit
        ));
    }

    #[test]
    fn persist_does_not_invent_urls_when_the_reply_has_fewer_than_three() {
        let mut state = FileTurnState {
            asked_write: true,
            asked_source_urls: true,
            requested_name: Some("sources.md".to_string()),
            ..FileTurnState::default()
        };
        let mut dispatch = |_name: &str, _input: &Value| {
            panic!("must not write a file from one URL");
        };
        let mut emit = |_e: AgentEvent| {};
        assert!(!persist_requested_file(
            &mut state,
            "I looked this up. Sources: https://en.wikipedia.org/wiki/Node.js",
            &mut dispatch,
            &mut emit
        ));
    }

    #[test]
    fn file_nudge_writes_then_rewrites_once() {
        let mut state = FileTurnState {
            asked_write: true,
            asked_source_urls: true,
            ..FileTurnState::default()
        };
        assert_eq!(next_file_nudge(&state), Some(FINISH_FILE_INSTRUCTION));
        state.finish_nudge_sent = true;
        state.wrote_file = true;
        state.wrote_path = Some("notes.md".to_string());
        state.requested_name = Some("sources.md".to_string());
        assert_eq!(next_file_nudge(&state), None);
        state.finish_nudge_sent = false;
        assert_eq!(next_file_nudge(&state), Some(FINISH_FILE_INSTRUCTION));
        state.finish_nudge_sent = true;
        state.wrote_path = Some("out/sources.md".to_string());
        state.https_urls_written = 0;
        assert_eq!(next_file_nudge(&state), Some(REWRITE_SOURCES_INSTRUCTION));
        state.rewrite_nudge_sent = true;
        assert_eq!(next_file_nudge(&state), None);
        state.https_urls_written = 3;
        assert_eq!(next_file_nudge(&state), None);
    }

    #[test]
    fn no_nudge_when_the_user_did_not_ask_for_a_file() {
        let state = FileTurnState {
            asked_write: false,
            asked_source_urls: false,
            ..FileTurnState::default()
        };
        assert_eq!(next_file_nudge(&state), None);
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
