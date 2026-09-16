//! Opt-in debug log: one plain-text line per event, appended to a file in the
//! logs folder so a person (or support) can see *what the app did* — which
//! skill ran, which files were read and written, which turn failed and why.
//!
//! Off by default. Never contains the API key, message text or file contents:
//! callers pass paths, names, counts and error messages only, and every line
//! still goes through [`redact`] as a belt-and-braces guard.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// File name inside the logs folder.
pub const FILE_NAME: &str = "desktop-debug.log";
/// The previous generation after a rotation.
pub const ROTATED_FILE_NAME: &str = "desktop-debug.1.log";
/// Rotate when the live file grows past this (bytes).
pub const MAX_BYTES: u64 = 5 * 1024 * 1024;

/// Shared, cheaply clonable handle. `enabled` flips at runtime from Settings.
#[derive(Debug, Clone)]
pub struct DebugLog {
    path: PathBuf,
    enabled: Arc<AtomicBool>,
    write_lock: Arc<Mutex<()>>,
}

impl DebugLog {
    pub fn new(logs_dir: &Path, enabled: bool) -> Self {
        Self {
            path: logs_dir.join(FILE_NAME),
            enabled: Arc::new(AtomicBool::new(enabled)),
            write_lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Append one line: `<utc timestamp> <scope> <message>`. A no-op when the
    /// log is off. I/O errors are swallowed on purpose — a logging failure must
    /// never break the feature being logged.
    pub fn log(&self, scope: &str, message: &str) {
        if !self.is_enabled() {
            return;
        }
        let line = format!(
            "{} {:<7} {}\n",
            timestamp(),
            scope,
            redact(&message.replace(['\n', '\r'], " "))
        );
        let Ok(_guard) = self.write_lock.lock() else {
            return;
        };
        let _ = self.rotate_if_needed();
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = file.write_all(line.as_bytes());
        }
    }

    fn rotate_if_needed(&self) -> std::io::Result<()> {
        let size = match std::fs::metadata(&self.path) {
            Ok(meta) => meta.len(),
            Err(_) => return Ok(()),
        };
        if size < MAX_BYTES {
            return Ok(());
        }
        let rotated = self.path.with_file_name(ROTATED_FILE_NAME);
        let _ = std::fs::remove_file(&rotated);
        std::fs::rename(&self.path, rotated)
    }
}

/// Mask anything that looks like a Synaplan API key (`sk_` + hex) so a stray
/// error message can never leak it into the log.
pub fn redact(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find("sk_") {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + 3..];
        let hex_len = after.chars().take_while(|c| c.is_ascii_hexdigit()).count();
        if hex_len >= 16 {
            out.push_str("sk_…");
            rest = &after[hex_len..];
        } else {
            out.push_str("sk_");
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

/// `YYYY-MM-DDTHH:MM:SSZ` from the system clock, without a date crate.
fn timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    crate::projects::ids::iso8601_from_unix(secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_lines_only_when_enabled() {
        let dir = tempfile::tempdir().unwrap();
        let log = DebugLog::new(dir.path(), false);
        log.log("agent", "turn start");
        assert!(!log.path().exists(), "disabled log writes nothing");

        log.set_enabled(true);
        log.log("agent", "turn start project=p1 model=m");
        log.log("tool", "write_file ok path=/x/y.txt");
        let text = std::fs::read_to_string(log.path()).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("agent   turn start project=p1 model=m"));
        assert!(lines[0].ends_with('Z') || lines[0].contains("Z "));
        assert!(lines[1].contains("tool    write_file ok"));
    }

    #[test]
    fn redacts_api_keys_and_flattens_newlines() {
        let dir = tempfile::tempdir().unwrap();
        let log = DebugLog::new(dir.path(), true);
        log.log(
            "chat",
            "error: header x-api-key: sk_0123456789abcdef0123456789abcdef\nsecond line",
        );
        let text = std::fs::read_to_string(log.path()).unwrap();
        assert!(!text.contains("0123456789abcdef"));
        assert!(text.contains("sk_…"));
        assert_eq!(text.lines().count(), 1);
        assert_eq!(
            redact("sk_short and sk_0123456789abcdef01"),
            "sk_short and sk_…"
        );
    }

    #[test]
    fn rotates_when_the_file_is_large() {
        let dir = tempfile::tempdir().unwrap();
        let log = DebugLog::new(dir.path(), true);
        std::fs::write(log.path(), vec![b'x'; MAX_BYTES as usize + 1]).unwrap();
        log.log("agent", "after rotation");
        assert!(dir.path().join(ROTATED_FILE_NAME).is_file());
        let text = std::fs::read_to_string(log.path()).unwrap();
        assert_eq!(text.lines().count(), 1);
    }
}
