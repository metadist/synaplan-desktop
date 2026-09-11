//! Tauri command surface. This module is *wiring only*: it maps `#[tauri::command]`
//! entry points to `synaplan-core` functions, holds shared state, and emits chat
//! stream events. No business logic and no platform branch lives here — those
//! are in `synaplan-core` (and, for OS differences, `synaplan-core::platform`).

pub mod dictation;
pub mod files;
pub mod generation;
pub mod projects;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Value};
use synaplan_core::agent::{self, AgentEvent, AgentTool, ToolDispatchResult};
use synaplan_core::config::{DesktopConfig, UiPrefs};
use synaplan_core::filesystem::{FilesystemPolicy, FsPolicyError};
use synaplan_core::install::{self, InstallError, InstallPreview};
use synaplan_core::messages::{self, ChatError, ChatMessage, ModelInfo};
use synaplan_core::pairing::{self, PairError};
use synaplan_core::platform::app_dirs::AppDirs;
use synaplan_core::platform::confinement::Confinement;
use synaplan_core::platform::doctor;
use synaplan_core::platform::secret_store::SecretStore;
use synaplan_core::poll::PollStatus;
use synaplan_core::skills::{self, Skill, SkillSource};
use synaplan_core::sse::ChatEvent;
use synaplan_core::tools::{self, ToolPolicy};
use synaplan_core::{hostname, url as core_url};
use tauri::{AppHandle, Emitter, State};

/// Process-wide state shared by every command.
pub struct AppState {
    pub app_dirs: AppDirs,
    pub secret: Arc<dyn SecretStore>,
    /// Set to true by `cancel_chat` to stop an in-flight streaming turn.
    pub cancel: Arc<AtomicBool>,
    /// Bumped at the start of every turn and on cancel so late events from an
    /// obsolete stream are dropped instead of landing in the next view.
    pub turn_gen: Arc<AtomicU64>,
    pub poll_stop: Arc<AtomicBool>,
    pub poll_running: Arc<AtomicBool>,
    pub poll_status: Arc<Mutex<PollStatus>>,
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

impl From<InstallError> for CommandError {
    fn from(e: InstallError) -> Self {
        CommandError::new(e.code(), e.to_string())
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

pub(crate) fn status_of(state: &AppState) -> Result<StatusDto, CommandError> {
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
    app: AppHandle,
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
    let existing = DesktopConfig::load(&state.app_dirs.config_file()).unwrap_or_default();
    let cfg = DesktopConfig {
        api_base_url: Some(device.api_base_url),
        device_id: device.device_id,
        last_chat_model: existing.last_chat_model,
        studio_tiles: existing.studio_tiles,
        tools: existing.tools,
        ui: existing.ui,
    };
    cfg.save(&state.app_dirs.config_file())?;
    let _ = state.project_store().clear_assistant_bindings();

    let status = status_of(&state)?;
    crate::poll_loop::start_if_paired(&app);
    Ok(status)
}

/// Recovery / dev path: store a scoped key pasted by the user after verifying it
/// works against the instance. No device row is created server-side.
#[tauri::command]
pub async fn pair_with_key(
    app: AppHandle,
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
    let existing = DesktopConfig::load(&state.app_dirs.config_file()).unwrap_or_default();
    let cfg = DesktopConfig {
        api_base_url: Some(base),
        device_id: None,
        last_chat_model: existing.last_chat_model,
        studio_tiles: existing.studio_tiles,
        tools: existing.tools,
        ui: existing.ui,
    };
    cfg.save(&state.app_dirs.config_file())?;
    let _ = state.project_store().clear_assistant_bindings();

    let status = status_of(&state)?;
    crate::poll_loop::start_if_paired(&app);
    Ok(status)
}

#[tauri::command]
pub fn sign_out(app: AppHandle, state: State<'_, AppState>) -> Result<(), CommandError> {
    crate::poll_loop::stop(&app);
    let _ = state.project_store().clear_assistant_bindings();
    state.secret.delete()?;
    DesktopConfig::forget_pairing(&state.app_dirs.config_file())?;
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
    state.turn_gen.fetch_add(1, Ordering::SeqCst);
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

    pub(crate) fn load_policy(&self) -> Result<FilesystemPolicy, CommandError> {
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

pub(crate) fn listed_skills(state: &AppState) -> Vec<Skill> {
    let mut skills = skills::load_skills(&state.app_dirs.skills_dir);
    let cfg = DesktopConfig::load(&state.app_dirs.config_file()).unwrap_or_default();
    let imports: Vec<String> = skills
        .iter()
        .flat_map(|s| s.python_imports.iter().cloned())
        .collect();
    let snapshot = doctor::runtime_snapshot(&cfg.tools, &imports);
    skills::apply_runtime_blocks(&mut skills, &snapshot);
    skills
}

#[tauri::command]
pub fn list_skills(state: State<'_, AppState>) -> Vec<Skill> {
    listed_skills(&state)
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
    Ok(listed_skills(&state))
}

#[tauri::command]
pub fn set_skill_unattended(
    state: State<'_, AppState>,
    name: String,
    allow: bool,
) -> Result<Vec<Skill>, CommandError> {
    skills::set_allow_unattended(&state.app_dirs.skills_dir, &name, allow)
        .map_err(|e| CommandError::new("skills", e.to_string()))?;
    Ok(listed_skills(&state))
}

#[tauri::command]
pub fn preview_skill_folder(folder: String) -> Result<InstallPreview, CommandError> {
    Ok(install::preview_folder(Path::new(folder.trim()))?)
}

#[tauri::command]
pub fn preview_skill_zip(zip_path: String) -> Result<InstallPreview, CommandError> {
    Ok(install::preview_zip(Path::new(zip_path.trim()))?)
}

#[tauri::command]
pub async fn preview_skill_url(
    state: State<'_, AppState>,
    url: String,
) -> Result<InstallPreview, CommandError> {
    Ok(install::preview_url(url.trim(), &state.app_dirs.skills_dir).await?)
}

#[tauri::command]
pub fn install_skill_from_folder(
    state: State<'_, AppState>,
    folder: String,
) -> Result<Vec<Skill>, CommandError> {
    let name = install::install_folder(
        Path::new(folder.trim()),
        &state.app_dirs.skills_dir,
        SkillSource::Folder,
        None,
        None,
    )?;
    install::strip_quarantine(&state.app_dirs.skills_dir.join(&name));
    Ok(listed_skills(&state))
}

#[tauri::command]
pub fn install_skill_from_zip(
    state: State<'_, AppState>,
    zip_path: String,
) -> Result<Vec<Skill>, CommandError> {
    let name = install::install_zip(
        Path::new(zip_path.trim()),
        &state.app_dirs.skills_dir,
        SkillSource::Zip,
        None,
        None,
        None,
    )?;
    install::strip_quarantine(&state.app_dirs.skills_dir.join(&name));
    Ok(listed_skills(&state))
}

#[tauri::command]
pub async fn install_skill_from_url(
    state: State<'_, AppState>,
    url: String,
    cache_path: Option<String>,
) -> Result<Vec<Skill>, CommandError> {
    let name = if let Some(cache) = cache_path.filter(|s| !s.is_empty()) {
        install::install_cached_zip(
            Path::new(&cache),
            &state.app_dirs.skills_dir,
            Some(url.trim()),
            None,
        )?
    } else {
        install::install_url(url.trim(), &state.app_dirs.skills_dir).await?
    };
    install::strip_quarantine(&state.app_dirs.skills_dir.join(&name));
    Ok(listed_skills(&state))
}

#[tauri::command]
pub fn remove_skill(state: State<'_, AppState>, name: String) -> Result<Vec<Skill>, CommandError> {
    install::remove_skill(&state.app_dirs.skills_dir, name.trim())?;
    Ok(listed_skills(&state))
}

#[tauri::command]
pub fn skills_dir(state: State<'_, AppState>) -> String {
    state.app_dirs.skills_dir.to_string_lossy().to_string()
}

/// The payload for a `chat://error` event.
#[derive(Debug, Clone, Serialize)]
struct StreamError {
    code: String,
    message: String,
}

/// Stream one assistant turn inside `project_id`, emitting `chat://token`,
/// `chat://done`, and `chat://error` events. The body model is always the
/// project's Chat model (C15); the send is refused when none is set. On 401
/// the stored key + config are wiped so the UI returns to the pairing screen.
#[tauri::command]
pub async fn send_chat(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    messages: Vec<ChatMessage>,
    assistant_id: Option<i64>,
) -> Result<(), CommandError> {
    let cfg = DesktopConfig::load(&state.app_dirs.config_file())?;
    let base = cfg.api_base_url.ok_or_else(CommandError::not_paired)?;
    let key = state.secret.get()?.ok_or_else(CommandError::not_paired)?;
    let ctx = state.turn_context(&project_id, assistant_id)?;

    let turn = state.begin_turn();
    let emitter = app.clone();
    let turn_for_emit = turn.clone();
    let result = messages::stream_chat(
        &base,
        &key,
        &ctx,
        &messages,
        1024,
        &state.cancel,
        // stream_chat surfaces provider/SSE errors as Err (handled below), so the
        // closure only ever receives text tokens and the terminal Done.
        move |event| {
            if !turn_for_emit.is_live() {
                return;
            }
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
        let (code, message) = classify_turn_error(&state, &base, &key, err).await;
        if turn.is_live() {
            let _ = app.emit(
                "chat://error",
                StreamError {
                    code: code.clone(),
                    message: message.clone(),
                },
            );
        }
        return Err(CommandError::new(&code, message));
    }

    Ok(())
}

/// Map a turn error to `(code, message)`, wiping credentials only on a genuine
/// revoked-key 401 (re-verified against `/v1/models`). Shared by chat + agent.
pub(crate) async fn classify_turn_error(
    state: &AppState,
    base: &str,
    key: &str,
    err: ChatError,
) -> (String, String) {
    if matches!(err, ChatError::Unauthorized) {
        if state.wipe_if_key_revoked(base, key).await {
            ("unauthorized".to_string(), err.to_string())
        } else {
            ("server".to_string(), err.to_string())
        }
    } else {
        (err.code().to_string(), err.to_string())
    }
}

/// Identifies one streaming turn so events from a cancelled or superseded
/// turn are not emitted into the next view.
#[derive(Clone)]
struct TurnGuard {
    gen: Arc<AtomicU64>,
    mine: u64,
}

impl TurnGuard {
    fn is_live(&self) -> bool {
        self.gen.load(Ordering::Relaxed) == self.mine
    }
}

impl AppState {
    fn begin_turn(&self) -> TurnGuard {
        let mine = self.turn_gen.fetch_add(1, Ordering::SeqCst) + 1;
        self.cancel.store(false, Ordering::SeqCst);
        TurnGuard {
            gen: self.turn_gen.clone(),
            mine,
        }
    }

    fn execution_consent_path(&self) -> PathBuf {
        self.app_dirs.config_dir.join("execution-consent")
    }

    pub(crate) fn has_execution_consent(&self) -> bool {
        self.execution_consent_path().exists()
    }
}

/// Whether the user has granted this install permission to run installed skills.
#[tauri::command]
pub fn get_execution_consent(state: State<'_, AppState>) -> bool {
    state.execution_consent_path().exists()
}

/// Grant execution consent (after the first-run confirmation dialog). Persisted
/// as a marker file so it is asked only once per install.
#[tauri::command]
pub fn set_execution_consent(state: State<'_, AppState>) -> Result<(), CommandError> {
    std::fs::create_dir_all(&state.app_dirs.config_dir)
        .map_err(|e| CommandError::new("consent", e.to_string()))?;
    std::fs::write(state.execution_consent_path(), b"1")
        .map_err(|e| CommandError::new("consent", e.to_string()))?;
    Ok(())
}

/// The payload for the `agent://*` events the run activity UI renders.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentToolEvent {
    phase: String,
    name: String,
    summary: String,
    ok: bool,
    artifact: Option<String>,
}

/// Run one agentic turn with the skill tools (Read/Write/Bash). Emits
/// `agent://text`, `agent://tool`, `agent://done`, `agent://error`. Programs are
/// only offered to the model when `allow_exec` is true (execution consent given).
#[tauri::command]
pub async fn send_agent_chat(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    messages: Vec<ChatMessage>,
    allow_exec: bool,
    assistant_id: Option<i64>,
) -> Result<(), CommandError> {
    let cfg = DesktopConfig::load(&state.app_dirs.config_file())?;
    let base = cfg.api_base_url.ok_or_else(CommandError::not_paired)?;
    let key = state.secret.get()?.ok_or_else(CommandError::not_paired)?;
    let store = state.project_store();
    let project = store.get_project(&project_id)?;
    let ctx = projects::turn_context_for(&project, assistant_id)?;

    let turn = state.begin_turn();

    // The project's `out/` is the write root and artifact folder for this turn
    // (never the computer-level outbox, which web-queued jobs keep using).
    store.create_dirs(&project)?;
    let mut fs_policy = state.load_policy()?;
    let skills_dir = state.app_dirs.skills_dir.clone();
    let outbox = store.out_dir(&project);
    fs_policy.ensure_outbox(&outbox);
    let mut loaded = skills::load_skills(&skills_dir);
    let imports: Vec<String> = loaded
        .iter()
        .flat_map(|s| s.python_imports.iter().cloned())
        .collect();
    let snapshot = doctor::runtime_snapshot(&cfg.tools, &imports);
    skills::apply_runtime_blocks(&mut loaded, &snapshot);
    // Only the skills this project enabled — a project narrows the computer's set.
    let enabled: Vec<Skill> = skills::project_overlay(loaded, &project.enabled_skills);

    // Interpreter allowlist (blocking discovery on a worker thread).
    let tools_cfg = cfg.tools.clone();
    let programs =
        tauri::async_runtime::spawn_blocking(move || doctor::allowlisted_programs_with(&tools_cfg))
            .await
            .unwrap_or_default();
    let allow_exec = allow_exec && state.has_execution_consent() && !programs.is_empty();

    let policy = build_tool_policy(&fs_policy, &skills_dir, &outbox, programs)
        .map_err(|e| CommandError::new("filesystem", e))?;

    let system = build_system_prompt(&enabled, &skills_dir, &outbox, &fs_policy.read, allow_exec);
    let mut tools = vec![read_file_tool(), write_file_tool()];
    if allow_exec {
        tools.push(run_program_tool());
    }

    let msgs: Vec<Value> = messages
        .iter()
        .map(|m| json!({ "role": m.role, "content": m.content }))
        .collect();

    let emitter = app.clone();
    let outbox_for_dispatch = outbox.clone();
    let turn_for_emit = turn.clone();
    let before_out = snapshot_files(&outbox);
    let result = agent::run_agent_turn(
        &base,
        &key,
        &ctx,
        &system,
        msgs,
        &tools,
        &state.cancel,
        |name, input| dispatch_tool(&policy, &outbox_for_dispatch, name, input),
        |event| {
            if turn_for_emit.is_live() {
                emit_agent_event(&emitter, event);
            }
        },
    )
    .await;

    if result.is_ok() {
        let created: Vec<String> = snapshot_files(&outbox)
            .difference(&before_out)
            .cloned()
            .collect();
        for path in created {
            let ext = std::path::Path::new(&path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default();
            if !synaplan_core::artifacts::is_supported(ext) {
                continue;
            }
            let _ =
                generation::publish_out_file(&state, &base, &key, &project, path.as_ref()).await;
        }
    }

    if let Err(err) = result {
        let (code, message) = classify_turn_error(&state, &base, &key, err).await;
        if turn.is_live() {
            let _ = app.emit(
                "agent://error",
                StreamError {
                    code: code.clone(),
                    message: message.clone(),
                },
            );
        }
        return Err(CommandError::new(&code, message));
    }

    Ok(())
}

fn emit_agent_event(app: &AppHandle, event: AgentEvent) {
    match event {
        AgentEvent::Text(text) => {
            let _ = app.emit("agent://text", text);
        }
        AgentEvent::ToolStart { name, input } => {
            let _ = app.emit(
                "agent://tool",
                AgentToolEvent {
                    phase: "start".to_string(),
                    summary: tool_start_summary(&name, &input),
                    name,
                    ok: true,
                    artifact: None,
                },
            );
        }
        AgentEvent::ToolEnd { name, result } => {
            let _ = app.emit(
                "agent://tool",
                AgentToolEvent {
                    phase: "end".to_string(),
                    name,
                    summary: result.summary,
                    ok: !result.is_error,
                    artifact: result.artifact,
                },
            );
        }
        AgentEvent::Cancelled | AgentEvent::Done => {
            let _ = app.emit("agent://done", ());
        }
    }
}

/// Build the confined tool policy: read = user folders + the skills dir + the
/// out-box; write = the out-box; workdir = the out-box.
pub(crate) fn build_tool_policy(
    fs_policy: &FilesystemPolicy,
    skills_dir: &Path,
    outbox: &Path,
    programs: Vec<PathBuf>,
) -> Result<ToolPolicy, String> {
    let mut read: Vec<PathBuf> = fs_policy.read.iter().map(PathBuf::from).collect();
    read.push(skills_dir.to_path_buf());
    read.push(outbox.to_path_buf());
    let write: Vec<PathBuf> = fs_policy.write.iter().map(PathBuf::from).collect();
    let confinement =
        Confinement::new(&read, &write, &fs_policy.deny).map_err(|e| e.to_string())?;
    let tool_dirs: Vec<PathBuf> = programs
        .iter()
        .filter_map(|p| p.parent().map(Path::to_path_buf))
        .collect();
    Ok(ToolPolicy {
        confinement,
        allow_programs: programs,
        skills_dir: skills_dir.to_path_buf(),
        run_scratch: outbox.to_path_buf(),
        tool_dirs,
        timeout: Duration::from_secs(120),
        max_output_bytes: 200_000,
        max_file_bytes: fs_policy.max_file_bytes,
    })
}

pub(crate) fn read_file_tool() -> AgentTool {
    AgentTool {
        name: "read_file".to_string(),
        description: "Read a UTF-8 text file the user allowed (a skill file or a folder they added). Returns the file contents. Use the full SKILL.md path listed for the skill, or a path under the skills folder (for example vcard/SKILL.md). Do not pass only the file name SKILL.md.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": { "path": { "type": "string", "description": "Full path, or a path relative to the skills folder such as vcard/SKILL.md." } },
            "required": ["path"]
        }),
    }
}

pub(crate) fn write_file_tool() -> AgentTool {
    AgentTool {
        name: "write_file".to_string(),
        description: "Write a text file into the out-box folder. Use this for text/markdown results. Returns the saved path.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Absolute path inside the out-box." },
                "content": { "type": "string", "description": "The file contents." }
            },
            "required": ["path", "content"]
        }),
    }
}

pub(crate) fn run_program_tool() -> AgentTool {
    AgentTool {
        name: "run_program".to_string(),
        description: "Run an installed skill's script with an allowlisted interpreter (Python/Node) or LibreOffice. Provide a single command line: the interpreter, the skill's script path, then arguments. No shell features (no pipes, redirects, &&, inline -c/-e code). Write outputs into the out-box.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": { "command": { "type": "string", "description": "e.g. python3 /path/to/skill/script.py <outfile> <args>" } },
            "required": ["command"]
        }),
    }
}

pub(crate) fn build_system_prompt(
    skills: &[Skill],
    skills_dir: &Path,
    outbox: &Path,
    read_roots: &[String],
    allow_exec: bool,
) -> String {
    let mut s = String::new();
    s.push_str("You are Synaplan Desktop, a local assistant that can create files on this computer using installed skills. Be concise and friendly.\n\n");
    s.push_str(&format!(
        "OUT-BOX (write all results here): {}\n",
        outbox.display()
    ));
    s.push_str(&format!("SKILLS FOLDER: {}\n", skills_dir.display()));
    if !read_roots.is_empty() {
        s.push_str(&format!("READABLE FOLDERS: {}\n", read_roots.join(", ")));
    }
    s.push('\n');
    if skills.is_empty() {
        s.push_str("No skills are enabled. You can still write text files into the out-box with write_file.\n");
    } else {
        s.push_str("ENABLED SKILLS:\n");
        for skill in skills {
            let folder = Path::new(&skill.dir);
            let folder = if folder.as_os_str().is_empty() {
                skills_dir.join(&skill.name)
            } else {
                folder.to_path_buf()
            };
            s.push_str(&format!(
                "- {} — {}\n  SKILL.md: {}\n  folder: {}\n",
                skill.name,
                skill.description,
                folder.join("SKILL.md").display(),
                folder.display()
            ));
        }
    }
    s.push('\n');
    s.push_str("HOW TO WORK:\n");
    s.push_str("1. If a skill fits the request, read_file the SKILL.md path listed above (the full path, never just the file name).\n");
    if allow_exec {
        s.push_str("2. Run the skill's script with run_program (interpreter + the script path inside the skill folder + arguments). Write outputs into the out-box.\n");
        s.push_str(
            "3. If no skill fits, you may still produce text/markdown results with write_file.\n",
        );
    } else {
        s.push_str("2. Program execution is not enabled, so produce results as text/markdown files with write_file into the out-box.\n");
    }
    s.push_str("Never invent paths. Only write inside the out-box. When finished, tell the user what you created and where.\n");
    s
}

fn tool_start_summary(name: &str, input: &Value) -> String {
    match name {
        "read_file" => format!(
            "Reading {}",
            short_path(input.get("path").and_then(Value::as_str).unwrap_or(""))
        ),
        "write_file" => format!(
            "Writing {}",
            short_path(input.get("path").and_then(Value::as_str).unwrap_or(""))
        ),
        "run_program" => format!(
            "Running {}",
            program_name(input.get("command").and_then(Value::as_str).unwrap_or(""))
        ),
        other => format!("Using {other}"),
    }
}

/// Execute a single tool call against the confined policy.
pub(crate) fn dispatch_tool(
    policy: &ToolPolicy,
    outbox: &Path,
    name: &str,
    input: &Value,
) -> ToolDispatchResult {
    match name {
        "read_file" => {
            let path = input.get("path").and_then(Value::as_str).unwrap_or("");
            match tools::tool_read(policy, path) {
                Ok(text) => ToolDispatchResult {
                    content: truncate(&text, 60_000),
                    is_error: false,
                    summary: format!("Read {}", short_path(path)),
                    artifact: None,
                },
                Err(e) => error_result(
                    &e.to_string(),
                    format!("Could not read {}: {e}", short_path(path)),
                ),
            }
        }
        "write_file" => {
            let path = input.get("path").and_then(Value::as_str).unwrap_or("");
            let content = input
                .get("content")
                .or_else(|| input.get("contents"))
                .and_then(Value::as_str)
                .unwrap_or("");
            match tools::tool_write(policy, path, content) {
                Ok(saved) => ToolDispatchResult {
                    content: format!("Saved file: {saved}"),
                    is_error: false,
                    summary: format!("Saved {}", file_name(&saved)),
                    artifact: Some(saved),
                },
                Err(e) => error_result(
                    &e.to_string(),
                    format!("Could not write {}", short_path(path)),
                ),
            }
        }
        "run_program" => {
            let command = input.get("command").and_then(Value::as_str).unwrap_or("");
            let before = snapshot_files(outbox);
            match tools::tool_bash(policy, command) {
                Ok(run) => {
                    let mut created: Vec<String> = snapshot_files(outbox)
                        .difference(&before)
                        .cloned()
                        .collect();
                    created.sort();
                    let ok = run.code == Some(0) && !run.timed_out;
                    let code = run
                        .code
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "killed".to_string());
                    let mut content = String::new();
                    if run.timed_out {
                        content.push_str("The program was stopped after the time limit.\n");
                    }
                    content.push_str(&format!("exit_code: {code}\n"));
                    content.push_str(&format!("stdout:\n{}\n", truncate(&run.stdout, 16_000)));
                    if !run.stderr.trim().is_empty() {
                        content.push_str(&format!("stderr:\n{}\n", truncate(&run.stderr, 6_000)));
                    }
                    if !created.is_empty() {
                        content.push_str(&format!("created_files: {}\n", created.join(", ")));
                    }
                    let summary = if ok {
                        format!("Ran {}", program_name(command))
                    } else {
                        format!("{} exited with {}", program_name(command), code)
                    };
                    ToolDispatchResult {
                        content,
                        is_error: !ok,
                        summary,
                        artifact: created.into_iter().next(),
                    }
                }
                Err(e) => {
                    error_result(&e.to_string(), "Program blocked by the sandbox".to_string())
                }
            }
        }
        other => error_result(
            &format!("unknown tool {other}"),
            format!("Unknown tool {other}"),
        ),
    }
}

fn error_result(detail: &str, summary: String) -> ToolDispatchResult {
    ToolDispatchResult {
        content: format!("Error: {detail}"),
        is_error: true,
        summary,
        artifact: None,
    }
}

/// Collect the set of file paths under `dir` (recursive, bounded).
fn snapshot_files(dir: &Path) -> HashSet<String> {
    let mut out = HashSet::new();
    collect_files(dir, &mut out, 0);
    out
}

fn collect_files(dir: &Path, out: &mut HashSet<String>, depth: usize) {
    if depth > 6 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out, depth + 1);
        } else {
            out.insert(path.to_string_lossy().to_string());
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}… (truncated)", &s[..end])
}

fn short_path(path: &str) -> String {
    file_name(path)
}

fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

fn program_name(command: &str) -> String {
    command
        .split_whitespace()
        .next()
        .map(file_name)
        .unwrap_or_else(|| "program".to_string())
}

#[tauri::command]
pub fn get_poll_status(state: State<'_, AppState>) -> PollStatus {
    state
        .poll_status
        .lock()
        .map(|g| g.clone())
        .unwrap_or_else(|e| e.into_inner().clone())
}

#[tauri::command]
pub fn get_last_chat_model(state: State<'_, AppState>) -> Result<Option<String>, CommandError> {
    let cfg = DesktopConfig::load(&state.app_dirs.config_file())?;
    Ok(cfg.last_chat_model)
}

#[tauri::command]
pub fn set_last_chat_model(state: State<'_, AppState>, model: String) -> Result<(), CommandError> {
    let mut cfg = DesktopConfig::load(&state.app_dirs.config_file())?;
    let trimmed = model.trim();
    cfg.last_chat_model = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    };
    cfg.save(&state.app_dirs.config_file())?;
    Ok(())
}

#[tauri::command]
pub fn get_studio_tiles(state: State<'_, AppState>) -> Result<Vec<String>, CommandError> {
    let cfg = DesktopConfig::load(&state.app_dirs.config_file())?;
    Ok(cfg.studio_tiles)
}

#[tauri::command]
pub fn set_studio_tiles(
    state: State<'_, AppState>,
    tiles: Vec<String>,
) -> Result<Vec<String>, CommandError> {
    let mut cfg = DesktopConfig::load(&state.app_dirs.config_file())?;
    cfg.studio_tiles = synaplan_core::config::sanitize_studio_tiles(tiles);
    cfg.save(&state.app_dirs.config_file())?;
    Ok(cfg.studio_tiles)
}

#[tauri::command]
pub fn get_ui_prefs(state: State<'_, AppState>) -> Result<UiPrefs, CommandError> {
    let cfg = DesktopConfig::load(&state.app_dirs.config_file())?;
    Ok(cfg.ui)
}

#[tauri::command]
pub fn set_ui_prefs(state: State<'_, AppState>, prefs: UiPrefs) -> Result<UiPrefs, CommandError> {
    let mut cfg = DesktopConfig::load(&state.app_dirs.config_file())?;
    cfg.ui = prefs.sanitized();
    cfg.save(&state.app_dirs.config_file())?;
    Ok(cfg.ui)
}

/// Where this install keeps things, for the Settings page. Read-only: the
/// layout is fixed once installed (see `platform::app_dirs`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfoDto {
    pub projects_dir: String,
    pub outbox_dir: String,
    pub skills_dir: String,
    pub config_dir: String,
}

#[tauri::command]
pub fn get_storage_info(state: State<'_, AppState>) -> StorageInfoDto {
    let d = &state.app_dirs;
    StorageInfoDto {
        projects_dir: d.projects_dir.to_string_lossy().to_string(),
        outbox_dir: d.outbox_dir.to_string_lossy().to_string(),
        skills_dir: d.skills_dir.to_string_lossy().to_string(),
        config_dir: d.config_dir.to_string_lossy().to_string(),
    }
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> Result<bool, CommandError> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch()
        .is_enabled()
        .map_err(|e| CommandError::new("autostart", e.to_string()))
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool, CommandError> {
    use tauri_plugin_autostart::ManagerExt;
    let mgr = app.autolaunch();
    if enabled {
        mgr.enable()
            .map_err(|e| CommandError::new("autostart", e.to_string()))?;
    } else {
        mgr.disable()
            .map_err(|e| CommandError::new("autostart", e.to_string()))?;
    }
    mgr.is_enabled()
        .map_err(|e| CommandError::new("autostart", e.to_string()))
}
