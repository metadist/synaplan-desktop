//! The filesystem allowlist policy (DC8): which folders skills may read, where
//! they may write (the out-box), the always-on deny globs, and the file-size
//! cap. Persisted as `filesystem.toml` in the config dir; turned into a
//! [`Confinement`] for every file operation.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::platform::confinement::{default_deny_globs, Access, Confinement, ConfinementError};

#[derive(Debug, Error)]
pub enum FsPolicyError {
    #[error("could not read the filesystem policy: {0}")]
    Read(String),
    #[error("could not write the filesystem policy: {0}")]
    Write(String),
    #[error("the filesystem policy is not valid TOML: {0}")]
    Parse(String),
    #[error("that folder could not be resolved")]
    BadFolder,
}

fn default_max_file_bytes() -> u64 {
    10_000_000
}

/// The user-editable filesystem allowlist. Paths are stored canonicalized and
/// platform-native.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemPolicy {
    #[serde(default)]
    pub read: Vec<String>,
    #[serde(default)]
    pub write: Vec<String>,
    #[serde(default = "default_deny_globs")]
    pub deny: Vec<String>,
    #[serde(default = "default_max_file_bytes")]
    pub max_file_bytes: u64,
}

impl Default for FilesystemPolicy {
    fn default() -> Self {
        Self {
            read: Vec::new(),
            write: Vec::new(),
            deny: default_deny_globs(),
            max_file_bytes: default_max_file_bytes(),
        }
    }
}

impl FilesystemPolicy {
    pub fn load(path: &Path) -> Result<Self, FsPolicyError> {
        match std::fs::read_to_string(path) {
            Ok(s) => toml::from_str(&s).map_err(|e| FsPolicyError::Parse(e.to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(FsPolicyError::Read(e.to_string())),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), FsPolicyError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| FsPolicyError::Write(e.to_string()))?;
        }
        let toml = toml::to_string_pretty(self).map_err(|e| FsPolicyError::Write(e.to_string()))?;
        std::fs::write(path, toml).map_err(|e| FsPolicyError::Write(e.to_string()))
    }

    /// Ensure the out-box exists and is a canonicalized write root. Idempotent.
    pub fn ensure_outbox(&mut self, outbox: &Path) {
        let _ = std::fs::create_dir_all(outbox);
        if let Ok(canon) = std::fs::canonicalize(outbox) {
            let s = canon.to_string_lossy().to_string();
            if !self.write.iter().any(|w| w == &s) {
                self.write.push(s);
            }
        }
        if self.deny.is_empty() {
            self.deny = default_deny_globs();
        }
    }

    /// Add a read folder (canonicalized). No-op if already present.
    pub fn add_read(&mut self, folder: &str) -> Result<(), FsPolicyError> {
        let canon = std::fs::canonicalize(folder).map_err(|_| FsPolicyError::BadFolder)?;
        let s = canon.to_string_lossy().to_string();
        if !self.read.iter().any(|r| r == &s) {
            self.read.push(s);
        }
        Ok(())
    }

    /// Remove a read folder by its stored (canonical) string.
    pub fn remove_read(&mut self, folder: &str) {
        self.read.retain(|r| r != folder);
    }

    /// Build a [`Confinement`] from this policy.
    pub fn confinement(&self) -> Result<Confinement, ConfinementError> {
        let read: Vec<PathBuf> = self.read.iter().map(PathBuf::from).collect();
        let write: Vec<PathBuf> = self.write.iter().map(PathBuf::from).collect();
        Confinement::new(&read, &write, &self.deny)
    }
}

#[derive(Debug, Error)]
pub enum FileToolError {
    #[error("path not allowed: {0}")]
    Confinement(#[from] ConfinementError),
    #[error("file is too large")]
    TooLarge,
    #[error("io error: {0}")]
    Io(String),
}

/// Read a file, enforcing the allowlist and the size cap. UTF-8 lossy.
pub fn read_file(
    confinement: &Confinement,
    path: &str,
    max_bytes: u64,
) -> Result<String, FileToolError> {
    let resolved = confinement.resolve(path, Access::Read)?;
    let meta = std::fs::metadata(&resolved).map_err(|e| FileToolError::Io(e.to_string()))?;
    if meta.len() > max_bytes {
        return Err(FileToolError::TooLarge);
    }
    let bytes = std::fs::read(&resolved).map_err(|e| FileToolError::Io(e.to_string()))?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

/// Write a file, enforcing the write-root allowlist and the size cap. Returns
/// the canonical path written.
pub fn write_file(
    confinement: &Confinement,
    path: &str,
    contents: &str,
    max_bytes: u64,
) -> Result<PathBuf, FileToolError> {
    if contents.len() as u64 > max_bytes {
        return Err(FileToolError::TooLarge);
    }
    let resolved = confinement.resolve(path, Access::Write)?;
    if let Some(parent) = resolved.parent() {
        std::fs::create_dir_all(parent).map_err(|e| FileToolError::Io(e.to_string()))?;
    }
    std::fs::write(&resolved, contents).map_err(|e| FileToolError::Io(e.to_string()))?;
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal deny for fixtures: the default globs include `**/AppData/**`, and
    // Windows temp dirs live under AppData, which would deny every fixture path.
    fn test_policy() -> FilesystemPolicy {
        FilesystemPolicy {
            deny: vec!["**/.ssh/**".to_string()],
            ..Default::default()
        }
    }

    #[test]
    fn write_then_read_within_outbox() {
        let dir = tempfile::tempdir().unwrap();
        let outbox = std::fs::canonicalize(dir.path()).unwrap();
        let mut policy = test_policy();
        policy.ensure_outbox(&outbox);
        let confinement = policy.confinement().unwrap();

        let target = outbox.join("note.txt");
        let written = write_file(
            &confinement,
            target.to_str().unwrap(),
            "hello",
            policy.max_file_bytes,
        )
        .unwrap();
        assert!(written.starts_with(&outbox));
        let back = read_file(
            &confinement,
            target.to_str().unwrap(),
            policy.max_file_bytes,
        )
        .unwrap();
        assert_eq!(back, "hello");
    }

    #[test]
    fn write_outside_outbox_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let outbox = std::fs::canonicalize(dir.path()).unwrap().join("out");
        let mut policy = test_policy();
        policy.ensure_outbox(&outbox);
        let confinement = policy.confinement().unwrap();

        let outside = std::fs::canonicalize(dir.path()).unwrap().join("evil.txt");
        let err = write_file(
            &confinement,
            outside.to_str().unwrap(),
            "x",
            policy.max_file_bytes,
        );
        assert!(matches!(err, Err(FileToolError::Confinement(_))));
    }

    #[test]
    fn size_cap_enforced() {
        let dir = tempfile::tempdir().unwrap();
        let outbox = std::fs::canonicalize(dir.path()).unwrap();
        let mut policy = test_policy();
        policy.ensure_outbox(&outbox);
        let confinement = policy.confinement().unwrap();
        let target = outbox.join("big.txt");
        let err = write_file(&confinement, target.to_str().unwrap(), "toolong", 3);
        assert!(matches!(err, Err(FileToolError::TooLarge)));
    }
}
