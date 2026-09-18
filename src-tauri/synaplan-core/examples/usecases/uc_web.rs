//! UC-03: Web toggle on, a current-events question in plain chat. The client
//! view is `stream_chat` with the `web_search` server tool declared (exactly
//! `send_chat` with `project.web_search = true`); the contract view is the raw
//! SSE: did the turn end in text, or did the gateway hand back a tool call
//! the desktop cannot serve?

use synaplan_core::agent::web_server_tools;
use synaplan_core::messages::TurnContext;

use crate::raw::{self, user};
use crate::report::{snippet, Case, CaseResult};
use crate::uc_chat::{client_turn, PLAIN_CHAT_MAX_TOKENS};
use crate::Ctx;

pub const WEB_QUESTION: &str = "Use web search: what is the current LTS version of Node.js? Answer in two sentences and cite the URL you used.";
const ROUNDS_CEILING: usize = 4;
const TOTAL_CEILING_MS: u64 = 60_000;

pub async fn uc03_web_search_plain(ctx: &Ctx) -> CaseResult {
    let mut case = Case::new("UC-03", "Turn Web on, ask a current-events question");
    let tools: Vec<_> = web_server_tools()
        .into_iter()
        .map(|t| t.to_declaration())
        .collect();
    let models = crate::with_default_first(ctx, ctx.models.clone());
    let mut good = 0usize;
    for model in &models {
        let turn = TurnContext::model(ctx.wire(model));
        let msgs = [user(WEB_QUESTION)];
        let contract = raw::stream_capture(
            &ctx.base,
            &ctx.key,
            &turn,
            &msgs,
            PLAIN_CHAT_MAX_TOKENS,
            Some(&tools),
        )
        .await;
        let client = client_turn(ctx, &turn, &msgs, Some(&tools)).await;

        let has_url = client.text.contains("http");
        let ended_in_text = contract
            .stop_reason
            .as_deref()
            .is_some_and(|s| s != "tool_use")
            && contract.error.is_none();
        let handed_back = contract.stop_reason.as_deref() == Some("tool_use");
        let rounds = contract.pings.max(1);
        let detail = format!(
            "stop_reason={:?} rounds≈{} error={:?} client_text=\"{}\" total {} ms",
            contract.stop_reason,
            contract.pings,
            contract.error.as_deref().map(|e| snippet(e, 100)),
            snippet(&client.text, 90),
            contract.total_ms
        );
        let ok = case.check(
            format!("{model}: answer with a source"),
            client.error.is_none() && !client.text.trim().is_empty() && has_url && ended_in_text,
            detail,
        );
        if handed_back {
            case.check(
                format!("{model}: gateway never returns an unserved web_search"),
                false,
                format!(
                    "final stop_reason=tool_use with tool_use {:?} after {} rounds — the client cannot run a server tool; the person saw only \"{}\"",
                    contract.tool_use_names,
                    contract.pings,
                    snippet(&client.text, 60)
                ),
            );
        }
        if rounds > ROUNDS_CEILING || contract.total_ms > TOTAL_CEILING_MS {
            case.check(
                format!("{model}: within budget"),
                false,
                format!("{} gateway rounds, {} ms (ceiling {ROUNDS_CEILING} rounds / {TOTAL_CEILING_MS} ms)", contract.pings, contract.total_ms),
            );
        }
        if ok {
            good += 1;
        }
        case.metric(format!("rounds {model}"), contract.pings as u64);
        case.metric(format!("total_ms {model}"), contract.total_ms);
        if ctx.verbose {
            ctx.log(&format!("{model} → {}", snippet(&client.text, 400)));
        }
    }
    case.finish(format!(
        "{good}/{} models answered with a source",
        models.len()
    ))
}
