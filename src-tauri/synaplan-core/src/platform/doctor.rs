//! The "doctor" (DC16/DC26): discover the local tools skills rely on — Python,
//! Node.js, LibreOffice — per platform, probe them (run `--version` with a
//! timeout, so a hanging Store stub counts as missing), and report the resolved
//! absolute path + version. The resolved paths form the binary allowlist
//! ([`crate::tools`]); a bare name is never trusted because `PATH` is
//! attacker-influenced.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use crate::config::ToolsConfig;
use crate::platform::exec;
use crate::skills::RuntimeSnapshot;

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
pub fn is_store_stub(path: &Path) -> bool {
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

/// macOS `/usr/bin/python3` is a Command Line Tools stub that can pop an
/// installer dialog. Treat it as missing unless a real CLT/Xcode python exists.
pub fn is_macos_clt_shim(path: &Path) -> bool {
    if path != Path::new("/usr/bin/python3") {
        return false;
    }
    let clt = Path::new("/Library/Developer/CommandLineTools/usr/bin/python3");
    let xcode = Path::new("/Applications/Xcode.app/Contents/Developer/usr/bin/python3");
    !clt.is_file() && !xcode.is_file()
}

fn push_if_file(out: &mut Vec<PathBuf>, path: PathBuf) {
    if path.is_file() {
        out.push(path);
    }
}

fn detect_python_with(configured: Option<&Path>) -> Tool {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(p) = configured {
        push_if_file(&mut candidates, p.to_path_buf());
    }

    #[cfg(windows)]
    {
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
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            if let Ok(entries) =
                std::fs::read_dir(Path::new(&local).join("Programs").join("Python"))
            {
                for entry in entries.flatten() {
                    push_if_file(&mut candidates, entry.path().join("python.exe"));
                }
            }
        }
        for name in ["python3", "python"] {
            if let Some(p) = resolve_on_path(name) {
                candidates.push(p);
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        push_if_file(&mut candidates, PathBuf::from("/opt/homebrew/bin/python3"));
        push_if_file(&mut candidates, PathBuf::from("/usr/local/bin/python3"));
        for name in ["python3", "python"] {
            if let Some(p) = resolve_on_path(name) {
                candidates.push(p);
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        for name in ["python3", "python"] {
            if let Some(p) = resolve_on_path(name) {
                candidates.push(p);
            }
        }
    }

    for cand in candidates {
        if is_store_stub(&cand) || is_macos_clt_shim(&cand) {
            continue;
        }
        if let Some(version) = probe_version(&cand, &["--version"]) {
            let hint = if probe_version(&cand, &["-m", "venv", "--help"]).is_none() {
                "doctor.hintPythonVenv".into()
            } else {
                String::new()
            };
            return Tool {
                id: "python".into(),
                name: "Python".into(),
                found: true,
                path: Some(cand.to_string_lossy().to_string()),
                version: Some(version),
                hint,
            };
        }
    }

    Tool {
        id: "python".into(),
        name: "Python".into(),
        found: false,
        path: None,
        version: None,
        hint: python_hint_key(),
    }
}

fn detect_node_with(configured: Option<&Path>) -> Tool {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(p) = configured {
        push_if_file(&mut candidates, p.to_path_buf());
    }
    #[cfg(windows)]
    {
        push_if_file(
            &mut candidates,
            PathBuf::from(r"C:\Program Files\nodejs\node.exe"),
        );
    }
    #[cfg(target_os = "macos")]
    {
        push_if_file(&mut candidates, PathBuf::from("/opt/homebrew/bin/node"));
        push_if_file(&mut candidates, PathBuf::from("/usr/local/bin/node"));
    }
    if let Some(p) = resolve_on_path("node") {
        candidates.push(p);
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
        hint: "doctor.hintNode".into(),
    }
}

fn detect_libreoffice_with(configured: Option<&Path>) -> Tool {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(p) = configured {
        push_if_file(&mut candidates, p.to_path_buf());
    }
    #[cfg(windows)]
    {
        for p in [
            r"C:\Program Files\LibreOffice\program\soffice.com",
            r"C:\Program Files\LibreOffice\program\soffice.exe",
            r"C:\Program Files (x86)\LibreOffice\program\soffice.com",
            r"C:\Program Files (x86)\LibreOffice\program\soffice.exe",
        ] {
            push_if_file(&mut candidates, PathBuf::from(p));
        }
    }
    #[cfg(target_os = "macos")]
    {
        push_if_file(
            &mut candidates,
            PathBuf::from("/Applications/LibreOffice.app/Contents/MacOS/soffice"),
        );
    }
    #[cfg(target_os = "linux")]
    {
        for p in [
            "/usr/bin/soffice",
            "/usr/lib/libreoffice/program/soffice",
            "/var/lib/flatpak/exports/bin/soffice",
        ] {
            push_if_file(&mut candidates, PathBuf::from(p));
        }
        if let Ok(home) = std::env::var("HOME") {
            push_if_file(
                &mut candidates,
                PathBuf::from(home).join(".local/share/flatpak/exports/bin/soffice"),
            );
        }
    }
    if let Some(p) = resolve_on_path("soffice") {
        candidates.push(p);
    }
    for cand in candidates {
        if cand.is_file() {
            if let Some(version) = probe_version(&cand, &["--version"]) {
                let hint = if is_flatpak_libreoffice(&cand) {
                    "doctor.hintLibreofficeFlatpak".into()
                } else {
                    String::new()
                };
                return Tool {
                    id: "libreoffice".into(),
                    name: "LibreOffice".into(),
                    found: true,
                    path: Some(cand.to_string_lossy().to_string()),
                    version: Some(version),
                    hint,
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
        hint: "doctor.hintLibreoffice".into(),
    }
}

fn python_hint_key() -> String {
    #[cfg(windows)]
    {
        "doctor.hintPythonWindows".into()
    }
    #[cfg(target_os = "macos")]
    {
        "doctor.hintPythonMac".into()
    }
    #[cfg(target_os = "linux")]
    {
        "doctor.hintPythonLinux".into()
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        "doctor.hintPythonLinux".into()
    }
}

/// Detect all supported tools using optional user-configured paths.
pub fn detect_all_with(tools: &ToolsConfig) -> Vec<Tool> {
    let python = tools.python.as_deref().map(Path::new);
    let node = tools.node.as_deref().map(Path::new);
    let lo = tools.libreoffice.as_deref().map(Path::new);
    vec![
        detect_python_with(python),
        detect_node_with(node),
        detect_libreoffice_with(lo),
    ]
}

/// Detect all supported tools (no user-configured paths).
pub fn detect_all() -> Vec<Tool> {
    detect_all_with(&ToolsConfig::default())
}

/// The resolved absolute interpreter paths that form the binary allowlist.
pub fn allowlisted_programs() -> Vec<PathBuf> {
    allowlisted_programs_with(&ToolsConfig::default())
}

/// Like [`allowlisted_programs`], honouring optional configured paths.
pub fn allowlisted_programs_with(tools: &ToolsConfig) -> Vec<PathBuf> {
    detect_all_with(tools)
        .into_iter()
        .filter_map(|t| t.path.map(PathBuf::from))
        .collect()
}

/// True when the only LibreOffice we found is a Flatpak export.
pub fn is_flatpak_libreoffice(path: &Path) -> bool {
    let s = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    s.contains("/flatpak/") || s.contains("/app/libreoffice")
}

/// Probe whether `python` can `import module` (doctor only — not a skill run).
pub fn python_has_import(python: &Path, module: &str) -> bool {
    if !is_valid_module_name(module) {
        return false;
    }
    let code = format!("import {module}");
    probe_version(python, &["-c", &code]).is_some()
}

fn is_valid_module_name(module: &str) -> bool {
    !module.is_empty()
        && module
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}

/// Build the snapshot used to block skills whose runtime is missing.
pub fn runtime_snapshot(tools: &ToolsConfig, imports: &[String]) -> RuntimeSnapshot {
    let detected = detect_all_with(tools);
    let python = detected.iter().find(|t| t.id == "python");
    let node = detected.iter().find(|t| t.id == "node");
    let lo = detected.iter().find(|t| t.id == "libreoffice");
    let python_found = python.is_some_and(|t| t.found);
    let mut ok_imports = Vec::new();
    if python_found {
        if let Some(path) = python.and_then(|t| t.path.as_deref()) {
            let p = Path::new(path);
            for module in imports {
                if python_has_import(p, module) {
                    ok_imports.push(module.clone());
                }
            }
        }
    }
    RuntimeSnapshot {
        python: python_found,
        node: node.is_some_and(|t| t.found),
        libreoffice: lo.is_some_and(|t| t.found),
        python_imports: ok_imports,
    }
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
        let node = detect_node_with(None);
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
        let lo = detect_libreoffice_with(None);
        if !lo.found {
            assert_eq!(lo.hint, "doctor.hintLibreoffice");
        }
    }

    #[test]
    fn store_stub_classifier() {
        assert!(is_store_stub(Path::new(
            r"C:\Users\x\AppData\Local\Microsoft\WindowsApps\python.exe"
        )));
        assert!(!is_store_stub(Path::new(
            r"C:\Users\x\AppData\Local\Programs\Python\Python312\python.exe"
        )));
    }

    #[test]
    fn clt_shim_only_matches_usr_bin_python3() {
        assert!(!is_macos_clt_shim(Path::new("/opt/homebrew/bin/python3")));
    }

    #[test]
    fn rejects_invalid_import_names() {
        assert!(!is_valid_module_name("pptx;import os"));
        assert!(is_valid_module_name("pptx"));
    }

    #[test]
    fn flatpak_classifier_is_path_only() {
        assert!(is_flatpak_libreoffice(Path::new(
            "/var/lib/flatpak/exports/bin/soffice"
        )));
        assert!(is_flatpak_libreoffice(Path::new(
            "/home/x/.local/share/flatpak/exports/bin/soffice"
        )));
        assert!(!is_flatpak_libreoffice(Path::new("/usr/bin/soffice")));
    }

    #[test]
    fn silent_configured_binary_is_not_trusted() {
        let dir = tempfile::tempdir().unwrap();
        let fake = dir.path().join("python3");
        std::fs::write(&fake, b"").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&fake).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&fake, perms).unwrap();
        }
        // A file that does not answer `--version` must not win. PATH may still
        // supply a real interpreter afterwards — that is discovery, not trust
        // of the silent configured path.
        let tool = detect_python_with(Some(&fake));
        if let Some(path) = tool.path.as_deref() {
            assert_ne!(Path::new(path), fake.as_path());
        }
    }
}
