//! The skill tool dispatch (`Read` / `Write` / `Bash`) — the security policy that
//! sits on top of [`crate::platform::exec`]. It enforces, in order:
//!
//! - **No shell, ever.** The `Bash` command is tokenized with a strict splitter
//!   that rejects every shell metacharacter (`| & ; > < ` `` ` `` $( ${` newline).
//!   No `sh -c`, `cmd /c`, `powershell -Command` is ever constructed (C12).
//! - **Allowlisted interpreters only.** `program` must resolve to a doctor-found
//!   interpreter (Python/Node/LibreOffice) by absolute path; shells, script
//!   hosts, `curl`/`wget`/`ssh` are denied even by name.
//! - **No model-authored code.** Inline-code flags (`-c`, `-e`, `--eval`, `-`)
//!   are refused, so an interpreter can only run a **script that lives inside an
//!   installed skill** — reviewed code, not something the model made up.
//! - **Confined paths.** `Read`/`Write` and every path-like argument go through
//!   [`Confinement`]; the working directory is always the run scratch dir.

use std::path::{Path, PathBuf};
use std::time::Duration;

use thiserror::Error;

use crate::platform::confinement::{Access, Confinement, ConfinementError};
use crate::platform::exec;

/// Programs that are denied even if somehow present on the allowlist path — a
/// defense-in-depth backstop against shells and network tools.
pub const DENIED_PROGRAMS: &[&str] = &[
    "sh",
    "bash",
    "zsh",
    "dash",
    "ksh",
    "fish",
    "cmd",
    "powershell",
    "pwsh",
    "wscript",
    "cscript",
    "mshta",
    "rundll32",
    "regsvr32",
    "osascript",
    "env",
    "curl",
    "wget",
    "ssh",
    "scp",
    "sftp",
    "nc",
    "ncat",
    "telnet",
    "bash.exe",
    "cmd.exe",
    "powershell.exe",
    "curl.exe",
];

/// Interpreter flags that would run model-authored inline code — always refused.
const INLINE_CODE_FLAGS: &[&str] = &["-c", "-e", "--eval", "--exec", "-", "-i"];

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ToolError {
    #[error("path is not allowed: {0}")]
    Confinement(String),
    #[error("file is too large")]
    TooLarge,
    #[error("io error: {0}")]
    Io(String),
    #[error("the command contains a shell metacharacter")]
    ShellMetachar,
    #[error("empty command")]
    EmptyCommand,
    #[error("that program is not an allowed interpreter")]
    ProgramNotAllowed,
    #[error("running inline code is not allowed; run a skill's script file instead")]
    InlineCodeDenied,
    #[error("the script must be a file inside an installed skill")]
    ScriptNotInSkill,
    #[error("execution error: {0}")]
    Exec(String),
}

impl From<ConfinementError> for ToolError {
    fn from(e: ConfinementError) -> Self {
        ToolError::Confinement(e.to_string())
    }
}

/// Everything a tool call needs, assembled by the caller (never by the model).
pub struct ToolPolicy {
    pub confinement: Confinement,
    /// Absolute, doctor-resolved interpreter paths (Python/Node/LibreOffice).
    pub allow_programs: Vec<PathBuf>,
    /// Scripts must live under one of these (installed skill directories).
    pub skills_dir: PathBuf,
    /// Working directory for every run (a scratch dir inside the out-box).
    pub run_scratch: PathBuf,
    /// Directories put on the child's PATH.
    pub tool_dirs: Vec<PathBuf>,
    pub timeout: Duration,
    pub max_output_bytes: usize,
    pub max_file_bytes: u64,
}

/// Split a command string into argv, quote-aware, rejecting shell metacharacters.
pub fn tokenize(command: &str) -> Result<Vec<String>, ToolError> {
    if command.contains("$(") || command.contains("${") {
        return Err(ToolError::ShellMetachar);
    }
    const META: &[char] = &[
        '|', '&', ';', '<', '>', '`', '\n', '\r', '$', '(', ')', '*', '?', '~',
    ];

    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut has_token = false;
    let mut in_single = false;
    let mut in_double = false;

    for c in command.chars() {
        if in_single {
            if c == '\'' {
                in_single = false;
            } else {
                cur.push(c);
            }
            continue;
        }
        if in_double {
            if c == '"' {
                in_double = false;
            } else {
                cur.push(c);
            }
            continue;
        }
        match c {
            '\'' => {
                in_single = true;
                has_token = true;
            }
            '"' => {
                in_double = true;
                has_token = true;
            }
            ' ' | '\t' => {
                if has_token {
                    tokens.push(std::mem::take(&mut cur));
                    has_token = false;
                }
            }
            m if META.contains(&m) => return Err(ToolError::ShellMetachar),
            other => {
                cur.push(other);
                has_token = true;
            }
        }
    }
    if in_single || in_double {
        return Err(ToolError::ShellMetachar);
    }
    if has_token {
        tokens.push(cur);
    }
    if tokens.is_empty() {
        return Err(ToolError::EmptyCommand);
    }
    Ok(tokens)
}

/// Read a file (confined, size-capped).
pub fn tool_read(policy: &ToolPolicy, path: &str) -> Result<String, ToolError> {
    let resolved = policy.confinement.resolve(path, Access::Read)?;
    let meta = std::fs::metadata(&resolved).map_err(|e| ToolError::Io(e.to_string()))?;
    if meta.len() > policy.max_file_bytes {
        return Err(ToolError::TooLarge);
    }
    let bytes = std::fs::read(&resolved).map_err(|e| ToolError::Io(e.to_string()))?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

/// Write a file (confined to write roots, size-capped). Returns the path.
pub fn tool_write(policy: &ToolPolicy, path: &str, contents: &str) -> Result<String, ToolError> {
    if contents.len() as u64 > policy.max_file_bytes {
        return Err(ToolError::TooLarge);
    }
    let resolved = policy.confinement.resolve(path, Access::Write)?;
    if let Some(parent) = resolved.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ToolError::Io(e.to_string()))?;
    }
    std::fs::write(&resolved, contents).map_err(|e| ToolError::Io(e.to_string()))?;
    Ok(resolved.to_string_lossy().to_string())
}

fn basename_stem(token: &str) -> String {
    Path::new(token)
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn looks_like_path(token: &str) -> bool {
    token.contains('/')
        || token.contains('\\')
        || Path::new(token).is_absolute()
        || Path::new(token).exists()
}

fn within(dir: &Path, candidate: &Path) -> bool {
    match (std::fs::canonicalize(dir), std::fs::canonicalize(candidate)) {
        (Ok(d), Ok(c)) => c.starts_with(d),
        _ => false,
    }
}

/// Resolve the program token to an allowlisted interpreter path.
fn resolve_program(policy: &ToolPolicy, token: &str) -> Result<PathBuf, ToolError> {
    let denied = {
        let base = Path::new(token)
            .file_name()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_else(|| token.to_lowercase());
        DENIED_PROGRAMS.iter().any(|d| *d == base)
    };
    if denied {
        return Err(ToolError::ProgramNotAllowed);
    }

    // A path token must canonicalize to an allowlist entry.
    if looks_like_path(token) {
        let canon = std::fs::canonicalize(token).map_err(|_| ToolError::ProgramNotAllowed)?;
        if policy.allow_programs.iter().any(|p| {
            std::fs::canonicalize(p)
                .map(|c| c == canon)
                .unwrap_or(false)
        }) {
            return Ok(canon);
        }
        return Err(ToolError::ProgramNotAllowed);
    }

    // A bare name matches an allowlist entry by file stem (python3 -> python…).
    let stem = token.to_lowercase();
    for prog in &policy.allow_programs {
        if basename_stem(&prog.to_string_lossy()) == stem
            || prog
                .file_name()
                .map(|f| f.to_string_lossy().to_lowercase() == stem)
                .unwrap_or(false)
        {
            return Ok(prog.clone());
        }
    }
    Err(ToolError::ProgramNotAllowed)
}

/// Run a `Bash` tool call: an allowlisted interpreter running an installed
/// skill's script (or a converter like LibreOffice), never a shell.
pub fn tool_bash(policy: &ToolPolicy, command: &str) -> Result<exec::RunResult, ToolError> {
    let tokens = tokenize(command)?;
    let program = resolve_program(policy, &tokens[0])?;
    let args = &tokens[1..];

    // No model-authored inline code.
    if args.iter().any(|a| INLINE_CODE_FLAGS.contains(&a.as_str())) {
        return Err(ToolError::InlineCodeDenied);
    }

    let is_libreoffice = basename_stem(&program.to_string_lossy()).starts_with("soffice");

    // For interpreters (python/node), require a script that lives inside an
    // installed skill. LibreOffice is a converter and takes no script.
    if !is_libreoffice {
        let script = args
            .iter()
            .find(|a| !a.starts_with('-'))
            .ok_or(ToolError::ScriptNotInSkill)?;
        let script_path = std::fs::canonicalize(script).map_err(|_| ToolError::ScriptNotInSkill)?;
        if !within(&policy.skills_dir, &script_path) {
            return Err(ToolError::ScriptNotInSkill);
        }
    }

    // Every path-like argument must be inside the read/write allowlist.
    for arg in args {
        if arg.starts_with('-') || !looks_like_path(arg) {
            continue;
        }
        let ok = policy.confinement.resolve(arg, Access::Read).is_ok()
            || policy.confinement.resolve(arg, Access::Write).is_ok();
        if !ok {
            return Err(ToolError::Confinement(arg.clone()));
        }
    }

    let opts = exec::RunOptions {
        workdir: policy.run_scratch.clone(),
        env: exec::base_env(&policy.tool_dirs, &policy.run_scratch),
        timeout: policy.timeout,
        max_output_bytes: policy.max_output_bytes,
    };
    exec::run(&program, args, &opts).map_err(|e| ToolError::Exec(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::confinement::default_deny_globs;
    use crate::platform::doctor;

    #[test]
    fn tokenize_rejects_metacharacters() {
        for bad in [
            "a | b", "a && b", "a; b", "a > f", "$(x)", "a `b`", "a\nb", "a & b", "${x}",
        ] {
            assert!(
                matches!(tokenize(bad), Err(ToolError::ShellMetachar)),
                "should reject: {bad}"
            );
        }
    }

    #[test]
    fn tokenize_handles_quotes() {
        assert_eq!(
            tokenize(r#"python3 script.py "hello world" 'a b'"#).unwrap(),
            vec!["python3", "script.py", "hello world", "a b"]
        );
    }

    fn policy(skills_dir: &Path, scratch: &Path, programs: Vec<PathBuf>) -> ToolPolicy {
        let read = vec![skills_dir.to_path_buf(), scratch.to_path_buf()];
        let write = vec![scratch.to_path_buf()];
        let confinement = Confinement::new(&read, &write, &default_deny_globs()).unwrap();
        let tool_dirs = programs
            .iter()
            .filter_map(|p| p.parent().map(Path::to_path_buf))
            .collect();
        ToolPolicy {
            confinement,
            allow_programs: programs,
            skills_dir: skills_dir.to_path_buf(),
            run_scratch: scratch.to_path_buf(),
            tool_dirs,
            timeout: Duration::from_secs(10),
            max_output_bytes: 1_000_000,
            max_file_bytes: 10_000_000,
        }
    }

    #[test]
    fn denies_shells_and_network_tools() {
        let dir = tempfile::tempdir().unwrap();
        let skills = std::fs::canonicalize(dir.path()).unwrap();
        let p = policy(&skills, &skills, vec![]);
        for cmd in [
            "sh script.sh",
            "bash -c x",
            "curl http://x",
            "powershell -Command x",
            "cmd /c dir",
        ] {
            let err = tool_bash(&p, cmd).unwrap_err();
            assert!(
                matches!(
                    err,
                    ToolError::ProgramNotAllowed
                        | ToolError::ShellMetachar
                        | ToolError::InlineCodeDenied
                ),
                "cmd {cmd} -> {err:?}"
            );
        }
    }

    #[test]
    fn denies_inline_code_and_out_of_skill_scripts() {
        let node = match doctor::resolve_on_path("node") {
            Some(n) => n,
            None => return,
        };
        let dir = tempfile::tempdir().unwrap();
        let skills = std::fs::canonicalize(dir.path()).unwrap();
        let p = policy(&skills, &skills, vec![node.clone()]);
        // inline code (paren-free so it passes the tokenizer and hits the flag check)
        assert_eq!(
            tool_bash(&p, "node -e process.exit").unwrap_err(),
            ToolError::InlineCodeDenied
        );
        // a script outside the skills dir
        let outside = std::env::temp_dir().join("evil.js");
        std::fs::write(&outside, "console.log('x')").unwrap();
        let cmd = format!("node {}", outside.to_string_lossy());
        assert_eq!(
            tool_bash(&p, &cmd).unwrap_err(),
            ToolError::ScriptNotInSkill
        );
    }

    #[test]
    fn runs_a_skill_script_via_node() {
        let node = match doctor::resolve_on_path("node") {
            Some(n) => n,
            None => return,
        };
        let dir = tempfile::tempdir().unwrap();
        let skills = std::fs::canonicalize(dir.path()).unwrap();
        let script = skills.join("hello.js");
        std::fs::write(&script, "process.stdout.write('skill-ran')").unwrap();
        let p = policy(&skills, &skills, vec![node]);
        let cmd = format!("node {}", script.to_string_lossy());
        let res = tool_bash(&p, &cmd).unwrap();
        assert_eq!(res.code, Some(0));
        assert!(res.stdout.contains("skill-ran"), "stdout: {}", res.stdout);
    }

    #[test]
    fn read_write_are_confined() {
        let dir = tempfile::tempdir().unwrap();
        let scratch = std::fs::canonicalize(dir.path()).unwrap();
        let p = policy(&scratch, &scratch, vec![]);
        let target = scratch.join("note.txt");
        let written = tool_write(&p, target.to_str().unwrap(), "hi").unwrap();
        assert!(written.contains("note.txt"));
        assert_eq!(tool_read(&p, target.to_str().unwrap()).unwrap(), "hi");
        // outside write root
        let outside = std::env::temp_dir().join("nope.txt");
        assert!(matches!(
            tool_write(&p, outside.to_str().unwrap(), "x"),
            Err(ToolError::Confinement(_))
        ));
    }
}
