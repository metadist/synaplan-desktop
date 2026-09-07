//! Device poll loop wiring (B5). Logic lives in `synaplan-core`; this module
//! starts/stops the loop and maps one tick onto MCP + the same agent tools
//! as interactive chat.

use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::Duration;

use serde_json::{json, Value};
use synaplan_core::agent::{run_agent_turn, AgentEvent, AgentTool};
use synaplan_core::config::DesktopConfig;
use synaplan_core::contract::{DeviceJob, ReportRequest};
use synaplan_core::files;
use synaplan_core::mcp::{McpClient, McpError};
use synaplan_core::messages::ChatError;
use synaplan_core::pairing;
use synaplan_core::platform::doctor;
use synaplan_core::poll::{self, PollStatus};
use synaplan_core::skills::Skill;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::commands::{
    build_system_prompt, build_tool_policy, dispatch_tool, read_file_tool, run_program_tool,
    status_of, write_file_tool, AppState,
};

enum Tick {
    Ok { next_call_at: i64, jobs: u32 },
    Backoff,
    Stop,
}

pub fn start_if_paired(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Ok(status) = status_of(&state) else {
        return;
    };
    if !status.paired {
        return;
    }
    if status.key_is_plaintext {
        publish(
            app,
            &state,
            PollStatus {
                running: false,
                plaintext_blocked: true,
                last_error: Some(
                    "Background jobs are off while the key is stored in a plaintext file.".into(),
                ),
                ..PollStatus::default()
            },
        );
        return;
    }
    if state.poll_running.swap(true, Ordering::SeqCst) {
        return;
    }
    state.poll_stop.store(false, Ordering::SeqCst);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        run_loop(app.clone()).await;
        let state = app.state::<AppState>();
        state.poll_running.store(false, Ordering::SeqCst);
    });
}

pub fn stop(app: &AppHandle) {
    let state = app.state::<AppState>();
    state.poll_stop.store(true, Ordering::SeqCst);
    publish(
        app,
        &state,
        PollStatus {
            running: false,
            ..PollStatus::default()
        },
    );
}

async fn run_loop(app: AppHandle) {
    let mut session: Option<McpClient> = None;
    let mut backoff = 15u64;
    loop {
        {
            let state = app.state::<AppState>();
            if state.poll_stop.load(Ordering::Relaxed) {
                break;
            }
            if state.secret.is_plaintext() {
                publish(
                    &app,
                    &state,
                    PollStatus {
                        running: false,
                        plaintext_blocked: true,
                        last_error: Some(
                            "Background jobs are off while the key is stored in a plaintext file."
                                .into(),
                        ),
                        ..PollStatus::default()
                    },
                );
                break;
            }
        }

        match one_tick(&app, &mut session).await {
            Tick::Ok { next_call_at, jobs } => {
                backoff = 15;
                let now = poll::unix_now();
                {
                    let state = app.state::<AppState>();
                    let mut next = current_status(&state);
                    next.running = true;
                    next.last_checkin_unix = Some(now);
                    next.next_call_at = Some(next_call_at);
                    next.jobs_waiting = jobs;
                    next.last_error = None;
                    next.plaintext_blocked = false;
                    publish(&app, &state, next);
                }
                sleep_interruptible(&app, poll::sleep_secs(now, next_call_at)).await;
            }
            Tick::Backoff => {
                {
                    let state = app.state::<AppState>();
                    let mut next = current_status(&state);
                    next.running = true;
                    next.last_error = Some("Could not reach Synaplan. Trying again.".into());
                    publish(&app, &state, next);
                }
                sleep_interruptible(&app, backoff).await;
                backoff = (backoff.saturating_mul(2)).min(300);
            }
            Tick::Stop => break,
        }
    }
}

async fn one_tick(app: &AppHandle, session: &mut Option<McpClient>) -> Tick {
    let state = app.state::<AppState>();
    let cfg = match DesktopConfig::load(&state.app_dirs.config_file()) {
        Ok(c) => c,
        Err(_) => return Tick::Backoff,
    };
    let Some(base) = cfg.api_base_url.clone() else {
        return Tick::Stop;
    };
    let Ok(Some(key)) = state.secret.get() else {
        return Tick::Stop;
    };
    let skills = crate::commands::listed_skills(&state);
    let request = poll::checkin_request(&skills);

    if session.is_none() {
        match McpClient::initialize(&base, &key).await {
            Ok(client) => *session = Some(client),
            Err(McpError::Unauthorized) => {
                return handle_unauthorized(app, &base, &key).await;
            }
            Err(e) if is_transient(&e) => return Tick::Backoff,
            Err(_) => return Tick::Backoff,
        }
    }

    let client = match session.as_mut() {
        Some(c) => c,
        None => return Tick::Backoff,
    };

    let raw = match client
        .call_tool(
            "agent_checkin",
            serde_json::to_value(&request).unwrap_or(json!({})),
        )
        .await
    {
        Ok(v) => v,
        Err(McpError::Unauthorized) => {
            *session = None;
            return handle_unauthorized(app, &base, &key).await;
        }
        Err(e) if is_transient(&e) => {
            *session = None;
            return Tick::Backoff;
        }
        Err(_) => {
            *session = None;
            return Tick::Backoff;
        }
    };

    let resp = match poll::parse_checkin_response(raw) {
        Ok(r) if poll::protocol_ok(&r) => r,
        _ => return Tick::Backoff,
    };

    for job in &resp.jobs {
        process_job(app, &base, &key, &cfg, &skills, client, job).await;
    }

    Tick::Ok {
        next_call_at: resp.next_call_at,
        jobs: resp.jobs.len() as u32,
    }
}

#[allow(clippy::too_many_arguments)]
async fn process_job(
    app: &AppHandle,
    base: &str,
    key: &str,
    cfg: &DesktopConfig,
    skills: &[Skill],
    client: &mut McpClient,
    job: &DeviceJob,
) {
    if let Err(refusal) = poll::classify_job(job, skills) {
        let _ = report(client, &poll::refusal_report(&job.lease_token, &refusal)).await;
        return;
    }

    match run_job(app, base, key, cfg, skills, job).await {
        Ok((summary, file_ids, artifact)) => {
            notify_first(app, &job.input.skill, artifact.as_deref());
            if report(
                client,
                &poll::success_report(&job.lease_token, &summary, file_ids),
            )
            .await
            .is_err()
            {
                // Leave the lease to expire (DC20).
            }
        }
        Err(JobRun::Unauthorized) => {
            let _ = handle_unauthorized(app, base, key).await;
        }
        Err(JobRun::Failed(refusal)) => {
            let _ = report(client, &poll::refusal_report(&job.lease_token, &refusal)).await;
        }
    }
}

enum JobRun {
    Unauthorized,
    Failed(poll::JobRefusal),
}

async fn run_job(
    app: &AppHandle,
    base: &str,
    key: &str,
    cfg: &DesktopConfig,
    skills: &[Skill],
    job: &DeviceJob,
) -> Result<(String, Vec<i64>, Option<String>), JobRun> {
    let state = app.state::<AppState>();
    let fs_policy = state.load_policy().map_err(|e| {
        JobRun::Failed(poll::JobRefusal {
            error_code: synaplan_core::contract::ERROR_LOCAL.into(),
            message: e.message,
        })
    })?;
    let skills_dir = state.app_dirs.skills_dir.clone();
    let outbox = state.app_dirs.outbox_dir.clone();
    let _ = std::fs::create_dir_all(&outbox);

    let tools_cfg = cfg.tools.clone();
    let programs =
        tauri::async_runtime::spawn_blocking(move || doctor::allowlisted_programs_with(&tools_cfg))
            .await
            .unwrap_or_default();
    let allow_exec = !programs.is_empty();
    let policy = build_tool_policy(&fs_policy, &skills_dir, &outbox, programs).map_err(|e| {
        JobRun::Failed(poll::JobRefusal {
            error_code: synaplan_core::contract::ERROR_LOCAL.into(),
            message: e,
        })
    })?;

    let enabled: Vec<Skill> = skills
        .iter()
        .filter(|s| s.enabled && !s.blocked)
        .cloned()
        .collect();
    let system = build_system_prompt(&enabled, &skills_dir, &outbox, &fs_policy.read, allow_exec);
    let mut tools: Vec<AgentTool> = vec![read_file_tool(), write_file_tool()];
    if allow_exec {
        tools.push(run_program_tool());
    }

    let prompt = format!(
        "The workspace asked this computer to run the skill '{}'.\n\n{}",
        job.input.skill, job.input.prompt
    );
    let msgs = vec![json!({ "role": "user", "content": prompt })];
    let artifacts = std::sync::Mutex::new(Vec::<String>::new());
    let summary = std::sync::Mutex::new(String::new());
    let cancel = std::sync::atomic::AtomicBool::new(false);

    let result = run_agent_turn(
        base,
        key,
        None,
        &system,
        msgs,
        &tools,
        &cancel,
        |name, input| {
            let out = dispatch_tool(&policy, &outbox, name, input);
            if let Some(path) = &out.artifact {
                artifacts
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(path.clone());
            }
            out
        },
        |event| {
            if let AgentEvent::Text(text) = event {
                let mut s = summary.lock().unwrap_or_else(|e| e.into_inner());
                s.push_str(&text);
            }
        },
    )
    .await;

    if let Err(err) = result {
        let message = err.to_string();
        if matches!(err, ChatError::Unauthorized) {
            let state = app.state::<AppState>();
            let (code, _) = crate::commands::classify_turn_error(&state, base, key, err).await;
            if code == "unauthorized" {
                return Err(JobRun::Unauthorized);
            }
        }
        return Err(JobRun::Failed(poll::JobRefusal {
            error_code: synaplan_core::contract::ERROR_LOCAL.into(),
            message,
        }));
    }

    let created = artifacts.into_inner().unwrap_or_else(|e| e.into_inner());
    let mut file_ids = Vec::new();
    for path in &created {
        if let Ok(id) = files::upload_store(base, key, Path::new(path)).await {
            file_ids.push(id);
        }
    }
    let text = summary.into_inner().unwrap_or_else(|e| e.into_inner());
    let summary = if text.trim().is_empty() {
        format!("Finished '{}'.", job.input.skill)
    } else {
        text.chars().take(400).collect()
    };
    Ok((summary, file_ids, created.into_iter().next()))
}

async fn report(client: &mut McpClient, body: &ReportRequest) -> Result<Value, McpError> {
    client
        .call_tool(
            "agent_report_result",
            serde_json::to_value(body).unwrap_or(json!({})),
        )
        .await
}

async fn handle_unauthorized(app: &AppHandle, base: &str, key: &str) -> Tick {
    let state = app.state::<AppState>();
    if pairing::verify_key(base, key).await.is_err() {
        let _ = state.secret.delete();
        let _ = DesktopConfig::clear(&state.app_dirs.config_file());
        state.poll_stop.store(true, Ordering::SeqCst);
        publish(
            app,
            &state,
            PollStatus {
                running: false,
                last_error: Some("This computer was disconnected. Pair again.".into()),
                ..PollStatus::default()
            },
        );
        Tick::Stop
    } else {
        Tick::Backoff
    }
}

fn is_transient(err: &McpError) -> bool {
    match err {
        McpError::Network => true,
        McpError::Protocol(msg) => msg.contains("HTTP 5") || msg.contains("HTTP 429"),
        McpError::Unauthorized => false,
    }
}

fn current_status(state: &AppState) -> PollStatus {
    state
        .poll_status
        .lock()
        .map(|g| g.clone())
        .unwrap_or_else(|e| e.into_inner().clone())
}

fn publish(app: &AppHandle, state: &AppState, status: PollStatus) {
    if let Ok(mut g) = state.poll_status.lock() {
        *g = status.clone();
    }
    let _ = app.emit("poll://status", &status);
    crate::tray::refresh(app, &status);
}

async fn sleep_interruptible(app: &AppHandle, secs: u64) {
    let mut left = secs;
    while left > 0 {
        {
            let state = app.state::<AppState>();
            if state.poll_stop.load(Ordering::Relaxed) {
                return;
            }
        }
        let slice = left.min(1);
        tokio::time::sleep(Duration::from_secs(slice)).await;
        left -= slice;
    }
}

fn notify_first(app: &AppHandle, skill: &str, artifact: Option<&str>) {
    let state = app.state::<AppState>();
    let marker = state.app_dirs.config_dir.join("unattended-notified");
    if marker.exists() {
        return;
    }
    let _ = std::fs::create_dir_all(&state.app_dirs.config_dir);
    let _ = std::fs::write(&marker, b"1");
    let file = artifact
        .and_then(|p| Path::new(p).file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "a file".into());
    let _ = app
        .notification()
        .builder()
        .title("Synaplan Desktop")
        .body(format!("{skill} created {file}"))
        .show();
}
