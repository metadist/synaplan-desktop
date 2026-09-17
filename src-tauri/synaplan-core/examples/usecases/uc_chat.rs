//! UC-00 preflight, UC-01 model matrix (plain chat), UC-09 long answer,
//! UC-10 error copy. All go through `messages::stream_chat` — the function
//! behind the Tauri `send_chat` command — with the same `max_tokens 1024`.

use std::sync::atomic::AtomicBool;
use std::time::Instant;

use futures_util::future::join_all;
use serde_json::json;
use synaplan_core::catalog::{fetch_catalog, ModelCatalog};
use synaplan_core::messages::{stream_chat, ChatError, TurnContext};
use synaplan_core::ChatEvent;

use crate::raw::{self, user};
use crate::report::{snippet, Case, CaseResult};
use crate::Ctx;

/// The desktop's plain-chat output budget (`send_chat` in `commands/mod.rs`).
pub const PLAIN_CHAT_MAX_TOKENS: u32 = 1024;
const MATRIX_CONCURRENCY: usize = 4;
const FIRST_TOKEN_CEILING_MS: u64 = 15_000;
const TOTAL_CEILING_MS: u64 = 60_000;

/// One `stream_chat` turn as the UI would receive it.
#[derive(Debug, Default)]
pub struct ClientTurn {
    pub text: String,
    pub error: Option<ChatError>,
    pub first_token_ms: Option<u64>,
    pub total_ms: u64,
    pub done: bool,
}

pub async fn client_turn(
    ctx: &Ctx,
    turn: &TurnContext,
    messages: &[synaplan_core::messages::ChatMessage],
    tools: Option<&[serde_json::Value]>,
) -> ClientTurn {
    let started = Instant::now();
    let cancel = AtomicBool::new(false);
    let mut out = ClientTurn::default();
    let result = stream_chat(
        &ctx.base,
        &ctx.key,
        turn,
        messages,
        PLAIN_CHAT_MAX_TOKENS,
        tools,
        &cancel,
        |event| match event {
            ChatEvent::Token(t) => {
                if out.first_token_ms.is_none() && !t.is_empty() {
                    out.first_token_ms = Some(started.elapsed().as_millis() as u64);
                }
                out.text.push_str(&t);
            }
            ChatEvent::Done => out.done = true,
            ChatEvent::Error(_) => {}
        },
    )
    .await;
    if let Err(e) = result {
        out.error = Some(e);
    }
    out.total_ms = started.elapsed().as_millis() as u64;
    out
}

pub async fn uc00_preflight(ctx: &Ctx) -> (CaseResult, Option<ModelCatalog>) {
    let mut case = Case::new("UC-00", "Open the app: models and workspace reachable");
    println!("\n== UC-00");
    let catalog = match fetch_catalog(&ctx.base, &ctx.key).await {
        Ok(c) => c,
        Err(e) => {
            case.check("catalog reachable", false, format!("{e}"));
            return (case.finish("workspace not reachable"), None);
        }
    };
    case.check(
        "catalog route present",
        !catalog.catalog_missing,
        if catalog.catalog_missing {
            "/v1/models/catalog is 404; the client fell back to /v1/models"
        } else {
            "/v1/models/catalog answered"
        },
    );
    let chat: Vec<_> = catalog.slots.get("chat").into_iter().flatten().collect();
    let chat_available = chat.iter().filter(|e| e.available).count();
    case.metric("chat_models_total", chat.len() as u64);
    case.metric("chat_models_available", chat_available as u64);
    case.check(
        "at least one chat model available",
        chat_available > 0,
        format!("{chat_available} of {} CHAT entries available", chat.len()),
    );
    for (cap, slot) in [
        ("VECTORIZE", "embed"),
        ("SOUND2TEXT", "voice"),
        ("TEXT2SOUND", "speak"),
    ] {
        let default = catalog.defaults.get(cap).cloned().unwrap_or_default();
        let entry = catalog
            .slots
            .get(slot)
            .into_iter()
            .flatten()
            .find(|e| e.id == default);
        let available = entry.map(|e| e.available).unwrap_or(false);
        let reason = entry
            .and_then(|e| e.unavailable_reason.clone())
            .unwrap_or_default();
        case.metric(format!("default_{}", cap.to_lowercase()), default.clone());
        case.check(
            format!("{cap} default is available"),
            available,
            if default.is_empty() {
                "no default advertised".to_string()
            } else if available {
                default.clone()
            } else {
                format!("{default} is listed as unavailable ({reason}) — every knowledge-folder upload indexes with it")
            },
        );
    }
    (
        case.finish(format!("{chat_available} chat models available")),
        Some(catalog),
    )
}

/// UC-01: "Reply with the single word PONG." on every model in the matrix.
pub async fn uc01_model_matrix(ctx: &Ctx) -> CaseResult {
    let mut case = Case::new("UC-01", "Pick a chat model, say hello");
    let models = crate::with_default_first(ctx, ctx.models.clone());
    let mut rows: Vec<(String, ClientTurn)> = Vec::new();
    for chunk in models.chunks(MATRIX_CONCURRENCY) {
        let futs = chunk.iter().map(|m| async move {
            let turn = TurnContext::model(ctx.wire(m));
            let msgs = [user("Reply with the single word PONG.")];
            (m.clone(), client_turn(ctx, &turn, &msgs, None).await)
        });
        rows.extend(join_all(futs).await);
    }
    let mut ok_models = 0usize;
    for (model, t) in &rows {
        let answered = t.error.is_none() && !t.text.trim().is_empty();
        let pong = t.text.to_uppercase().contains("PONG");
        let ttft = t.first_token_ms.unwrap_or(u64::MAX);
        let fast = ttft <= FIRST_TOKEN_CEILING_MS && t.total_ms <= TOTAL_CEILING_MS;
        let detail = match &t.error {
            Some(e) => format!("error shown to the user: \"{e}\" ({})", e.code()),
            None => format!(
                "ttft {} ms, total {} ms, text \"{}\"",
                t.first_token_ms.unwrap_or(0),
                t.total_ms,
                snippet(&t.text, 40)
            ),
        };
        let pass = case.check(format!("{model} answers"), answered && pong && fast, detail);
        if pass {
            ok_models += 1;
        }
        case.metric(
            format!("ttft_ms {model}"),
            t.first_token_ms.map(|v| json!(v)).unwrap_or(json!(null)),
        );
        case.metric(format!("total_ms {model}"), t.total_ms);
    }
    case.finish(format!("{ok_models}/{} models answered PONG", rows.len()))
}

/// UC-09: the desktop asks with `max_tokens 1024`; a long answer is cut. The
/// raw stream says `max_tokens`; the client must not pretend it finished.
pub async fn uc09_long_answer(ctx: &Ctx) -> CaseResult {
    let mut case = Case::new("UC-09", "Ask for a long answer");
    let model = crate::with_default_first(ctx, ctx.models.clone())
        .into_iter()
        .find(|m| m.starts_with("groq:"))
        .or_else(|| ctx.models.first().cloned())
        .unwrap_or_else(|| "groq:openai/gpt-oss-120b:chat".into());
    let turn = TurnContext::model(ctx.wire(&model));
    let msgs = [user(
        "Write a 1500-word essay about the history of container shipping in Northern Europe. Do not summarise; write the full essay.",
    )];
    let contract = raw::stream_capture(
        &ctx.base,
        &ctx.key,
        &turn,
        &msgs,
        PLAIN_CHAT_MAX_TOKENS,
        None,
    )
    .await;
    let client = client_turn(ctx, &turn, &msgs, None).await;
    case.metric("model", model.clone());
    case.metric(
        "raw_stop_reason",
        contract.stop_reason.clone().unwrap_or_default(),
    );
    case.metric(
        "client_words",
        client.text.split_whitespace().count() as u64,
    );
    let cut = contract.stop_reason.as_deref() == Some("max_tokens");
    case.check(
        "raw stream reports how the answer ended",
        contract.stop_reason.is_some(),
        format!("stop_reason={:?}", contract.stop_reason),
    );
    if cut {
        // The client has no signal today: `SseParser` maps message_stop to Done
        // and drops message_delta. A person sees a sentence end mid-air.
        case.check(
            "client is told the answer was cut off",
            false,
            format!(
                "raw stop_reason=max_tokens after {} words; stream_chat delivered Done with no truncation signal",
                client.text.split_whitespace().count()
            ),
        );
    } else {
        case.check(
            "answer completed within the budget",
            client.error.is_none() && client.done,
            format!("stop_reason={:?}", contract.stop_reason),
        );
    }
    case.finish(format!(
        "{} words, stop_reason {:?}",
        client.text.split_whitespace().count(),
        contract.stop_reason
    ))
}

/// UC-10: what does the person read when a turn fails?
pub async fn uc10_error_copy(ctx: &Ctx) -> CaseResult {
    let mut case = Case::new("UC-10", "Something goes wrong: what does the person read?");
    // 1. Unknown model → the friendly "Choose a model" copy.
    let unknown = client_turn(
        ctx,
        &TurnContext::model("no-such-model-usecases"),
        &[user("hi")],
        None,
    )
    .await;
    let friendly = matches!(unknown.error, Some(ChatError::ModelUnavailable));
    case.check(
        "unknown model → friendly copy",
        friendly,
        unknown
            .error
            .as_ref()
            .map(|e| format!("\"{e}\" ({})", e.code()))
            .unwrap_or_else(|| "no error at all".into()),
    );
    // 2. Every model in the matrix that fails must fail in plain language.
    let mut raw_texts = Vec::new();
    for model in &ctx.models {
        let t = client_turn(
            ctx,
            &TurnContext::model(ctx.wire(model)),
            &[user("Reply with OK.")],
            None,
        )
        .await;
        if let Some(e) = &t.error {
            let msg = e.to_string();
            let technical = [
                "Unsupported parameter",
                "max_completion_tokens",
                "max_tokens",
                "status 4",
                "status 5",
                "invalid_request",
                "Parsing failed",
                "JSON",
            ]
            .iter()
            .any(|needle| msg.contains(needle));
            raw_texts.push(format!("{model}: \"{}\"", snippet(&msg, 120)));
            case.check(
                format!("{model} failure is plain language"),
                !technical,
                format!("user sees: \"{}\"", snippet(&msg, 160)),
            );
        }
    }
    case.metric("failing_models", raw_texts.len() as u64);
    case.finish(if raw_texts.is_empty() {
        "no model failed; unknown-model copy checked".to_string()
    } else {
        format!("{} model(s) failed — copy checked", raw_texts.len())
    })
}
