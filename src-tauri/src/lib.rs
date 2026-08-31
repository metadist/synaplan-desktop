//! The Synaplan Desktop Tauri application. This crate is a thin shell: it builds
//! the shared state (resolved [`AppDirs`] + the OS [`SecretStore`]) and registers
//! the commands in [`commands`]. All logic lives in the `synaplan-core` crate.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use synaplan_core::platform::app_dirs::AppDirs;
use synaplan_core::platform::secret_store::{default_secret_store, SecretStore};

mod commands;

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
        .manage(AppState {
            app_dirs,
            secret,
            cancel: Arc::new(AtomicBool::new(false)),
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running Synaplan Desktop");
}
