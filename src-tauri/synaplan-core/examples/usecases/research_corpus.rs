//! B1 export: one Desktop agent turn per smoke item, Web on, no skill.
//!
//! The scorer imports `{ "items": [ { "id", "text" } ] }` and does not call a
//! model itself (`B1Importer`).

use std::path::Path;

use serde::Deserialize;
use serde_json::json;

use crate::uc_agent::agent_turn;
use crate::Ctx;

#[derive(Debug, Deserialize)]
struct CorpusItem {
    id: String,
    question: String,
    #[serde(default)]
    smoke: bool,
}

/// Run every `smoke: true` item with the workspace default chat model and
/// write the B1 import file. Returns the path written.
pub async fn export_b1(ctx: &Ctx, corpus: &Path, out: &Path) -> Result<usize, String> {
    let raw =
        std::fs::read_to_string(corpus).map_err(|e| format!("read {}: {e}", corpus.display()))?;
    let items: Vec<CorpusItem> =
        serde_json::from_str(&raw).map_err(|e| format!("corpus is not a JSON list: {e}"))?;
    let smoke: Vec<CorpusItem> = items.into_iter().filter(|item| item.smoke).collect();
    if smoke.is_empty() {
        return Err("corpus has no smoke items".into());
    }
    let model = ctx
        .catalog
        .as_ref()
        .and_then(|c| c.defaults.get("CHAT"))
        .cloned()
        .or_else(|| ctx.models.first().cloned())
        .ok_or_else(|| "no chat model for the B1 export".to_string())?;
    println!("B1 export — {} smoke items on {model}", smoke.len());
    let mut rows = Vec::new();
    for item in &smoke {
        println!("  {} …", item.id);
        let run = agent_turn(ctx, &model, "b1", &item.question, true).await;
        if let Some(err) = &run.error {
            println!("  {} error: {err}", item.id);
        }
        rows.push(json!({ "id": item.id, "text": run.text }));
    }
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let body = json!({ "items": rows });
    std::fs::write(
        out,
        serde_json::to_string_pretty(&body).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("write {}: {e}", out.display()))?;
    println!("B1 import: {}", out.display());
    Ok(smoke.len())
}
