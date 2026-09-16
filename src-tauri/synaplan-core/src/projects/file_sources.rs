//! Local source path of a file that was copied into a project's knowledge folder.
//!
//! The workspace only keeps the uploaded copy. This sidecar remembers where the
//! original lived on this computer so the Files view can open it (or its folder)
//! after the send. Never sent to Synaplan.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{is_safe_id, write_atomic, ProjectError};
use crate::files::KnowledgeFile;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileSource {
    pub path: String,
    pub dir: String,
}

pub fn path(meta_dir: &Path, project_id: &str) -> PathBuf {
    meta_dir.join(format!("{project_id}.sources.json"))
}

pub fn load(meta_dir: &Path, project_id: &str) -> HashMap<i64, FileSource> {
    if !is_safe_id(project_id) {
        return HashMap::new();
    }
    let raw = match std::fs::read_to_string(path(meta_dir, project_id)) {
        Ok(s) => s,
        Err(_) => return HashMap::new(),
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn remember(
    meta_dir: &Path,
    project_id: &str,
    file_id: i64,
    source: &Path,
) -> Result<(), ProjectError> {
    if !is_safe_id(project_id) {
        return Err(ProjectError::InvalidId);
    }
    let mut map = load(meta_dir, project_id);
    map.insert(file_id, source_from_path(source));
    save(meta_dir, project_id, &map)
}

pub fn forget(meta_dir: &Path, project_id: &str, file_id: i64) -> Result<(), ProjectError> {
    if !is_safe_id(project_id) {
        return Err(ProjectError::InvalidId);
    }
    let mut map = load(meta_dir, project_id);
    if map.remove(&file_id).is_none() {
        return Ok(());
    }
    if map.is_empty() {
        match std::fs::remove_file(path(meta_dir, project_id)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(ProjectError::Write(e.to_string())),
        }
    } else {
        save(meta_dir, project_id, &map)
    }
}

pub fn attach(files: &mut [KnowledgeFile], sources: &HashMap<i64, FileSource>) {
    for file in files {
        attach_one(file, sources);
    }
}

pub fn attach_one(file: &mut KnowledgeFile, sources: &HashMap<i64, FileSource>) {
    let Some(src) = sources.get(&file.id) else {
        return;
    };
    file.source_path = Some(src.path.clone());
    file.source_dir = Some(src.dir.clone());
    file.source_available = Path::new(&src.path).is_file() || Path::new(&src.dir).is_dir();
}

pub fn source_from_path(source: &Path) -> FileSource {
    let path = source.to_string_lossy().to_string();
    let dir = if source.is_dir() {
        path.clone()
    } else {
        source
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone())
    };
    FileSource { path, dir }
}

fn save(
    meta_dir: &Path,
    project_id: &str,
    map: &HashMap<i64, FileSource>,
) -> Result<(), ProjectError> {
    let bytes = serde_json::to_vec_pretty(map).map_err(|e| ProjectError::Write(e.to_string()))?;
    write_atomic(&path(meta_dir, project_id), &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::{KnowledgeFile, KnowledgeState};

    fn tmp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "synaplan-sources-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn remembers_and_attaches_a_source_that_still_exists() {
        let meta = tmp();
        let original = meta.join("voucher.pdf");
        std::fs::write(&original, b"pdf").unwrap();
        remember(&meta, "01ARZ3NDEKTSV4RRFFQ69G5FAV", 12, &original).unwrap();

        let mut files = vec![KnowledgeFile {
            id: 12,
            name: "voucher.pdf".into(),
            size: 3,
            state: KnowledgeState::Ready,
            detail: None,
            uploaded_at: String::new(),
            source_path: None,
            source_dir: None,
            source_available: false,
        }];
        attach(&mut files, &load(&meta, "01ARZ3NDEKTSV4RRFFQ69G5FAV"));
        assert_eq!(files[0].source_path.as_deref(), Some(original.to_str().unwrap()));
        assert!(files[0].source_available);
        assert_eq!(
            files[0].source_dir.as_deref(),
            Some(meta.to_str().unwrap())
        );

        forget(&meta, "01ARZ3NDEKTSV4RRFFQ69G5FAV", 12).unwrap();
        assert!(load(&meta, "01ARZ3NDEKTSV4RRFFQ69G5FAV").is_empty());
        let _ = std::fs::remove_dir_all(meta);
    }
}
