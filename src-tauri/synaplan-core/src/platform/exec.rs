//! Low-level process execution (cross-platform §6). This module is intentionally
//! *neutral*: it runs a program with an EXACT constructed environment, a working
//! directory, an output cap, and a timeout that kills the process (a process
//! group on Unix). The security policy (which programs may run, no shell, path
//! confinement) lives one layer up in [`crate::tools`].
//!
//! Nothing here ever constructs a shell — callers pass `{program, args[]}`.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Options for a single execution.
pub struct RunOptions {
    pub workdir: PathBuf,
    /// The COMPLETE environment (inheritance is cleared first).
    pub env: Vec<(String, String)>,
    pub timeout: Duration,
    pub max_output_bytes: usize,
}

/// The captured result of an execution.
#[derive(Debug, Clone)]
pub struct RunResult {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

/// Build the minimal, constructed environment a skill subprocess gets. It
/// deliberately excludes the Synaplan key, the pairing code, and injection
/// vectors (`LD_PRELOAD`, `DYLD_INSERT_LIBRARIES`, `PYTHONPATH`, `PYTHONSTARTUP`,
/// `NODE_OPTIONS`, `PSModulePath`). `PATH` is limited to the allowlisted tool
/// directories; `HOME`/`USERPROFILE`/`TMP` point at the run scratch dir.
pub fn base_env(tool_dirs: &[PathBuf], scratch: &Path) -> Vec<(String, String)> {
    let sep = if cfg!(windows) { ";" } else { ":" };
    let path = tool_dirs
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(sep);
    let scratch_s = scratch.to_string_lossy().to_string();

    let mut env = vec![
        ("PATH".to_string(), path),
        ("PYTHONIOENCODING".to_string(), "utf-8".to_string()),
        ("PYTHONUTF8".to_string(), "1".to_string()),
        ("LANG".to_string(), "C.UTF-8".to_string()),
        ("TMPDIR".to_string(), scratch_s.clone()),
    ];

    #[cfg(windows)]
    {
        env.push(("TEMP".to_string(), scratch_s.clone()));
        env.push(("TMP".to_string(), scratch_s.clone()));
        env.push(("USERPROFILE".to_string(), scratch_s.clone()));
        // The Windows loader and many tools need these to start at all.
        for key in [
            "SystemRoot",
            "SystemDrive",
            "windir",
            "NUMBER_OF_PROCESSORS",
            "PROCESSOR_ARCHITECTURE",
        ] {
            if let Some(v) = std::env::var_os(key) {
                env.push((key.to_string(), v.to_string_lossy().to_string()));
            }
        }
    }
    #[cfg(not(windows))]
    {
        env.push(("HOME".to_string(), scratch_s));
    }

    env
}

/// Run `program` with `args`. Never spawns a shell.
pub fn run(program: &Path, args: &[String], opts: &RunOptions) -> std::io::Result<RunResult> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(&opts.workdir)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in &opts.env {
        cmd.env(k, v);
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // New process group so a timeout can kill the whole tree via killpg.
        cmd.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = cmd.spawn()?;
    let pid = child.id();

    let max = opts.max_output_bytes;
    let mut out_pipe = child.stdout.take().expect("piped stdout");
    let mut err_pipe = child.stderr.take().expect("piped stderr");
    let out_handle = std::thread::spawn(move || read_capped(&mut out_pipe, max));
    let err_handle = std::thread::spawn(move || read_capped(&mut err_pipe, max));

    let start = Instant::now();
    let mut timed_out = false;
    let code = loop {
        match child.try_wait()? {
            Some(status) => break status.code(),
            None => {
                if start.elapsed() >= opts.timeout {
                    kill_tree(&mut child, pid);
                    timed_out = true;
                    let _ = child.wait();
                    break None;
                }
                std::thread::sleep(Duration::from_millis(30));
            }
        }
    };

    let stdout = out_handle.join().unwrap_or_default();
    let stderr = err_handle.join().unwrap_or_default();
    Ok(RunResult {
        code,
        stdout,
        stderr,
        timed_out,
    })
}

/// Read a stream, keeping at most `max` bytes but always draining so the child
/// never blocks on a full pipe.
fn read_capped(reader: &mut impl Read, max: usize) -> String {
    let mut kept: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                if kept.len() < max {
                    let take = (max - kept.len()).min(n);
                    kept.extend_from_slice(&chunk[..take]);
                }
                // else: discard, but keep reading to drain the pipe.
            }
            Err(_) => break,
        }
    }
    String::from_utf8_lossy(&kept).to_string()
}

#[cfg(unix)]
fn kill_tree(child: &mut Child, pid: u32) {
    // Kill the whole process group (negative pid), then the child as a fallback.
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
    }
    let _ = child.kill();
}

#[cfg(windows)]
fn kill_tree(child: &mut Child, _pid: u32) {
    // v1: terminate the child. Full Job Object tree-kill is a hardening follow-up.
    let _ = child.kill();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Find an executable on PATH (tests use `node`, present in CI).
    fn which(name: &str) -> Option<PathBuf> {
        let exe = if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.to_string()
        };
        std::env::var_os("PATH").and_then(|paths| {
            std::env::split_paths(&paths)
                .map(|d| d.join(&exe))
                .find(|p| p.is_file())
        })
    }

    fn opts(scratch: &Path, tool_dir: &Path, timeout_ms: u64) -> RunOptions {
        RunOptions {
            workdir: scratch.to_path_buf(),
            env: base_env(&[tool_dir.to_path_buf()], scratch),
            timeout: Duration::from_millis(timeout_ms),
            max_output_bytes: 1_000_000,
        }
    }

    #[test]
    fn runs_and_captures_stdout() {
        let node = match which("node") {
            Some(n) => n,
            None => return, // node not on PATH in this environment; skip
        };
        let dir = tempfile::tempdir().unwrap();
        let res = run(
            &node,
            &["-e".into(), "process.stdout.write('hello-exec')".into()],
            &opts(dir.path(), node.parent().unwrap(), 10_000),
        )
        .unwrap();
        assert_eq!(res.code, Some(0));
        assert!(res.stdout.contains("hello-exec"), "stdout: {}", res.stdout);
        assert!(!res.timed_out);
    }

    #[test]
    fn timeout_kills_process() {
        let node = match which("node") {
            Some(n) => n,
            None => return,
        };
        let dir = tempfile::tempdir().unwrap();
        let res = run(
            &node,
            &["-e".into(), "setTimeout(()=>{}, 60000)".into()],
            &opts(dir.path(), node.parent().unwrap(), 800),
        )
        .unwrap();
        assert!(res.timed_out, "expected timeout");
    }

    #[test]
    fn env_is_constructed_no_leaks() {
        let node = match which("node") {
            Some(n) => n,
            None => return,
        };
        let dir = tempfile::tempdir().unwrap();
        // Set a secret + an injection var in THIS process; they must not leak.
        std::env::set_var("SYNAPLAN_TEST_SECRET", "top-secret");
        std::env::set_var("NODE_OPTIONS", "--max-old-space-size=16");
        let res = run(
            &node,
            &[
                "-e".into(),
                "process.stdout.write(String(process.env.SYNAPLAN_TEST_SECRET||'')+'|'+String(process.env.PYTHONIOENCODING||''))".into(),
            ],
            &opts(dir.path(), node.parent().unwrap(), 10_000),
        )
        .unwrap();
        std::env::remove_var("SYNAPLAN_TEST_SECRET");
        std::env::remove_var("NODE_OPTIONS");
        // Secret absent; constructed var present.
        assert_eq!(
            res.stdout, "|utf-8",
            "env leaked or missing: {}",
            res.stdout
        );
    }

    #[test]
    fn output_is_capped() {
        let node = match which("node") {
            Some(n) => n,
            None => return,
        };
        let dir = tempfile::tempdir().unwrap();
        let mut o = opts(dir.path(), node.parent().unwrap(), 10_000);
        o.max_output_bytes = 100;
        let res = run(
            &node,
            &[
                "-e".into(),
                "process.stdout.write('x'.repeat(10000))".into(),
            ],
            &o,
        )
        .unwrap();
        assert!(res.stdout.len() <= 100, "not capped: {}", res.stdout.len());
    }
}
