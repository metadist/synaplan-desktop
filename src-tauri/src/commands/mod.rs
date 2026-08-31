//! Tauri command surface. This module is *wiring only*: it maps `#[tauri::command]`
//! entry points to `synaplan-core` functions, holds shared state, and emits chat
//! stream events. No business logic and no platform branch lives here — those
//! are in `synaplan-core` (and, for OS differences, `synaplan-core::platform`).

use std::sync::Arc;

use serde::Serialize;
use synaplan_core::config::DesktopConfig;
use synaplan_core::messages::{self, ChatError, ChatMessage};
use synaplan_core::pairing::{self, PairError};
use synaplan_core::platform::app_dirs::AppDirs;
use synaplan_core::platform::secret_store::SecretStore;
use synaplan_core::sse::ChatEvent;
use synaplan_core::{hostname, url as core_url};
use tauri::{AppHandle, Emitter, State};

/// Process-wide state shared by every command.
pub struct AppState {
    pub app_dirs: AppDirs,
    pub secret: Arc<dyn SecretStore>,
}

/// A serialisable error the frontend maps to a localized message by `code`.
#[derive(Debug, Serialize)]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

impl CommandError {
    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
        }
    }

    fn not_paired() -> Self {
        Self::new("not_paired", "This computer is not paired yet.")
    }
}

impl From<PairError> for CommandError {
    fn from(e: PairError) -> Self {
        CommandError::new(e.code(), e.to_string())
    }
}

impl From<ChatError> for CommandError {
    fn from(e: ChatError) -> Self {
        CommandError::new(e.code(), e.to_string())
    }
}

impl From<synaplan_core::platform::secret_store::SecretStoreError> for CommandError {
    fn from(e: synaplan_core::platform::secret_store::SecretStoreError) -> Self {
        CommandError::new("secret_store", e.to_string())
    }
}

impl From<synaplan_core::config::ConfigError> for CommandError {
    fn from(e: synaplan_core::config::ConfigError) -> Self {
        CommandError::new("config", e.to_string())
    }
}

/// The paired/unpaired status the UI renders on start.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusDto {
    pub paired: bool,
    pub api_base_url: Option<String>,
    pub device_id: Option<i64>,
    pub key_backend: String,
    pub key_is_plaintext: bool,
}

fn status_of(state: &AppState) -> Result<StatusDto, CommandError> {
    let cfg = DesktopConfig::load(&state.app_dirs.config_file())?;
    let has_key = state.secret.get().unwrap_or(None).is_some();
    Ok(StatusDto {
        paired: cfg.is_paired() && has_key,
        api_base_url: cfg.api_base_url,
        device_id: cfg.device_id,
        key_backend: state.secret.backend_name().to_string(),
        key_is_plaintext: state.secret.is_plaintext(),
    })
}

#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> Result<StatusDto, CommandError> {
    status_of(&state)
}

#[tauri::command]
pub fn default_device_name() -> String {
    let raw = gethostname::gethostname().to_string_lossy().to_string();
    hostname::sanitize_device_name(&raw)
}

#[tauri::command]
pub fn validate_base_url(url: String) -> Result<String, CommandError> {
    core_url::validate_base_url(&url).map_err(|e| CommandError::new("invalid_url", e.to_string()))
}

#[tauri::command]
pub async fn pair(
    state: State<'_, AppState>,
    base_url: String,
    code: String,
    device_name: String,
) -> Result<StatusDto, CommandError> {
    let base = core_url::validate_base_url(&base_url)
        .map_err(|e| CommandError::new("invalid_url", e.to_string()))?;
    let device_name = hostname::sanitize_device_name(&device_name);

    let device = pairing::pair(&base, code.trim(), &device_name).await?;

    state.secret.set(&device.key)?;
    let cfg = DesktopConfig {
        api_base_url: Some(device.api_base_url),
        device_id: device.device_id,
    };
    cfg.save(&state.app_dirs.config_file())?;

    status_of(&state)
}

/// Recovery / dev path: store a scoped key pasted by the user after verifying it
/// works against the instance. No device row is created server-side.
#[tauri::command]
pub async fn pair_with_key(
    state: State<'_, AppState>,
    base_url: String,
    key: String,
) -> Result<StatusDto, CommandError> {
    let base = core_url::validate_base_url(&base_url)
        .map_err(|e| CommandError::new("invalid_url", e.to_string()))?;
    let key = key.trim().to_string();
    if key.is_empty() {
        return Err(CommandError::new("invalid_key", "Enter a scoped API key."));
    }

    pairing::verify_key(&base, &key).await?;

    state.secret.set(&key)?;
    let cfg = DesktopConfig {
        api_base_url: Some(base),
        device_id: None,
    };
    cfg.save(&state.app_dirs.config_file())?;

    status_of(&state)
}

#[tauri::command]
pub fn sign_out(state: State<'_, AppState>) -> Result<(), CommandError> {
    state.secret.delete()?;
    DesktopConfig::clear(&state.app_dirs.config_file())?;
    Ok(())
}

#[tauri::command]
pub async fn list_models(state: State<'_, AppState>) -> Result<Vec<String>, CommandError> {
    let cfg = DesktopConfig::load(&state.app_dirs.config_file())?;
    let base = cfg.api_base_url.ok_or_else(CommandError::not_paired)?;
    let key = state.secret.get()?.ok_or_else(CommandError::not_paired)?;
    Ok(messages::list_models(&base, &key).await?)
}

/// The payload for a `chat://error` event.
#[derive(Debug, Clone, Serialize)]
struct StreamError {
    code: String,
    message: String,
}

/// Stream one assistant turn, emitting `chat://token`, `chat://done`, and
/// `chat://error` events. On 401 the stored key + config are wiped so the UI
/// returns to the pairing screen.
#[tauri::command]
pub async fn send_chat(
    app: AppHandle,
    state: State<'_, AppState>,
    messages: Vec<ChatMessage>,
    model: Option<String>,
) -> Result<(), CommandError> {
    let cfg = DesktopConfig::load(&state.app_dirs.config_file())?;
    let base = cfg.api_base_url.ok_or_else(CommandError::not_paired)?;
    let key = state.secret.get()?.ok_or_else(CommandError::not_paired)?;

    let emitter = app.clone();
    let result = messages::stream_chat(
        &base,
        &key,
        model.as_deref(),
        &messages,
        1024,
        move |event| match event {
            ChatEvent::Token(text) => {
                let _ = emitter.emit("chat://token", text);
            }
            ChatEvent::Done => {
                let _ = emitter.emit("chat://done", ());
            }
            ChatEvent::Error(message) => {
                let _ = emitter.emit(
                    "chat://error",
                    StreamError {
                        code: "server".to_string(),
                        message,
                    },
                );
            }
        },
    )
    .await;

    if let Err(err) = result {
        if matches!(err, ChatError::Unauthorized) {
            // A revoked/rotated key: wipe local credentials so the next launch
            // starts at the pairing screen (cross-platform §4 rule 4).
            let _ = state.secret.delete();
            let _ = DesktopConfig::clear(&state.app_dirs.config_file());
        }
        let _ = app.emit(
            "chat://error",
            StreamError {
                code: err.code().to_string(),
                message: err.to_string(),
            },
        );
        return Err(err.into());
    }

    Ok(())
}
