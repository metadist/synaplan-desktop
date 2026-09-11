//! Create an image, sound, video, or document in a project and keep it:
//! write the bytes under `{project}/out/`, then upload into `DESKTOP:{id}`.

use synaplan_core::artifacts::{self, ChatArtifact};
use synaplan_core::files::{self, FilesError, UploadHints};
use synaplan_core::generation::{self, GenerationKind};
use synaplan_core::media::{self, MediaError};
use synaplan_core::projects::{ModelSlot, Project};
use tauri::State;

use super::{AppState, CommandError};

impl From<MediaError> for CommandError {
    fn from(e: MediaError) -> Self {
        CommandError::new(e.code(), e.to_string())
    }
}

#[tauri::command]
pub fn classify_generation(text: String) -> Option<String> {
    generation::classify(&text).map(|k| k.as_str().to_string())
}

/// Generate on the paired workspace, save in the project out folder, and add
/// the file to the project's knowledge folder.
#[tauri::command]
pub async fn generate_and_attach(
    state: State<'_, AppState>,
    project_id: String,
    kind: String,
    prompt: String,
) -> Result<ChatArtifact, CommandError> {
    let kind = GenerationKind::parse(&kind).ok_or_else(|| {
        CommandError::new("unsupported_type", "That file type cannot be created.")
    })?;
    if matches!(kind, GenerationKind::Document) {
        return Err(CommandError::new(
            "unsupported_type",
            "Documents are written after the chat reply.",
        ));
    }

    let (base, key) = state.paired()?;
    let store = state.project_store();
    let project = store.get_project(&project_id)?;
    store.create_dirs(&project)?;
    let model = required_model(&project, kind)?;
    let remote = match kind {
        GenerationKind::Audio => media::generate_speech(&base, &key, &prompt, model).await,
        GenerationKind::Image => media::generate_media(&base, &key, &prompt, "image", model).await,
        GenerationKind::Video => media::generate_media(&base, &key, &prompt, "video", model).await,
        GenerationKind::Document => unreachable!(),
    };
    let remote = match remote {
        Err(MediaError::Unauthorized) => {
            state.on_files_unauthorized(&base, &key).await;
            return Err(MediaError::Unauthorized.into());
        }
        other => other?,
    };

    let bytes = match media::download_bytes(&base, &key, &remote.url).await {
        Err(MediaError::Unauthorized) => {
            state.on_files_unauthorized(&base, &key).await;
            return Err(MediaError::Unauthorized.into());
        }
        other => other?,
    };

    let ext = artifacts::extension_for_mime(&remote.mime, kind.as_str());
    if !artifacts::is_supported(&ext) {
        return Err(CommandError::new(
            "unsupported_type",
            "That file type cannot be added to this project.",
        ));
    }
    let out_dir = store.out_dir(&project);
    let path = artifacts::write_bytes(&out_dir, &prompt, &ext, &bytes)
        .map_err(|e| CommandError::new("project_io", e.to_string()))?;
    publish_out_file(&state, &base, &key, &project, &path).await
}

/// Save chat/skill text as a markdown document in the project and index it.
#[tauri::command]
pub async fn save_text_artifact(
    state: State<'_, AppState>,
    project_id: String,
    content: String,
    name: String,
) -> Result<ChatArtifact, CommandError> {
    let text = content.trim();
    if text.is_empty() {
        return Err(CommandError::new(
            "generation_failed",
            "There is no document text to save.",
        ));
    }
    let (base, key) = state.paired()?;
    let store = state.project_store();
    let project = store.get_project(&project_id)?;
    store.create_dirs(&project)?;
    let out_dir = store.out_dir(&project);
    let path = artifacts::write_bytes(&out_dir, &name, "md", text.as_bytes())
        .map_err(|e| CommandError::new("project_io", e.to_string()))?;
    publish_out_file(&state, &base, &key, &project, &path).await
}

/// Copy a local file into the project out folder (if needed) and index it.
#[tauri::command]
pub async fn attach_local_artifact(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
) -> Result<ChatArtifact, CommandError> {
    let (base, key) = state.paired()?;
    let store = state.project_store();
    let project = store.get_project(&project_id)?;
    store.create_dirs(&project)?;
    let src = std::path::PathBuf::from(path);
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or_default();
    if !artifacts::is_supported(ext) {
        return Err(CommandError::new(
            "unsupported_type",
            "That file type cannot be added to this project.",
        ));
    }
    let dest = artifacts::copy_into_out(&store.out_dir(&project), &src)
        .map_err(|e| CommandError::new("project_io", e.to_string()))?;
    publish_out_file(&state, &base, &key, &project, &dest).await
}

pub(crate) async fn publish_out_file(
    state: &AppState,
    base: &str,
    key: &str,
    project: &Project,
    path: &std::path::Path,
) -> Result<ChatArtifact, CommandError> {
    let hints = UploadHints::from_models(&project.models);
    match files::upload_project_file(base, key, path, &project.id, &hints).await {
        Err(FilesError::Unauthorized) => {
            state.on_files_unauthorized(base, key).await;
            Err(FilesError::Unauthorized.into())
        }
        Err(_) => {
            // Already on this computer even if the knowledge-folder copy failed.
            Ok(ChatArtifact::new(path, None))
        }
        Ok(row) => {
            if files::needs_describe(artifacts::kind_from_path(path)) {
                // Already in the project folder even if describe failed.
                if let Err(FilesError::Unauthorized) = files::describe_file(base, key, row.id).await
                {
                    state.on_files_unauthorized(base, key).await;
                    return Err(FilesError::Unauthorized.into());
                }
            }
            Ok(ChatArtifact::new(path, Some(row.id)))
        }
    }
}

fn required_model(project: &Project, kind: GenerationKind) -> Result<&str, CommandError> {
    let (slot, code, message) = match kind {
        GenerationKind::Image => (
            ModelSlot::Image,
            "image_model_unset",
            "Pick an Images (create) model for this project under Models first.",
        ),
        GenerationKind::Audio => (
            ModelSlot::Speak,
            "speak_model_unset",
            "Pick a Read aloud model for this project under Models first.",
        ),
        GenerationKind::Video => (
            ModelSlot::Video,
            "video_model_unset",
            "Pick a Video model for this project under Models first.",
        ),
        GenerationKind::Document => {
            return Err(CommandError::new(
                "unsupported_type",
                "Documents are written after the chat reply.",
            ))
        }
    };
    let model = project.models.get(slot);
    if model.is_empty() {
        return Err(CommandError::new(code, message));
    }
    Ok(model)
}
