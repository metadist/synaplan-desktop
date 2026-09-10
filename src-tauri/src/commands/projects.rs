//! Project + chat commands (`PC3`). Wiring only: camelCase DTOs in and out,
//! everything else is `synaplan_core::projects`. Paths handed to the webview are
//! already platform-native strings so Vue never concatenates a path itself.

use serde::{Deserialize, Serialize};
use synaplan_core::config::DesktopConfig;
use synaplan_core::projects::chats::{ChatMessage, ChatRole, ChatSummary, ChatThread};
use synaplan_core::projects::{
    PersonalSeed, Project, ProjectError, ProjectIndex, ProjectKind, ProjectModels, ProjectPatch,
    ProjectStore,
};
use synaplan_core::skills;
use tauri::State;

use super::{AppState, CommandError};

impl From<ProjectError> for CommandError {
    fn from(e: ProjectError) -> Self {
        CommandError::new(e.code(), e.to_string())
    }
}

impl AppState {
    pub(crate) fn project_store(&self) -> ProjectStore {
        ProjectStore::new(&self.app_dirs)
    }

    /// Idempotent first-launch step: make sure the Personal project exists,
    /// seeded from the computer-level config. Safe to call on every start.
    pub(crate) fn ensure_projects(&self) -> Result<ProjectIndex, CommandError> {
        let cfg = DesktopConfig::load(&self.app_dirs.config_file()).unwrap_or_default();
        let seed = PersonalSeed {
            last_chat_model: cfg.last_chat_model,
            enabled_skills: skills::enabled_skill_names(&self.app_dirs.skills_dir),
        };
        Ok(self.project_store().ensure_personal(&seed)?)
    }
}

// ---- DTOs -------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectModelsDto {
    #[serde(default)]
    pub chat: String,
    #[serde(default)]
    pub voice: String,
    #[serde(default)]
    pub speak: String,
    #[serde(default)]
    pub vision: String,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub video: String,
    #[serde(default)]
    pub embed: String,
    #[serde(default)]
    pub docs: String,
    #[serde(default)]
    pub chat_legacy_provider_id: Option<String>,
}

impl From<ProjectModels> for ProjectModelsDto {
    fn from(m: ProjectModels) -> Self {
        Self {
            chat: m.chat,
            voice: m.voice,
            speak: m.speak,
            vision: m.vision,
            image: m.image,
            video: m.video,
            embed: m.embed,
            docs: m.docs,
            chat_legacy_provider_id: m.chat_legacy_provider_id,
        }
    }
}

impl From<ProjectModelsDto> for ProjectModels {
    fn from(m: ProjectModelsDto) -> Self {
        Self {
            chat: m.chat,
            voice: m.voice,
            speak: m.speak,
            vision: m.vision,
            image: m.image,
            video: m.video,
            embed: m.embed,
            docs: m.docs,
            chat_legacy_provider_id: m.chat_legacy_provider_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub kind: ProjectKind,
    pub created_at: String,
    pub updated_at: String,
    pub dictation_language: String,
    pub default_assistant_id: Option<i64>,
    pub assistant_ids: Vec<i64>,
    pub enabled_skills: Vec<String>,
    pub models: ProjectModelsDto,
    pub knowledge_folder: String,
    pub project_dir: String,
    pub notes_dir: String,
    pub out_dir: String,
}

fn project_dto(store: &ProjectStore, p: Project) -> ProjectDto {
    let project_dir = store.project_dir(&p).to_string_lossy().to_string();
    let notes_dir = store.notes_dir(&p).to_string_lossy().to_string();
    let out_dir = store.out_dir(&p).to_string_lossy().to_string();
    ProjectDto {
        id: p.id,
        slug: p.slug,
        name: p.name,
        kind: p.kind,
        created_at: p.created_at,
        updated_at: p.updated_at,
        dictation_language: p.dictation_language,
        default_assistant_id: p.default_assistant_id,
        assistant_ids: p.assistant_ids,
        enabled_skills: p.enabled_skills,
        models: p.models.into(),
        knowledge_folder: p.knowledge_folder,
        project_dir,
        notes_dir,
        out_dir,
    }
}

/// Everything the project switcher needs in one round-trip.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectsStateDto {
    pub projects: Vec<ProjectDto>,
    pub active_id: String,
    pub personal_id: String,
}

fn projects_state(state: &AppState) -> Result<ProjectsStateDto, CommandError> {
    let index = state.ensure_projects()?;
    let store = state.project_store();
    let projects = store
        .list_projects()?
        .into_iter()
        .map(|p| project_dto(&store, p))
        .collect::<Vec<_>>();
    let active_id = if projects.iter().any(|p| p.id == index.active_id) {
        index.active_id
    } else {
        projects.first().map(|p| p.id.clone()).unwrap_or_default()
    };
    Ok(ProjectsStateDto {
        projects,
        active_id,
        personal_id: index.personal_id,
    })
}

/// Partial update; absent keys leave the field alone, `defaultAssistantId: null`
/// clears the default.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPatchDto {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub dictation_language: Option<String>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub default_assistant_id: Option<Option<i64>>,
    #[serde(default)]
    pub assistant_ids: Option<Vec<i64>>,
    #[serde(default)]
    pub enabled_skills: Option<Vec<String>>,
    #[serde(default)]
    pub models: Option<ProjectModelsDto>,
}

fn deserialize_double_option<'de, D>(de: D) -> Result<Option<Option<i64>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<i64>::deserialize(de).map(Some)
}

impl From<ProjectPatchDto> for ProjectPatch {
    fn from(p: ProjectPatchDto) -> Self {
        ProjectPatch {
            name: p.name,
            dictation_language: p.dictation_language,
            default_assistant_id: p.default_assistant_id,
            assistant_ids: p.assistant_ids,
            enabled_skills: p.enabled_skills,
            models: p.models.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessageDto {
    pub role: ChatRole,
    pub content: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub created_at: String,
}

impl From<ChatMessage> for ChatMessageDto {
    fn from(m: ChatMessage) -> Self {
        Self {
            role: m.role,
            content: m.content,
            model: m.model,
            created_at: m.created_at,
        }
    }
}

impl From<ChatMessageDto> for ChatMessage {
    fn from(m: ChatMessageDto) -> Self {
        Self {
            role: m.role,
            content: m.content,
            model: m.model,
            created_at: m.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChatThreadDto {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub assistant_id: Option<i64>,
    #[serde(default)]
    pub messages: Vec<ChatMessageDto>,
}

impl From<ChatThread> for ChatThreadDto {
    fn from(t: ChatThread) -> Self {
        Self {
            id: t.id,
            project_id: t.project_id,
            title: t.title,
            created_at: t.created_at,
            updated_at: t.updated_at,
            assistant_id: t.assistant_id,
            messages: t.messages.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<ChatThreadDto> for ChatThread {
    fn from(t: ChatThreadDto) -> Self {
        Self {
            id: t.id,
            project_id: t.project_id,
            title: t.title,
            created_at: t.created_at,
            updated_at: t.updated_at,
            assistant_id: t.assistant_id,
            messages: t.messages.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChatSummaryDto {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: usize,
    pub assistant_id: Option<i64>,
}

impl From<ChatSummary> for ChatSummaryDto {
    fn from(s: ChatSummary) -> Self {
        Self {
            id: s.id,
            project_id: s.project_id,
            title: s.title,
            created_at: s.created_at,
            updated_at: s.updated_at,
            message_count: s.message_count,
            assistant_id: s.assistant_id,
        }
    }
}

// ---- project commands -------------------------------------------------------

#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> Result<ProjectsStateDto, CommandError> {
    projects_state(&state)
}

#[tauri::command]
pub fn get_project(state: State<'_, AppState>, id: String) -> Result<ProjectDto, CommandError> {
    let store = state.project_store();
    Ok(project_dto(&store, store.get_project(&id)?))
}

#[tauri::command]
pub fn get_active_project(state: State<'_, AppState>) -> Result<ProjectDto, CommandError> {
    state.ensure_projects()?;
    let store = state.project_store();
    Ok(project_dto(&store, store.active_project()?))
}

#[tauri::command]
pub fn create_project(
    state: State<'_, AppState>,
    name: String,
    dictation_language: String,
    copy_models_from: Option<String>,
) -> Result<ProjectDto, CommandError> {
    state.ensure_projects()?;
    let store = state.project_store();
    let project = store.create_project(
        &name,
        &dictation_language,
        copy_models_from.as_deref().filter(|s| !s.is_empty()),
    )?;
    Ok(project_dto(&store, project))
}

#[tauri::command]
pub fn update_project(
    state: State<'_, AppState>,
    id: String,
    patch: ProjectPatchDto,
) -> Result<ProjectDto, CommandError> {
    let store = state.project_store();
    Ok(project_dto(
        &store,
        store.update_project(&id, patch.into())?,
    ))
}

#[tauri::command]
pub fn delete_project(
    state: State<'_, AppState>,
    id: String,
    remove_files: bool,
) -> Result<ProjectsStateDto, CommandError> {
    state.project_store().delete_project(&id, remove_files)?;
    projects_state(&state)
}

#[tauri::command]
pub fn set_active_project(
    state: State<'_, AppState>,
    id: String,
) -> Result<ProjectsStateDto, CommandError> {
    state.project_store().set_active(&id)?;
    projects_state(&state)
}

// ---- chat commands ----------------------------------------------------------

#[tauri::command]
pub fn list_chats(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<ChatSummaryDto>, CommandError> {
    Ok(state
        .project_store()
        .list_chats(&project_id)?
        .into_iter()
        .map(Into::into)
        .collect())
}

/// A fresh, not-yet-saved thread for `project_id` (ids are minted in Rust).
#[tauri::command]
pub fn new_chat(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<ChatThreadDto, CommandError> {
    state.project_store().get_project(&project_id)?;
    Ok(ChatThread::new(&project_id).into())
}

#[tauri::command]
pub fn load_chat(
    state: State<'_, AppState>,
    project_id: String,
    chat_id: String,
) -> Result<ChatThreadDto, CommandError> {
    Ok(state
        .project_store()
        .load_chat(&project_id, &chat_id)?
        .into())
}

#[tauri::command]
pub fn save_chat(state: State<'_, AppState>, thread: ChatThreadDto) -> Result<(), CommandError> {
    let mut thread: ChatThread = thread.into();
    thread.updated_at = synaplan_core::projects::now_iso8601();
    if thread.title.is_empty() {
        if let Some(first_user) = thread.messages.iter().find(|m| m.role == ChatRole::User) {
            thread.title = synaplan_core::projects::chats::auto_title(&first_user.content);
        }
    }
    Ok(state.project_store().save_chat(&thread)?)
}

#[tauri::command]
pub fn delete_chat(
    state: State<'_, AppState>,
    project_id: String,
    chat_id: String,
) -> Result<(), CommandError> {
    Ok(state.project_store().delete_chat(&project_id, &chat_id)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_dto_null_clears_and_absent_leaves() {
        let absent: ProjectPatchDto = serde_json::from_str(r#"{"name":"x"}"#).unwrap();
        let patch: ProjectPatch = absent.into();
        assert_eq!(patch.default_assistant_id, None);
        let cleared: ProjectPatchDto =
            serde_json::from_str(r#"{"defaultAssistantId":null}"#).unwrap();
        let patch: ProjectPatch = cleared.into();
        assert_eq!(patch.default_assistant_id, Some(None));
    }

    #[test]
    fn models_dto_is_camel_case_on_the_wire() {
        let dto = ProjectModelsDto {
            chat: "ollama:llama3.2:chat".into(),
            chat_legacy_provider_id: Some("x".into()),
            ..Default::default()
        };
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"chatLegacyProviderId\""));
        assert!(!json.contains("chat_legacy"));
        let core: ProjectModels = dto.clone().into();
        assert_eq!(ProjectModelsDto::from(core), dto);
    }

    #[test]
    fn thread_dto_round_trips() {
        let mut t = ChatThread::new("01ARZ3NDEKTSV4RRFFQ69G5FAV");
        t.push(ChatRole::User, "hello", "");
        let dto: ChatThreadDto = t.clone().into();
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"projectId\""));
        assert!(json.contains("\"createdAt\""));
        let back: ChatThread = serde_json::from_str::<ChatThreadDto>(&json).unwrap().into();
        assert_eq!(back, t);
    }
}
