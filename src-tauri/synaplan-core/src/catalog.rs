//! The model catalog behind "This project's models" (`GET /v1/models/catalog`).
//!
//! The workspace tells the desktop which models exist per use (chat, dictation,
//! index files, …) together with their provider and whether they are ready.
//! Each entry's `id` is the stable catalog key `service:providerId:tag`; the UI
//! shows the provider id. When the workspace does not offer the catalog yet
//! (404), the desktop keeps the persisted picks and says so — it never builds a
//! fake catalog. Two narrow, labelled fallbacks exist for the slots a user needs
//! before the catalog ships: `/v1/audio/models` for dictation (it carries the
//! tag, so a real catalog key can be formed) and the flat `/v1/models` list for
//! chat (bare provider ids, remembered as legacy picks and rebound later).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::http;
use crate::projects::{is_catalog_key, ModelSlot, ProjectModels};

/// One selectable model as the workspace advertises it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogEntry {
    /// Catalog key `service:providerId:tag`, or a bare provider id when the
    /// entry came from the flat `/v1/models` fallback.
    pub id: String,
    pub provider_id: String,
    pub service: String,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_true")]
    pub available: bool,
    #[serde(default)]
    pub unavailable_reason: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Where a slot's entries came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotSource {
    /// `GET /v1/models/catalog` — the source of truth.
    Catalog,
    /// `GET /v1/audio/models` — dictation models with a real catalog key.
    AudioModels,
    /// `GET /v1/models` — flat chat list; bare ids, no availability.
    FlatModels,
    /// Nothing reachable; the slot shows only the persisted pick.
    None,
}

/// The eight slots with their entries and provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCatalog {
    /// Keyed by UI slot id (`chat`, `voice`, …). Every slot is present.
    pub slots: BTreeMap<String, Vec<CatalogEntry>>,
    /// Provenance per slot id. `catalog` for all eight when the route exists.
    pub sources: BTreeMap<String, SlotSource>,
    /// `true` when the workspace does not offer `/v1/models/catalog` yet.
    pub catalog_missing: bool,
    /// Workspace DEFAULTMODEL catalog keys, keyed by capability (`VECTORIZE`, …).
    /// Absent when the workspace is older than the `defaults` field.
    #[serde(default)]
    pub defaults: BTreeMap<String, String>,
}

impl ModelCatalog {
    fn empty(source: SlotSource) -> Self {
        let mut slots = BTreeMap::new();
        let mut sources = BTreeMap::new();
        for slot in ModelSlot::ALL {
            slots.insert(slot.id().to_string(), Vec::new());
            sources.insert(slot.id().to_string(), source);
        }
        Self {
            slots,
            sources,
            catalog_missing: false,
            defaults: BTreeMap::new(),
        }
    }

    pub fn entries(&self, slot: ModelSlot) -> &[CatalogEntry] {
        self.slots.get(slot.id()).map(Vec::as_slice).unwrap_or(&[])
    }

    /// The workspace DEFAULTMODEL key for this slot, when the catalog sent one.
    pub fn default_id(&self, slot: ModelSlot) -> Option<&str> {
        self.defaults
            .get(slot.capability())
            .map(String::as_str)
            .filter(|id| is_catalog_key(id))
    }

    fn set(&mut self, slot: ModelSlot, entries: Vec<CatalogEntry>, source: SlotSource) {
        self.slots.insert(slot.id().to_string(), entries);
        self.sources.insert(slot.id().to_string(), source);
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CatalogError {
    #[error("This computer was disconnected. Pair again.")]
    Unauthorized,
    #[error("Could not reach Synaplan. Check your connection.")]
    Network,
    #[error("The model list could not be read: {0}")]
    Malformed(String),
    #[error("{0}")]
    Server(String),
}

impl CatalogError {
    pub fn code(&self) -> &'static str {
        match self {
            CatalogError::Unauthorized => "unauthorized",
            CatalogError::Network => "network",
            CatalogError::Malformed(_) => "catalog_malformed",
            CatalogError::Server(_) => "server",
        }
    }
}

#[derive(Debug, Deserialize)]
struct CatalogResponse {
    #[serde(default)]
    capabilities: BTreeMap<String, Vec<CatalogEntry>>,
    #[serde(default)]
    defaults: BTreeMap<String, String>,
}

/// Map the server's capability groups onto the eight UI slots. Groups the
/// desktop does not use are ignored; missing groups become empty slots.
pub fn parse_catalog(body: &str) -> Result<ModelCatalog, CatalogError> {
    let parsed: CatalogResponse =
        serde_json::from_str(body).map_err(|e| CatalogError::Malformed(e.to_string()))?;
    let mut catalog = ModelCatalog::empty(SlotSource::Catalog);
    for (capability, entries) in parsed.capabilities {
        if let Some(slot) = ModelSlot::from_capability(&capability) {
            let entries = entries
                .into_iter()
                .filter(|e| !e.id.is_empty())
                .collect::<Vec<_>>();
            catalog.set(slot, entries, SlotSource::Catalog);
        }
    }
    catalog.defaults = parsed
        .defaults
        .into_iter()
        .filter(|(capability, id)| {
            ModelSlot::from_capability(capability).is_some() && is_catalog_key(id)
        })
        .collect();
    Ok(catalog)
}

#[derive(Debug, Deserialize)]
struct ListResponse {
    #[serde(default)]
    data: Vec<ListEntry>,
}

#[derive(Debug, Deserialize)]
struct ListEntry {
    id: String,
    #[serde(default)]
    owned_by: String,
    #[serde(default)]
    tag: String,
}

/// `/v1/audio/models` entries carry `owned_by` and `tag`, so a real catalog
/// key can be formed: `owned_by:id:tag`.
pub fn parse_audio_models(body: &str) -> Result<Vec<CatalogEntry>, CatalogError> {
    let parsed: ListResponse =
        serde_json::from_str(body).map_err(|e| CatalogError::Malformed(e.to_string()))?;
    Ok(parsed
        .data
        .into_iter()
        .filter(|m| !m.id.is_empty() && !m.owned_by.is_empty() && !m.tag.is_empty())
        .map(|m| {
            let service = m.owned_by.to_lowercase();
            let tag = m.tag.to_lowercase();
            CatalogEntry {
                id: format!("{service}:{}:{tag}", m.id),
                provider_id: m.id,
                service,
                name: String::new(),
                available: true,
                unavailable_reason: None,
            }
        })
        .collect())
}

/// The flat `/v1/models` list has no tag: entries keep the bare provider id as
/// `id` and carry no availability. Ids that look like a chat model only —
/// embedding, speech and image models are hidden by the same heuristic the
/// old picker used.
pub fn parse_flat_models(body: &str) -> Result<Vec<CatalogEntry>, CatalogError> {
    let parsed: ListResponse =
        serde_json::from_str(body).map_err(|e| CatalogError::Malformed(e.to_string()))?;
    Ok(parsed
        .data
        .into_iter()
        .filter(|m| !m.id.is_empty() && looks_like_chat_model(&m.id))
        .map(|m| CatalogEntry {
            id: m.id.clone(),
            provider_id: m.id,
            service: if m.owned_by.is_empty() {
                "unknown".to_string()
            } else {
                m.owned_by.to_lowercase()
            },
            name: String::new(),
            available: true,
            unavailable_reason: None,
        })
        .collect())
}

const NON_CHAT_MARKERS: [&str; 10] = [
    "embed",
    "bge-",
    "whisper",
    "tts",
    "dall-e",
    "image",
    "stable-diffusion",
    "flux",
    "sora",
    "veo",
];
/// Fixture / non-production ids the old picker never offered. Kept out of the
/// fallback list so they cannot become a project binding when the catalog is down.
const FALLBACK_EXCLUDED_IDS: [&str; 2] = ["stub-chat-model", "test-model"];

fn looks_like_chat_model(id: &str) -> bool {
    let lower = id.to_lowercase();
    if FALLBACK_EXCLUDED_IDS.contains(&lower.as_str()) {
        return false;
    }
    !NON_CHAT_MARKERS.iter().any(|m| lower.contains(m))
}

/// Once the real catalog is known, a Chat pick made from the flat list (a bare
/// provider id remembered in `chat_legacy_provider_id`) is upgraded to the
/// catalog key — but only when exactly one catalog Chat entry has that
/// provider id. Ambiguity is left for the user; nothing is guessed.
/// Returns `true` when the binding changed.
pub fn rebind_legacy_chat(models: &mut ProjectModels, catalog: &ModelCatalog) -> bool {
    if catalog.sources.get(ModelSlot::Chat.id()) != Some(&SlotSource::Catalog) {
        return false;
    }
    let Some(legacy) = models.chat_legacy_provider_id.clone() else {
        return false;
    };
    if is_catalog_key(&models.chat) {
        models.chat_legacy_provider_id = None;
        return true;
    }
    let matches: Vec<&CatalogEntry> = catalog
        .entries(ModelSlot::Chat)
        .iter()
        .filter(|e| e.provider_id == legacy)
        .collect();
    if let [only] = matches.as_slice() {
        models.chat = only.id.clone();
        models.chat_legacy_provider_id = None;
        return true;
    }
    false
}

/// The workspace's recommended model for a slot.
///
/// Prefer the workspace DEFAULTMODEL key (`defaults` on the catalog) when that
/// entry is listed — even if a higher-quality cloud row is "available" first.
/// Quality order is not the platform default: VECTORIZE is Ollama bge-m3, not
/// OpenAI text-embedding-3-large. Fall back to the first ready catalog key
/// only when the workspace sent no default or that default is not in the list.
/// The flat `/v1/models` fallback carries bare ids and is never used to guess.
pub fn recommended_entry(catalog: &ModelCatalog, slot: ModelSlot) -> Option<&CatalogEntry> {
    match catalog.sources.get(slot.id()) {
        Some(SlotSource::Catalog) | Some(SlotSource::AudioModels) => {}
        _ => return None,
    }
    if let Some(default_id) = catalog.default_id(slot) {
        if let Some(entry) = catalog
            .entries(slot)
            .iter()
            .find(|e| e.id == default_id && is_catalog_key(&e.id))
        {
            return Some(entry);
        }
    }
    catalog
        .entries(slot)
        .iter()
        .find(|e| e.available && is_catalog_key(&e.id))
}

/// Give every unset slot the workspace's recommended model so a fresh project
/// works right away; slots the user already picked are left alone. Returns
/// the slots that were filled (empty when nothing changed).
pub fn fill_unset_slots(models: &mut ProjectModels, catalog: &ModelCatalog) -> Vec<ModelSlot> {
    let mut filled = Vec::new();
    for slot in ModelSlot::ALL {
        if models.is_set(slot) {
            continue;
        }
        if let Some(entry) = recommended_entry(catalog, slot) {
            models.set(slot, &entry.id);
            filled.push(slot);
        }
    }
    filled
}

async fn get(
    client: &reqwest::Client,
    base_url: &str,
    key: &str,
    path: &str,
) -> Result<(u16, String), CatalogError> {
    let resp = client
        .get(http::join(base_url, path))
        .header("x-api-key", key)
        .send()
        .await
        .map_err(|_| CatalogError::Network)?;
    let status = resp.status().as_u16();
    let body = resp.text().await.map_err(|_| CatalogError::Network)?;
    Ok((status, body))
}

/// Fetch the catalog. A 404 on the catalog route means the workspace does not
/// offer it yet: the result is flagged `catalog_missing`, dictation is filled
/// from `/v1/audio/models` and chat from `/v1/models` when those answer, and
/// every other slot stays empty. Nothing is invented.
pub async fn fetch_catalog(base_url: &str, key: &str) -> Result<ModelCatalog, CatalogError> {
    let client = http::client().map_err(|_| CatalogError::Network)?;
    let (status, body) = get(&client, base_url, key, "/v1/models/catalog").await?;
    match status {
        200 => return parse_catalog(&body),
        401 => return Err(CatalogError::Unauthorized),
        404 => {}
        other => return Err(CatalogError::Server(format!("status {other}"))),
    }

    let mut catalog = ModelCatalog::empty(SlotSource::None);
    catalog.catalog_missing = true;

    match get(&client, base_url, key, "/v1/audio/models").await {
        Ok((200, body)) => {
            if let Ok(entries) = parse_audio_models(&body) {
                catalog.set(ModelSlot::Voice, entries, SlotSource::AudioModels);
            }
        }
        Ok((401, _)) => return Err(CatalogError::Unauthorized),
        Ok(_) | Err(_) => {}
    }

    match get(&client, base_url, key, "/v1/models").await {
        Ok((200, body)) => {
            if let Ok(entries) = parse_flat_models(&body) {
                catalog.set(ModelSlot::Chat, entries, SlotSource::FlatModels);
            }
        }
        Ok((401, _)) => return Err(CatalogError::Unauthorized),
        Ok(_) | Err(_) => {}
    }

    Ok(catalog)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_groups_map_onto_the_eight_slots_and_extras_are_ignored() {
        let body = r#"{
          "object": "catalog",
          "capabilities": {
            "CHAT": [{"id":"ollama:llama3.2:chat","providerId":"llama3.2","service":"ollama","name":"Llama 3.2","available":true,"unavailableReason":null}],
            "VECTORIZE": [{"id":"ollama:bge-m3:vectorize","providerId":"bge-m3","service":"ollama","name":"","available":false,"unavailableReason":"Provider key missing"}],
            "SORT": [{"id":"x:y:sort","providerId":"y","service":"x"}]
          },
          "defaults": {"VECTORIZE":"ollama:bge-m3:vectorize","CHAT":"ollama:llama3.2:chat"}
        }"#;
        let catalog = parse_catalog(body).unwrap();
        assert_eq!(catalog.slots.len(), 8);
        assert!(!catalog.catalog_missing);
        assert_eq!(catalog.entries(ModelSlot::Chat)[0].provider_id, "llama3.2");
        let embed = &catalog.entries(ModelSlot::Embed)[0];
        assert!(!embed.available);
        assert_eq!(
            embed.unavailable_reason.as_deref(),
            Some("Provider key missing")
        );
        assert!(catalog.entries(ModelSlot::Speak).is_empty());
        assert!(catalog.entries(ModelSlot::Video).is_empty());
        assert!(catalog.slots.values().flatten().all(|e| e.id != "x:y:sort"));
        assert!(catalog.sources.values().all(|s| *s == SlotSource::Catalog));
        assert_eq!(
            catalog.default_id(ModelSlot::Embed),
            Some("ollama:bge-m3:vectorize")
        );
        assert_eq!(
            catalog.default_id(ModelSlot::Chat),
            Some("ollama:llama3.2:chat")
        );
    }

    #[test]
    fn malformed_catalog_is_an_error_not_an_empty_list() {
        assert!(matches!(
            parse_catalog("not json"),
            Err(CatalogError::Malformed(_))
        ));
    }

    #[test]
    fn audio_models_become_real_catalog_keys() {
        let body = r#"{"object":"list","data":[
          {"id":"whisper-1","object":"model","created":1,"owned_by":"OpenAI","tag":"SOUND2TEXT"},
          {"id":"","owned_by":"x","tag":"SOUND2TEXT"}
        ]}"#;
        let entries = parse_audio_models(body).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "openai:whisper-1:sound2text");
        assert_eq!(entries[0].provider_id, "whisper-1");
        assert_eq!(entries[0].service, "openai");
    }

    #[test]
    fn flat_models_keep_bare_ids_and_hide_non_chat_models() {
        let body = r#"{"object":"list","data":[
          {"id":"gpt-4o-mini","owned_by":"openai"},
          {"id":"text-embedding-3-small","owned_by":"openai"},
          {"id":"whisper-1","owned_by":"openai"},
          {"id":"llama3.2","owned_by":""},
          {"id":"stub-chat-model","owned_by":"test"},
          {"id":"test-model","owned_by":"test"}
        ]}"#;
        let entries = parse_flat_models(body).unwrap();
        let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["gpt-4o-mini", "llama3.2"]);
        assert_eq!(entries[1].service, "unknown");
        assert!(entries.iter().all(|e| e.id == e.provider_id));
    }

    fn entry(id: &str, provider_id: &str) -> CatalogEntry {
        CatalogEntry {
            id: id.to_string(),
            provider_id: provider_id.to_string(),
            service: id.split(':').next().unwrap_or("").to_string(),
            name: String::new(),
            available: true,
            unavailable_reason: None,
        }
    }

    fn legacy_models(chat: &str) -> ProjectModels {
        ProjectModels {
            chat: chat.to_string(),
            chat_legacy_provider_id: Some(chat.to_string()),
            ..ProjectModels::default()
        }
    }

    #[test]
    fn legacy_chat_pick_is_upgraded_only_on_a_unique_catalog_match() {
        let mut catalog = ModelCatalog::empty(SlotSource::Catalog);
        catalog.set(
            ModelSlot::Chat,
            vec![
                entry("openai:gpt-4o-mini:chat", "gpt-4o-mini"),
                entry("groq:llama3:chat", "llama3"),
                entry("ollama:llama3:chat", "llama3"),
            ],
            SlotSource::Catalog,
        );

        let mut unique = legacy_models("gpt-4o-mini");
        assert!(rebind_legacy_chat(&mut unique, &catalog));
        assert_eq!(unique.chat, "openai:gpt-4o-mini:chat");
        assert_eq!(unique.chat_legacy_provider_id, None);

        let mut ambiguous = legacy_models("llama3");
        assert!(!rebind_legacy_chat(&mut ambiguous, &catalog));
        assert_eq!(ambiguous.chat, "llama3");
        assert_eq!(ambiguous.chat_legacy_provider_id.as_deref(), Some("llama3"));

        let mut gone = legacy_models("old-model");
        assert!(!rebind_legacy_chat(&mut gone, &catalog));
        assert_eq!(gone.chat, "old-model");
    }

    #[test]
    fn rebind_never_runs_against_a_fallback_list() {
        let mut fallback = ModelCatalog::empty(SlotSource::None);
        fallback.catalog_missing = true;
        fallback.set(
            ModelSlot::Chat,
            vec![entry("gpt-4o-mini", "gpt-4o-mini")],
            SlotSource::FlatModels,
        );
        let mut models = legacy_models("gpt-4o-mini");
        assert!(!rebind_legacy_chat(&mut models, &fallback));
        assert_eq!(models.chat, "gpt-4o-mini");
    }

    fn ready(id: &str, provider_id: &str, available: bool) -> CatalogEntry {
        CatalogEntry {
            available,
            ..entry(id, provider_id)
        }
    }

    #[test]
    fn unset_slots_take_the_first_ready_catalog_model_and_picks_stay() {
        let mut catalog = ModelCatalog::empty(SlotSource::Catalog);
        catalog.set(
            ModelSlot::Chat,
            vec![
                ready("openai:gpt-4o:chat", "gpt-4o", false),
                ready("ollama:llama3.2:chat", "llama3.2", true),
                ready("groq:llama3:chat", "llama3", true),
            ],
            SlotSource::Catalog,
        );
        catalog.set(
            ModelSlot::Embed,
            vec![
                ready(
                    "openai:text-embedding-3-large:vectorize",
                    "text-embedding-3-large",
                    true,
                ),
                ready("ollama:bge-m3:vectorize", "bge-m3", true),
            ],
            SlotSource::Catalog,
        );
        catalog
            .defaults
            .insert("VECTORIZE".into(), "ollama:bge-m3:vectorize".into());
        catalog.set(
            ModelSlot::Voice,
            vec![ready("openai:whisper-1:sound2text", "whisper-1", true)],
            SlotSource::AudioModels,
        );

        let mut models = ProjectModels {
            embed: "openai:text-embedding-3-small:vectorize".into(),
            ..ProjectModels::default()
        };
        let filled = fill_unset_slots(&mut models, &catalog);

        // The unavailable first entry is skipped; the user's own pick is kept.
        assert_eq!(models.chat, "ollama:llama3.2:chat");
        assert_eq!(models.voice, "openai:whisper-1:sound2text");
        assert_eq!(models.embed, "openai:text-embedding-3-small:vectorize");
        assert_eq!(models.chat_legacy_provider_id, None);
        assert_eq!(filled, vec![ModelSlot::Chat, ModelSlot::Voice]);
        // Slots with no ready model stay unset rather than being guessed.
        assert!(models.docs.is_empty());
        assert!(models.speak.is_empty());

        // Running it again is a no-op.
        assert!(fill_unset_slots(&mut models, &catalog).is_empty());
    }

    #[test]
    fn unset_embed_binds_workspace_vectorize_default_not_highest_quality() {
        let mut catalog = ModelCatalog::empty(SlotSource::Catalog);
        catalog.set(
            ModelSlot::Embed,
            vec![
                ready(
                    "openai:text-embedding-3-large:vectorize",
                    "text-embedding-3-large",
                    true,
                ),
                ready("ollama:bge-m3:vectorize", "bge-m3", true),
            ],
            SlotSource::Catalog,
        );
        catalog
            .defaults
            .insert("VECTORIZE".into(), "ollama:bge-m3:vectorize".into());

        let mut models = ProjectModels::default();
        let filled = fill_unset_slots(&mut models, &catalog);
        assert_eq!(models.embed, "ollama:bge-m3:vectorize");
        assert_eq!(filled, vec![ModelSlot::Embed]);
        assert_eq!(
            recommended_entry(&catalog, ModelSlot::Embed).map(|e| e.id.as_str()),
            Some("ollama:bge-m3:vectorize"),
        );
    }

    #[test]
    fn fallback_lists_never_become_a_default() {
        let mut fallback = ModelCatalog::empty(SlotSource::None);
        fallback.catalog_missing = true;
        fallback.set(
            ModelSlot::Chat,
            vec![ready("gpt-4o-mini", "gpt-4o-mini", true)],
            SlotSource::FlatModels,
        );
        let mut models = ProjectModels::default();
        assert!(fill_unset_slots(&mut models, &fallback).is_empty());
        assert!(models.chat.is_empty());
        assert!(recommended_entry(&fallback, ModelSlot::Chat).is_none());
    }

    #[test]
    fn empty_catalog_lists_every_slot() {
        let catalog = ModelCatalog::empty(SlotSource::None);
        let ids: Vec<&String> = catalog.slots.keys().collect();
        assert_eq!(ids.len(), 8);
        for slot in ModelSlot::ALL {
            assert!(catalog.slots.contains_key(slot.id()));
        }
    }
}
