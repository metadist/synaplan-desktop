//! The contract view: raw calls to the gateway that show *why* a client-visible
//! result looked the way it did (final `stop_reason`, block types, gateway
//! tool rounds as `ping` events, error events), plus the pairing exchange the
//! desktop's Pair screen performs.

use std::path::Path;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use serde_json::{json, Value};
use synaplan_core::messages::{ChatMessage, TurnContext};

pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(600))
        .user_agent("SynaplanDesktop-usecases/1")
        .build()
        .expect("reqwest client")
}

pub fn user(text: &str) -> ChatMessage {
    ChatMessage {
        role: "user".into(),
        content: text.into(),
    }
}

pub fn assistant(text: &str) -> ChatMessage {
    ChatMessage {
        role: "assistant".into(),
        content: text.into(),
    }
}

/// What one raw streaming turn produced.
#[derive(Debug, Default, Clone)]
pub struct RawStream {
    pub status: u16,
    pub text: String,
    pub stop_reason: Option<String>,
    pub pings: usize,
    pub block_types: Vec<String>,
    pub tool_use_names: Vec<String>,
    pub error: Option<String>,
    pub first_token_ms: Option<u64>,
    pub total_ms: u64,
    pub events: usize,
}

/// Stream `/v1/messages` exactly like the desktop (`stream: true`, headers from
/// `TurnContext`) but keep every event.
pub async fn stream_capture(
    base: &str,
    key: &str,
    ctx: &TurnContext,
    messages: &[ChatMessage],
    max_tokens: u32,
    tools: Option<&[Value]>,
) -> RawStream {
    let body = synaplan_core::messages::chat_body(ctx, messages, max_tokens, tools);
    let started = Instant::now();
    let mut out = RawStream::default();
    let req = client()
        .post(format!("{base}/v1/messages"))
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .header("accept", "text/event-stream");
    let resp = match ctx.apply_headers(req).json(&body).send().await {
        Ok(r) => r,
        Err(e) => {
            out.error = Some(format!("network: {e}"));
            out.total_ms = started.elapsed().as_millis() as u64;
            return out;
        }
    };
    out.status = resp.status().as_u16();
    if out.status >= 400 {
        let text = resp.text().await.unwrap_or_default();
        out.error = Some(extract_error(&text).unwrap_or(text));
        out.total_ms = started.elapsed().as_millis() as u64;
        return out;
    }
    let mut buffer = String::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let Ok(bytes) = chunk else {
            out.error.get_or_insert_with(|| "stream broke".to_string());
            break;
        };
        buffer.push_str(&String::from_utf8_lossy(&bytes).replace("\r\n", "\n"));
        while let Some(idx) = buffer.find("\n\n") {
            let block = buffer[..idx].to_string();
            buffer.drain(..idx + 2);
            for line in block.lines() {
                let Some(data) = line.strip_prefix("data:") else {
                    continue;
                };
                let Ok(v) = serde_json::from_str::<Value>(data.trim()) else {
                    continue;
                };
                out.events += 1;
                match v.get("type").and_then(Value::as_str) {
                    Some("ping") => out.pings += 1,
                    Some("content_block_start") => {
                        if let Some(b) = v.get("content_block") {
                            let kind = b.get("type").and_then(Value::as_str).unwrap_or("?");
                            out.block_types.push(kind.to_string());
                            if kind == "tool_use" || kind == "server_tool_use" {
                                if let Some(n) = b.get("name").and_then(Value::as_str) {
                                    out.tool_use_names.push(n.to_string());
                                }
                            }
                        }
                    }
                    Some("content_block_delta") => {
                        if let Some(t) = v
                            .get("delta")
                            .filter(|d| d.get("type").and_then(Value::as_str) == Some("text_delta"))
                            .and_then(|d| d.get("text"))
                            .and_then(Value::as_str)
                        {
                            if out.first_token_ms.is_none() && !t.is_empty() {
                                out.first_token_ms = Some(started.elapsed().as_millis() as u64);
                            }
                            out.text.push_str(t);
                        }
                    }
                    Some("message_delta") => {
                        if let Some(s) = v
                            .get("delta")
                            .and_then(|d| d.get("stop_reason"))
                            .and_then(Value::as_str)
                        {
                            out.stop_reason = Some(s.to_string());
                        }
                    }
                    Some("error") => {
                        out.error = Some(
                            v.get("error")
                                .and_then(|e| e.get("message"))
                                .and_then(Value::as_str)
                                .unwrap_or("error event")
                                .to_string(),
                        );
                    }
                    _ => {}
                }
            }
        }
    }
    out.total_ms = started.elapsed().as_millis() as u64;
    out
}

pub fn extract_error(body: &str) -> Option<String> {
    serde_json::from_str::<Value>(body).ok().and_then(|v| {
        v.get("error")
            .and_then(|e| e.get("message"))
            .and_then(Value::as_str)
            .map(str::to_string)
    })
}

/// `GET /api/v1/files` row for one id, or `None`.
pub async fn file_row(base: &str, key: &str, group_key: &str, id: i64) -> Option<Value> {
    let resp = client()
        .get(format!("{base}/api/v1/files"))
        .query(&[("group_key", group_key), ("limit", "100")])
        .header("x-api-key", key)
        .send()
        .await
        .ok()?;
    let body: Value = resp.json().await.ok()?;
    body.get("files")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|f| f.get("id").and_then(Value::as_i64) == Some(id))
        .cloned()
}

/// The desktop's Pair screen, headless: login → mint code → exchange. Writes
/// the creds file (mode 0600 on Unix) and returns the device id.
pub async fn pair(
    base: &str,
    email: &str,
    password: &str,
    name: &str,
    creds: &Path,
) -> Result<i64, String> {
    let client = client();
    let login = client
        .post(format!("{base}/api/v1/auth/login"))
        .json(&json!({ "email": email, "password": password }))
        .send()
        .await
        .map_err(|e| format!("login: {e}"))?;
    if login.status().as_u16() != 200 {
        return Err(format!(
            "login failed with HTTP {}",
            login.status().as_u16()
        ));
    }
    let cookie = login
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .filter_map(|v| v.split(';').next())
        .collect::<Vec<_>>()
        .join("; ");
    if cookie.is_empty() {
        return Err("login returned no session cookie".into());
    }
    let mint = client
        .post(format!("{base}/api/v1/desktop/pairing-codes"))
        .header("cookie", &cookie)
        .send()
        .await
        .map_err(|e| format!("mint: {e}"))?;
    if mint.status().as_u16() != 201 {
        return Err(format!(
            "pairing-code creation failed with HTTP {} (is DESKTOP_AGENT.ENABLED on?)",
            mint.status().as_u16()
        ));
    }
    let code = mint
        .json::<Value>()
        .await
        .ok()
        .and_then(|v| v.get("code").and_then(Value::as_str).map(str::to_string))
        .ok_or("pairing code missing")?;
    let paired = client
        .post(format!("{base}/api/v1/desktop/pair"))
        .json(&json!({ "code": code, "deviceName": name, "capabilities": ["skill.run"] }))
        .send()
        .await
        .map_err(|e| format!("pair: {e}"))?;
    if paired.status().as_u16() != 201 {
        return Err(format!(
            "pair failed with HTTP {}",
            paired.status().as_u16()
        ));
    }
    let body: Value = paired.json().await.map_err(|e| e.to_string())?;
    let key = body
        .get("key")
        .and_then(Value::as_str)
        .ok_or("pair response without key")?;
    let device = body.get("deviceId").and_then(Value::as_i64).unwrap_or(0);
    let api_base = body
        .get("apiBaseUrl")
        .and_then(Value::as_str)
        .unwrap_or(base);
    let text = format!(
        "DESKTOP_KEY={key}\nDEVICE_ID={device}\nAPI_BASE_URL={api_base}\nUSER_EMAIL={email}\n"
    );
    std::fs::write(creds, text).map_err(|e| format!("write creds: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(creds, std::fs::Permissions::from_mode(0o600));
    }
    Ok(device)
}
