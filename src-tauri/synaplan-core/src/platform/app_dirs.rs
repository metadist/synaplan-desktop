//! `app_dirs` (DC22) — the ONE module that turns the operating environment into
//! Synaplan Desktop's four canonical directories. No other code may expand a
//! home directory or read `%APPDATA%` / `$XDG_*` (cross-platform §2).
//!
//! | Purpose | Windows | macOS | Linux |
//! | ------- | ------- | ----- | ----- |
//! | Config  | `%APPDATA%\Synaplan\Desktop\` | `~/Library/Application Support/com.synaplan.desktop/` | `$XDG_CONFIG_HOME/synaplan-desktop/` |
//! | Skills  | `%LOCALAPPDATA%\Synaplan\Desktop\skills\` | `~/Library/Application Support/com.synaplan.desktop/skills/` | `$XDG_DATA_HOME/synaplan-desktop/skills/` |
//! | Out-box | `%USERPROFILE%\Synaplan\out\` | `~/Synaplan/out/` | `~/Synaplan/out/` |
//! | Audit   | `%LOCALAPPDATA%\Synaplan\Desktop\logs\` | `~/Library/Logs/com.synaplan.desktop/` | `$XDG_STATE_HOME/synaplan-desktop/` |
//!
//! The out-box deliberately lives in the user's home on every platform so a
//! generated file can be found in Explorer/Finder without knowing what
//! `%LOCALAPPDATA%` means. The bundle identifier (`com.synaplan.desktop`) and
//! the Windows vendor path (`Synaplan\Desktop`) are permanent — changing them
//! orphans an existing install's config, skills, and stored key.

use std::path::PathBuf;

use thiserror::Error;

/// The fixed macOS bundle identifier / Linux XDG app folder anchor. Permanent.
pub const MACOS_BUNDLE_ID: &str = "com.synaplan.desktop";
/// The fixed Linux XDG application directory name. Permanent.
pub const LINUX_APP_DIR: &str = "synaplan-desktop";

#[derive(Debug, Error)]
pub enum AppDirsError {
    #[error("required environment variable {0} is not set; cannot locate {1}")]
    MissingVar(&'static str, &'static str),
}

/// A snapshot of the environment variables that decide where files live.
///
/// Tests construct this pointing at a tempdir so a developer's real
/// installation is never read or written (testing doc §7). Production reads the
/// live process environment via [`Env::from_system`].
#[derive(Debug, Clone, Default)]
pub struct Env {
    pub home: Option<PathBuf>,
    pub xdg_config_home: Option<PathBuf>,
    pub xdg_data_home: Option<PathBuf>,
    pub xdg_state_home: Option<PathBuf>,
    pub appdata: Option<PathBuf>,
    pub local_appdata: Option<PathBuf>,
    pub userprofile: Option<PathBuf>,
}

impl Env {
    /// Read the relevant variables from the live process environment.
    pub fn from_system() -> Self {
        let var = |k: &str| {
            std::env::var_os(k)
                .map(PathBuf::from)
                .filter(|p| !p.as_os_str().is_empty())
        };
        Env {
            home: var("HOME"),
            xdg_config_home: var("XDG_CONFIG_HOME"),
            xdg_data_home: var("XDG_DATA_HOME"),
            xdg_state_home: var("XDG_STATE_HOME"),
            appdata: var("APPDATA"),
            local_appdata: var("LOCALAPPDATA"),
            userprofile: var("USERPROFILE"),
        }
    }
}

/// The resolved, absolute directories for this install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppDirs {
    pub config_dir: PathBuf,
    pub skills_dir: PathBuf,
    pub outbox_dir: PathBuf,
    pub audit_dir: PathBuf,
}

impl AppDirs {
    /// The path to the TOML config file (`config_dir/config.toml`).
    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    /// The path to the audit log file (`audit_dir/audit.log`).
    pub fn audit_log(&self) -> PathBuf {
        self.audit_dir.join("audit.log")
    }

    /// Resolve directories from the live environment.
    pub fn from_system() -> Result<Self, AppDirsError> {
        Self::resolve(&Env::from_system())
    }

    /// Resolve directories from an explicit environment (used by tests).
    #[cfg(target_os = "linux")]
    pub fn resolve(env: &Env) -> Result<Self, AppDirsError> {
        let home = || {
            env.home
                .clone()
                .ok_or(AppDirsError::MissingVar("HOME", "home directory"))
        };
        let config_base = match &env.xdg_config_home {
            Some(p) => p.clone(),
            None => home()?.join(".config"),
        };
        let data_base = match &env.xdg_data_home {
            Some(p) => p.clone(),
            None => home()?.join(".local").join("share"),
        };
        let state_base = match &env.xdg_state_home {
            Some(p) => p.clone(),
            None => home()?.join(".local").join("state"),
        };
        Ok(AppDirs {
            config_dir: config_base.join(LINUX_APP_DIR),
            skills_dir: data_base.join(LINUX_APP_DIR).join("skills"),
            outbox_dir: home()?.join("Synaplan").join("out"),
            audit_dir: state_base.join(LINUX_APP_DIR),
        })
    }

    #[cfg(target_os = "macos")]
    pub fn resolve(env: &Env) -> Result<Self, AppDirsError> {
        let home = env
            .home
            .clone()
            .ok_or(AppDirsError::MissingVar("HOME", "home directory"))?;
        let app_support = home
            .join("Library")
            .join("Application Support")
            .join(MACOS_BUNDLE_ID);
        Ok(AppDirs {
            config_dir: app_support.clone(),
            skills_dir: app_support.join("skills"),
            outbox_dir: home.join("Synaplan").join("out"),
            audit_dir: home.join("Library").join("Logs").join(MACOS_BUNDLE_ID),
        })
    }

    #[cfg(target_os = "windows")]
    pub fn resolve(env: &Env) -> Result<Self, AppDirsError> {
        let appdata = env
            .appdata
            .clone()
            .ok_or(AppDirsError::MissingVar("APPDATA", "config directory"))?;
        let local = env
            .local_appdata
            .clone()
            .ok_or(AppDirsError::MissingVar("LOCALAPPDATA", "skills directory"))?;
        let profile = env
            .userprofile
            .clone()
            .ok_or(AppDirsError::MissingVar("USERPROFILE", "out-box directory"))?;
        let vendor = |base: PathBuf| base.join("Synaplan").join("Desktop");
        Ok(AppDirs {
            config_dir: vendor(appdata),
            skills_dir: vendor(local.clone()).join("skills"),
            outbox_dir: profile.join("Synaplan").join("out"),
            audit_dir: vendor(local).join("logs"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_paths_use_xdg_when_set() {
        let env = Env {
            xdg_config_home: Some(PathBuf::from("/tmp/xcfg")),
            xdg_data_home: Some(PathBuf::from("/tmp/xdata")),
            xdg_state_home: Some(PathBuf::from("/tmp/xstate")),
            home: Some(PathBuf::from("/home/anna")),
            ..Default::default()
        };
        let dirs = AppDirs::resolve(&env).unwrap();
        assert_eq!(dirs.config_dir, Path::new("/tmp/xcfg/synaplan-desktop"));
        assert_eq!(
            dirs.skills_dir,
            Path::new("/tmp/xdata/synaplan-desktop/skills")
        );
        assert_eq!(dirs.audit_dir, Path::new("/tmp/xstate/synaplan-desktop"));
        assert_eq!(dirs.outbox_dir, Path::new("/home/anna/Synaplan/out"));
        assert_eq!(
            dirs.config_file(),
            Path::new("/tmp/xcfg/synaplan-desktop/config.toml")
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_paths_fall_back_to_home_without_xdg() {
        let env = Env {
            home: Some(PathBuf::from("/home/anna")),
            ..Default::default()
        };
        let dirs = AppDirs::resolve(&env).unwrap();
        assert_eq!(
            dirs.config_dir,
            Path::new("/home/anna/.config/synaplan-desktop")
        );
        assert_eq!(
            dirs.skills_dir,
            Path::new("/home/anna/.local/share/synaplan-desktop/skills")
        );
        assert_eq!(
            dirs.audit_dir,
            Path::new("/home/anna/.local/state/synaplan-desktop")
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_errors_without_home_or_xdg() {
        let env = Env::default();
        assert!(AppDirs::resolve(&env).is_err());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_paths() {
        let env = Env {
            home: Some(PathBuf::from("/Users/anna")),
            ..Default::default()
        };
        let dirs = AppDirs::resolve(&env).unwrap();
        assert_eq!(
            dirs.config_dir,
            Path::new("/Users/anna/Library/Application Support/com.synaplan.desktop")
        );
        assert_eq!(
            dirs.skills_dir,
            Path::new("/Users/anna/Library/Application Support/com.synaplan.desktop/skills")
        );
        assert_eq!(
            dirs.audit_dir,
            Path::new("/Users/anna/Library/Logs/com.synaplan.desktop")
        );
        assert_eq!(dirs.outbox_dir, Path::new("/Users/anna/Synaplan/out"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_paths() {
        let env = Env {
            appdata: Some(PathBuf::from(r"C:\Users\anna\AppData\Roaming")),
            local_appdata: Some(PathBuf::from(r"C:\Users\anna\AppData\Local")),
            userprofile: Some(PathBuf::from(r"C:\Users\anna")),
            ..Default::default()
        };
        let dirs = AppDirs::resolve(&env).unwrap();
        assert_eq!(
            dirs.config_dir,
            Path::new(r"C:\Users\anna\AppData\Roaming\Synaplan\Desktop")
        );
        assert_eq!(
            dirs.skills_dir,
            Path::new(r"C:\Users\anna\AppData\Local\Synaplan\Desktop\skills")
        );
        assert_eq!(
            dirs.audit_dir,
            Path::new(r"C:\Users\anna\AppData\Local\Synaplan\Desktop\logs")
        );
        assert_eq!(dirs.outbox_dir, Path::new(r"C:\Users\anna\Synaplan\out"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_errors_without_appdata() {
        let env = Env {
            userprofile: Some(PathBuf::from(r"C:\Users\anna")),
            ..Default::default()
        };
        assert!(AppDirs::resolve(&env).is_err());
    }
}
