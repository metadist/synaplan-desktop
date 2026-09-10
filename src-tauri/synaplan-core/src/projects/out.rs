//! The project's `out/` folder — what skills produced for this project.
//!
//! Read-only listing for the In/Out board. Files only, newest first; the path
//! is handed to the webview solely so "Show in folder" can reveal it.

use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use super::ids::iso8601_from_unix;
use super::{ProjectError, ProjectStore};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutFile {
    pub name: String,
    pub size: u64,
    pub modified_at: String,
    /// Platform-native absolute path, for reveal only.
    pub path: String,
}

impl ProjectStore {
    /// Files in the project's `out/` folder, newest first. Sub-folders and
    /// hidden files are skipped; a missing folder is an empty list.
    pub fn list_out_files(&self, project_id: &str) -> Result<Vec<OutFile>, ProjectError> {
        let project = self.get_project(project_id)?;
        let dir = self.out_dir(&project);
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(ProjectError::Read(e.to_string())),
        };
        let mut out: Vec<(u64, OutFile)> = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            if !meta.is_file() {
                continue;
            }
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            out.push((
                modified,
                OutFile {
                    name,
                    size: meta.len(),
                    modified_at: iso8601_from_unix(modified),
                    path: entry.path().to_string_lossy().to_string(),
                },
            ));
        }
        out.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.name.cmp(&b.1.name)));
        Ok(out.into_iter().map(|(_, f)| f).collect())
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
    fn lists_only_plain_files_newest_first() {
        let (_tmp, store, id) = store();
        let project = store.get_project(&id).unwrap();
        let dir = store.out_dir(&project);
        std::fs::write(dir.join("older.txt"), "a").unwrap();
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join(".hidden"), "x").unwrap();
        let older = std::time::SystemTime::now() - std::time::Duration::from_secs(120);
        std::fs::File::open(dir.join("older.txt"))
            .unwrap()
            .set_modified(older)
            .unwrap();
        std::fs::write(dir.join("newer.pptx"), "bbb").unwrap();

        let files = store.list_out_files(&id).unwrap();
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["newer.pptx", "older.txt"]);
        assert_eq!(files[0].size, 3);
        assert!(files[0].path.ends_with("newer.pptx"));
        assert!(!files[0].modified_at.is_empty());
    }

    #[test]
    fn a_missing_out_folder_is_an_empty_list() {
        let (_tmp, store, id) = store();
        let project = store.get_project(&id).unwrap();
        std::fs::remove_dir_all(store.out_dir(&project)).unwrap();
        assert!(store.list_out_files(&id).unwrap().is_empty());
    }
}
