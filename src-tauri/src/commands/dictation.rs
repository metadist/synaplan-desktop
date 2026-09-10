//! Dictation commands (`PC12`). The webview records; these commands carry the
//! bytes to the workspace with the project's Dictation model, language and the
//! UI's prompt. The API key never crosses the seam — the webview only ever sees
//! an opaque session id and text.

use serde::Serialize;
use synaplan_core::dictation::{self, DictationContext, DictationError};
use tauri::State;

use super::{AppState, CommandError};

impl From<DictationError> for CommandError {
    fn from(e: DictationError) -> Self {
        CommandError::new(e.code(), e.to_string())
    }
}

impl AppState {
    fn dictation_context(
        &self,
        project_id: &str,
        prompt: &str,
    ) -> Result<DictationContext, CommandError> {
        let project = self.project_store().get_project(project_id)?;
        Ok(dictation::dictation_context(&project, prompt)?)
    }

    async fn dictation_result<T>(
        &self,
        base: &str,
        key: &str,
        result: Result<T, DictationError>,
    ) -> Result<T, CommandError> {
        match result {
            Err(DictationError::Unauthorized) => {
                self.on_files_unauthorized(base, key).await;
                Err(DictationError::Unauthorized.into())
            }
            other => Ok(other?),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictationSessionDto {
    pub session_id: String,
}

/// Open a live dictation session for a project. Refuses with
/// `voice_model_unset` before the mic is even needed.
#[tauri::command]
pub async fn dictation_start(
    state: State<'_, AppState>,
    project_id: String,
    prompt: String,
) -> Result<DictationSessionDto, CommandError> {
    let (base, key) = state.paired()?;
    let ctx = state.dictation_context(&project_id, &prompt)?;
    let client_id = format!("desktop-{project_id}");
    let session_id = state
        .dictation_result(
            &base,
            &key,
            dictation::open_session(&base, &key, &ctx, &client_id).await,
        )
        .await?;
    Ok(DictationSessionDto { session_id })
}

/// Append a chunk of 16 kHz mono PCM; `commit` closes the phrase.
#[tauri::command]
pub async fn dictation_chunk(
    state: State<'_, AppState>,
    session_id: String,
    pcm: Vec<u8>,
    commit: bool,
) -> Result<(), CommandError> {
    let (base, key) = state.paired()?;
    state
        .dictation_result(
            &base,
            &key,
            dictation::append_chunk(&base, &key, &session_id, pcm, commit).await,
        )
        .await
}

/// The live text recognised so far.
#[tauri::command]
pub async fn dictation_poll(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<String, CommandError> {
    let (base, key) = state.paired()?;
    state
        .dictation_result(
            &base,
            &key,
            dictation::session_text(&base, &key, &session_id).await,
        )
        .await
}

/// Commit what is pending and return the whole live text of the session.
#[tauri::command]
pub async fn dictation_commit(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<String, CommandError> {
    let (base, key) = state.paired()?;
    state
        .dictation_result(
            &base,
            &key,
            dictation::commit_session(&base, &key, &session_id).await,
        )
        .await
}

/// Close the session (best effort).
#[tauri::command]
pub async fn dictation_close(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), CommandError> {
    let (base, key) = state.paired()?;
    dictation::close_session(&base, &key, &session_id).await;
    Ok(())
}

/// One-shot transcription of a whole take with the project's model, language
/// and the given prompt.
#[tauri::command]
pub async fn dictation_transcribe(
    state: State<'_, AppState>,
    project_id: String,
    prompt: String,
    audio: Vec<u8>,
    mime: String,
) -> Result<String, CommandError> {
    let (base, key) = state.paired()?;
    let ctx = state.dictation_context(&project_id, &prompt)?;
    state
        .dictation_result(
            &base,
            &key,
            dictation::transcribe(&base, &key, &ctx, audio, &mime).await,
        )
        .await
}
