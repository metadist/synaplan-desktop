//! Project notes (`PC8`): plain UTF-8 Markdown files in
//! `{projects_dir}/{slug}/notes/*.md`. Notes never leave this computer unless
//! the user shares one explicitly (`PC9`).
//!
//! A note is addressed by its **file name** only. The webview never passes a
//! path: the name is validated here, resolved under the project's notes folder,
//! and the resolved parent is checked against the canonical notes folder so a
//! symlink or `..` can never point outside it.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use super::ids::{iso8601_from_unix, now_millis};
use super::{write_atomic, ProjectError, ProjectStore};

pub const NOTE_EXTENSION: &str = "md";
const MAX_NOTE_NAME_LEN: usize = 128;
/// Notes larger than this are listed but not opened in the editor.
pub const MAX_NOTE_BYTES: u64 = 2_000_000;

/// What the note manager shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteSummary {
    /// File name including `.md`; the only handle the UI holds.
    pub name: String,
    /// First `# ` heading, else the file stem.
    pub title: String,
    pub updated_at: String,
    pub size: u64,
}

/// An open note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub name: String,
    pub title: String,
    pub content: String,
    pub updated_at: String,
    /// Platform-native path, for "reveal in file manager" only.
    pub path: String,
}

/// A note file name is a single path component ending in `.md`, without
/// separators, control characters, or a leading dot.
pub fn is_valid_note_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_NOTE_NAME_LEN
        && name.ends_with(&format!(".{NOTE_EXTENSION}"))
        && name.len() > NOTE_EXTENSION.len() + 1
        && !name.starts_with('.')
        && name != format!("..{NOTE_EXTENSION}")
        && !name.chars().any(|c| {
            c == '/'
                || c == '\\'
                || c == ':'
                || c == '\0'
                || c.is_control()
                || c == '*'
                || c == '?'
                || c == '"'
                || c == '<'
                || c == '>'
                || c == '|'
        })
}

/// The title the manager shows: the first `# ` heading, else the file stem.
pub fn note_title(name: &str, content: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("# ") {
            let heading = rest.trim().trim_end_matches('#').trim();
            if !heading.is_empty() {
                return heading.chars().take(120).collect();
            }
        } else if !trimmed.is_empty() {
            // Only a heading at the very top counts as the title.
            break;
        }
    }
    name.strip_suffix(&format!(".{NOTE_EXTENSION}"))
        .unwrap_or(name)
        .to_string()
}

/// `YYYY-MM-DD-hhmm.md` from the current UTC time.
pub fn timestamp_note_name(unix_secs: u64) -> String {
    let iso = iso8601_from_unix(unix_secs);
    // 2026-09-10T13:45:12Z → 2026-09-10-1345
    let date = &iso[0..10];
    let hh = &iso[11..13];
    let mm = &iso[14..16];
    format!("{date}-{hh}{mm}.{NOTE_EXTENSION}")
}

fn unique_name(dir: &Path, base: &str) -> String {
    if !dir.join(base).exists() {
        return base.to_string();
    }
    let stem = base
        .strip_suffix(&format!(".{NOTE_EXTENSION}"))
        .unwrap_or(base);
    let mut n = 2;
    loop {
        let candidate = format!("{stem}-{n}.{NOTE_EXTENSION}");
        if !dir.join(&candidate).exists() {
            return candidate;
        }
        n += 1;
    }
}

fn modified_iso(meta: &std::fs::Metadata) -> String {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| iso8601_from_unix(d.as_secs()))
        .unwrap_or_default()
}

impl ProjectStore {
    /// The notes folder for a project, created if missing. The folder itself
    /// must be a real directory contained by the project's folder.
    fn notes_root(&self, project_id: &str) -> Result<PathBuf, ProjectError> {
        let project = self.get_project(project_id)?;
        self.create_dirs(&project)?;
        let dir = self.notes_dir(&project);
        let project_root = self.contained_project_dir(&project)?;
        let notes_canon =
            std::fs::canonicalize(&dir).map_err(|e| ProjectError::Read(e.to_string()))?;
        let project_canon =
            std::fs::canonicalize(&project_root).map_err(|e| ProjectError::Read(e.to_string()))?;
        if !notes_canon.starts_with(&project_canon) || notes_canon == project_canon {
            return Err(ProjectError::UnsafePath(
                "notes folder escaped the project folder".into(),
            ));
        }
        Ok(dir)
    }

    /// Resolve a validated note name under the notes folder and prove the
    /// result still lives there (no `..`, no symlinked escape).
    fn note_path(&self, notes_dir: &Path, name: &str) -> Result<PathBuf, ProjectError> {
        if !is_valid_note_name(name) {
            return Err(ProjectError::InvalidId);
        }
        if let Ok(meta) = std::fs::symlink_metadata(notes_dir) {
            if meta.file_type().is_symlink() {
                return Err(ProjectError::UnsafePath("notes folder is a symlink".into()));
            }
        }
        let canonical_dir =
            std::fs::canonicalize(notes_dir).map_err(|e| ProjectError::Read(e.to_string()))?;
        let path = canonical_dir.join(name);
        if let Ok(meta) = std::fs::symlink_metadata(&path) {
            if meta.file_type().is_symlink() || meta.is_dir() {
                return Err(ProjectError::InvalidId);
            }
        }
        match path.parent() {
            Some(parent) if parent == canonical_dir => Ok(path),
            _ => Err(ProjectError::InvalidId),
        }
    }

    pub fn list_notes(&self, project_id: &str) -> Result<Vec<NoteSummary>, ProjectError> {
        let dir = self.notes_root(project_id)?;
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&dir).map_err(|e| ProjectError::Read(e.to_string()))? {
            let entry = entry.map_err(|e| ProjectError::Read(e.to_string()))?;
            let name = entry.file_name().to_string_lossy().to_string();
            if !is_valid_note_name(&name) {
                continue;
            }
            let meta = match std::fs::symlink_metadata(entry.path()) {
                Ok(m) if m.is_file() && !m.file_type().is_symlink() => m,
                _ => continue,
            };
            let content = if meta.len() <= MAX_NOTE_BYTES {
                std::fs::read_to_string(entry.path()).unwrap_or_default()
            } else {
                String::new()
            };
            out.push(NoteSummary {
                title: note_title(&name, &content),
                name,
                updated_at: modified_iso(&meta),
                size: meta.len(),
            });
        }
        out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then(a.name.cmp(&b.name)));
        Ok(out)
    }

    /// Case-insensitive substring search over file name, title and text.
    pub fn search_notes(
        &self,
        project_id: &str,
        query: &str,
    ) -> Result<Vec<NoteSummary>, ProjectError> {
        let needle = query.trim().to_lowercase();
        let all = self.list_notes(project_id)?;
        if needle.is_empty() {
            return Ok(all);
        }
        let dir = self.notes_root(project_id)?;
        Ok(all
            .into_iter()
            .filter(|n| {
                if n.name.to_lowercase().contains(&needle)
                    || n.title.to_lowercase().contains(&needle)
                {
                    return true;
                }
                n.size <= MAX_NOTE_BYTES
                    && std::fs::read_to_string(dir.join(&n.name))
                        .map(|c| c.to_lowercase().contains(&needle))
                        .unwrap_or(false)
            })
            .collect())
    }

    pub fn read_note(&self, project_id: &str, name: &str) -> Result<Note, ProjectError> {
        let dir = self.notes_root(project_id)?;
        let path = self.note_path(&dir, name)?;
        let meta = std::fs::metadata(&path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => ProjectError::NotFound(name.to_string()),
            _ => ProjectError::Read(e.to_string()),
        })?;
        if meta.len() > MAX_NOTE_BYTES {
            return Err(ProjectError::Read(format!(
                "note {name} is too large to open ({} bytes)",
                meta.len()
            )));
        }
        let content =
            std::fs::read_to_string(&path).map_err(|e| ProjectError::Read(e.to_string()))?;
        Ok(Note {
            title: note_title(name, &content),
            name: name.to_string(),
            content,
            updated_at: modified_iso(&meta),
            path: path.to_string_lossy().to_string(),
        })
    }

    /// Create an empty note named after the current time (`-2`, `-3` on clash).
    pub fn create_note(&self, project_id: &str) -> Result<Note, ProjectError> {
        let dir = self.notes_root(project_id)?;
        let name = unique_name(&dir, &timestamp_note_name(now_millis() / 1000));
        let path = self.note_path(&dir, &name)?;
        write_atomic(&path, b"")?;
        self.read_note(project_id, &name)
    }

    /// Overwrite a note's Markdown (atomic temp + rename).
    pub fn write_note(
        &self,
        project_id: &str,
        name: &str,
        content: &str,
    ) -> Result<NoteSummary, ProjectError> {
        let dir = self.notes_root(project_id)?;
        let path = self.note_path(&dir, name)?;
        write_atomic(&path, content.as_bytes())?;
        let meta = std::fs::metadata(&path).map_err(|e| ProjectError::Read(e.to_string()))?;
        Ok(NoteSummary {
            title: note_title(name, content),
            name: name.to_string(),
            updated_at: modified_iso(&meta),
            size: meta.len(),
        })
    }

    pub fn delete_note(&self, project_id: &str, name: &str) -> Result<(), ProjectError> {
        let dir = self.notes_root(project_id)?;
        let path = self.note_path(&dir, name)?;
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(ProjectError::Write(e.to_string())),
        }
    }
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
    fn note_names_are_single_markdown_components() {
        assert!(is_valid_note_name("2026-09-10-1345.md"));
        assert!(is_valid_note_name("Kitchen plan.md"));
        assert!(!is_valid_note_name(""));
        assert!(!is_valid_note_name(".md"));
        assert!(!is_valid_note_name("notes.txt"));
        assert!(!is_valid_note_name("../secret.md"));
        assert!(!is_valid_note_name("sub/dir.md"));
        assert!(!is_valid_note_name("sub\\dir.md"));
        assert!(!is_valid_note_name(".hidden.md"));
        assert!(!is_valid_note_name("bad\0name.md"));
        assert!(!is_valid_note_name(&format!("{}.md", "x".repeat(200))));
    }

    #[test]
    fn title_is_the_top_heading_or_the_stem() {
        assert_eq!(note_title("a.md", "# Kitchen plan\n\ntext"), "Kitchen plan");
        assert_eq!(note_title("a.md", "\n\n  # Spaced ##\n"), "Spaced");
        assert_eq!(note_title("a.md", "text first\n# Not a title"), "a");
        assert_eq!(note_title("2026-09-10-1345.md", ""), "2026-09-10-1345");
    }

    #[test]
    fn timestamp_names_are_sortable() {
        // 2026-09-10T15:38:32Z
        assert_eq!(timestamp_note_name(1_789_054_712), "2026-09-10-1538.md");
        // 1970-01-01T00:00:00Z
        assert_eq!(timestamp_note_name(0), "1970-01-01-0000.md");
    }

    #[test]
    fn create_write_list_search_delete_round_trip() {
        let (_tmp, s, pid) = store();
        assert!(s.list_notes(&pid).unwrap().is_empty());

        let note = s.create_note(&pid).unwrap();
        assert!(note.name.ends_with(".md"));
        assert_eq!(note.content, "");

        let second = s.create_note(&pid).unwrap();
        assert_ne!(second.name, note.name, "same-minute names get a suffix");

        let summary = s
            .write_note(&pid, &note.name, "# Kitchen plan\n\nBuy a fridge.")
            .unwrap();
        assert_eq!(summary.title, "Kitchen plan");

        let opened = s.read_note(&pid, &note.name).unwrap();
        assert_eq!(opened.content, "# Kitchen plan\n\nBuy a fridge.");
        assert!(opened.path.ends_with(&note.name));

        let listed = s.list_notes(&pid).unwrap();
        assert_eq!(listed.len(), 2);
        assert!(listed.iter().any(|n| n.title == "Kitchen plan"));

        let hits = s.search_notes(&pid, "FRIDGE").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, note.name);
        assert_eq!(s.search_notes(&pid, "nothing-here").unwrap().len(), 0);
        assert_eq!(s.search_notes(&pid, "   ").unwrap().len(), 2);

        s.delete_note(&pid, &note.name).unwrap();
        assert!(matches!(
            s.read_note(&pid, &note.name),
            Err(ProjectError::NotFound(_))
        ));
        s.delete_note(&pid, &note.name).unwrap(); // idempotent
    }

    #[test]
    fn names_that_escape_the_notes_folder_are_rejected() {
        let (_tmp, s, pid) = store();
        for bad in ["../x.md", "/etc/passwd.md", "a/b.md", ".env.md", "x.txt"] {
            assert!(
                matches!(s.read_note(&pid, bad), Err(ProjectError::InvalidId)),
                "{bad}"
            );
            assert!(matches!(
                s.write_note(&pid, bad, "x"),
                Err(ProjectError::InvalidId)
            ));
            assert!(matches!(
                s.delete_note(&pid, bad),
                Err(ProjectError::InvalidId)
            ));
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_inside_the_notes_folder_is_not_followed() {
        let (tmp, s, pid) = store();
        let outside = tmp.path().join("outside.md");
        std::fs::write(&outside, "# secret").unwrap();
        let project = s.get_project(&pid).unwrap();
        let dir = s.notes_dir(&project);
        std::fs::create_dir_all(&dir).unwrap();
        std::os::unix::fs::symlink(&outside, dir.join("link.md")).unwrap();

        assert!(matches!(
            s.read_note(&pid, "link.md"),
            Err(ProjectError::InvalidId)
        ));
        assert!(matches!(
            s.write_note(&pid, "link.md", "overwrite"),
            Err(ProjectError::InvalidId)
        ));
        assert_eq!(std::fs::read_to_string(&outside).unwrap(), "# secret");
        assert!(
            s.list_notes(&pid)
                .unwrap()
                .iter()
                .all(|n| n.name != "link.md"),
            "listing must not follow a .md symlink"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_notes_directory_is_refused() {
        let (tmp, s, pid) = store();
        let project = s.get_project(&pid).unwrap();
        let notes = s.notes_dir(&project);
        std::fs::remove_dir_all(&notes).unwrap();
        let outside = tmp.path().join("outside-notes");
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("stolen.md"), "# secret").unwrap();
        std::os::unix::fs::symlink(&outside, &notes).unwrap();
        assert!(matches!(
            s.list_notes(&pid),
            Err(ProjectError::UnsafePath(_))
        ));
        assert!(matches!(
            s.read_note(&pid, "stolen.md"),
            Err(ProjectError::UnsafePath(_))
        ));
    }

    #[test]
    fn non_markdown_files_in_the_folder_are_ignored() {
        let (_tmp, s, pid) = store();
        let project = s.get_project(&pid).unwrap();
        let dir = s.notes_dir(&project);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("image.png"), b"\x89PNG").unwrap();
        std::fs::write(dir.join("real.md"), "# Real").unwrap();
        let listed = s.list_notes(&pid).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].title, "Real");
    }
}
