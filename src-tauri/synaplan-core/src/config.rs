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

/// Optional absolute paths the user configured for local tools.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolsConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub python: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub libreoffice: Option<String>,
}

impl ToolsConfig {
    pub fn is_empty(&self) -> bool {
        self.python.is_none() && self.node.is_none() && self.libreoffice.is_none()
    }
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
    /// Optional user-configured interpreter paths (step 1 of doctor discovery).
    #[serde(default, skip_serializing_if = "ToolsConfig::is_empty")]
    pub tools: ToolsConfig,
    /// Last model id the user picked in the chat dropdown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_chat_model: Option<String>,
    /// Up to three skill names shown as empty-chat example tiles.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub studio_tiles: Vec<String>,
}

/// Keep at most three unique, non-empty skill names.
pub fn sanitize_studio_tiles<I>(names: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut out: Vec<String> = Vec::new();
    for raw in names {
        let name = raw.trim();
        if name.is_empty() || out.iter().any(|n| n == name) {
            continue;
        }
        out.push(name.to_string());
        if out.len() == 3 {
            break;
        }
    }
    out
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
            tools: ToolsConfig::default(),
            last_chat_model: Some("claude-fable-5-1".to_string()),
            studio_tiles: vec!["email-draft".into(), "vcard".into()],
        };
        cfg.save(&path).unwrap();
        let loaded = DesktopConfig::load(&path).unwrap();
        assert_eq!(loaded, cfg);
        assert!(loaded.is_paired());
        assert!(std::fs::read_to_string(&path)
            .unwrap()
            .contains("last_chat_model"));

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

    #[test]
    fn default_config_does_not_enable_autostart() {
        let raw = toml::to_string(&DesktopConfig::default()).unwrap();
        assert!(
            !raw.to_ascii_lowercase().contains("autostart"),
            "the installer must not write an autostart preference"
        );
        assert!(
            !raw.contains("last_chat_model"),
            "an empty last-model pick must not be written"
        );
        assert!(
            !raw.contains("studio_tiles"),
            "empty example tiles must not be written"
        );
    }

    #[test]
    fn studio_tiles_keep_three_unique_names() {
        assert_eq!(
            sanitize_studio_tiles([
                " email-draft ".into(),
                "email-draft".into(),
                "".into(),
                "vcard".into(),
                "slides".into(),
                "invoice".into(),
            ]),
            vec!["email-draft", "vcard", "slides"]
        );
    }
}
