//! Knowledge-folder commands (`PC9`): list, add and remove files of a project on
//! the paired workspace. Wiring only — the multipart shape, the model hints and
//! the state mapping live in `synaplan_core::files`.
//!
//! A file offered for upload is read by Rust from a path the webview received
//! from the OS (drag-and-drop) or the person typed. That path must lie inside a
//! folder this app may use (the "This computer" allowlist or the project's own
//! folder) and must not match a deny rule; adding a file is a copy to Synaplan,
//! never a new write root.

use std::path::PathBuf;

use synaplan_core::config::DesktopConfig;
use synaplan_core::files::{self, FilesError, KnowledgeFile, UploadHints};
use synaplan_core::pairing;
use synaplan_core::platform::confinement::{Access, Confinement, ConfinementError};
use tauri::State;

use super::{AppState, CommandError};

impl From<FilesError> for CommandError {
    fn from(e: FilesError) -> Self {
        CommandError::new(e.code(), e.to_string())
    }
}

/// Error the UI maps to "add this folder under This computer first".
pub const FILE_OUTSIDE_ALLOWED: &str = "file_outside_allowed";

fn confinement_error(e: ConfinementError) -> CommandError {
    match e {
        ConfinementError::OutsideRoots | ConfinementError::NotWritable => CommandError::new(
            FILE_OUTSIDE_ALLOWED,
            "This file is not in a folder this app may use.",
        ),
        ConfinementError::Denied => {
            CommandError::new("file_denied", "This file is protected and cannot be sent.")
        }
        other => CommandError::new("file_unreadable", other.to_string()),
    }
}

impl AppState {
    pub(crate) fn paired(&self) -> Result<(String, String), CommandError> {
        let cfg = DesktopConfig::load(&self.app_dirs.config_file())?;
        let base = cfg.api_base_url.ok_or_else(CommandError::not_paired)?;
        let key = self.secret.get()?.ok_or_else(CommandError::not_paired)?;
        Ok((base, key))
    }

    /// Resolve a source file for upload: inside an allowed folder or the
    /// project's own folder, not denied, canonical.
    fn upload_source(&self, project_id: &str, raw: &str) -> Result<PathBuf, CommandError> {
        let policy = self.load_policy()?;
        let store = self.project_store();
        let project = store.get_project(project_id)?;
        store.create_dirs(&project)?;
        let mut read_roots: Vec<PathBuf> = policy.read.iter().map(PathBuf::from).collect();
        read_roots.push(store.contained_project_dir(&project)?);
        let write_roots: Vec<PathBuf> = policy.write.iter().map(PathBuf::from).collect();
        let confinement =
            Confinement::new(&read_roots, &write_roots, &policy.deny).map_err(confinement_error)?;
        confinement
            .resolve(raw.trim(), Access::Read)
            .map_err(confinement_error)
    }

    /// Wipe credentials only when the key itself is rejected (401/403).
    /// Network or "feature disabled" from `/v1/models` must not unpair.
    pub(crate) async fn wipe_if_key_revoked(&self, base: &str, key: &str) -> bool {
        if matches!(
            pairing::verify_key(base, key).await,
            Err(pairing::PairError::Unauthorized)
        ) {
            let _ = self.secret.delete();
            let _ = DesktopConfig::clear(&self.app_dirs.config_file());
            true
        } else {
            false
        }
    }

    /// Wipe credentials only when the key itself no longer authenticates.
    pub(crate) async fn on_files_unauthorized(&self, base: &str, key: &str) {
        let _ = self.wipe_if_key_revoked(base, key).await;
    }
}

#[tauri::command]
pub async fn list_project_files(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<KnowledgeFile>, CommandError> {
    let (base, key) = state.paired()?;
    state.project_store().get_project(&project_id)?;
    match files::list_project_files(&base, &key, &project_id).await {
        Err(FilesError::Unauthorized) => {
            state.on_files_unauthorized(&base, &key).await;
            Err(FilesError::Unauthorized.into())
        }
        other => Ok(other?),
    }
}

/// Send one local file into the project's knowledge folder. Index uses the
/// workspace VECTORIZE default (same as chat search). Documents binding is
/// optional and only sent as `analyze_model`.
#[tauri::command]
pub async fn upload_project_file(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
) -> Result<KnowledgeFile, CommandError> {
    let (base, key) = state.paired()?;
    let project = state.project_store().get_project(&project_id)?;
    let hints = UploadHints::from_models(&project.models);
    let source = state.upload_source(&project_id, &path)?;
    match files::upload_project_file(&base, &key, &source, &project_id, &hints).await {
        Err(FilesError::Unauthorized) => {
            state.on_files_unauthorized(&base, &key).await;
            Err(FilesError::Unauthorized.into())
        }
        other => Ok(other?),
    }
}

#[tauri::command]
pub async fn delete_project_file(
    state: State<'_, AppState>,
    project_id: String,
    file_id: i64,
) -> Result<(), CommandError> {
    let (base, key) = state.paired()?;
    state.project_store().get_project(&project_id)?;
    match files::delete_owned_project_file(&base, &key, &project_id, file_id).await {
        Err(FilesError::Unauthorized) => {
            state.on_files_unauthorized(&base, &key).await;
            Err(FilesError::Unauthorized.into())
        }
        other => Ok(other?),
    }
}
