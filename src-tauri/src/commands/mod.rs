//! Tauri command surface. This module is *wiring only*: it maps `#[tauri::command]`
//! entry points to `synaplan-core` functions, holds shared state, and emits chat
//! stream events. No business logic and no platform branch lives here — those
//! are in `synaplan-core` (and, for OS differences, `synaplan-core::platform`).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Value};
use synaplan_core::agent::{self, AgentEvent, AgentTool, ToolDispatchResult};
use synaplan_core::config::DesktopConfig;
use synaplan_core::filesystem::{FilesystemPolicy, FsPolicyError};
use synaplan_core::messages::{self, ChatError, ChatMessage, ModelInfo};
use synaplan_core::pairing::{self, PairError};
use synaplan_core::platform::app_dirs::AppDirs;
use synaplan_core::platform::confinement::Confinement;
use synaplan_core::platform::doctor;
use synaplan_core::platform::secret_store::SecretStore;
use synaplan_core::skills::{self, Skill};
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
    pub plaintext_key_path: Option<String>,
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
        plaintext_key_path: state
            .secret
            .plaintext_path()
            .map(|p| p.to_string_lossy().into_owned()),
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
    // Keep the address the user actually reached. The server also returns
    // APP_URL, which in Docker can be an internal hostname the desktop cannot
    // call (or a different published port than the one that worked).
    let cfg = DesktopConfig {
        api_base_url: Some(base),
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
        let (code, message) = classify_turn_error(&state, &base, &key, err).await;
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

/// Map a turn error to `(code, message)`, wiping credentials only on a genuine
/// revoked-key 401 (re-verified against `/v1/models`). Shared by chat + agent.
async fn classify_turn_error(
    state: &AppState,
    base: &str,
    key: &str,
    err: ChatError,
) -> (String, String) {
    if matches!(err, ChatError::Unauthorized) {
        if pairing::verify_key(base, key).await.is_err() {
            let _ = state.secret.delete();
            let _ = DesktopConfig::clear(&state.app_dirs.config_file());
            ("unauthorized".to_string(), err.to_string())
        } else {
            ("server".to_string(), err.to_string())
        }
    } else {
        (err.code().to_string(), err.to_string())
    }
}

impl AppState {
    fn execution_consent_path(&self) -> PathBuf {
        self.app_dirs.config_dir.join("execution-consent")
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
    messages: Vec<ChatMessage>,
    model: Option<String>,
    allow_exec: bool,
) -> Result<(), CommandError> {
    let cfg = DesktopConfig::load(&state.app_dirs.config_file())?;
    let base = cfg.api_base_url.ok_or_else(CommandError::not_paired)?;
    let key = state.secret.get()?.ok_or_else(CommandError::not_paired)?;

    state.cancel.store(false, Ordering::Relaxed);

    let fs_policy = state.load_policy()?;
    let skills_dir = state.app_dirs.skills_dir.clone();
    let outbox = state.app_dirs.outbox_dir.clone();
    let _ = std::fs::create_dir_all(&outbox);
    let enabled: Vec<Skill> = skills::load_skills(&skills_dir)
        .into_iter()
        .filter(|s| s.enabled)
        .collect();

    // Interpreter allowlist (blocking discovery on a worker thread).
    let programs = tauri::async_runtime::spawn_blocking(doctor::allowlisted_programs)
        .await
        .unwrap_or_default();
    let allow_exec = allow_exec && !programs.is_empty();

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
    let result = agent::run_agent_turn(
        &base,
        &key,
        model.as_deref(),
        &system,
        msgs,
        &tools,
        &state.cancel,
        |name, input| dispatch_tool(&policy, &outbox_for_dispatch, name, input),
        |event| emit_agent_event(&emitter, event),
    )
    .await;

    if let Err(err) = result {
        let (code, message) = classify_turn_error(&state, &base, &key, err).await;
        let _ = app.emit(
            "agent://error",
            StreamError {
                code: code.clone(),
                message: message.clone(),
            },
        );
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
fn build_tool_policy(
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

fn read_file_tool() -> AgentTool {
    AgentTool {
        name: "read_file".to_string(),
        description: "Read a UTF-8 text file the user allowed (a skill file or a folder they added). Returns the file contents.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": { "path": { "type": "string", "description": "Absolute path to the file." } },
            "required": ["path"]
        }),
    }
}

fn write_file_tool() -> AgentTool {
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

fn run_program_tool() -> AgentTool {
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

fn build_system_prompt(
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
            s.push_str(&format!(
                "- {} — {} (folder: {})\n",
                skill.name,
                skill.description,
                skills_dir.join(&skill.name).display()
            ));
        }
    }
    s.push('\n');
    s.push_str("HOW TO WORK:\n");
    s.push_str("1. If a skill fits the request, read its SKILL.md with read_file to learn how to invoke it.\n");
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
fn dispatch_tool(
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
                    format!("Could not read {}", short_path(path)),
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
