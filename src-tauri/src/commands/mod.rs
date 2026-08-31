//! Tauri command surface. This module is *wiring only*: it maps `#[tauri::command]`
//! entry points to `synaplan-core` functions, holds shared state, and emits chat
//! stream events. No business logic and no platform branch lives here — those
//! are in `synaplan-core` (and, for OS differences, `synaplan-core::platform`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::Serialize;
use synaplan_core::config::DesktopConfig;
use synaplan_core::filesystem::{FilesystemPolicy, FsPolicyError};
use synaplan_core::messages::{self, ChatError, ChatMessage, ModelInfo};
use synaplan_core::pairing::{self, PairError};
use synaplan_core::platform::app_dirs::AppDirs;
use synaplan_core::platform::secret_store::SecretStore;
use synaplan_core::skills::{self, Skill};
use synaplan_core::sse::ChatEvent;
use synaplan_core::{hostname, url as core_url};
use tauri::{AppHandle, Emitter, State};

/// Process-wide state shared by every command.
pub struct AppState {
    pub app_dirs: AppDirs,
    pub secret: Arc<dyn SecretStore>,
    /// Set to true by `cancel_chat` to stop an in-flight streaming turn.
    pub cancel: Arc<AtomicBool>,
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
        use synaplan_core::platform::secret_store::SecretStoreError as E;
        let code = match e {
            E::Unavailable => "secret_store_unavailable",
            _ => "secret_store",
        };
        CommandError::new(code, e.to_string())
    }
}

impl From<synaplan_core::config::ConfigError> for CommandError {
    fn from(e: synaplan_core::config::ConfigError) -> Self {
        CommandError::new("config", e.to_string())
    }
}

impl From<FsPolicyError> for CommandError {
    fn from(e: FsPolicyError) -> Self {
        CommandError::new("filesystem", e.to_string())
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
pub async fn list_models(state: State<'_, AppState>) -> Result<Vec<ModelInfo>, CommandError> {
    let cfg = DesktopConfig::load(&state.app_dirs.config_file())?;
    let base = cfg.api_base_url.ok_or_else(CommandError::not_paired)?;
    let key = state.secret.get()?.ok_or_else(CommandError::not_paired)?;
    Ok(messages::list_models(&base, &key).await?)
}

/// Stop an in-flight streaming chat turn (the Stop button).
#[tauri::command]
pub fn cancel_chat(state: State<'_, AppState>) {
    state.cancel.store(true, Ordering::Relaxed);
}

/// Open an http(s) URL in the user's default browser (used for "Learn more"
/// documentation links). Only web links are allowed.
#[tauri::command]
pub fn open_url(url: String) -> Result<(), CommandError> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(CommandError::new(
            "invalid_url",
            "Only http(s) links can be opened.",
        ));
    }
    open::that(&url).map_err(|e| CommandError::new("open_failed", e.to_string()))
}

/// Reveal a local folder/file in the OS file manager (e.g. the out-box).
#[tauri::command]
pub fn reveal_path(path: String) -> Result<(), CommandError> {
    open::that(&path).map_err(|e| CommandError::new("open_failed", e.to_string()))
}

/// The filesystem allowlist as shown on the "This computer" screen.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilesystemPolicyDto {
    pub read: Vec<String>,
    pub outbox: String,
    pub deny: Vec<String>,
    pub max_file_bytes: u64,
}

impl AppState {
    fn filesystem_policy_path(&self) -> std::path::PathBuf {
        self.app_dirs.config_dir.join("filesystem.toml")
    }

    fn load_policy(&self) -> Result<FilesystemPolicy, CommandError> {
        let mut policy = FilesystemPolicy::load(&self.filesystem_policy_path())?;
        policy.ensure_outbox(&self.app_dirs.outbox_dir);
        policy.save(&self.filesystem_policy_path())?;
        Ok(policy)
    }

    fn policy_dto(&self, policy: FilesystemPolicy) -> FilesystemPolicyDto {
        FilesystemPolicyDto {
            read: policy.read,
            outbox: self.app_dirs.outbox_dir.to_string_lossy().to_string(),
            deny: policy.deny,
            max_file_bytes: policy.max_file_bytes,
        }
    }
}

#[tauri::command]
pub fn get_filesystem_policy(
    state: State<'_, AppState>,
) -> Result<FilesystemPolicyDto, CommandError> {
    let policy = state.load_policy()?;
    Ok(state.policy_dto(policy))
}

#[tauri::command]
pub fn add_read_folder(
    state: State<'_, AppState>,
    path: String,
) -> Result<FilesystemPolicyDto, CommandError> {
    let mut policy = state.load_policy()?;
    policy.add_read(path.trim())?;
    policy.save(&state.filesystem_policy_path())?;
    Ok(state.policy_dto(policy))
}

#[tauri::command]
pub fn remove_read_folder(
    state: State<'_, AppState>,
    path: String,
) -> Result<FilesystemPolicyDto, CommandError> {
    let mut policy = state.load_policy()?;
    policy.remove_read(&path);
    policy.save(&state.filesystem_policy_path())?;
    Ok(state.policy_dto(policy))
}

#[tauri::command]
pub fn list_skills(state: State<'_, AppState>) -> Vec<Skill> {
    skills::load_skills(&state.app_dirs.skills_dir)
}

/// Probe the local tools skills rely on (Python/Node/LibreOffice). Runs on a
/// blocking thread because it spawns short `--version` subprocesses.
#[tauri::command]
pub async fn run_doctor() -> Vec<synaplan_core::platform::doctor::Tool> {
    tauri::async_runtime::spawn_blocking(synaplan_core::platform::doctor::detect_all)
        .await
        .unwrap_or_default()
}

#[tauri::command]
pub fn set_skill_enabled(
    state: State<'_, AppState>,
    name: String,
    enabled: bool,
) -> Result<Vec<Skill>, CommandError> {
    skills::set_enabled(&state.app_dirs.skills_dir, &name, enabled)
        .map_err(|e| CommandError::new("skills", e.to_string()))?;
    Ok(skills::load_skills(&state.app_dirs.skills_dir))
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

    state.cancel.store(false, Ordering::Relaxed);
    let emitter = app.clone();
    let result = messages::stream_chat(
        &base,
        &key,
        model.as_deref(),
        &messages,
        1024,
        &state.cancel,
        // stream_chat surfaces provider/SSE errors as Err (handled below), so the
        // closure only ever receives text tokens and the terminal Done.
        move |event| {
            if let ChatEvent::Token(text) = event {
                let _ = emitter.emit("chat://token", text);
            } else {
                let _ = emitter.emit("chat://done", ());
            }
        },
    )
    .await;

    if let Err(err) = result {
        // A chat 401 is ambiguous: the desktop key could be revoked, OR the
        // gateway's upstream provider rejected the request with a valid key.
        // Only wipe local credentials when the desktop key itself no longer
        // authenticates (re-checked against /v1/models). A 403 (gateway
        // disabled / scope) is never a wipe.
        let (code, message) = if matches!(err, ChatError::Unauthorized) {
            if pairing::verify_key(&base, &key).await.is_err() {
                let _ = state.secret.delete();
                let _ = DesktopConfig::clear(&state.app_dirs.config_file());
                ("unauthorized".to_string(), err.to_string())
            } else {
                ("server".to_string(), err.to_string())
            }
        } else {
            (err.code().to_string(), err.to_string())
        };

        let _ = app.emit(
            "chat://error",
            StreamError {
                code: code.clone(),
                message: message.clone(),
            },
        );
        return Err(CommandError::new(&code, message));
    }

    Ok(())
}
