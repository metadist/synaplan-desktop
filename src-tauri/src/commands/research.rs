//! Research commands. Wiring only: the key stays in the secret store, and the
//! webview only sees run ids, events and file paths.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;
use serde_json::{json, Value};
use synaplan_core::files::MAX_UPLOAD_BYTES;
use synaplan_core::research::{self, ResearchError, ResearchEvent, ResearchInput};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_notification::NotificationExt;

use super::{AppState, CommandError};

impl From<ResearchError> for CommandError {
    fn from(error: ResearchError) -> Self {
        CommandError::new(error.code(), error.to_string())
    }
}

fn attached() -> &'static Mutex<HashSet<String>> {
    static CELL: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(HashSet::new()))
}

fn cancels() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    static CELL: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cancel_flag(id: &str) -> Arc<AtomicBool> {
    cancels()
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .entry(id.to_string())
        .or_insert_with(|| Arc::new(AtomicBool::new(false)))
        .clone()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResearchOpen {
    project_id: String,
    chat_id: String,
    sentence: String,
}

static PENDING_OPEN: Mutex<Option<ResearchOpen>> = Mutex::new(None);

fn terminal(event: &ResearchEvent) -> bool {
    matches!(
        event,
        ResearchEvent::Complete { .. }
            | ResearchEvent::Stopped { .. }
            | ResearchEvent::Failed { .. }
    )
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilitiesDto {
    pub enabled: bool,
    pub research_protocol: u32,
    pub depths: Vec<String>,
    pub web_search_available: bool,
    pub web_search_provider: Option<research::WebSearchProvider>,
    pub knowledge_available: bool,
    pub read_documents: bool,
    pub strict_world: bool,
    pub max_active_thorough: u32,
    pub limits: LimitsDto,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitsDto {
    pub standard_per_day: Option<u32>,
    pub thorough_per_day: Option<u32>,
}

#[tauri::command]
pub async fn research_capabilities(
    state: State<'_, AppState>,
) -> Result<Option<CapabilitiesDto>, CommandError> {
    let (base, key) = state.paired()?;
    let Some(caps) = research::capabilities(&base, &key).await? else {
        return Ok(None);
    };
    Ok(Some(CapabilitiesDto {
        enabled: caps.enabled,
        research_protocol: caps.research_protocol,
        depths: caps.depths,
        web_search_available: caps.web_search_available,
        web_search_provider: caps.web_search_provider,
        knowledge_available: caps.knowledge_available,
        read_documents: caps.read_documents,
        strict_world: caps.strict_world,
        max_active_thorough: caps.max_active_thorough,
        limits: LimitsDto {
            standard_per_day: caps.limits.standard_per_day,
            thorough_per_day: caps.limits.thorough_per_day,
        },
    }))
}

#[tauri::command]
pub async fn research_start(
    state: State<'_, AppState>,
    project_id: String,
    question: String,
    depth: String,
    web: bool,
    use_files: bool,
    idempotency_key: String,
) -> Result<research::RunRef, CommandError> {
    let (base, key) = state.paired()?;
    let project = state.project_store().get_project(&project_id)?;
    let knowledge = if use_files {
        Some(project.knowledge_folder.as_str())
    } else {
        None
    };
    let body = research::start_body(&question, &depth, web, knowledge, &project.models);
    Ok(research::start(&base, &key, &body, &idempotency_key).await?)
}

#[tauri::command]
pub async fn research_attach(
    app: AppHandle,
    state: State<'_, AppState>,
    run_id: String,
) -> Result<(), CommandError> {
    {
        let mut attached = attached().lock().unwrap_or_else(|err| err.into_inner());
        if !research::claim_attach(&mut attached, &run_id) {
            return Ok(());
        }
    }
    let (base, key) = match state.paired() {
        Ok(pair) => pair,
        Err(error) => {
            release(&run_id);
            return Err(error);
        }
    };
    let flag = cancel_flag(&run_id);
    flag.store(false, Ordering::Relaxed);
    let run_id_loop = run_id.clone();
    tauri::async_runtime::spawn(async move {
        let mut after: Option<u64> = None;
        let mut failures: u32 = 0;
        loop {
            if flag.load(Ordering::Relaxed) {
                break;
            }
            match research::poll_events(&base, &key, &run_id_loop, after).await {
                Ok(frames) => {
                    failures = 0;
                    let mut done = false;
                    for frame in frames {
                        if let Some(seq) = frame.seq {
                            after = Some(seq);
                        }
                        emit_frame(&app, &run_id_loop, frame.seq, &frame.name, frame.data);
                        if terminal(&frame.event) {
                            done = true;
                        }
                    }
                    if done {
                        break;
                    }
                    tokio::time::sleep(POLL_EVERY).await;
                }
                // Offline or a busy server: keep following, a little slower each time.
                Err(ResearchError::Network) => {
                    failures = failures.saturating_add(1);
                    tokio::time::sleep(backoff(failures)).await;
                }
                Err(error) => {
                    emit_frame(
                        &app,
                        &run_id_loop,
                        None,
                        "detached",
                        json!({ "code": error.code(), "message": error.to_string() }),
                    );
                    break;
                }
            }
        }
        release(&run_id_loop);
        forget_cancel(&run_id_loop, &flag);
    });
    Ok(())
}

const POLL_EVERY: std::time::Duration = std::time::Duration::from_secs(2);

fn backoff(failures: u32) -> std::time::Duration {
    std::time::Duration::from_secs((2u64 << failures.min(4)).min(30))
}

fn emit_frame(app: &AppHandle, run_id: &str, seq: Option<u64>, name: &str, data: Value) {
    let _ = app.emit(
        "research://event",
        json!({ "runId": run_id, "seq": seq, "name": name, "data": data }),
    );
}

fn release(id: &str) {
    let mut attached = attached().lock().unwrap_or_else(|err| err.into_inner());
    attached.remove(id);
}

fn forget_cancel(id: &str, flag: &Arc<AtomicBool>) {
    let mut cancels = cancels().lock().unwrap_or_else(|err| err.into_inner());
    if cancels
        .get(id)
        .is_some_and(|stored| Arc::ptr_eq(stored, flag))
    {
        cancels.remove(id);
    }
}

fn stop_following(id: &str) {
    let flag = cancels()
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .get(id)
        .cloned();
    if let Some(flag) = flag {
        flag.store(true, Ordering::Relaxed);
    }
}

#[tauri::command]
pub async fn research_state(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<research::RunState, CommandError> {
    let (base, key) = state.paired()?;
    Ok(research::state(&base, &key, &run_id).await?)
}

#[tauri::command]
pub async fn research_input(
    state: State<'_, AppState>,
    run_id: String,
    input: Value,
) -> Result<research::RunState, CommandError> {
    let (base, key) = state.paired()?;
    let parsed: ResearchInput = serde_json::from_value(input)
        .map_err(|_| CommandError::new("invalid", "That reply does not fit."))?;
    Ok(research::input(&base, &key, &run_id, &parsed).await?)
}

#[tauri::command]
pub async fn research_cancel(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<research::RunState, CommandError> {
    // Keep following: a step underway finishes first, then `stopped` arrives.
    let (base, key) = state.paired()?;
    Ok(research::cancel_run(&base, &key, &run_id).await?)
}

#[tauri::command]
pub async fn research_follow_up(
    state: State<'_, AppState>,
    run_id: String,
    kind: String,
    text: Option<String>,
    idempotency_key: String,
) -> Result<research::RunRef, CommandError> {
    let (base, key) = state.paired()?;
    let follow = research::FollowUp { kind, text };
    Ok(research::follow_up(&base, &key, &run_id, &follow, &idempotency_key).await?)
}

#[tauri::command]
pub async fn research_list_active(
    state: State<'_, AppState>,
) -> Result<Vec<research::RunRef>, CommandError> {
    let (base, key) = state.paired()?;
    Ok(research::list_active(&base, &key).await?)
}

#[tauri::command]
pub async fn research_delete(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<(), CommandError> {
    stop_following(&run_id);
    let (base, key) = state.paired()?;
    research::delete_run(&base, &key, &run_id).await?;
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedReport {
    pub path: String,
    pub sources_path: String,
    pub version: u32,
}

#[tauri::command]
pub fn research_save_report(
    state: State<'_, AppState>,
    project_id: String,
    question: String,
    version: u32,
    markdown: String,
    sources_json: String,
) -> Result<SavedReport, CommandError> {
    let store = state.project_store();
    let project = store.get_project(&project_id)?;
    let dir = store.out_dir(&project);
    let slug = research::report_slug(&question);
    let files = research::write_report_files(&dir, &slug, version, &markdown, &sources_json)
        .map_err(|err| CommandError::new("project_io", err.to_string()))?;
    Ok(SavedReport {
        path: files.report.to_string_lossy().to_string(),
        sources_path: files.sources.to_string_lossy().to_string(),
        version,
    })
}

#[tauri::command]
pub fn research_list_versions(
    state: State<'_, AppState>,
    project_id: String,
    question: String,
) -> Result<Vec<u32>, CommandError> {
    let store = state.project_store();
    let project = store.get_project(&project_id)?;
    Ok(research::list_versions(
        &store.out_dir(&project),
        &research::report_slug(&question),
    ))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoredReport {
    pub path: String,
    pub markdown: String,
    pub version: u32,
}

#[tauri::command]
pub fn research_restore_version(
    state: State<'_, AppState>,
    project_id: String,
    question: String,
    version: u32,
) -> Result<RestoredReport, CommandError> {
    let store = state.project_store();
    let project = store.get_project(&project_id)?;
    let (path, markdown) = research::restore_version(
        &store.out_dir(&project),
        &research::report_slug(&question),
        version,
    )
    .map_err(|err| CommandError::new("file_unreadable", err.to_string()))?;
    Ok(RestoredReport {
        path: path.to_string_lossy().to_string(),
        markdown,
        version,
    })
}

/// Async so a large folder never freezes the window.
#[tauri::command]
pub async fn research_walk_folder(
    path: String,
    subfolders: bool,
) -> Result<research::FolderWalk, CommandError> {
    let root = std::path::PathBuf::from(path.trim());
    if !root.is_dir() {
        return Err(CommandError::new(
            "path_missing",
            "That folder no longer exists on this computer.",
        ));
    }
    tauri::async_runtime::spawn_blocking(move || {
        research::walk_folder(&root, subfolders, MAX_UPLOAD_BYTES)
    })
    .await
    .map_err(|err| CommandError::new("unexpected", err.to_string()))?
    .map_err(|err| CommandError::new("file_unreadable", err.to_string()))
}

#[tauri::command]
pub fn research_notify(
    app: AppHandle,
    sentence: String,
    project_id: String,
    chat_id: String,
) -> Result<(), CommandError> {
    *PENDING_OPEN.lock().unwrap_or_else(|err| err.into_inner()) = Some(ResearchOpen {
        project_id,
        chat_id,
        sentence: sentence.clone(),
    });
    let _ = app
        .notification()
        .builder()
        .title("Synaplan Desktop")
        .body(sentence)
        .show();
    Ok(())
}

#[tauri::command]
pub fn research_take_open() -> Result<Option<ResearchOpen>, CommandError> {
    Ok(PENDING_OPEN
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .take())
}

#[tauri::command]
pub fn research_set_tray(app: AppHandle, text: String) {
    crate::tray::set_research_tip(&app, text);
}
