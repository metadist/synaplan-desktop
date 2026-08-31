//! The on-disk configuration (`config.toml`). It stores the paired instance's
//! base URL and the device id — and deliberately NOT the API key, which lives in
//! the OS secret store (see [`crate::platform::secret_store`]).

use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not read config: {0}")]
    Read(String),
    #[error("could not write config: {0}")]
    Write(String),
    #[error("config is not valid TOML: {0}")]
    Parse(String),
}

/// Persistent, non-secret desktop configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopConfig {
    /// The paired Synaplan instance base URL (e.g. `https://web.synaplan.com`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_base_url: Option<String>,
    /// The server-assigned device id from pairing (absent for a pasted-key
    /// recovery pairing).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<i64>,
}

impl DesktopConfig {
    /// Load config from `path`, returning defaults if the file does not exist.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(contents) => {
                toml::from_str(&contents).map_err(|e| ConfigError::Parse(e.to_string()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(ConfigError::Read(e.to_string())),
        }
    }

    /// Write config to `path`, creating parent directories as needed.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ConfigError::Write(e.to_string()))?;
        }
        let toml = toml::to_string_pretty(self).map_err(|e| ConfigError::Write(e.to_string()))?;
        std::fs::write(path, toml).map_err(|e| ConfigError::Write(e.to_string()))
    }

    /// Remove the config file (used on sign-out / 401). A missing file is fine.
    pub fn clear(path: &Path) -> Result<(), ConfigError> {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(ConfigError::Write(e.to_string())),
        }
    }

    /// True when this install has been paired (has a base URL).
    pub fn is_paired(&self) -> bool {
        self.api_base_url
            .as_deref()
            .map(|u| !u.is_empty())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_default() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = DesktopConfig::load(&dir.path().join("config.toml")).unwrap();
        assert_eq!(cfg, DesktopConfig::default());
        assert!(!cfg.is_paired());
    }

    #[test]
    fn roundtrip_and_clear() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("config.toml");
        let cfg = DesktopConfig {
            api_base_url: Some("https://web.synaplan.com".to_string()),
            device_id: Some(7),
        };
        cfg.save(&path).unwrap();
        let loaded = DesktopConfig::load(&path).unwrap();
        assert_eq!(loaded, cfg);
        assert!(loaded.is_paired());

        // The key must never be serialised into the config file.
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(
            !raw.contains("sk_"),
            "config file must not contain an API key"
        );

        DesktopConfig::clear(&path).unwrap();
        assert_eq!(
            DesktopConfig::load(&path).unwrap(),
            DesktopConfig::default()
        );
    }
}
