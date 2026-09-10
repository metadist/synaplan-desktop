//! Assistants a paired key may run (`GET /v1/assistants`).
//!
//! An Assistant is a recipe that lives on the workspace (prompt, tools, extra
//! knowledge folders). The desktop never installs or edits one; it only binds
//! ids to a project and names the chosen one in the `x-synaplan-agent-id`
//! header of a turn. The recipe's own `models.*` are read solely so the UI can
//! warn when they differ from the project's — the project's bindings always
//! stay on the wire (C15).
//!
//! When Assistants are turned off for the workspace user the route answers 404;
//! that is a normal state the UI names, not a failure to hide.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::http;

/// The catalog keys a published recipe names, `None` meaning "workspace default".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantModels {
    #[serde(default)]
    pub chat: Option<String>,
    #[serde(default)]
    pub vision: Option<String>,
    #[serde(default)]
    pub vectorize: Option<String>,
}

/// The reader view of an Assistant. Unknown keys (and any `draft`) are dropped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assistant {
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    /// Absent when the workspace's `publicView` does not include the recipe
    /// models yet; the UI then cannot compare worlds and says nothing.
    #[serde(default)]
    pub models: Option<AssistantModels>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AssistantsError {
    #[error("This computer was disconnected. Pair again.")]
    Unauthorized,
    #[error("Could not reach Synaplan. Check your connection.")]
    Network,
    #[error("Assistants are turned off on this workspace.")]
    Disabled,
    #[error("The Assistant list could not be read: {0}")]
    Malformed(String),
    #[error("{0}")]
    Server(String),
}

impl AssistantsError {
    pub fn code(&self) -> &'static str {
        match self {
            AssistantsError::Unauthorized => "unauthorized",
            AssistantsError::Network => "network",
            AssistantsError::Disabled => "assistants_disabled",
            AssistantsError::Malformed(_) => "assistants_malformed",
            AssistantsError::Server(_) => "server",
        }
    }
}

/// Accepts the Anthropic-style `{object, data}` envelope, an `{assistants}`
/// envelope, or a bare array — the server route is new and may settle on any.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ListBody {
    Data { data: Vec<Assistant> },
    Named { assistants: Vec<Assistant> },
    Bare(Vec<Assistant>),
}

pub fn parse_assistants(body: &str) -> Result<Vec<Assistant>, AssistantsError> {
    let parsed: ListBody =
        serde_json::from_str(body).map_err(|e| AssistantsError::Malformed(e.to_string()))?;
    Ok(match parsed {
        ListBody::Data { data } => data,
        ListBody::Named { assistants } => assistants,
        ListBody::Bare(list) => list,
    })
}

/// List the Assistants this key may run. 404 is "turned off", never an empty
/// list — the two are shown differently.
pub async fn fetch_assistants(
    base_url: &str,
    key: &str,
) -> Result<Vec<Assistant>, AssistantsError> {
    let client = http::client().map_err(|_| AssistantsError::Network)?;
    let resp = client
        .get(http::join(base_url, "/v1/assistants"))
        .header("x-api-key", key)
        .send()
        .await
        .map_err(|_| AssistantsError::Network)?;
    let status = resp.status().as_u16();
    let body = resp.text().await.map_err(|_| AssistantsError::Network)?;
    match status {
        200 => parse_assistants(&body),
        401 => Err(AssistantsError::Unauthorized),
        404 => Err(AssistantsError::Disabled),
        other => Err(AssistantsError::Server(format!("status {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_list_envelope_with_recipe_models_and_ignores_extras() {
        let body = r#"{"object":"list","data":[
          {"id":7,"slug":"writer","name":"Writer","description":"Drafts text","icon":"pen",
           "status":"published","promptId":3,"routable":true,"publishedVersionId":12,
           "createdAt":"2026-09-01T00:00:00Z","updatedAt":"2026-09-02T00:00:00Z",
           "models":{"chat":"anthropic:claude-sonnet-4:chat","vision":null,"vectorize":"ollama:bge-m3:vectorize"}}
        ]}"#;
        let list = parse_assistants(body).unwrap();
        assert_eq!(list.len(), 1);
        let a = &list[0];
        assert_eq!(a.id, 7);
        assert_eq!(a.name, "Writer");
        assert_eq!(a.description.as_deref(), Some("Drafts text"));
        let models = a.models.as_ref().unwrap();
        assert_eq!(
            models.chat.as_deref(),
            Some("anthropic:claude-sonnet-4:chat")
        );
        assert_eq!(models.vision, None);
    }

    #[test]
    fn models_are_optional_and_a_draft_is_never_kept() {
        let body = r#"[{"id":1,"name":"Plain","draft":{"schema":"agent.v1"}}]"#;
        let list = parse_assistants(body).unwrap();
        assert_eq!(list[0].models, None);
        let json = serde_json::to_string(&list[0]).unwrap();
        assert!(!json.contains("draft"));
    }

    #[test]
    fn accepts_the_named_envelope() {
        let list = parse_assistants(r#"{"assistants":[{"id":2,"name":"B"}]}"#).unwrap();
        assert_eq!(list[0].id, 2);
    }

    #[test]
    fn malformed_bodies_are_reported_not_emptied() {
        assert!(matches!(
            parse_assistants("not json"),
            Err(AssistantsError::Malformed(_))
        ));
        assert!(matches!(
            parse_assistants(r#"{"data":[{"name":"no id"}]}"#),
            Err(AssistantsError::Malformed(_))
        ));
    }

    #[test]
    fn error_codes_are_stable() {
        assert_eq!(AssistantsError::Disabled.code(), "assistants_disabled");
        assert_eq!(AssistantsError::Unauthorized.code(), "unauthorized");
    }
}
