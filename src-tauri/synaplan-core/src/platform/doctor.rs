//! The "doctor" (DC16/DC26): discover the local tools skills rely on — Python,
//! Node.js, LibreOffice — per platform, probe them (run `--version` with a
//! timeout, so a hanging Store stub counts as missing), and report the resolved
//! absolute path + version. The resolved paths form the binary allowlist
//! ([`crate::tools`]); a bare name is never trusted because `PATH` is
//! attacker-influenced.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use crate::platform::exec;

/// A detected (or missing) tool.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub id: String,
    pub name: String,
    pub found: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub hint: String,
}

/// Resolve an executable on `PATH`, honouring `PATHEXT` on Windows.
pub fn resolve_on_path(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        let direct = dir.join(name);
        if direct.is_file() {
            return Some(direct);
        }
        #[cfg(windows)]
        {
            let exts = std::env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".into());
            for ext in exts.split(';') {
                let ext = ext.trim();
                if ext.is_empty() {
                    continue;
                }
                let cand = dir.join(format!("{name}{}", ext.to_lowercase()));
                if cand.is_file() {
                    return Some(cand);
                }
                let cand_upper = dir.join(format!("{name}{ext}"));
                if cand_upper.is_file() {
                    return Some(cand_upper);
                }
            }
        }
    }
    None
}

/// True for the Microsoft Store `python.exe` placeholder that opens the Store.
fn is_store_stub(path: &Path) -> bool {
    let s = path.to_string_lossy().to_lowercase();
    s.contains("\\windowsapps\\") && s.ends_with("python.exe")
}

/// Probe a program's version line with a short timeout.
fn probe_version(program: &Path, args: &[&str]) -> Option<String> {
    let scratch = std::env::temp_dir();
    let tool_dir = program
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| scratch.clone());
    let opts = exec::RunOptions {
        workdir: scratch.clone(),
        env: exec::base_env(&[tool_dir], &scratch),
        timeout: Duration::from_secs(6),
        max_output_bytes: 8192,
    };
    let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let res = exec::run(program, &args_owned, &opts).ok()?;
    if res.timed_out {
        return None;
    }
    let text = if res.stdout.trim().is_empty() {
        res.stderr
    } else {
        res.stdout
    };
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(str::to_string)
}

fn detect_python() -> Tool {
    let mut candidates: Vec<PathBuf> = Vec::new();

    #[cfg(windows)]
    {
        // Prefer the `py -3` launcher's real interpreter, if present.
        if let Some(py) = resolve_on_path("py") {
            if let Some(exe) = probe_version(
                &py,
                &["-3", "-c", "import sys;sys.stdout.write(sys.executable)"],
            ) {
                let p = PathBuf::from(exe.trim());
                if p.is_file() {
                    candidates.push(p);
                }
            }
        }
        for name in ["python3", "python"] {
            if let Some(p) = resolve_on_path(name) {
                candidates.push(p);
            }
        }
    }
    #[cfg(not(windows))]
    {
        for name in ["python3", "python"] {
            if let Some(p) = resolve_on_path(name) {
                candidates.push(p);
            }
        }
    }

    for cand in candidates {
        if is_store_stub(&cand) {
            continue;
        }
        if let Some(version) = probe_version(&cand, &["--version"]) {
            return Tool {
                id: "python".into(),
                name: "Python".into(),
                found: true,
                path: Some(cand.to_string_lossy().to_string()),
                version: Some(version),
                hint: String::new(),
            };
        }
    }

    Tool {
        id: "python".into(),
        name: "Python".into(),
        found: false,
        path: None,
        version: None,
        hint: python_hint(),
    }
}

fn detect_node() -> Tool {
    #[allow(unused_mut)]
    let mut candidates: Vec<PathBuf> = resolve_on_path("node").into_iter().collect();
    #[cfg(windows)]
    {
        candidates.push(PathBuf::from(r"C:\Program Files\nodejs\node.exe"));
    }
    for cand in candidates {
        if cand.is_file() {
            if let Some(version) = probe_version(&cand, &["--version"]) {
                return Tool {
                    id: "node".into(),
                    name: "Node.js".into(),
                    found: true,
                    path: Some(cand.to_string_lossy().to_string()),
                    version: Some(version),
                    hint: String::new(),
                };
            }
        }
    }
    Tool {
        id: "node".into(),
        name: "Node.js".into(),
        found: false,
        path: None,
        version: None,
        hint: "Install Node.js from nodejs.org.".into(),
    }
}

fn detect_libreoffice() -> Tool {
    let mut candidates: Vec<PathBuf> = Vec::new();
    #[cfg(windows)]
    {
        candidates.push(PathBuf::from(
            r"C:\Program Files\LibreOffice\program\soffice.com",
        ));
        candidates.push(PathBuf::from(
            r"C:\Program Files (x86)\LibreOffice\program\soffice.com",
        ));
    }
    #[cfg(target_os = "macos")]
    {
        candidates.push(PathBuf::from(
            "/Applications/LibreOffice.app/Contents/MacOS/soffice",
        ));
    }
    if let Some(p) = resolve_on_path("soffice") {
        candidates.push(p);
    }
    for cand in candidates {
        if cand.is_file() {
            if let Some(version) = probe_version(&cand, &["--version"]) {
                return Tool {
                    id: "libreoffice".into(),
                    name: "LibreOffice".into(),
                    found: true,
                    path: Some(cand.to_string_lossy().to_string()),
                    version: Some(version),
                    hint: String::new(),
                };
            }
        }
    }
    Tool {
        id: "libreoffice".into(),
        name: "LibreOffice".into(),
        found: false,
        path: None,
        version: None,
        hint: "Install LibreOffice from libreoffice.org for document conversion skills.".into(),
    }
}

#[cfg(windows)]
fn python_hint() -> String {
    "Windows may show a Python placeholder; install Python from python.org or the Microsoft Store."
        .into()
}
#[cfg(target_os = "macos")]
fn python_hint() -> String {
    "Install Python 3 from python.org or Homebrew (brew install python).".into()
}
#[cfg(target_os = "linux")]
fn python_hint() -> String {
    "Install python3 with your distribution's package manager (and python3-venv).".into()
}

/// Detect all supported tools.
pub fn detect_all() -> Vec<Tool> {
    vec![detect_python(), detect_node(), detect_libreoffice()]
}

/// The resolved absolute interpreter paths that form the binary allowlist.
pub fn allowlisted_programs() -> Vec<PathBuf> {
    detect_all()
        .into_iter()
        .filter_map(|t| t.path.map(PathBuf::from))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_node_on_path() {
        // node is present in CI (setup-node); if not, skip.
        if let Some(p) = resolve_on_path("node") {
            assert!(p.is_file());
            let v = probe_version(&p, &["--version"]);
            assert!(v.is_some(), "node --version should probe");
        }
    }

    #[test]
    fn detects_node_tool() {
        let node = detect_node();
        if resolve_on_path("node").is_some() {
            assert!(node.found);
            assert!(node.version.is_some());
        } else {
            assert!(!node.found);
            assert!(!node.hint.is_empty());
        }
    }

    #[test]
    fn missing_tool_reports_hint() {
        let lo = detect_libreoffice();
        if !lo.found {
            assert!(!lo.hint.is_empty());
        }
    }
}
