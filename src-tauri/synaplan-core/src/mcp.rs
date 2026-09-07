//! Minimal MCP client for the desktop check-in loop (`POST /mcp`).
//! Session init + `tools/call`. Parses either a JSON-RPC body or an SSE wrapper.

use serde_json::{json, Value};
use thiserror::Error;

use crate::http;

const MCP_PROTOCOL: &str = "2025-11-25";

#[derive(Debug, Error)]
pub enum McpError {
    #[error("could not reach Synaplan")]
    Network,
    #[error("this computer was disconnected")]
    Unauthorized,
    #[error("{0}")]
    Protocol(String),
}

impl McpError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Network => "network",
            Self::Unauthorized => "unauthorized",
            Self::Protocol(_) => "server",
        }
    }
}

/// An initialized MCP session bound to one desktop API key.
pub struct McpClient {
    base_url: String,
    key: String,
    session_id: String,
    next_id: u64,
}

impl McpClient {
    pub async fn initialize(base_url: &str, key: &str) -> Result<Self, McpError> {
        let client = http::client().map_err(|_| McpError::Network)?;
        let url = http::join(base_url, "/mcp");
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": MCP_PROTOCOL,
                "capabilities": {},
                "clientInfo": { "name": "synaplan-desktop", "version": env!("CARGO_PKG_VERSION") }
            }
        });
        let resp = client
            .post(&url)
            .header("x-api-key", key)
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", MCP_PROTOCOL)
            .json(&body)
            .send()
            .await
            .map_err(|_| McpError::Network)?;
        let status = resp.status();
        if status.as_u16() == 401 {
            return Err(McpError::Unauthorized);
        }
        if !status.is_success() {
            return Err(McpError::Protocol(format!("initialize HTTP {status}")));
        }
        let session_id = resp
            .headers()
            .get("mcp-session-id")
            .or_else(|| resp.headers().get("Mcp-Session-Id"))
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        if session_id.is_empty() {
            return Err(McpError::Protocol("missing MCP session id".into()));
        }
        Ok(Self {
            base_url: base_url.to_string(),
            key: key.to_string(),
            session_id,
            next_id: 2,
        })
    }

    pub async fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value, McpError> {
        let id = self.next_id;
        self.next_id += 1;
        let client = http::client().map_err(|_| McpError::Network)?;
        let url = http::join(&self.base_url, "/mcp");
        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": name, "arguments": arguments }
        });
        let resp = client
            .post(&url)
            .header("x-api-key", &self.key)
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", MCP_PROTOCOL)
            .header("mcp-session-id", &self.session_id)
            .json(&body)
            .send()
            .await
            .map_err(|_| McpError::Network)?;
        let status = resp.status();
        if status.as_u16() == 401 {
            return Err(McpError::Unauthorized);
        }
        if !status.is_success() {
            return Err(McpError::Protocol(format!("tools/call HTTP {status}")));
        }
        let text = resp.text().await.map_err(|_| McpError::Network)?;
        let parsed = parse_mcp_body(&text)?;
        if parsed.get("error").is_some() {
            return Err(McpError::Protocol(
                parsed["error"]
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("tool error")
                    .to_string(),
            ));
        }
        let result = parsed.get("result").cloned().unwrap_or(Value::Null);
        if result.get("isError").and_then(Value::as_bool) == Some(true) {
            return Err(McpError::Protocol("tool returned an error".into()));
        }
        Ok(result.get("structuredContent").cloned().unwrap_or(result))
    }
}

fn parse_mcp_body(text: &str) -> Result<Value, McpError> {
    let mut json_text = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("data:") {
            let rest = rest.trim();
            if !rest.is_empty() && rest != "[DONE]" {
                json_text = Some(rest);
            }
        }
    }
    let json_text = json_text.unwrap_or(text);
    serde_json::from_str(json_text).map_err(|e| McpError::Protocol(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_json_rpc() {
        let raw = r#"{"jsonrpc":"2.0","id":2,"result":{"structuredContent":{"protocol":1}}}"#;
        let v = parse_mcp_body(raw).unwrap();
        assert_eq!(v["result"]["structuredContent"]["protocol"], 1);
    }

    #[test]
    fn parses_sse_wrapped_json() {
        let raw = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"result\":{\"ok\":true}}\n\n";
        let v = parse_mcp_body(raw).unwrap();
        assert_eq!(v["result"]["ok"], true);
    }
}
