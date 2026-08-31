//! A pure, streaming parser for the Anthropic-Messages SSE format that Synaplan's
//! `/v1/messages` gateway emits when `stream: true`. Kept free of any network or
//! Tauri dependency so it is unit-tested against a recorded fixture, with no
//! live server (DC3).
//!
//! We only extract what the chat UI needs in Sprint B1:
//! - text deltas (`content_block_delta` → `delta.text_delta`) → [`ChatEvent::Token`]
//! - stream end (`message_stop`) → [`ChatEvent::Done`]
//! - stream errors (`error`) → [`ChatEvent::Error`]
//!
//! Tool-use blocks (`input_json_delta`) are ignored here; the local skill tool
//! loop arrives in Sprint B2.

use serde_json::Value;

/// A parsed chat event for the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatEvent {
    /// A chunk of assistant text to append to the current message.
    Token(String),
    /// The stream finished normally.
    Done,
    /// The stream reported an error (message is safe to show).
    Error(String),
}

/// Incremental SSE parser. Feed it byte chunks as they arrive; it returns any
/// complete events found so far.
#[derive(Default)]
pub struct SseParser {
    buffer: String,
}

impl SseParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a chunk of the response body. Returns the events completed by this
    /// chunk (possibly empty). Events are delimited by a blank line.
    pub fn push(&mut self, chunk: &str) -> Vec<ChatEvent> {
        // Normalise CRLF so a Windows upstream and a Unix one parse identically.
        self.buffer.push_str(&chunk.replace("\r\n", "\n"));
        let mut events = Vec::new();

        while let Some(idx) = self.buffer.find("\n\n") {
            let block: String = self.buffer[..idx].to_string();
            self.buffer.drain(..idx + 2);
            if let Some(event) = parse_block(&block) {
                events.push(event);
            }
        }
        events
    }

    /// Parse a complete response body in one shot (used by tests).
    pub fn parse_all(input: &str) -> Vec<ChatEvent> {
        let mut parser = SseParser::new();
        let mut events = parser.push(input);
        // Flush a trailing block that was not terminated by a blank line.
        if !parser.buffer.trim().is_empty() {
            if let Some(event) = parse_block(&parser.buffer.clone()) {
                events.push(event);
            }
        }
        events
    }
}

fn parse_block(block: &str) -> Option<ChatEvent> {
    let mut event_name: Option<String> = None;
    let mut data = String::new();

    for line in block.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(rest) = line.strip_prefix("event:") {
            event_name = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(rest.trim_start());
        }
        // Comment lines (starting with ':') and unknown fields are ignored.
    }

    let data = data.trim();
    if data.is_empty() || data == "[DONE]" {
        // A bare `event: message_stop` with no JSON still ends the stream.
        if event_name.as_deref() == Some("message_stop") {
            return Some(ChatEvent::Done);
        }
        return None;
    }

    let json: Value = serde_json::from_str(data).ok()?;
    let kind = json
        .get("type")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or(event_name);

    match kind.as_deref() {
        Some("content_block_delta") => {
            let delta = json.get("delta")?;
            match delta.get("type").and_then(Value::as_str) {
                Some("text_delta") => delta
                    .get("text")
                    .and_then(Value::as_str)
                    .map(|t| ChatEvent::Token(t.to_string())),
                _ => None,
            }
        }
        Some("message_stop") => Some(ChatEvent::Done),
        Some("error") => {
            let msg = json
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("The server reported an error.")
                .to_string();
            Some(ChatEvent::Error(msg))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = include_str!("../../../tests/fixtures/messages_stream.sse");

    #[test]
    fn parses_recorded_stream_to_pong() {
        let events = SseParser::parse_all(SAMPLE);
        assert_eq!(
            events,
            vec![
                ChatEvent::Token("PONG".to_string()),
                ChatEvent::Token("!".to_string()),
                ChatEvent::Done,
            ]
        );
    }

    #[test]
    fn tokens_can_arrive_split_across_chunks() {
        let mut parser = SseParser::new();
        let mut got = Vec::new();
        got.extend(parser.push("event: content_block_delta\n"));
        got.extend(parser.push("data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"Hel"));
        // Nothing complete yet (no blank line).
        assert!(got.is_empty());
        got.extend(parser.push("lo\"}}\n\n"));
        assert_eq!(got, vec![ChatEvent::Token("Hello".to_string())]);
    }

    #[test]
    fn surfaces_error_events() {
        let sse = "event: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Try again later.\"}}\n\n";
        assert_eq!(
            SseParser::parse_all(sse),
            vec![ChatEvent::Error("Try again later.".to_string())]
        );
    }

    #[test]
    fn ignores_ping_and_unknown_blocks() {
        let sse = "event: ping\ndata: {\"type\":\"ping\"}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";
        assert_eq!(SseParser::parse_all(sse), vec![ChatEvent::Done]);
    }
}
