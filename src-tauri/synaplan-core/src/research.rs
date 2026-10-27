//! Research runs against `/v1/research` (`research_protocol: 1`).
//!
//! The API key stays in the caller. This module parses the vendored fixtures,
//! builds the desktop request (no `chat_id` — the API-key route rejects it),
//! and polls the event stream. Unknown event names are kept and ignored.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

use crate::http;
use crate::projects::ProjectModels;

pub const PROTOCOL: u32 = 1;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ResearchError {
    #[error("This computer was disconnected. Pair again.")]
    Unauthorized,
    #[error("Could not reach Synaplan. Check your connection.")]
    Network,
    #[error("That research was not found.")]
    NotFound,
    #[error("{0}")]
    Server(String),
}

impl ResearchError {
    pub fn code(&self) -> &'static str {
        match self {
            ResearchError::Unauthorized => "unauthorized",
            ResearchError::Network => "unreachable",
            ResearchError::NotFound => "not_found",
            ResearchError::Server(_) => "server",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchProvider {
    pub id: String,
    #[serde(default)]
    pub self_hosted: bool,
    #[serde(default)]
    pub country: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ResearchLimits {
    #[serde(default)]
    pub standard_per_day: Option<u32>,
    #[serde(default)]
    pub thorough_per_day: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Capabilities {
    pub enabled: bool,
    pub research_protocol: u32,
    pub depths: Vec<String>,
    pub web_search_available: bool,
    #[serde(default)]
    pub web_search_provider: Option<WebSearchProvider>,
    pub knowledge_available: bool,
    #[serde(default)]
    pub knowledge_index_model: Option<String>,
    pub read_documents: bool,
    pub strict_world: bool,
    pub max_active_thorough: u32,
    pub limits: ResearchLimits,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunRef {
    pub id: String,
    pub status: String,
    pub research_protocol: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResearchFrame {
    pub seq: Option<u64>,
    pub name: String,
    pub event: ResearchEvent,
    pub data: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResearchEvent {
    Run {
        id: String,
        status: String,
        depth: String,
        research_protocol: u32,
    },
    Queued {
        since_seconds: u64,
    },
    Brief,
    Question {
        id: String,
        text: String,
        options: Vec<String>,
        allow_text: bool,
    },
    Plan {
        points: Vec<PlanPoint>,
        disclosures: Vec<String>,
        minutes: Option<u32>,
        cost: Option<f64>,
    },
    PlanWaiting,
    Search {
        point: String,
        query: String,
        scope: String,
        results: u64,
    },
    Source {
        id: String,
        kind: String,
        title: String,
        url: Option<String>,
        host: Option<String>,
        file_id: Option<String>,
        status: String,
        reason: Option<String>,
    },
    Round {
        n: u64,
        max: u64,
    },
    Point {
        id: String,
        status: String,
    },
    Writing {
        section: String,
    },
    Section {
        title: String,
        index: u64,
        total: u64,
    },
    Checking,
    Check {
        statements: u64,
        supported: u64,
        softened: u64,
        removed: u64,
    },
    Complete {
        outcome: Outcome,
    },
    Stopped {
        outcome: Outcome,
    },
    Failed {
        outcome: Outcome,
    },
    Ping,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanPoint {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    pub code: String,
    #[serde(default)]
    pub params: serde_json::Map<String, Value>,
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResearchInput {
    Answer {
        question: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        choice: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
    },
    SkipQuestion {
        question: String,
    },
    ApprovePlan,
    EditPlan {
        points: Vec<PlanEdit>,
    },
    StopGathering,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanEdit {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FollowUp {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunState {
    pub id: String,
    pub status: String,
    pub depth: String,
    pub question: String,
    #[serde(default)]
    pub report: Option<ReportBody>,
    #[serde(default)]
    pub sources: Vec<SourceRow>,
    #[serde(default)]
    pub outcome: Option<Outcome>,
    #[serde(default)]
    pub check: Option<CheckBody>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportBody {
    pub markdown: String,
    #[serde(default = "one")]
    pub version: u32,
    #[serde(default)]
    pub citations: Vec<Citation>,
    #[serde(default)]
    pub sources_order: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Citation {
    pub n: u32,
    #[serde(default)]
    pub marker: String,
    pub source: String,
    #[serde(default)]
    pub sentence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRow {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub file_id: Option<Value>,
    #[serde(default)]
    pub host: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckBody {
    #[serde(default)]
    pub statements: u64,
    #[serde(default)]
    pub supported: u64,
    #[serde(default)]
    pub removed: u64,
}

fn one() -> u32 {
    1
}

/// Parse one SSE body. A frame without `id:` (the live `queued` frame) keeps
/// `seq` empty so the client resumes from the last recorded id.
pub fn parse_sse(input: &str) -> Vec<ResearchFrame> {
    let mut frames = Vec::new();
    let mut block = String::new();
    for line in input.replace("\r\n", "\n").lines() {
        if line.is_empty() {
            if let Some(frame) = parse_block(&block) {
                frames.push(frame);
            }
            block.clear();
        } else {
            block.push_str(line);
            block.push('\n');
        }
    }
    if let Some(frame) = parse_block(&block) {
        frames.push(frame);
    }
    frames
}

fn parse_block(block: &str) -> Option<ResearchFrame> {
    if block.trim().is_empty() {
        return None;
    }
    let mut seq = None;
    let mut name = String::new();
    let mut data = String::new();
    for line in block.lines() {
        if let Some(rest) = line.strip_prefix("id:") {
            seq = rest.trim().parse().ok();
        } else if let Some(rest) = line.strip_prefix("event:") {
            name = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("data:") {
            data.push_str(rest.trim_start());
        }
    }
    if name.is_empty() {
        return None;
    }
    let json: Value = serde_json::from_str(&data).unwrap_or(Value::Null);
    Some(ResearchFrame {
        seq,
        event: decode_event(&name, &json),
        name,
        data: json,
    })
}

fn decode_event(name: &str, json: &Value) -> ResearchEvent {
    match name {
        "run" => ResearchEvent::Run {
            id: str_field(json, "id"),
            status: str_field(json, "status"),
            depth: str_field(json, "depth"),
            research_protocol: json
                .get("research_protocol")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
        },
        "queued" => ResearchEvent::Queued {
            since_seconds: json
                .get("since_seconds")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        },
        "brief" => ResearchEvent::Brief,
        "question" => ResearchEvent::Question {
            id: str_field(json, "id"),
            text: str_field(json, "text"),
            options: json
                .get("options")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
            allow_text: json
                .get("allow_text")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        },
        "plan" => ResearchEvent::Plan {
            points: json
                .get("points")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .map(|row| PlanPoint {
                            id: str_field(row, "id"),
                            title: str_field(row, "title"),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            disclosures: string_list(json.get("disclosures")),
            minutes: json
                .pointer("/estimate/minutes")
                .and_then(Value::as_u64)
                .map(|n| n as u32),
            cost: json.pointer("/estimate/cost").and_then(Value::as_f64),
        },
        "plan_waiting" => ResearchEvent::PlanWaiting,
        "search" => ResearchEvent::Search {
            point: str_field(json, "point"),
            query: str_field(json, "query"),
            scope: str_field(json, "scope"),
            results: json.get("results").and_then(Value::as_u64).unwrap_or(0),
        },
        "source" => ResearchEvent::Source {
            id: str_field(json, "id"),
            kind: str_field(json, "kind"),
            title: str_field(json, "title"),
            url: opt_str(json, "url"),
            host: opt_str(json, "host"),
            file_id: json.get("file_id").and_then(value_as_id),
            status: str_field(json, "status"),
            reason: opt_str(json, "reason"),
        },
        "round" => ResearchEvent::Round {
            n: json.get("n").and_then(Value::as_u64).unwrap_or(0),
            max: json.get("max").and_then(Value::as_u64).unwrap_or(0),
        },
        "point" => ResearchEvent::Point {
            id: str_field(json, "id"),
            status: str_field(json, "status"),
        },
        "writing" => ResearchEvent::Writing {
            section: str_field(json, "section"),
        },
        "section" => ResearchEvent::Section {
            title: str_field(json, "title"),
            index: json.get("index").and_then(Value::as_u64).unwrap_or(0),
            total: json.get("total").and_then(Value::as_u64).unwrap_or(0),
        },
        "checking" => ResearchEvent::Checking,
        "check" => ResearchEvent::Check {
            statements: json.get("statements").and_then(Value::as_u64).unwrap_or(0),
            supported: json.get("supported").and_then(Value::as_u64).unwrap_or(0),
            softened: json.get("softened").and_then(Value::as_u64).unwrap_or(0),
            removed: json.get("removed").and_then(Value::as_u64).unwrap_or(0),
        },
        "complete" => ResearchEvent::Complete {
            outcome: outcome_of(json),
        },
        "stopped" => ResearchEvent::Stopped {
            outcome: outcome_of(json),
        },
        "failed" => ResearchEvent::Failed {
            outcome: outcome_of(json),
        },
        "ping" => ResearchEvent::Ping,
        _ => ResearchEvent::Unknown,
    }
}

fn outcome_of(json: &Value) -> Outcome {
    json.get("outcome")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or(Outcome {
            code: String::new(),
            params: serde_json::Map::new(),
            text: String::new(),
        })
}

fn str_field(json: &Value, key: &str) -> String {
    json.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn opt_str(json: &Value, key: &str) -> Option<String> {
    json.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn value_as_id(value: &Value) -> Option<String> {
    match value {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Last recorded sequence, ignoring a `queued` frame that has no id.
pub fn last_seq(frames: &[ResearchFrame]) -> Option<u64> {
    frames.iter().filter_map(|frame| frame.seq).max()
}

/// `true` the first time `id` is claimed.
pub fn claim_attach(attached: &mut HashSet<String>, id: &str) -> bool {
    attached.insert(id.to_string())
}

pub fn report_slug(question: &str) -> String {
    let mut slug = String::new();
    for ch in question.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if (ch.is_whitespace() || ch == '-') && !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
        if slug.len() >= 48 {
            break;
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "report".to_string()
    } else {
        slug.to_string()
    }
}

pub fn version_file_name(slug: &str, version: u32) -> String {
    format!("{slug}.v{version}.md")
}

/// Providers already used by this project's eight slots, in slot order.
pub fn world_providers(models: &ProjectModels) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for value in [
        &models.chat,
        &models.voice,
        &models.speak,
        &models.vision,
        &models.image,
        &models.video,
        &models.embed,
        &models.docs,
        &models.research_lead,
        &models.research_reader,
        &models.research_checker,
    ] {
        let Some(service) = value.split(':').next() else {
            continue;
        };
        if service.is_empty() || value.split(':').count() < 3 {
            continue;
        }
        if seen.insert(service.to_string()) {
            out.push(service.to_string());
        }
    }
    out
}

/// Role picks sent with a run. An empty slot is omitted (the server chooses).
pub fn model_hints(models: &ProjectModels) -> serde_json::Map<String, Value> {
    let mut map = serde_json::Map::new();
    let lead = first_key(&models.research_lead, &models.chat);
    let reader = first_key(&models.research_reader, &models.docs);
    let checker = catalog_key(&models.research_checker);
    if let Some(lead) = lead {
        map.insert("lead".into(), Value::String(lead));
    }
    if let Some(reader) = reader {
        map.insert("reader".into(), Value::String(reader));
    }
    if let Some(checker) = checker {
        map.insert("checker".into(), Value::String(checker));
    }
    map
}

fn first_key(custom: &str, fallback: &str) -> Option<String> {
    catalog_key(custom).or_else(|| catalog_key(fallback))
}

fn catalog_key(value: &str) -> Option<String> {
    let value = value.trim();
    if value.split(':').count() >= 3 {
        Some(value.to_string())
    } else {
        None
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StartBody {
    pub question: String,
    pub depth: String,
    pub sources: Value,
    pub models: serde_json::Map<String, Value>,
    pub world: Value,
}

pub fn start_body(
    question: &str,
    depth: &str,
    web: bool,
    knowledge_key: Option<&str>,
    models: &ProjectModels,
) -> Value {
    let mut sources = serde_json::Map::new();
    sources.insert("web".into(), Value::Bool(web));
    sources.insert("library".into(), Value::Bool(false));
    if let Some(key) = knowledge_key.filter(|k| !k.is_empty()) {
        sources.insert("knowledge".into(), json!([key]));
    }
    json!({
        "question": question,
        "depth": depth,
        "sources": sources,
        "models": model_hints(models),
        "world": { "providers": world_providers(models) },
    })
}

pub fn citations_of(report: &ReportBody, sources: &[SourceRow]) -> Vec<Citation> {
    if !report.citations.is_empty() {
        return report.citations.clone();
    }
    let order: Vec<String> = if report.sources_order.is_empty() {
        sources.iter().map(|row| row.id.clone()).collect()
    } else {
        report.sources_order.clone()
    };
    let mut found = Vec::new();
    for (index, marker) in marker_numbers(&report.markdown).into_iter().enumerate() {
        let source = order
            .get(marker.saturating_sub(1) as usize)
            .cloned()
            .unwrap_or_default();
        found.push(Citation {
            n: (index + 1) as u32,
            marker: format!("[{marker}]"),
            source,
            sentence: String::new(),
        });
    }
    found
}

fn marker_numbers(markdown: &str) -> Vec<u32> {
    let mut nums = Vec::new();
    let bytes = markdown.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' {
            let mut j = i + 1;
            let start = j;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > start && j < bytes.len() && bytes[j] == b']' {
                if let Ok(n) = markdown[start..j].parse::<u32>() {
                    if n > 0 {
                        nums.push(n);
                    }
                }
                i = j;
            }
        }
        i += 1;
    }
    nums
}

pub fn write_report_files(
    dir: &Path,
    slug: &str,
    version: u32,
    markdown: &str,
    sources_json: &str,
) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let current = dir.join(format!("{slug}.md"));
    std::fs::write(&current, markdown)?;
    std::fs::write(dir.join(format!("{slug}.sources.json")), sources_json)?;
    if version > 0 {
        std::fs::write(dir.join(version_file_name(slug, version)), markdown)?;
    }
    Ok(current)
}

pub fn list_versions(dir: &Path, slug: &str) -> Vec<u32> {
    let mut versions = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return versions;
    };
    let prefix = format!("{slug}.v");
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let Some(rest) = name.strip_prefix(&prefix) else {
            continue;
        };
        let Some(number) = rest.strip_suffix(".md") else {
            continue;
        };
        if let Ok(n) = number.parse::<u32>() {
            versions.push(n);
        }
    }
    versions.sort_unstable();
    versions
}

pub fn restore_version(dir: &Path, slug: &str, version: u32) -> std::io::Result<PathBuf> {
    let from = dir.join(version_file_name(slug, version));
    let text = std::fs::read_to_string(&from)?;
    let current = dir.join(format!("{slug}.md"));
    std::fs::write(&current, text)?;
    Ok(current)
}

fn run_id_ok(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

async fn send(
    method: reqwest::Method,
    base_url: &str,
    key: &str,
    path: &str,
    body: Option<&Value>,
    extra: &[(&str, &str)],
) -> Result<reqwest::Response, ResearchError> {
    let client = http::client().map_err(|_| ResearchError::Network)?;
    let mut req = client.request(method, http::join(base_url, path));
    for (name, value) in extra {
        req = req.header(*name, *value);
    }
    req = req
        .header("x-api-key", key)
        .header("accept", "application/json");
    if let Some(body) = body {
        req = req.json(body);
    }
    req.send().await.map_err(|_| ResearchError::Network)
}

fn error_from(status: u16, text: &str) -> ResearchError {
    if status == 401 || status == 403 {
        return ResearchError::Unauthorized;
    }
    if status == 404 {
        return ResearchError::NotFound;
    }
    let message = serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|v| {
            v.pointer("/error/message")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "The research request was refused.".into());
    ResearchError::Server(message)
}

/// `Ok(None)` when research is off (404). The chip stays hidden.
pub async fn capabilities(
    base_url: &str,
    key: &str,
) -> Result<Option<Capabilities>, ResearchError> {
    let resp = send(
        reqwest::Method::GET,
        base_url,
        key,
        "/v1/research/capabilities",
        None,
        &[],
    )
    .await?;
    let status = resp.status().as_u16();
    if status == 404 {
        return Ok(None);
    }
    if !(200..300).contains(&status) {
        let text = resp.text().await.unwrap_or_default();
        return Err(error_from(status, &text));
    }
    resp.json()
        .await
        .map(Some)
        .map_err(|_| ResearchError::Network)
}

pub async fn start(
    base_url: &str,
    key: &str,
    body: &Value,
    idempotency_key: &str,
) -> Result<RunRef, ResearchError> {
    let resp = send(
        reqwest::Method::POST,
        base_url,
        key,
        "/v1/research",
        Some(body),
        &[("Idempotency-Key", idempotency_key)],
    )
    .await?;
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(error_from(status, &text));
    }
    serde_json::from_str(&text).map_err(|_| ResearchError::Server(text))
}

pub async fn state(base_url: &str, key: &str, id: &str) -> Result<RunState, ResearchError> {
    if !run_id_ok(id) {
        return Err(ResearchError::NotFound);
    }
    let resp = send(
        reqwest::Method::GET,
        base_url,
        key,
        &format!("/v1/research/{id}"),
        None,
        &[],
    )
    .await?;
    read_state(resp).await
}

pub async fn input(
    base_url: &str,
    key: &str,
    id: &str,
    input: &ResearchInput,
) -> Result<RunState, ResearchError> {
    if !run_id_ok(id) {
        return Err(ResearchError::NotFound);
    }
    let body = serde_json::to_value(input).map_err(|_| ResearchError::Network)?;
    let resp = send(
        reqwest::Method::POST,
        base_url,
        key,
        &format!("/v1/research/{id}/input"),
        Some(&body),
        &[],
    )
    .await?;
    read_state(resp).await
}

pub async fn cancel_run(base_url: &str, key: &str, id: &str) -> Result<RunState, ResearchError> {
    if !run_id_ok(id) {
        return Err(ResearchError::NotFound);
    }
    let resp = send(
        reqwest::Method::POST,
        base_url,
        key,
        &format!("/v1/research/{id}/cancel"),
        None,
        &[],
    )
    .await?;
    read_state(resp).await
}

pub async fn follow_up(
    base_url: &str,
    key: &str,
    id: &str,
    follow: &FollowUp,
    idempotency_key: &str,
) -> Result<RunRef, ResearchError> {
    if !run_id_ok(id) {
        return Err(ResearchError::NotFound);
    }
    let body = serde_json::to_value(follow).map_err(|_| ResearchError::Network)?;
    let resp = send(
        reqwest::Method::POST,
        base_url,
        key,
        &format!("/v1/research/{id}/followups"),
        Some(&body),
        &[("Idempotency-Key", idempotency_key)],
    )
    .await?;
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(error_from(status, &text));
    }
    serde_json::from_str(&text).map_err(|_| ResearchError::Server(text))
}

pub async fn list_active(base_url: &str, key: &str) -> Result<Vec<RunRef>, ResearchError> {
    let resp = send(
        reqwest::Method::GET,
        base_url,
        key,
        "/v1/research?active=1",
        None,
        &[],
    )
    .await?;
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(error_from(status, &text));
    }
    let value: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    let runs = value
        .get("runs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(runs
        .into_iter()
        .filter_map(|row| {
            Some(RunRef {
                id: row.get("id")?.as_str()?.to_string(),
                status: row
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .into(),
                research_protocol: PROTOCOL,
            })
        })
        .collect())
}

pub async fn delete_run(base_url: &str, key: &str, id: &str) -> Result<(), ResearchError> {
    if !run_id_ok(id) {
        return Err(ResearchError::NotFound);
    }
    let resp = send(
        reqwest::Method::DELETE,
        base_url,
        key,
        &format!("/v1/research/{id}"),
        None,
        &[],
    )
    .await?;
    let status = resp.status().as_u16();
    if status == 204 || (200..300).contains(&status) {
        return Ok(());
    }
    let text = resp.text().await.unwrap_or_default();
    Err(error_from(status, &text))
}

/// One poll of the event stream. The server closes after the frames it has.
pub async fn poll_events(
    base_url: &str,
    key: &str,
    id: &str,
    after: Option<u64>,
) -> Result<Vec<ResearchFrame>, ResearchError> {
    if !run_id_ok(id) {
        return Err(ResearchError::NotFound);
    }
    let header = after.map(|n| n.to_string()).unwrap_or_default();
    let extra: &[(&str, &str)] = if header.is_empty() {
        &[]
    } else {
        &[("Last-Event-ID", header.as_str())]
    };
    let resp = send(
        reqwest::Method::GET,
        base_url,
        key,
        &format!("/v1/research/{id}/events"),
        None,
        extra,
    )
    .await?;
    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        let text = resp.text().await.unwrap_or_default();
        return Err(error_from(status, &text));
    }
    let text = resp.text().await.map_err(|_| ResearchError::Network)?;
    Ok(parse_sse(&text))
}

async fn read_state(resp: reqwest::Response) -> Result<RunState, ResearchError> {
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(error_from(status, &text));
    }
    serde_json::from_str(&text)
        .map_err(|_| ResearchError::Server("The research state was not readable.".into()))
}

const SKIP_EXT: &[&str] = &[
    "mp4", "mov", "avi", "mkv", "webm", "zip", "gz", "tgz", "7z", "rar", "dmg", "exe", "iso", "app",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderFile {
    pub path: String,
    pub name: String,
    pub kind: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedFile {
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderWalk {
    pub root: String,
    pub files: Vec<FolderFile>,
    pub skipped: Vec<SkippedFile>,
    pub pdf: u32,
    pub word: u32,
    pub excel: u32,
    pub other: u32,
    pub total_bytes: u64,
}

/// Walk a folder for the knowledge upload. Videos and archives are skipped.
/// Files over `max_bytes` are skipped. Symlinks are not followed.
pub fn walk_folder(root: &Path, subfolders: bool, max_bytes: u64) -> std::io::Result<FolderWalk> {
    let mut walk = FolderWalk {
        root: root.display().to_string(),
        files: Vec::new(),
        skipped: Vec::new(),
        pdf: 0,
        word: 0,
        excel: 0,
        other: 0,
        total_bytes: 0,
    };
    walk_dir(root, subfolders, max_bytes, &mut walk)?;
    Ok(walk)
}

fn walk_dir(
    dir: &Path,
    subfolders: bool,
    max_bytes: u64,
    walk: &mut FolderWalk,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let meta = match std::fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(_) => continue,
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            if subfolders {
                walk_dir(&path, true, max_bytes, walk)?;
            }
            continue;
        }
        if !meta.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if SKIP_EXT.contains(&ext.as_str()) {
            walk.skipped.push(SkippedFile {
                name,
                reason: "unreadable".into(),
            });
            continue;
        }
        let bytes = meta.len();
        if bytes > max_bytes {
            walk.skipped.push(SkippedFile {
                name,
                reason: "too_large".into(),
            });
            continue;
        }
        let kind = match ext.as_str() {
            "pdf" => "pdf",
            "doc" | "docx" => "word",
            "xls" | "xlsx" => "excel",
            _ => "other",
        };
        match kind {
            "pdf" => walk.pdf += 1,
            "word" => walk.word += 1,
            "excel" => walk.excel += 1,
            _ => walk.other += 1,
        }
        walk.total_bytes += bytes;
        walk.files.push(FolderFile {
            path: path.display().to_string(),
            name,
            kind: kind.into(),
            bytes,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn hash(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        format!("{:x}", hasher.finalize())
    }

    #[test]
    fn vendored_research_fixtures_match_recorded_checksums() {
        let cases: &[(&str, &[u8], &str)] = &[
            (
                "capabilities.json",
                include_bytes!("../../../tests/fixtures/research-contract/capabilities.json"),
                "c57c570650eeffa3a8a814600485a32a512661167a34153262ea2fac27aa9ec4",
            ),
            (
                "events-standard.sse",
                include_bytes!("../../../tests/fixtures/research-contract/events-standard.sse"),
                "68a9921388b2185514a5f4585bb7a25d8e3eb62679ff067e9824eda3b7b4f3c2",
            ),
            (
                "events-thorough-waiting.sse",
                include_bytes!(
                    "../../../tests/fixtures/research-contract/events-thorough-waiting.sse"
                ),
                "3d37f122ae2fbb9096a477bdf7f80d143f6b51247e8f5a939a040cfd283de297",
            ),
            (
                "events-unknown.sse",
                include_bytes!("../../../tests/fixtures/research-contract/events-unknown.sse"),
                "1227fb938a901c5cc2c1e2c94becadf59e2626ce5de7e0592992bcd169a50b9d",
            ),
            (
                "follow-up-question.json",
                include_bytes!("../../../tests/fixtures/research-contract/follow-up-question.json"),
                "ef86c6c0973eca87fed4c2f80cd71916274716b188f2c0fa16c8f43c9f7a27a3",
            ),
            (
                "input-answer-choice.json",
                include_bytes!(
                    "../../../tests/fixtures/research-contract/input-answer-choice.json"
                ),
                "9ba008460a755006c6a17deb46be44fc296ae0c2b4288bab42b0e70a4cf857bd",
            ),
            (
                "input-answer-text.json",
                include_bytes!("../../../tests/fixtures/research-contract/input-answer-text.json"),
                "67ace1aae3a87ce5c0686c78055e06245e4739e504ff8cbc55bcd884adbaa755",
            ),
            (
                "input-approve-plan.json",
                include_bytes!("../../../tests/fixtures/research-contract/input-approve-plan.json"),
                "22069d05c5b04b342c16cf195ef6a42f82545ada54168e2c55f9cc67fe7fe0bd",
            ),
            (
                "input-edit-plan.json",
                include_bytes!("../../../tests/fixtures/research-contract/input-edit-plan.json"),
                "f6d5213e4eb5051c93d81c6a14520c6737cea2f01e7bed3bbf5dbf1b5c46aa46",
            ),
            (
                "input-skip-question.json",
                include_bytes!(
                    "../../../tests/fixtures/research-contract/input-skip-question.json"
                ),
                "535a279427573c2644dec0204a6cc021aa5cd83439853c9dbcf8332bc487675b",
            ),
            (
                "input-stop-gathering.json",
                include_bytes!(
                    "../../../tests/fixtures/research-contract/input-stop-gathering.json"
                ),
                "bc206d51e3a63d95da6673eda3fd60fa27743e931f4cfb36c3cd8a7cdf6ae063",
            ),
            (
                "outcomes.json",
                include_bytes!("../../../tests/fixtures/research-contract/outcomes.json"),
                "24b3d1356b10275e00267d090fbdccf8e2847ade090723b5cc7f6bf14f471f47",
            ),
            (
                "start-request.json",
                include_bytes!("../../../tests/fixtures/research-contract/start-request.json"),
                "37826df480f6bfcdc663088cf4406267f504175c8e66ef37d7792d17de9a5fd6",
            ),
            (
                "start-response.json",
                include_bytes!("../../../tests/fixtures/research-contract/start-response.json"),
                "2209e26563a715d8b06484df8e11e2c6decdf1c94bd779a4143477771b0378d7",
            ),
            (
                "state-complete.json",
                include_bytes!("../../../tests/fixtures/research-contract/state-complete.json"),
                "bfb9b3228584db492978a13dc895aa9ba877b47fe385d6f05d787142084e5006",
            ),
            (
                "state-failed.json",
                include_bytes!("../../../tests/fixtures/research-contract/state-failed.json"),
                "524ea3c86052b2d6e79005516b2e558beeef88a4f567c76d201d6b1113ae8429",
            ),
            (
                "state-gathering.json",
                include_bytes!("../../../tests/fixtures/research-contract/state-gathering.json"),
                "0acc6d2b922e87b01d08c87d73c655ca60825a46a77e987950e655ec0354221a",
            ),
        ];
        for (name, bytes, expected) in cases {
            assert_eq!(&hash(bytes), expected, "{name} drifted");
        }
    }

    #[test]
    fn standard_events_parse_and_resume_after_the_last_id() {
        let frames = parse_sse(include_str!(
            "../../../tests/fixtures/research-contract/events-standard.sse"
        ));
        assert!(frames
            .iter()
            .all(|frame| frame.event != ResearchEvent::Unknown));
        assert!(matches!(
            frames.last().unwrap().event,
            ResearchEvent::Queued { since_seconds: 20 }
        ));
        assert_eq!(frames.last().unwrap().seq, None);
        assert_eq!(last_seq(&frames), Some(12));
        let thorough = parse_sse(include_str!(
            "../../../tests/fixtures/research-contract/events-thorough-waiting.sse"
        ));
        assert!(matches!(thorough[2].event, ResearchEvent::Question { .. }));
        assert!(matches!(
            thorough.last().unwrap().event,
            ResearchEvent::PlanWaiting
        ));
        let unknown = parse_sse(include_str!(
            "../../../tests/fixtures/research-contract/events-unknown.sse"
        ));
        assert!(matches!(unknown[1].event, ResearchEvent::Unknown));
    }

    #[test]
    fn states_and_capabilities_parse() {
        let caps: Capabilities = serde_json::from_str(include_str!(
            "../../../tests/fixtures/research-contract/capabilities.json"
        ))
        .unwrap();
        assert_eq!(caps.research_protocol, PROTOCOL);
        assert!(caps.depths.contains(&"standard".to_string()));
        let gathering: RunState = serde_json::from_str(include_str!(
            "../../../tests/fixtures/research-contract/state-gathering.json"
        ))
        .unwrap();
        assert!(gathering.report.is_none());
        let done: RunState = serde_json::from_str(include_str!(
            "../../../tests/fixtures/research-contract/state-complete.json"
        ))
        .unwrap();
        let report = done.report.unwrap();
        assert_eq!(citations_of(&report, &done.sources).len(), 1);
        let failed: RunState = serde_json::from_str(include_str!(
            "../../../tests/fixtures/research-contract/state-failed.json"
        ))
        .unwrap();
        assert_eq!(failed.outcome.unwrap().code, "failed_no_sources");
        let outcomes: Vec<Outcome> = serde_json::from_str(include_str!(
            "../../../tests/fixtures/research-contract/outcomes.json"
        ))
        .unwrap();
        assert_eq!(outcomes.len(), 13);
    }

    #[test]
    fn input_and_follow_up_match_the_fixtures() {
        let cases = [
            (
                ResearchInput::Answer {
                    question: "q1".into(),
                    choice: Some("EU".into()),
                    text: None,
                },
                include_str!("../../../tests/fixtures/research-contract/input-answer-choice.json"),
            ),
            (
                ResearchInput::Answer {
                    question: "q1".into(),
                    choice: None,
                    text: Some("Austria and Switzerland".into()),
                },
                include_str!("../../../tests/fixtures/research-contract/input-answer-text.json"),
            ),
            (
                ResearchInput::SkipQuestion {
                    question: "q1".into(),
                },
                include_str!("../../../tests/fixtures/research-contract/input-skip-question.json"),
            ),
            (
                ResearchInput::ApprovePlan,
                include_str!("../../../tests/fixtures/research-contract/input-approve-plan.json"),
            ),
            (
                ResearchInput::EditPlan {
                    points: vec![
                        PlanEdit {
                            id: Some("p1".into()),
                            title: None,
                        },
                        PlanEdit {
                            id: None,
                            title: Some("Delivery times".into()),
                        },
                    ],
                },
                include_str!("../../../tests/fixtures/research-contract/input-edit-plan.json"),
            ),
            (
                ResearchInput::StopGathering,
                include_str!("../../../tests/fixtures/research-contract/input-stop-gathering.json"),
            ),
        ];
        for (input, fixture) in cases {
            let got: Value = serde_json::to_value(&input).unwrap();
            let want: Value = serde_json::from_str(fixture).unwrap();
            assert_eq!(got, want);
        }
        let follow = FollowUp {
            kind: "question".into(),
            text: Some("What about delivery times?".into()),
        };
        let got: Value = serde_json::to_value(&follow).unwrap();
        let want: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/research-contract/follow-up-question.json"
        ))
        .unwrap();
        assert_eq!(got, want);
    }

    #[test]
    fn a_desktop_start_sends_roles_and_omits_chat_id() {
        let models = ProjectModels {
            chat: "anthropic:claude-sonnet:chat".into(),
            docs: "openai:gpt:chat".into(),
            research_checker: "groq:llama:chat".into(),
            voice: "openai:whisper:sound2text".into(),
            ..Default::default()
        };
        let body = start_body(
            "Which suppliers?",
            "standard",
            true,
            Some("DESKTOP:42"),
            &models,
        );
        assert!(body.get("chat_id").is_none());
        assert_eq!(body["models"]["lead"], "anthropic:claude-sonnet:chat");
        assert_eq!(body["models"]["reader"], "openai:gpt:chat");
        assert_eq!(body["models"]["checker"], "groq:llama:chat");
        assert_eq!(
            body["world"]["providers"],
            json!(["anthropic", "openai", "groq"])
        );
        assert_eq!(body["sources"]["knowledge"], json!(["DESKTOP:42"]));
    }

    #[test]
    fn citations_are_rebuilt_from_markers_when_the_state_omits_them() {
        let report = ReportBody {
            markdown: "One. [1] Two. [3]".into(),
            version: 1,
            citations: vec![],
            sources_order: vec!["s1".into(), "s2".into(), "s3".into()],
        };
        let cites = citations_of(&report, &[]);
        assert_eq!(cites[0].source, "s1");
        assert_eq!(cites[1].marker, "[3]");
        assert_eq!(cites[1].source, "s3");
    }

    #[test]
    fn report_files_keep_versions_and_restore() {
        let dir = std::env::temp_dir().join(format!("syn-research-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = write_report_files(&dir, "prices", 1, "v1\n", "{\"sources\":[]}").unwrap();
        assert!(path.ends_with("prices.md"));
        write_report_files(&dir, "prices", 2, "v2\n", "{\"sources\":[]}").unwrap();
        assert_eq!(list_versions(&dir, "prices"), vec![1, 2]);
        restore_version(&dir, "prices", 1).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("prices.md")).unwrap(),
            "v1\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(
            report_slug("Which suppliers deliver X?"),
            "which-suppliers-deliver-x"
        );
        assert_eq!(version_file_name("prices", 2), "prices.v2.md");
    }

    #[test]
    fn folder_walk_skips_archives_and_counts_types() {
        let dir = std::env::temp_dir().join(format!("syn-folder-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("nested")).unwrap();
        std::fs::write(dir.join("a.pdf"), b"pdf").unwrap();
        std::fs::write(dir.join("b.docx"), b"doc").unwrap();
        std::fs::write(dir.join("c.xlsx"), b"xls").unwrap();
        std::fs::write(dir.join("d.txt"), b"txt").unwrap();
        std::fs::write(dir.join("clip.mp4"), b"vid").unwrap();
        std::fs::write(dir.join("nested").join("e.pdf"), b"sub").unwrap();
        let flat = walk_folder(&dir, false, 100).unwrap();
        assert_eq!(flat.pdf, 1);
        assert_eq!(flat.word, 1);
        assert_eq!(flat.excel, 1);
        assert_eq!(flat.other, 1);
        assert_eq!(flat.skipped.len(), 1);
        let deep = walk_folder(&dir, true, 100).unwrap();
        assert_eq!(deep.pdf, 2);
        let huge = walk_folder(&dir, false, 1).unwrap();
        assert!(huge.files.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn each_active_run_is_attached_once() {
        let mut attached = HashSet::new();
        assert!(claim_attach(&mut attached, "rr_a"));
        assert!(claim_attach(&mut attached, "rr_b"));
        assert!(!claim_attach(&mut attached, "rr_a"));
    }
}
