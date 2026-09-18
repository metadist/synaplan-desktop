//! Reveal a file or folder in the OS file manager ("Show in folder").
//!
//! OS differences live here (master plan §0.3): a *file* is selected inside its
//! folder, a *folder* is opened directly. No shell is ever constructed (C12) —
//! a known file-manager binary is spawned with `{program, args[]}`. The naive
//! `open::that` opened a file in its default app (not its folder) and, from the
//! WSL dev launch, could fail silently on a mapped-drive working directory;
//! spawning the file manager directly with a safe working directory is reliable.

use std::path::Path;
use std::process::{Command, Stdio};

/// The file-manager program and arguments that reveal `path`. `is_file` selects
/// the item inside its folder; a directory is opened directly. Pure, so the
/// argument shape is unit-tested without spawning a process.
pub fn reveal_command(path: &Path, is_file: bool) -> (&'static str, Vec<String>) {
    let target = path.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        // `explorer /select,<file>` opens the folder with the file highlighted
        // and brings the window to the front; a folder path opens that folder.
        if is_file {
            return ("explorer.exe", vec![format!("/select,{target}")]);
        }
        ("explorer.exe", vec![target])
    }

    #[cfg(target_os = "macos")]
    {
        if is_file {
            return ("open", vec!["-R".to_string(), target]);
        }
        ("open", vec![target])
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // No portable "select in folder" on Linux; open the containing folder.
        let folder = if is_file {
            path.parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or(target)
        } else {
            target
        };
        ("xdg-open", vec![folder])
    }
}

/// Reveal `path` in the OS file manager. Errors when the path does not exist or
/// the file manager cannot be launched, so "Show in folder" is never a silent
/// no-op — the caller turns the error into a message.
pub fn reveal(path: &Path) -> std::io::Result<()> {
    // Fails with `NotFound` when the location is gone, before spawning anything.
    let meta = std::fs::metadata(path)?;
    let (program, args) = reveal_command(path, meta.is_file());

    let mut cmd = Command::new(program);
    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    // A mapped-network-drive working directory (the WSL dev launch) can break a
    // child launch on Windows; anchor the child in a folder that always exists.
    if let Some(parent) = path.parent() {
        if parent.is_dir() {
            cmd.current_dir(parent);
        }
    }

    // The file manager runs independently; its exit code is unreliable
    // (Explorer returns non-zero even on success), so we do not wait on it.
    cmd.spawn().map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_selects_a_file_and_opens_a_folder() {
        let (prog, args) = reveal_command(Path::new(r"C:\Users\u\out\report.docx"), true);
        assert_eq!(prog, "explorer.exe");
        assert_eq!(args, vec![r"/select,C:\Users\u\out\report.docx".to_string()]);

        let (prog, args) = reveal_command(Path::new(r"C:\Users\u\out"), false);
        assert_eq!(prog, "explorer.exe");
        assert_eq!(args, vec![r"C:\Users\u\out".to_string()]);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_reveals_a_file_and_opens_a_folder() {
        let (prog, args) = reveal_command(Path::new("/Users/u/out/report.docx"), true);
        assert_eq!(prog, "open");
        assert_eq!(args, vec!["-R".to_string(), "/Users/u/out/report.docx".to_string()]);

        let (prog, args) = reveal_command(Path::new("/Users/u/out"), false);
        assert_eq!(prog, "open");
        assert_eq!(args, vec!["/Users/u/out".to_string()]);
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn linux_opens_the_containing_folder() {
        // A file resolves to its folder (no portable select on Linux).
        let (prog, args) = reveal_command(Path::new("/home/u/out/report.docx"), true);
        assert_eq!(prog, "xdg-open");
        assert_eq!(args, vec!["/home/u/out".to_string()]);

        // A folder opens directly.
        let (prog, args) = reveal_command(Path::new("/home/u/out"), false);
        assert_eq!(prog, "xdg-open");
        assert_eq!(args, vec!["/home/u/out".to_string()]);
    }

    #[test]
    fn a_missing_path_is_an_error_not_a_silent_no_op() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope").join("gone.txt");
        let err = reveal(&missing).expect_err("missing path must error");
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }
}
