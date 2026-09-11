//! Per-project chat threads (`PC5`): `{config_dir}/projects/{projectId}/chats/{chatId}.json`.
//!
//! Threads are local only. They never contain the API key, and the server
//! never sees them as a unit — every turn is re-sent through `/v1/messages`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::ids::{is_safe_id, new_id, now_iso8601};
use super::{write_atomic, ProjectError, ProjectStore};
use crate::artifacts::ChatArtifact;

/// Sender of one chat message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    /// Chat model (catalog key or legacy id) in force when the assistant reply
    /// was produced; empty for user messages.
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub created_at: String,
    /// Local files this turn created (images, audio, documents, skill output).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<ChatArtifact>,
}

/// A full thread as stored on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatThread {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    /// Assistant (server recipe) pinned for this thread, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assistant_id: Option<i64>,
    #[serde(default)]
    pub messages: Vec<ChatMessage>,
}

/// What the thread list shows: no message bodies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatSummary {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assistant_id: Option<i64>,
}

/// Longest auto-title derived from the first user message.
pub const MAX_TITLE_LEN: usize = 60;

impl ChatThread {
    /// A fresh, empty thread for `project_id`.
    pub fn new(project_id: &str) -> Self {
        let now = now_iso8601();
        Self {
            id: new_id(),
            project_id: project_id.to_string(),
            title: String::new(),
            created_at: now.clone(),
            updated_at: now,
            assistant_id: None,
            messages: Vec::new(),
        }
    }

    pub fn summary(&self) -> ChatSummary {
        ChatSummary {
            id: self.id.clone(),
            project_id: self.project_id.clone(),
            title: self.title.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            message_count: self.messages.len(),
            assistant_id: self.assistant_id,
        }
    }

    /// Append a message, stamp it, and derive a title from the first user
    /// message when none is set yet.
    pub fn push(&mut self, role: ChatRole, content: &str, model: &str) {
        let now = now_iso8601();
        self.messages.push(ChatMessage {
            role,
            content: content.to_string(),
            model: if role == ChatRole::Assistant {
                model.to_string()
            } else {
                String::new()
            },
            created_at: now.clone(),
            artifacts: Vec::new(),
        });
        if self.title.is_empty() && role == ChatRole::User {
            self.title = auto_title(content);
        }
        self.updated_at = now;
    }
}

/// First line of `text`, collapsed whitespace, cut to [`MAX_TITLE_LEN`] chars.
pub fn auto_title(text: &str) -> String {
    let first_line = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let collapsed: Vec<&str> = first_line.split_whitespace().collect();
    let joined = collapsed.join(" ");
    if joined.chars().count() <= MAX_TITLE_LEN {
        return joined;
    }
    let cut: String = joined.chars().take(MAX_TITLE_LEN - 1).collect();
    format!("{}…", cut.trim_end())
}

impl ProjectStore {
    pub fn chats_dir(&self, project_id: &str) -> PathBuf {
        self.project_meta_dir(project_id).join("chats")
    }

    pub fn chat_file(&self, project_id: &str, chat_id: &str) -> PathBuf {
        self.chats_dir(project_id).join(format!("{chat_id}.json"))
    }

    /// Summaries of every thread in `project_id`, newest `updated_at` first.
    pub fn list_chats(&self, project_id: &str) -> Result<Vec<ChatSummary>, ProjectError> {
        if !is_safe_id(project_id) {
            return Err(ProjectError::InvalidId);
        }
        let dir = self.chats_dir(project_id);
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(ProjectError::Read(e.to_string())),
        };
        let mut out = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if let Some(thread) = read_thread(&path) {
                out.push(thread.summary());
            }
        }
        out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then(b.id.cmp(&a.id)));
        Ok(out)
    }

    pub fn load_chat(&self, project_id: &str, chat_id: &str) -> Result<ChatThread, ProjectError> {
        if !is_safe_id(project_id) || !is_safe_id(chat_id) {
            return Err(ProjectError::InvalidId);
        }
        match std::fs::read_to_string(self.chat_file(project_id, chat_id)) {
            Ok(raw) => serde_json::from_str(&raw)
                .map_err(|e| ProjectError::Parse(format!("chat {chat_id}: {e}"))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(ProjectError::NotFound(chat_id.to_string()))
            }
            Err(e) => Err(ProjectError::Read(e.to_string())),
        }
    }

    /// Persist `thread` under its own `project_id`. The project must exist.
    pub fn save_chat(&self, thread: &ChatThread) -> Result<(), ProjectError> {
        if !is_safe_id(&thread.project_id) || !is_safe_id(&thread.id) {
            return Err(ProjectError::InvalidId);
        }
        self.get_project(&thread.project_id)?;
        let json =
            serde_json::to_string_pretty(thread).map_err(|e| ProjectError::Write(e.to_string()))?;
        write_atomic(
            &self.chat_file(&thread.project_id, &thread.id),
            json.as_bytes(),
        )
    }

    pub fn delete_chat(&self, project_id: &str, chat_id: &str) -> Result<(), ProjectError> {
        if !is_safe_id(project_id) || !is_safe_id(chat_id) {
            return Err(ProjectError::InvalidId);
        }
        match std::fs::remove_file(self.chat_file(project_id, chat_id)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(ProjectError::Write(e.to_string())),
        }
    }
}

fn read_thread(path: &Path) -> Option<ChatThread> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projects::PersonalSeed;

    fn store() -> (tempfile::TempDir, ProjectStore, String) {
        let dir = tempfile::tempdir().unwrap();
        let s = ProjectStore::with_roots(
            dir.path().join("config").join("projects"),
            dir.path().join("Synaplan").join("projects"),
        );
        let index = s.ensure_personal(&PersonalSeed::default()).unwrap();
        (dir, s, index.personal_id)
    }

    #[test]
    fn round_trip_and_listing() {
        let (_tmp, s, pid) = store();
        assert!(s.list_chats(&pid).unwrap().is_empty());

        let mut a = ChatThread::new(&pid);
        a.push(ChatRole::User, "  Plan the   kitchen\nsecond line", "");
        a.push(ChatRole::Assistant, "Sure.", "ollama:llama3.2:chat");
        s.save_chat(&a).unwrap();

        let mut b = ChatThread::new(&pid);
        b.push(ChatRole::User, "Later thread", "");
        b.updated_at = "2099-01-01T00:00:00Z".into();
        s.save_chat(&b).unwrap();

        let loaded = s.load_chat(&pid, &a.id).unwrap();
        assert_eq!(loaded, a);
        assert_eq!(loaded.title, "Plan the kitchen");
        assert_eq!(loaded.messages[1].model, "ollama:llama3.2:chat");
        assert_eq!(loaded.messages[0].model, "");
        assert!(loaded.messages[1].artifacts.is_empty());

        a.messages[1]
            .artifacts
            .push(crate::artifacts::ChatArtifact {
                path: "/tmp/out/cat.png".into(),
                name: "cat.png".into(),
                kind: "image".into(),
                file_id: Some(12),
            });
        s.save_chat(&a).unwrap();
        let with_file = s.load_chat(&pid, &a.id).unwrap();
        assert_eq!(with_file.messages[1].artifacts[0].name, "cat.png");
        assert_eq!(with_file.messages[1].artifacts[0].file_id, Some(12));

        let list = s.list_chats(&pid).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, b.id, "newest first");
        assert_eq!(list[1].message_count, 2);

        s.delete_chat(&pid, &a.id).unwrap();
        assert!(matches!(
            s.load_chat(&pid, &a.id),
            Err(ProjectError::NotFound(_))
        ));
        s.delete_chat(&pid, &a.id).unwrap(); // idempotent
    }

    #[test]
    fn chats_live_under_the_project_meta_dir_and_die_with_it() {
        let (_tmp, s, pid) = store();
        let work = s.create_project("Work", "en", None).unwrap();
        let mut t = ChatThread::new(&work.id);
        t.push(ChatRole::User, "hi", "");
        s.save_chat(&t).unwrap();
        assert!(s
            .chat_file(&work.id, &t.id)
            .starts_with(s.project_meta_dir(&work.id)));
        assert!(
            s.list_chats(&pid).unwrap().is_empty(),
            "no cross-project leak"
        );

        s.delete_project(&work.id, false).unwrap();
        assert!(!s.chats_dir(&work.id).exists());
    }

    #[test]
    fn unsafe_ids_and_unknown_projects_are_rejected() {
        let (_tmp, s, pid) = store();
        assert!(matches!(
            s.load_chat(&pid, "../x"),
            Err(ProjectError::InvalidId)
        ));
        assert!(matches!(
            s.list_chats("..\\up"),
            Err(ProjectError::InvalidId)
        ));
        let orphan = ChatThread::new("01ARZ3NDEKTSV4RRFFQ69G5FAV");
        assert!(matches!(
            s.save_chat(&orphan),
            Err(ProjectError::NotFound(_))
        ));
    }

    #[test]
    fn auto_title_truncates_on_char_boundary() {
        let long = "ä".repeat(100);
        let title = auto_title(&long);
        assert!(title.ends_with('…'));
        assert_eq!(title.chars().count(), MAX_TITLE_LEN);
        assert_eq!(auto_title("\n\n  hello   world  \n"), "hello world");
        assert_eq!(auto_title(""), "");
    }
}
