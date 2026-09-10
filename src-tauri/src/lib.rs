//! The Synaplan Desktop Tauri application. This crate is a thin shell: it builds
//! the shared state (resolved [`AppDirs`] + the OS [`SecretStore`]) and registers
//! the commands in [`commands`]. All logic lives in the `synaplan-core` crate.

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use synaplan_core::platform::app_dirs::AppDirs;
use synaplan_core::platform::secret_store::{default_secret_store, SecretStore};
use synaplan_core::poll::PollStatus;
use tauri::Manager;

mod commands;
mod poll_loop;
mod tray;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_dirs = AppDirs::from_system().expect("failed to resolve application directories");

    // The plaintext fallback (headless Linux only) is opt-in via an env var and
    // is decided inside default_secret_store; everywhere else this is the native
    // OS secret store.
    let secret: Arc<dyn SecretStore> = Arc::from(
        default_secret_store(&app_dirs.config_dir).expect("failed to initialise the secret store"),
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_notification::init())
        .manage(AppState {
            app_dirs,
            secret,
            cancel: Arc::new(AtomicBool::new(false)),
            poll_stop: Arc::new(AtomicBool::new(false)),
            poll_running: Arc::new(AtomicBool::new(false)),
            poll_status: Arc::new(Mutex::new(PollStatus::default())),
        })
        .setup(|app| {
            tray::setup(app)?;
            // First-launch Personal project (idempotent). A failure here is
            // surfaced again by the first `list_projects` call with a code.
            if let Err(e) = app.state::<AppState>().ensure_projects() {
                eprintln!("projects: {}", e.message);
            }
            poll_loop::start_if_paired(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::default_device_name,
            commands::validate_base_url,
            commands::pair,
            commands::pair_with_key,
            commands::sign_out,
            commands::list_models,
            commands::send_chat,
            commands::cancel_chat,
            commands::open_url,
            commands::reveal_path,
            commands::get_filesystem_policy,
            commands::add_read_folder,
            commands::remove_read_folder,
            commands::list_skills,
            commands::set_skill_enabled,
            commands::set_skill_unattended,
            commands::preview_skill_folder,
            commands::preview_skill_zip,
            commands::preview_skill_url,
            commands::install_skill_from_folder,
            commands::install_skill_from_zip,
            commands::install_skill_from_url,
            commands::remove_skill,
            commands::skills_dir,
            commands::run_doctor,
            commands::send_agent_chat,
            commands::get_execution_consent,
            commands::set_execution_consent,
            commands::get_poll_status,
            commands::get_last_chat_model,
            commands::set_last_chat_model,
            commands::get_studio_tiles,
            commands::set_studio_tiles,
            commands::get_autostart,
            commands::set_autostart,
            commands::projects::list_projects,
            commands::projects::get_project,
            commands::projects::get_active_project,
            commands::projects::create_project,
            commands::projects::update_project,
            commands::projects::delete_project,
            commands::projects::set_active_project,
            commands::projects::get_model_catalog,
            commands::projects::list_chats,
            commands::projects::new_chat,
            commands::projects::load_chat,
            commands::projects::save_chat,
            commands::projects::delete_chat,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Synaplan Desktop");
}
