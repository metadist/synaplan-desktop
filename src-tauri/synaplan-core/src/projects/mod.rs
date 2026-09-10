//! Projects (project companion epic, `PC1`): the local unit of work.
//!
//! A project is a local record (`{config_dir}/projects/{id}.toml`), a local
//! directory (`{projects_dir}/{slug}/{notes,out}`), and a pointer at a
//! Synaplan knowledge folder (`DESKTOP:{id}`). The server never stores a
//! project. Nothing in this module talks to the network, and nothing here
//! ever sees the API key.
//!
//! Sovereignty lives in [`ProjectModels`]: eight per-capability bindings that
//! every outbound call (chat, dictation, file index, …) must read from. The
//! TOML keys are the UI slot ids (`chat`, `voice`, …), never the server's
//! `DEFAULTMODEL` names, so the file is readable without the PHP enum.

pub mod chats;
pub mod ids;
pub mod notes;
pub mod out;
pub mod slug;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::platform::app_dirs::AppDirs;

pub use ids::{is_safe_id, new_id, now_iso8601};
pub use slug::{is_safe_slug, slugify, unique_slug, PERSONAL_SLUG};

/// Prefix of the knowledge-folder `group_key` on Synaplan. The suffix is the
/// stable project `id` (never the slug — a rename must not orphan vectors).
pub const KNOWLEDGE_FOLDER_PREFIX: &str = "DESKTOP:";
/// On-disk schema version of `index.json`.
pub const INDEX_VERSION: u32 = 1;
/// Display name stored for the built-in first project (translated at display
/// time via `kind = personal`).
pub const PERSONAL_NAME: &str = "Personal";
/// Default dictation language when the user has not picked one.
pub const DEFAULT_DICTATION_LANGUAGE: &str = "en";

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("could not read project data: {0}")]
    Read(String),
    #[error("could not write project data: {0}")]
    Write(String),
    #[error("project data is not valid: {0}")]
    Parse(String),
    #[error("no project with id {0}")]
    NotFound(String),
    #[error("project name must not be empty")]
    InvalidName,
    #[error("the last project cannot be deleted")]
    LastProject,
    #[error("invalid identifier")]
    InvalidId,
    #[error("project folder is not safe: {0}")]
    UnsafePath(String),
}

impl ProjectError {
    pub fn code(&self) -> &'static str {
        match self {
            ProjectError::Read(_) | ProjectError::Write(_) => "project_io",
            ProjectError::Parse(_) => "project_corrupt",
            ProjectError::NotFound(_) => "project_not_found",
            ProjectError::InvalidName => "project_invalid_name",
            ProjectError::LastProject => "project_last",
            ProjectError::InvalidId => "project_invalid_id",
            ProjectError::UnsafePath(_) => "project_unsafe_path",
        }
    }
}

/// The eight sovereignty slots. Code and docs may name the server capability
/// (`DEFAULTMODEL`); the UI never does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelSlot {
    Chat,
    Voice,
    Speak,
    Vision,
    Image,
    Video,
    Embed,
    Docs,
}

impl ModelSlot {
    pub const ALL: [ModelSlot; 8] = [
        ModelSlot::Chat,
        ModelSlot::Voice,
        ModelSlot::Speak,
        ModelSlot::Vision,
        ModelSlot::Image,
        ModelSlot::Video,
        ModelSlot::Embed,
        ModelSlot::Docs,
    ];

    /// The lowercase slot id used in the project file and the UI.
    pub fn id(self) -> &'static str {
        match self {
            ModelSlot::Chat => "chat",
            ModelSlot::Voice => "voice",
            ModelSlot::Speak => "speak",
            ModelSlot::Vision => "vision",
            ModelSlot::Image => "image",
            ModelSlot::Video => "video",
            ModelSlot::Embed => "embed",
            ModelSlot::Docs => "docs",
        }
    }

    /// The Synaplan `DEFAULTMODEL` capability this slot maps to (server-facing
    /// only — the model catalog groups by these names).
    pub fn capability(self) -> &'static str {
        match self {
            ModelSlot::Chat => "CHAT",
            ModelSlot::Voice => "SOUND2TEXT",
            ModelSlot::Speak => "TEXT2SOUND",
            ModelSlot::Vision => "PIC2TEXT",
            ModelSlot::Image => "TEXT2PIC",
            ModelSlot::Video => "TEXT2VID",
            ModelSlot::Embed => "VECTORIZE",
            ModelSlot::Docs => "ANALYZE",
        }
    }

    pub fn parse(id: &str) -> Option<ModelSlot> {
        ModelSlot::ALL.into_iter().find(|s| s.id() == id)
    }

    pub fn from_capability(capability: &str) -> Option<ModelSlot> {
        ModelSlot::ALL
            .into_iter()
            .find(|s| s.capability() == capability)
    }
}

/// This project's models. Each value is a catalog key
/// (`service:providerId:tag`) or, for a pick made before the catalog existed,
/// a bare provider id mirrored in `chat_legacy_provider_id`. Empty = unset.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectModels {
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
    /// Set when `chat` holds a bare provider id from the pre-catalog picker so
    /// the Models panel can rebind it once the catalog arrives.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_legacy_provider_id: Option<String>,
}

impl ProjectModels {
    pub fn get(&self, slot: ModelSlot) -> &str {
        match slot {
            ModelSlot::Chat => &self.chat,
            ModelSlot::Voice => &self.voice,
            ModelSlot::Speak => &self.speak,
            ModelSlot::Vision => &self.vision,
            ModelSlot::Image => &self.image,
            ModelSlot::Video => &self.video,
            ModelSlot::Embed => &self.embed,
            ModelSlot::Docs => &self.docs,
        }
    }

    pub fn set(&mut self, slot: ModelSlot, value: &str) {
        let value = value.trim().to_string();
        let target = match slot {
            ModelSlot::Chat => {
                // Any explicit chat pick supersedes the migrated legacy id.
                self.chat_legacy_provider_id = None;
                &mut self.chat
            }
            ModelSlot::Voice => &mut self.voice,
            ModelSlot::Speak => &mut self.speak,
            ModelSlot::Vision => &mut self.vision,
            ModelSlot::Image => &mut self.image,
            ModelSlot::Video => &mut self.video,
            ModelSlot::Embed => &mut self.embed,
            ModelSlot::Docs => &mut self.docs,
        };
        *target = value;
    }

    pub fn is_set(&self, slot: ModelSlot) -> bool {
        !self.get(slot).is_empty()
    }

    /// Seed the CHAT slot from a pre-catalog pick (`last_chat_model`). A value
    /// that already looks like a catalog key is stored as is; a bare provider
    /// id is stored and mirrored so it can be rebound later.
    pub fn seed_chat_from_legacy(&mut self, legacy: Option<&str>) {
        let Some(raw) = legacy.map(str::trim).filter(|s| !s.is_empty()) else {
            return;
        };
        self.chat = raw.to_string();
        self.chat_legacy_provider_id = if is_catalog_key(raw) {
            None
        } else {
            Some(raw.to_string())
        };
    }
}

/// True if `value` has the `service:providerId:tag` shape (at least two colons
/// and a non-empty first and last segment).
pub fn is_catalog_key(value: &str) -> bool {
    let parts: Vec<&str> = value.split(':').collect();
    parts.len() >= 3 && !parts[0].is_empty() && !parts[parts.len() - 1].is_empty()
}

/// The model id an endpoint that still wants a bare provider id should receive
/// for `binding`. A catalog key `service:providerId:tag` yields the middle
/// (`providerId` may itself contain colons, e.g. `llama3.2:latest`); a legacy
/// or unknown value is sent verbatim. `None` when the slot is unset.
pub fn wire_model_id(binding: &str, legacy_provider_id: Option<&str>) -> Option<String> {
    let binding = binding.trim();
    if binding.is_empty() {
        return None;
    }
    if legacy_provider_id.is_some_and(|l| l == binding) {
        return Some(binding.to_string());
    }
    if is_catalog_key(binding) {
        let first = binding.find(':').unwrap_or(0);
        let last = binding.rfind(':').unwrap_or(binding.len());
        if last > first + 1 {
            return Some(binding[first + 1..last].to_string());
        }
    }
    Some(binding.to_string())
}

/// Whether a project is the built-in first project or a user-created one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectKind {
    Personal,
    #[default]
    Project,
}

/// One project record (`{config_dir}/projects/{id}.toml`). No API key, no
/// pairing code, ever.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub kind: ProjectKind,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default = "default_language")]
    pub dictation_language: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_assistant_id: Option<i64>,
    #[serde(default)]
    pub assistant_ids: Vec<i64>,
    #[serde(default)]
    pub enabled_skills: Vec<String>,
    #[serde(default)]
    pub models: ProjectModels,
    /// Always `DESKTOP:{id}`; persisted for readability, derived on load.
    #[serde(default)]
    pub knowledge_folder: String,
}

fn default_language() -> String {
    DEFAULT_DICTATION_LANGUAGE.to_string()
}

impl Project {
    /// The Synaplan `group_key` of this project's knowledge folder.
    pub fn knowledge_folder_for(id: &str) -> String {
        format!("{KNOWLEDGE_FOLDER_PREFIX}{id}")
    }

    pub fn is_personal(&self) -> bool {
        self.kind == ProjectKind::Personal
    }
}

/// Fields a caller may change on an existing project. `None` = leave as is.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPatch {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub dictation_language: Option<String>,
    /// `Some(None)` clears the default assistant.
    #[serde(default, with = "double_option")]
    pub default_assistant_id: Option<Option<i64>>,
    #[serde(default)]
    pub assistant_ids: Option<Vec<i64>>,
    #[serde(default)]
    pub enabled_skills: Option<Vec<String>>,
    #[serde(default)]
    pub models: Option<ProjectModels>,
}

/// Serde helper so `defaultAssistantId: null` means "clear" and an absent key
/// means "leave alone".
mod double_option {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S, T>(value: &Option<Option<T>>, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        match value {
            Some(inner) => inner.serialize(ser),
            None => ser.serialize_none(),
        }
    }

    pub fn deserialize<'de, D, T>(de: D) -> Result<Option<Option<T>>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        Option::<T>::deserialize(de).map(Some)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Migration {
    #[serde(default)]
    pub personal_v1: bool,
}

/// `{config_dir}/projects/index.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectIndex {
    pub version: u32,
    pub personal_id: String,
    #[serde(default)]
    pub migration: Migration,
    #[serde(default)]
    pub order: Vec<String>,
    pub active_id: String,
}

/// What the first-launch migration inherits from the computer-level config.
#[derive(Debug, Clone, Default)]
pub struct PersonalSeed {
    pub last_chat_model: Option<String>,
    pub enabled_skills: Vec<String>,
}

/// All project persistence, rooted at the two directories from [`AppDirs`].
#[derive(Debug, Clone)]
pub struct ProjectStore {
    meta_dir: PathBuf,
    projects_dir: PathBuf,
}

impl ProjectStore {
    pub fn new(dirs: &AppDirs) -> Self {
        Self {
            meta_dir: dirs.projects_meta_dir(),
            projects_dir: dirs.projects_dir.clone(),
        }
    }

    /// Construct from explicit roots (tests and callers without an `AppDirs`).
    pub fn with_roots(meta_dir: PathBuf, projects_dir: PathBuf) -> Self {
        Self {
            meta_dir,
            projects_dir,
        }
    }

    pub fn meta_dir(&self) -> &Path {
        &self.meta_dir
    }

    pub fn projects_dir(&self) -> &Path {
        &self.projects_dir
    }

    pub fn index_path(&self) -> PathBuf {
        self.meta_dir.join("index.json")
    }

    pub fn project_file(&self, id: &str) -> PathBuf {
        self.meta_dir.join(format!("{id}.toml"))
    }

    /// App-only per-project metadata directory (`{meta}/{id}/`, holds chats).
    pub fn project_meta_dir(&self, id: &str) -> PathBuf {
        self.meta_dir.join(id)
    }

    /// The user-visible project folder (`{projects_dir}/{slug}`).
    pub fn project_dir(&self, project: &Project) -> PathBuf {
        self.projects_dir.join(&project.slug)
    }

    /// `{projects_dir}/{slug}` after proving the slug is a single safe
    /// component. Does not create the directory and does not follow it.
    pub fn contained_project_dir(&self, project: &Project) -> Result<PathBuf, ProjectError> {
        if !is_safe_slug(&project.slug) {
            return Err(ProjectError::UnsafePath(format!(
                "slug {:?} is not a safe folder name",
                project.slug
            )));
        }
        let dir = self.projects_dir.join(&project.slug);
        if dir.file_name().and_then(|n| n.to_str()) != Some(project.slug.as_str()) {
            return Err(ProjectError::UnsafePath(
                "slug does not resolve to a single folder".into(),
            ));
        }
        if !dir.starts_with(&self.projects_dir) || dir == self.projects_dir {
            return Err(ProjectError::UnsafePath(
                "project folder is outside the projects home".into(),
            ));
        }
        Ok(dir)
    }

    pub fn notes_dir(&self, project: &Project) -> PathBuf {
        self.project_dir(project).join("notes")
    }

    pub fn out_dir(&self, project: &Project) -> PathBuf {
        self.project_dir(project).join("out")
    }

    // ---- index -------------------------------------------------------------

    /// Read the index. `Ok(None)` when it does not exist yet.
    pub fn load_index(&self) -> Result<Option<ProjectIndex>, ProjectError> {
        match std::fs::read_to_string(self.index_path()) {
            Ok(raw) => serde_json::from_str(&raw)
                .map(Some)
                .map_err(|e| ProjectError::Parse(format!("index.json: {e}"))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(ProjectError::Read(e.to_string())),
        }
    }

    pub fn save_index(&self, index: &ProjectIndex) -> Result<(), ProjectError> {
        let json =
            serde_json::to_string_pretty(index).map_err(|e| ProjectError::Write(e.to_string()))?;
        write_atomic(&self.index_path(), json.as_bytes())
    }

    /// Rebuild the index from the project files on disk. Used to recover from
    /// a corrupt `index.json` without touching `projects_dir`.
    pub fn rebuild_index(&self) -> Result<ProjectIndex, ProjectError> {
        let mut projects = self.scan_project_files()?;
        projects.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        let personal_id = projects
            .iter()
            .find(|p| p.is_personal())
            .or(projects.first())
            .map(|p| p.id.clone())
            .unwrap_or_default();
        let index = ProjectIndex {
            version: INDEX_VERSION,
            personal_id: personal_id.clone(),
            migration: Migration {
                personal_v1: !personal_id.is_empty(),
            },
            order: projects.iter().map(|p| p.id.clone()).collect(),
            active_id: personal_id,
        };
        if !index.order.is_empty() {
            self.save_index(&index)?;
        }
        Ok(index)
    }

    // ---- first launch ----------------------------------------------------

    /// Run the one-time Personal migration if it has not happened yet, and
    /// return the (possibly freshly created) index. Idempotent.
    pub fn ensure_personal(&self, seed: &PersonalSeed) -> Result<ProjectIndex, ProjectError> {
        match self.load_index() {
            Ok(Some(index)) if index.migration.personal_v1 => return Ok(index),
            Ok(Some(_)) | Ok(None) => {}
            Err(ProjectError::Parse(_)) => {
                // A corrupt index must not brick the companion: rebuild from
                // the project files and leave {projects_dir} untouched.
                if !self.scan_project_files()?.is_empty() {
                    return self.rebuild_index();
                }
            }
            Err(e) => return Err(e),
        }
        let existing = self.scan_project_files()?;
        if let Some(personal) = existing.iter().find(|p| p.is_personal()) {
            // Files exist but the index was lost: rebuild instead of a second Personal.
            let _ = personal;
            return self.rebuild_index();
        }

        let now = now_iso8601();
        let id = new_id();
        let mut models = ProjectModels::default();
        models.seed_chat_from_legacy(seed.last_chat_model.as_deref());
        let project = Project {
            id: id.clone(),
            slug: PERSONAL_SLUG.to_string(),
            name: PERSONAL_NAME.to_string(),
            kind: ProjectKind::Personal,
            created_at: now.clone(),
            updated_at: now,
            dictation_language: DEFAULT_DICTATION_LANGUAGE.to_string(),
            default_assistant_id: None,
            assistant_ids: Vec::new(),
            enabled_skills: seed.enabled_skills.clone(),
            models,
            knowledge_folder: Project::knowledge_folder_for(&id),
        };
        self.write_project(&project)?;
        self.create_dirs(&project)?;
        let index = ProjectIndex {
            version: INDEX_VERSION,
            personal_id: id.clone(),
            migration: Migration { personal_v1: true },
            order: vec![id.clone()],
            active_id: id,
        };
        self.save_index(&index)?;
        Ok(index)
    }

    // ---- CRUD ------------------------------------------------------------

    /// Projects in index order; files not in the index are appended.
    pub fn list_projects(&self) -> Result<Vec<Project>, ProjectError> {
        let index = self.load_index()?;
        let mut files = self.scan_project_files()?;
        let mut out = Vec::with_capacity(files.len());
        if let Some(index) = index {
            for id in &index.order {
                if let Some(pos) = files.iter().position(|p| &p.id == id) {
                    out.push(files.remove(pos));
                }
            }
        }
        files.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        out.extend(files);
        Ok(out)
    }

    pub fn get_project(&self, id: &str) -> Result<Project, ProjectError> {
        if !is_safe_id(id) {
            return Err(ProjectError::InvalidId);
        }
        match std::fs::read_to_string(self.project_file(id)) {
            Ok(raw) => parse_project(&raw),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(ProjectError::NotFound(id.to_string()))
            }
            Err(e) => Err(ProjectError::Read(e.to_string())),
        }
    }

    /// Create a project. Models are copied from `copy_models_from` when given
    /// (otherwise every slot starts unset). No server round-trip.
    pub fn create_project(
        &self,
        name: &str,
        dictation_language: &str,
        copy_models_from: Option<&str>,
    ) -> Result<Project, ProjectError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(ProjectError::InvalidName);
        }
        let taken = self.taken_slugs()?;
        let slug = unique_slug(&slugify(name), taken.iter().map(String::as_str));
        let models = match copy_models_from {
            Some(src) => self.get_project(src)?.models,
            None => ProjectModels::default(),
        };
        let now = now_iso8601();
        let id = new_id();
        let project = Project {
            id: id.clone(),
            slug,
            name: name.to_string(),
            kind: ProjectKind::Project,
            created_at: now.clone(),
            updated_at: now,
            dictation_language: normalize_language(dictation_language),
            default_assistant_id: None,
            assistant_ids: Vec::new(),
            enabled_skills: Vec::new(),
            models,
            knowledge_folder: Project::knowledge_folder_for(&id),
        };
        self.write_project(&project)?;
        if let Err(e) = self.create_dirs(&project) {
            self.rollback_create(&project);
            return Err(e);
        }
        let mut index = match self.index_or_rebuild() {
            Ok(index) => index,
            Err(e) => {
                self.rollback_create(&project);
                return Err(e);
            }
        };
        index.order.push(id);
        if let Err(e) = self.save_index(&index) {
            self.rollback_create(&project);
            return Err(e);
        }
        Ok(project)
    }

    /// Apply a patch. Rename changes `name` only — never `id`, never `slug`.
    pub fn update_project(&self, id: &str, patch: ProjectPatch) -> Result<Project, ProjectError> {
        let mut project = self.get_project(id)?;
        if let Some(name) = patch.name {
            let name = name.trim();
            if name.is_empty() {
                return Err(ProjectError::InvalidName);
            }
            project.name = name.to_string();
        }
        if let Some(lang) = patch.dictation_language {
            project.dictation_language = normalize_language(&lang);
        }
        if let Some(default_assistant) = patch.default_assistant_id {
            project.default_assistant_id = default_assistant;
        }
        if let Some(ids) = patch.assistant_ids {
            project.assistant_ids = ids;
        }
        if let Some(skills) = patch.enabled_skills {
            project.enabled_skills = dedupe(skills);
        }
        if let Some(models) = patch.models {
            project.models = models;
        }
        if let Some(default_id) = project.default_assistant_id {
            if !project.assistant_ids.contains(&default_id) {
                project.assistant_ids.push(default_id);
            }
        }
        project.updated_at = now_iso8601();
        self.write_project(&project)?;
        Ok(project)
    }

    /// Delete metadata and chats. `remove_files` also deletes the notes folder
    /// (`{projects_dir}/{slug}/notes`); `out/` and anything else the user put
    /// in the project folder stay on disk. Files already sent to Synaplan are
    /// never touched here.
    pub fn delete_project(&self, id: &str, remove_files: bool) -> Result<(), ProjectError> {
        let project = self.get_project(id)?;
        let mut index = self.index_or_rebuild()?;
        if index.order.iter().filter(|o| *o != id).count() == 0 {
            return Err(ProjectError::LastProject);
        }
        // Notes first so a locked file cannot orphan the folder after the
        // metadata is already gone; the user can retry the same delete.
        if remove_files {
            let notes = self.notes_dir(&project);
            if is_child_of(&notes, &self.projects_dir) {
                remove_dir_if_exists(&notes)?;
            }
        }
        index.order.retain(|o| o != id);
        if index.active_id == id {
            index.active_id = index.order.first().cloned().unwrap_or_default();
        }
        if index.personal_id == id {
            index.personal_id = index.active_id.clone();
        }
        self.save_index(&index)?;

        remove_file_if_exists(&self.project_file(id))?;
        remove_dir_if_exists(&self.project_meta_dir(id))?;
        Ok(())
    }

    /// Drop every Assistant bind. Called on pair / sign-out so a new workspace
    /// cannot inherit numeric ids that belonged to a different account.
    pub fn clear_assistant_bindings(&self) -> Result<(), ProjectError> {
        for project in self.list_projects()? {
            if project.assistant_ids.is_empty() && project.default_assistant_id.is_none() {
                continue;
            }
            self.update_project(
                &project.id,
                ProjectPatch {
                    default_assistant_id: Some(None),
                    assistant_ids: Some(Vec::new()),
                    ..ProjectPatch::default()
                },
            )?;
        }
        Ok(())
    }

    /// Keep only Assistant ids that the current workspace still advertises.
    pub fn retain_known_assistants(&self, known: &[i64]) -> Result<(), ProjectError> {
        for project in self.list_projects()? {
            let filtered: Vec<i64> = project
                .assistant_ids
                .iter()
                .copied()
                .filter(|id| known.contains(id))
                .collect();
            let default = project.default_assistant_id.filter(|id| known.contains(id));
            if filtered == project.assistant_ids && default == project.default_assistant_id {
                continue;
            }
            self.update_project(
                &project.id,
                ProjectPatch {
                    default_assistant_id: Some(default),
                    assistant_ids: Some(filtered),
                    ..ProjectPatch::default()
                },
            )?;
        }
        Ok(())
    }

    pub fn set_active(&self, id: &str) -> Result<ProjectIndex, ProjectError> {
        self.get_project(id)?;
        let mut index = self.index_or_rebuild()?;
        index.active_id = id.to_string();
        if !index.order.iter().any(|o| o == id) {
            index.order.push(id.to_string());
        }
        self.save_index(&index)?;
        Ok(index)
    }

    /// The active project, falling back to the first listed one.
    pub fn active_project(&self) -> Result<Project, ProjectError> {
        let index = self.index_or_rebuild()?;
        match self.get_project(&index.active_id) {
            Ok(p) => Ok(p),
            Err(ProjectError::NotFound(_)) | Err(ProjectError::InvalidId) => self
                .list_projects()?
                .into_iter()
                .next()
                .ok_or_else(|| ProjectError::NotFound(index.active_id.clone())),
            Err(e) => Err(e),
        }
    }

    /// Make sure the user-visible folders for `project` exist as real
    /// directories inside `projects_dir`. A symlink at the slug, `notes`, or
    /// `out` is refused so confinement cannot follow it outside.
    pub fn create_dirs(&self, project: &Project) -> Result<(), ProjectError> {
        let root = self.contained_project_dir(project)?;
        self.ensure_real_dir(&root)?;
        self.ensure_real_dir(&root.join("notes"))?;
        self.ensure_real_dir(&root.join("out"))?;
        Ok(())
    }

    // ---- internals -------------------------------------------------------

    fn index_or_rebuild(&self) -> Result<ProjectIndex, ProjectError> {
        match self.load_index() {
            Ok(Some(index)) => Ok(index),
            Ok(None) | Err(ProjectError::Parse(_)) => self.rebuild_index(),
            Err(e) => Err(e),
        }
    }

    /// Slugs already claimed by a project record **or** by a leftover folder
    /// under `projects_dir` (notes kept after a metadata-only delete).
    fn taken_slugs(&self) -> Result<Vec<String>, ProjectError> {
        let mut taken: Vec<String> = self
            .list_projects()?
            .iter()
            .map(|p| p.slug.clone())
            .collect();
        let entries = match std::fs::read_dir(&self.projects_dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(taken),
            Err(e) => return Err(ProjectError::Read(e.to_string())),
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if taken.iter().any(|s| s == name) {
                continue;
            }
            let is_dir = entry
                .file_type()
                .map(|t| t.is_dir() || t.is_symlink())
                .unwrap_or(false);
            if is_dir {
                taken.push(name.to_string());
            }
        }
        Ok(taken)
    }

    fn rollback_create(&self, project: &Project) {
        let _ = remove_file_if_exists(&self.project_file(&project.id));
        let _ = remove_dir_if_exists(&self.project_meta_dir(&project.id));
    }

    fn ensure_real_dir(&self, path: &Path) -> Result<(), ProjectError> {
        if let Ok(meta) = std::fs::symlink_metadata(path) {
            if meta.file_type().is_symlink() {
                return Err(ProjectError::UnsafePath(format!(
                    "refusing symlink at {}",
                    path.display()
                )));
            }
            if !meta.is_dir() {
                return Err(ProjectError::Write(format!(
                    "expected a directory at {}",
                    path.display()
                )));
            }
        } else {
            std::fs::create_dir_all(path).map_err(|e| ProjectError::Write(e.to_string()))?;
            if let Ok(meta) = std::fs::symlink_metadata(path) {
                if meta.file_type().is_symlink() {
                    return Err(ProjectError::UnsafePath(format!(
                        "refusing symlink at {}",
                        path.display()
                    )));
                }
            }
        }
        self.assert_inside_projects(path)
    }

    fn assert_inside_projects(&self, path: &Path) -> Result<(), ProjectError> {
        let Ok(home) = self.projects_dir.canonicalize() else {
            return Ok(());
        };
        let canon = path
            .canonicalize()
            .map_err(|e| ProjectError::Read(e.to_string()))?;
        if !canon.starts_with(&home) {
            return Err(ProjectError::UnsafePath(
                "project folder escaped the projects home".into(),
            ));
        }
        Ok(())
    }

    fn write_project(&self, project: &Project) -> Result<(), ProjectError> {
        let toml =
            toml::to_string_pretty(project).map_err(|e| ProjectError::Write(e.to_string()))?;
        write_atomic(&self.project_file(&project.id), toml.as_bytes())
    }

    fn scan_project_files(&self) -> Result<Vec<Project>, ProjectError> {
        let mut out = Vec::new();
        let entries = match std::fs::read_dir(&self.meta_dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
            Err(e) => return Err(ProjectError::Read(e.to_string())),
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            let Ok(raw) = std::fs::read_to_string(&path) else {
                continue;
            };
            if let Ok(project) = parse_project(&raw) {
                out.push(project);
            }
        }
        Ok(out)
    }
}

fn parse_project(raw: &str) -> Result<Project, ProjectError> {
    let mut project: Project =
        toml::from_str(raw).map_err(|e| ProjectError::Parse(e.to_string()))?;
    if !is_safe_id(&project.id) {
        return Err(ProjectError::Parse("project id is not safe".into()));
    }
    if !is_safe_slug(&project.slug) {
        return Err(ProjectError::Parse("project slug is not safe".into()));
    }
    project.knowledge_folder = Project::knowledge_folder_for(&project.id);
    Ok(project)
}

/// Lowercase ISO 639-1 (optionally `xx-YY`), defaulting when empty.
pub fn normalize_language(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return DEFAULT_DICTATION_LANGUAGE.to_string();
    }
    let mut parts = trimmed.splitn(2, ['-', '_']);
    let lang = parts.next().unwrap_or("").to_ascii_lowercase();
    match parts.next() {
        Some(region) if !region.is_empty() => format!("{lang}-{}", region.to_ascii_uppercase()),
        _ => lang,
    }
}

fn dedupe(items: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for item in items {
        let item = item.trim().to_string();
        if !item.is_empty() && !out.contains(&item) {
            out.push(item);
        }
    }
    out
}

/// Write `bytes` to `path` via a sibling temp file + rename so a crash never
/// leaves a half-written record.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), ProjectError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ProjectError::Write(e.to_string()))?;
    }
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|e| e.to_str()).unwrap_or("tmp")
    ));
    std::fs::write(&tmp, bytes).map_err(|e| ProjectError::Write(e.to_string()))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        ProjectError::Write(e.to_string())
    })
}

fn remove_file_if_exists(path: &Path) -> Result<(), ProjectError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(ProjectError::Write(e.to_string())),
    }
}

fn remove_dir_if_exists(path: &Path) -> Result<(), ProjectError> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(ProjectError::Write(e.to_string())),
    }
}

/// True if `child` is strictly inside `parent` (lexically, then canonically
/// when both exist so a symlinked slug cannot escape the projects folder).
fn is_child_of(child: &Path, parent: &Path) -> bool {
    if !child.starts_with(parent) || child == parent {
        return false;
    }
    match (child.canonicalize(), parent.canonicalize()) {
        (Ok(c), Ok(p)) => c.starts_with(&p) && c != p,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, ProjectStore) {
        let dir = tempfile::tempdir().unwrap();
        let s = ProjectStore::with_roots(
            dir.path().join("config").join("projects"),
            dir.path().join("Synaplan").join("projects"),
        );
        (dir, s)
    }

    fn seed() -> PersonalSeed {
        PersonalSeed {
            last_chat_model: Some("gpt-4o-mini".into()),
            enabled_skills: vec!["vcard".into(), "slides".into()],
        }
    }

    #[test]
    fn ensure_personal_runs_once_and_inherits() {
        let (_tmp, s) = store();
        let index = s.ensure_personal(&seed()).unwrap();
        assert!(index.migration.personal_v1);
        assert_eq!(index.order, vec![index.personal_id.clone()]);
        assert_eq!(index.active_id, index.personal_id);

        let personal = s.get_project(&index.personal_id).unwrap();
        assert_eq!(personal.slug, PERSONAL_SLUG);
        assert_eq!(personal.name, PERSONAL_NAME);
        assert!(personal.is_personal());
        assert_eq!(personal.models.chat, "gpt-4o-mini");
        assert_eq!(
            personal.models.chat_legacy_provider_id.as_deref(),
            Some("gpt-4o-mini")
        );
        assert_eq!(personal.enabled_skills, vec!["vcard", "slides"]);
        assert_eq!(
            personal.knowledge_folder,
            format!("DESKTOP:{}", personal.id)
        );
        assert!(s.notes_dir(&personal).is_dir());
        assert!(s.out_dir(&personal).is_dir());

        // Second call is a no-op: same id, still one project.
        let again = s.ensure_personal(&seed()).unwrap();
        assert_eq!(again, index);
        assert_eq!(s.list_projects().unwrap().len(), 1);
    }

    #[test]
    fn personal_without_seed_leaves_chat_unset() {
        let (_tmp, s) = store();
        let index = s.ensure_personal(&PersonalSeed::default()).unwrap();
        let personal = s.get_project(&index.personal_id).unwrap();
        assert!(!personal.models.is_set(ModelSlot::Chat));
        assert!(personal.models.chat_legacy_provider_id.is_none());
    }

    #[test]
    fn catalog_key_seed_has_no_legacy_marker() {
        let mut m = ProjectModels::default();
        m.seed_chat_from_legacy(Some("ollama:llama3.2:chat"));
        assert_eq!(m.chat, "ollama:llama3.2:chat");
        assert!(m.chat_legacy_provider_id.is_none());
    }

    #[test]
    fn create_switch_rename_delete() {
        let (_tmp, s) = store();
        let index = s.ensure_personal(&seed()).unwrap();

        let work = s.create_project("  Küchen Umbau ", "de", None).unwrap();
        assert_eq!(work.name, "Küchen Umbau");
        assert_eq!(work.slug, "kuchen-umbau");
        assert_eq!(work.dictation_language, "de");
        assert!(!work.models.is_set(ModelSlot::Chat));
        assert!(s.notes_dir(&work).is_dir());

        // Slug collision → -2; models copied from Personal.
        let work2 = s
            .create_project("Küchen-Umbau!", "de-de", Some(&index.personal_id))
            .unwrap();
        assert_eq!(work2.slug, "kuchen-umbau-2");
        assert_eq!(work2.dictation_language, "de-DE");
        assert_eq!(work2.models.chat, "gpt-4o-mini");

        let listed = s.list_projects().unwrap();
        assert_eq!(listed.len(), 3);
        assert_eq!(listed[0].id, index.personal_id);
        assert_eq!(listed[1].id, work.id);

        s.set_active(&work.id).unwrap();
        assert_eq!(s.active_project().unwrap().id, work.id);

        // Rename keeps id and slug.
        let renamed = s
            .update_project(
                &work.id,
                ProjectPatch {
                    name: Some("Kitchen".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(renamed.id, work.id);
        assert_eq!(renamed.slug, "kuchen-umbau");
        assert_eq!(renamed.name, "Kitchen");

        // Delete the active one keeping files: falls back to the first project.
        s.delete_project(&work.id, false).unwrap();
        assert!(matches!(
            s.get_project(&work.id),
            Err(ProjectError::NotFound(_))
        ));
        assert!(s.notes_dir(&work).is_dir(), "notes stay on disk");
        assert_eq!(s.active_project().unwrap().id, index.personal_id);

        // Delete with files removes the notes folder only — out/ stays.
        std::fs::write(s.out_dir(&work2).join("keep.txt"), "x").unwrap();
        s.delete_project(&work2.id, true).unwrap();
        assert!(!s.notes_dir(&work2).exists());
        assert!(s.out_dir(&work2).join("keep.txt").is_file());
    }

    #[test]
    fn cannot_delete_last_project() {
        let (_tmp, s) = store();
        let index = s.ensure_personal(&seed()).unwrap();
        assert!(matches!(
            s.delete_project(&index.personal_id, false),
            Err(ProjectError::LastProject)
        ));
    }

    #[test]
    fn personal_can_go_when_another_exists() {
        let (_tmp, s) = store();
        let index = s.ensure_personal(&seed()).unwrap();
        let other = s.create_project("Other", "en", None).unwrap();
        s.delete_project(&index.personal_id, false).unwrap();
        let idx = s.load_index().unwrap().unwrap();
        assert_eq!(idx.order, vec![other.id.clone()]);
        assert_eq!(idx.active_id, other.id);
        // Migration marker stays so Personal is not recreated.
        assert_eq!(s.ensure_personal(&seed()).unwrap().order.len(), 1);
    }

    #[test]
    fn empty_name_is_rejected() {
        let (_tmp, s) = store();
        s.ensure_personal(&seed()).unwrap();
        assert!(matches!(
            s.create_project("   ", "en", None),
            Err(ProjectError::InvalidName)
        ));
    }

    #[test]
    fn models_patch_persists_eight_slots_with_ui_keys() {
        let (_tmp, s) = store();
        let index = s.ensure_personal(&seed()).unwrap();
        let mut models = ProjectModels::default();
        models.set(ModelSlot::Chat, "ollama:llama3.2:chat");
        models.set(ModelSlot::Embed, "ollama:bge-m3:vectorize");
        models.set(ModelSlot::Voice, "openai:whisper-1:sound2text");
        let updated = s
            .update_project(
                &index.personal_id,
                ProjectPatch {
                    models: Some(models),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(updated.models.chat_legacy_provider_id.is_none());

        let raw = std::fs::read_to_string(s.project_file(&index.personal_id)).unwrap();
        for slot in ModelSlot::ALL {
            assert!(raw.contains(&format!("{} = ", slot.id())), "{raw}");
            assert!(
                !raw.contains(slot.capability()),
                "no DEFAULTMODEL names on disk"
            );
        }
        assert!(!raw.contains("sk_"));
        assert!(raw.contains("knowledge_folder = \"DESKTOP:"));
    }

    #[test]
    fn default_assistant_is_added_to_binds() {
        let (_tmp, s) = store();
        let index = s.ensure_personal(&seed()).unwrap();
        let p = s
            .update_project(
                &index.personal_id,
                ProjectPatch {
                    default_assistant_id: Some(Some(42)),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(p.default_assistant_id, Some(42));
        assert_eq!(p.assistant_ids, vec![42]);
        let p = s
            .update_project(
                &index.personal_id,
                ProjectPatch {
                    default_assistant_id: Some(None),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(p.default_assistant_id, None);
        assert_eq!(
            p.assistant_ids,
            vec![42],
            "unbind is separate from unset default"
        );
    }

    #[test]
    fn patch_json_distinguishes_null_from_absent() {
        let absent: ProjectPatch = serde_json::from_str(r#"{"name":"x"}"#).unwrap();
        assert_eq!(absent.default_assistant_id, None);
        let cleared: ProjectPatch = serde_json::from_str(r#"{"defaultAssistantId":null}"#).unwrap();
        assert_eq!(cleared.default_assistant_id, Some(None));
        let set: ProjectPatch = serde_json::from_str(r#"{"defaultAssistantId":7}"#).unwrap();
        assert_eq!(set.default_assistant_id, Some(Some(7)));
    }

    #[test]
    fn corrupt_index_is_rebuilt_from_files_without_touching_notes() {
        let (_tmp, s) = store();
        let index = s.ensure_personal(&seed()).unwrap();
        let work = s.create_project("Work", "en", None).unwrap();
        std::fs::write(s.notes_dir(&work).join("a.md"), "# keep me").unwrap();
        std::fs::write(s.index_path(), "{ not json").unwrap();

        assert!(matches!(s.load_index(), Err(ProjectError::Parse(_))));
        let rebuilt = s.rebuild_index().unwrap();
        assert_eq!(rebuilt.personal_id, index.personal_id);
        assert_eq!(rebuilt.order.len(), 2);
        assert!(rebuilt.migration.personal_v1);
        assert!(s.notes_dir(&work).join("a.md").is_file());
        // And the migration does not create a second Personal afterwards.
        assert_eq!(s.ensure_personal(&seed()).unwrap().order.len(), 2);
    }

    #[test]
    fn missing_index_with_existing_personal_file_rebuilds() {
        let (_tmp, s) = store();
        let index = s.ensure_personal(&seed()).unwrap();
        std::fs::remove_file(s.index_path()).unwrap();
        let again = s.ensure_personal(&seed()).unwrap();
        assert_eq!(again.personal_id, index.personal_id);
        assert_eq!(s.list_projects().unwrap().len(), 1);
    }

    #[test]
    fn unsafe_ids_are_rejected() {
        let (_tmp, s) = store();
        s.ensure_personal(&seed()).unwrap();
        assert!(matches!(
            s.get_project("../../etc"),
            Err(ProjectError::InvalidId)
        ));
    }

    #[test]
    fn slot_capability_map_is_exact() {
        assert_eq!(ModelSlot::Voice.capability(), "SOUND2TEXT");
        assert_eq!(ModelSlot::Embed.capability(), "VECTORIZE");
        assert_eq!(ModelSlot::Docs.capability(), "ANALYZE");
        assert_eq!(
            ModelSlot::from_capability("PIC2TEXT"),
            Some(ModelSlot::Vision)
        );
        assert_eq!(ModelSlot::parse("speak"), Some(ModelSlot::Speak));
        assert_eq!(ModelSlot::parse("SOUND2TEXT"), None);
    }

    #[test]
    fn wire_model_id_rules() {
        assert_eq!(wire_model_id("", None), None);
        assert_eq!(
            wire_model_id("ollama:llama3.2:chat", None).as_deref(),
            Some("llama3.2")
        );
        // Provider ids may contain colons; only the first and last segment go.
        assert_eq!(
            wire_model_id("ollama:llama3.2:latest:chat", None).as_deref(),
            Some("llama3.2:latest")
        );
        // Legacy pick is sent verbatim even if it happens to contain colons.
        assert_eq!(
            wire_model_id("llama3.2:latest", Some("llama3.2:latest")).as_deref(),
            Some("llama3.2:latest")
        );
        assert_eq!(
            wire_model_id("gpt-4o-mini", None).as_deref(),
            Some("gpt-4o-mini")
        );
        assert!(is_catalog_key("openai:gpt-4o-mini:chat"));
        assert!(!is_catalog_key("gpt-4o-mini"));
        assert!(!is_catalog_key(":x:"));
    }

    #[test]
    fn language_is_normalized() {
        assert_eq!(normalize_language(""), "en");
        assert_eq!(normalize_language(" DE "), "de");
        assert_eq!(normalize_language("de_at"), "de-AT");
        assert_eq!(normalize_language("pt-BR"), "pt-BR");
    }

    #[test]
    fn ensure_personal_rebuilds_a_corrupt_index_without_touching_notes() {
        let (_tmp, s) = store();
        let index = s.ensure_personal(&seed()).unwrap();
        let work = s.create_project("Work", "en", None).unwrap();
        std::fs::write(s.notes_dir(&work).join("a.md"), "# keep me").unwrap();
        std::fs::write(s.index_path(), "{ not json").unwrap();

        let rebuilt = s.ensure_personal(&seed()).unwrap();
        assert_eq!(rebuilt.personal_id, index.personal_id);
        assert_eq!(rebuilt.order.len(), 2);
        assert!(s.notes_dir(&work).join("a.md").is_file());
    }

    #[test]
    fn a_traversing_slug_is_rejected_before_any_path_is_exposed() {
        let (_tmp, s) = store();
        s.ensure_personal(&seed()).unwrap();
        let raw = r#"
id = "01ARZ3NDEKTSV4RRFFQ69G5FAV"
slug = "../outside"
name = "Evil"
created_at = "2026-09-10T00:00:00Z"
updated_at = "2026-09-10T00:00:00Z"
"#;
        std::fs::write(s.project_file("01ARZ3NDEKTSV4RRFFQ69G5FAV"), raw).unwrap();
        assert!(matches!(
            s.get_project("01ARZ3NDEKTSV4RRFFQ69G5FAV"),
            Err(ProjectError::Parse(_))
        ));
    }

    #[test]
    fn recreating_a_name_does_not_reuse_a_leftover_folder() {
        let (_tmp, s) = store();
        s.ensure_personal(&seed()).unwrap();
        let work = s.create_project("Work", "en", None).unwrap();
        std::fs::write(s.notes_dir(&work).join("old.md"), "secret").unwrap();
        s.delete_project(&work.id, false).unwrap();
        let again = s.create_project("Work", "en", None).unwrap();
        assert_eq!(again.slug, "work-2");
        assert!(s.notes_dir(&work).join("old.md").is_file());
        assert!(!s.notes_dir(&again).join("old.md").exists());
    }

    #[test]
    fn create_rolls_back_metadata_when_dirs_cannot_be_made() {
        let (_tmp, s) = store();
        s.ensure_personal(&seed()).unwrap();
        std::fs::create_dir_all(s.projects_dir()).unwrap();
        std::fs::write(s.projects_dir().join("rollback-test"), "not a directory").unwrap();
        assert!(s.create_project("Rollback Test", "en", None).is_err());
        assert!(s
            .list_projects()
            .unwrap()
            .iter()
            .all(|p| p.slug != "rollback-test"));
    }

    #[test]
    fn assistant_binds_can_be_cleared_and_pruned() {
        let (_tmp, s) = store();
        let index = s.ensure_personal(&seed()).unwrap();
        s.update_project(
            &index.personal_id,
            ProjectPatch {
                default_assistant_id: Some(Some(7)),
                assistant_ids: Some(vec![7, 9]),
                ..Default::default()
            },
        )
        .unwrap();
        s.retain_known_assistants(&[7]).unwrap();
        let p = s.get_project(&index.personal_id).unwrap();
        assert_eq!(p.assistant_ids, vec![7]);
        assert_eq!(p.default_assistant_id, Some(7));
        s.clear_assistant_bindings().unwrap();
        let p = s.get_project(&index.personal_id).unwrap();
        assert!(p.assistant_ids.is_empty());
        assert_eq!(p.default_assistant_id, None);
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_project_folder_is_refused() {
        let (tmp, s) = store();
        s.ensure_personal(&seed()).unwrap();
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(outside.join("notes")).unwrap();
        std::fs::create_dir_all(s.projects_dir()).unwrap();
        std::os::unix::fs::symlink(&outside, s.projects_dir().join("link-me")).unwrap();
        let work = s.create_project("Link Me", "en", None).unwrap();
        // unique_slug sees the leftover "link-me" folder and picks link-me-2.
        assert_eq!(work.slug, "link-me-2");
        assert!(s
            .create_dirs(&Project {
                slug: "link-me".into(),
                ..work.clone()
            })
            .is_err());
    }
}
