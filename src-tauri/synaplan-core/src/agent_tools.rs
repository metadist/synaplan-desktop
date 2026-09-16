//! The agent's tool surface, shared by the interactive chat turn and the
//! unattended job runner: the confined tool policy, the three client tools
//! (`read_file`, `write_file`, `run_program`), the system prompt that lists the
//! enabled skills, and the dispatcher that executes one call. Tauri-free so the
//! same code is unit-tested here and reused by any harness.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{json, Value};

use crate::agent::{AgentTool, ToolDispatchResult};
use crate::filesystem::FilesystemPolicy;
use crate::platform::confinement::Confinement;
use crate::skills::Skill;
use crate::tools::{self, ToolPolicy};

/// Appended to the system prompt when the project allows web search.
pub const WEB_SEARCH_PROMPT: &str = "\nWEB: This project allows web search. Use the web_search tool for current facts, news and sources; prefer recent, reputable pages and cite the URL of every source you use, both in your answer and inside the files you create.\n";

/// One-line, content-free description of a tool call for the debug log:
/// paths and program names, never file contents.
pub fn tool_log_line(name: &str, input: &Value) -> String {
    match name {
        "list_files" | "read_file" | "write_file" => format!(
            "{name} path={}",
            input.get("path").and_then(Value::as_str).unwrap_or("")
        ),
        "run_program" => {
            let command = input.get("command").and_then(Value::as_str).unwrap_or("");
            format!("{name} {}", command_shape(command))
        }
        other => other.to_string(),
    }
}

/// `program=… script=… args=N` for a command line: the program, the skill
/// script when the first argument is one, and only the *count* of the rest —
/// argument values carry the user's text (subjects, titles, body text).
fn command_shape(command: &str) -> String {
    // The same tokenizer the runner uses, so quoted arguments count once; a
    // command it rejects is still described, just by whitespace.
    let words: Vec<String> = tools::tokenize(command)
        .unwrap_or_else(|_| command.split_whitespace().map(str::to_string).collect());
    // Last path component on either separator: the log must not depend on
    // which OS wrote the command line.
    let base = |w: &str| w.rsplit(['/', '\\']).next().unwrap_or(w).to_string();
    let program = words.first().map(|w| base(w)).unwrap_or_default();
    let script = words
        .get(1)
        .filter(|w| w.ends_with(".py") || w.ends_with(".js") || w.ends_with(".mjs"))
        .map(|w| base(w));
    let rest = words
        .len()
        .saturating_sub(1 + usize::from(script.is_some()));
    match script {
        Some(s) => format!("program={program} script={s} args={rest}"),
        None => format!("program={program} args={rest}"),
    }
}

/// Build the confined tool policy: read = user folders + the skills dir + the
/// out-box; write = the out-box; workdir = the out-box.
pub fn build_tool_policy(
    fs_policy: &FilesystemPolicy,
    skills_dir: &Path,
    outbox: &Path,
    programs: Vec<PathBuf>,
) -> Result<ToolPolicy, String> {
    let mut read: Vec<PathBuf> = fs_policy.read.iter().map(PathBuf::from).collect();
    read.push(skills_dir.to_path_buf());
    read.push(outbox.to_path_buf());
    let write: Vec<PathBuf> = fs_policy.write.iter().map(PathBuf::from).collect();
    let confinement =
        Confinement::new(&read, &write, &fs_policy.deny).map_err(|e| e.to_string())?;
    let tool_dirs: Vec<PathBuf> = programs
        .iter()
        .filter_map(|p| p.parent().map(Path::to_path_buf))
        .collect();
    Ok(ToolPolicy {
        confinement,
        allow_programs: programs,
        skills_dir: skills_dir.to_path_buf(),
        run_scratch: outbox.to_path_buf(),
        tool_dirs,
        timeout: Duration::from_secs(120),
        max_output_bytes: 200_000,
        max_file_bytes: fs_policy.max_file_bytes,
    })
}

pub fn list_files_tool() -> AgentTool {
    AgentTool::client(
        "list_files",
        "List the files and sub-folders in a folder you may read: the project folder, the out-box, the skills folder, or a folder the user added. Use it first whenever the request mentions files whose exact names you do not know — never guess a file name.",
        json!({
            "type": "object",
            "properties": { "path": { "type": "string", "description": "Full path of the folder." } },
            "required": ["path"]
        }),
    )
}

pub fn read_file_tool() -> AgentTool {
    AgentTool::client(
        "read_file",
        "Read a UTF-8 text file the user allowed (a skill file, the project folder, or a folder they added). Returns the file contents. Use the full SKILL.md path listed for the skill, or a path under the skills folder (for example vcard/SKILL.md). Do not pass only the file name SKILL.md. Office files (.docx/.xlsx/.pptx) are binary: convert them first with the matching skill's `read` command, then read the result.",
        json!({
            "type": "object",
            "properties": { "path": { "type": "string", "description": "Full path, or a path relative to the skills folder such as vcard/SKILL.md." } },
            "required": ["path"]
        }),
    )
}

pub fn write_file_tool() -> AgentTool {
    AgentTool::client(
        "write_file",
        "Write a text file into the out-box folder. Use this for text/markdown results and for the Markdown/JSON inputs a skill script needs. Returns the saved path.",
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Absolute path inside the out-box." },
                "content": { "type": "string", "description": "The file contents." }
            },
            "required": ["path", "content"]
        }),
    )
}

pub fn run_program_tool() -> AgentTool {
    AgentTool::client(
        "run_program",
        "Run an installed skill's script with an allowlisted interpreter (Python/Node) or LibreOffice. Provide a single command line: the interpreter, the skill's script path, then arguments. No shell features (no pipes, redirects, &&, inline -c/-e code). Write outputs into the out-box.",
        json!({
            "type": "object",
            "properties": { "command": { "type": "string", "description": "e.g. python3 /path/to/skill/script.py <outfile> <args>" } },
            "required": ["command"]
        }),
    )
}

pub fn build_system_prompt(
    skills: &[Skill],
    skills_dir: &Path,
    outbox: &Path,
    read_roots: &[String],
    allow_exec: bool,
) -> String {
    let mut s = String::new();
    s.push_str("You are Synaplan Desktop, a local assistant that can create files on this computer using installed skills. Be concise and friendly.\n\n");
    s.push_str(&format!(
        "OUT-BOX (write all results here): {}\n",
        outbox.display()
    ));
    s.push_str(&format!("SKILLS FOLDER: {}\n", skills_dir.display()));
    if !read_roots.is_empty() {
        s.push_str(&format!("READABLE FOLDERS: {}\n", read_roots.join(", ")));
    }
    s.push('\n');
    if skills.is_empty() {
        s.push_str("No skills are enabled. You can still write text files into the out-box with write_file.\n");
    } else {
        s.push_str("ENABLED SKILLS:\n");
        for skill in skills {
            let folder = Path::new(&skill.dir);
            let folder = if folder.as_os_str().is_empty() {
                skills_dir.join(&skill.name)
            } else {
                folder.to_path_buf()
            };
            s.push_str(&format!(
                "- {} — {}\n  SKILL.md: {}\n  folder: {}\n",
                skill.name,
                skill.description,
                folder.join("SKILL.md").display(),
                folder.display()
            ));
        }
    }
    s.push('\n');
    s.push_str("HOW TO WORK:\n");
    s.push_str("0. When the request mentions files or a folder, list_files that folder first and use the exact names you see. Never guess a file name.\n");
    s.push_str("1. If a skill fits the request, read_file the SKILL.md path listed above (the full path, never just the file name).\n");
    if allow_exec {
        s.push_str("2. Run the skill's script with run_program (interpreter + the script path inside the skill folder + arguments). Write outputs into the out-box.\n");
        s.push_str(
            "3. If no skill fits, you may still produce text/markdown results with write_file.\n",
        );
    } else {
        s.push_str("2. Program execution is not enabled, so produce results as text/markdown files with write_file into the out-box.\n");
    }
    s.push_str("Never invent paths. Only write inside the out-box. Keep each write_file under a few thousand words; split big inputs into several files. When finished, tell the user what you created and where.\n");
    s
}

pub fn tool_start_summary(name: &str, input: &Value) -> String {
    match name {
        "list_files" => format!(
            "Listing {}",
            short_path(input.get("path").and_then(Value::as_str).unwrap_or(""))
        ),
        "read_file" => format!(
            "Reading {}",
            short_path(input.get("path").and_then(Value::as_str).unwrap_or(""))
        ),
        "write_file" => format!(
            "Writing {}",
            short_path(input.get("path").and_then(Value::as_str).unwrap_or(""))
        ),
        "run_program" => format!(
            "Running {}",
            program_name(input.get("command").and_then(Value::as_str).unwrap_or(""))
        ),
        other => format!("Using {other}"),
    }
}

/// Execute a single tool call against the confined policy.
pub fn dispatch_tool(
    policy: &ToolPolicy,
    outbox: &Path,
    name: &str,
    input: &Value,
) -> ToolDispatchResult {
    match name {
        "list_files" => {
            let path = input.get("path").and_then(Value::as_str).unwrap_or("");
            match tools::tool_list(policy, path) {
                Ok(entries) => {
                    let mut content = String::new();
                    if entries.is_empty() {
                        content.push_str("(empty folder)");
                    }
                    for e in &entries {
                        if e.is_dir {
                            content.push_str(&format!("{}/\n", e.name));
                        } else {
                            content.push_str(&format!("{}\t{} bytes\n", e.name, e.size));
                        }
                    }
                    if entries.len() >= tools::LIST_CAP {
                        content.push_str("(list cut off — narrow the folder)\n");
                    }
                    ToolDispatchResult {
                        content,
                        is_error: false,
                        summary: format!("Listed {} ({} entries)", short_path(path), entries.len()),
                        artifact: None,
                    }
                }
                Err(e) => error_result(
                    &format!(
                        "{e}. Only the readable folders listed in the instructions can be listed."
                    ),
                    format!("Could not list {}", short_path(path)),
                ),
            }
        }
        "read_file" => {
            let path = input.get("path").and_then(Value::as_str).unwrap_or("");
            match tools::tool_read(policy, path) {
                Ok(text) => ToolDispatchResult {
                    content: truncate(&text, 60_000),
                    is_error: false,
                    summary: format!("Read {}", short_path(path)),
                    artifact: None,
                },
                Err(tools::ToolError::Confinement(detail))
                    if detail.contains("could not be resolved") =>
                {
                    error_result(
                        &format!(
                            "file not found: {path}. Use list_files on its folder to see the real names."
                        ),
                        format!("File not found: {}", short_path(path)),
                    )
                }
                Err(e) => error_result(
                    &e.to_string(),
                    format!("Could not read {}: {e}", short_path(path)),
                ),
            }
        }
        "write_file" => {
            let path = input.get("path").and_then(Value::as_str).unwrap_or("");
            let content = input
                .get("content")
                .or_else(|| input.get("contents"))
                .and_then(Value::as_str)
                .unwrap_or("");
            match tools::tool_write(policy, path, content) {
                Ok(saved) => ToolDispatchResult {
                    content: format!("Saved file: {saved}"),
                    is_error: false,
                    summary: format!("Saved {}", file_name(&saved)),
                    artifact: Some(saved),
                },
                Err(e) => error_result(
                    &e.to_string(),
                    format!("Could not write {}", short_path(path)),
                ),
            }
        }
        "run_program" => {
            let command = input.get("command").and_then(Value::as_str).unwrap_or("");
            let before = snapshot_files(outbox);
            match tools::tool_bash(policy, command) {
                Ok(run) => {
                    let created = files_written_since(&before, outbox);
                    let ok = run.code == Some(0) && !run.timed_out;
                    let code = run
                        .code
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "killed".to_string());
                    let mut content = String::new();
                    if run.timed_out {
                        content.push_str("The program was stopped after the time limit.\n");
                    }
                    content.push_str(&format!("exit_code: {code}\n"));
                    content.push_str(&format!("stdout:\n{}\n", truncate(&run.stdout, 16_000)));
                    if !run.stderr.trim().is_empty() {
                        content.push_str(&format!("stderr:\n{}\n", truncate(&run.stderr, 6_000)));
                    }
                    if !created.is_empty() {
                        content.push_str(&format!("written_files: {}\n", created.join(", ")));
                    }
                    let summary = if ok {
                        format!("Ran {}", program_name(command))
                    } else {
                        format!("{} exited with {}", program_name(command), code)
                    };
                    ToolDispatchResult {
                        content,
                        is_error: !ok,
                        summary,
                        artifact: created.into_iter().next(),
                    }
                }
                Err(tools::ToolError::Confinement(arg)) => error_result(
                    &format!(
                        "path not allowed or does not exist: {arg}. Use list_files on the readable folders to find the real file name; results must go into the out-box."
                    ),
                    format!("Path not allowed or not found: {}", short_path(&arg)),
                ),
                Err(tools::ToolError::ScriptNotInSkill) => error_result(
                    "only a script that ships inside an installed skill folder may run — your own scripts in the out-box are never executed. Use the skills' run.py commands from their SKILL.md, do the arithmetic yourself, and pass data to a skill through a JSON/CSV/Markdown file written with write_file.",
                    "Own scripts cannot run — use a skill's run.py".to_string(),
                ),
                Err(tools::ToolError::InlineCodeDenied) => error_result(
                    "inline code (-c / -e) is not allowed. Use a skill's run.py with file arguments.",
                    "Inline code is not allowed".to_string(),
                ),
                Err(tools::ToolError::ProgramNotAllowed) => error_result(
                    "that program is not on this computer's allowlist. Only the interpreters the doctor found (python/node/soffice) can run, and never a shell.",
                    "Program not allowed".to_string(),
                ),
                Err(e) => {
                    error_result(&e.to_string(), "Program blocked by the sandbox".to_string())
                }
            }
        }
        other => error_result(
            &format!("unknown tool {other}"),
            format!("Unknown tool {other}"),
        ),
    }
}

fn error_result(detail: &str, summary: String) -> ToolDispatchResult {
    ToolDispatchResult {
        content: format!("Error: {detail}"),
        is_error: true,
        summary,
        artifact: None,
    }
}

/// What a file looked like at snapshot time; a rewrite changes it even when
/// the name stays the same.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileStamp {
    len: u64,
    modified: Option<std::time::SystemTime>,
}

/// The files under `dir` (recursive, bounded) with their size and mtime.
pub type FileSnapshot = HashMap<String, FileStamp>;

/// Snapshot the out-box so [`files_written_since`] can name every file a step
/// created *or overwrote* — a rerun that rewrites `report.docx` counts too.
pub fn snapshot_files(dir: &Path) -> FileSnapshot {
    let mut out = HashMap::new();
    collect_files(dir, &mut out, 0);
    out
}

/// Sorted paths that are new or changed compared to `before`.
pub fn files_written_since(before: &FileSnapshot, dir: &Path) -> Vec<String> {
    let mut written: Vec<String> = snapshot_files(dir)
        .into_iter()
        .filter(|(path, stamp)| before.get(path) != Some(stamp))
        .map(|(path, _)| path)
        .collect();
    written.sort();
    written
}

fn collect_files(dir: &Path, out: &mut FileSnapshot, depth: usize) {
    if depth > 6 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out, depth + 1);
        } else if let Ok(meta) = entry.metadata() {
            out.insert(
                path.to_string_lossy().to_string(),
                FileStamp {
                    len: meta.len(),
                    modified: meta.modified().ok(),
                },
            );
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}… (truncated)", &s[..end])
}

fn short_path(path: &str) -> String {
    file_name(path)
}

fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

fn program_name(command: &str) -> String {
    command
        .split_whitespace()
        .next()
        .map(file_name)
        .unwrap_or_else(|| "program".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_program_log_line_carries_no_argument_values() {
        let input = json!({
            "command": "C:\\Python312\\python.exe C:\\skills\\email-draft\\run.py --to team@nordlicht.example --subject \"Week 37 sales briefing\" --body-file body.md"
        });
        let line = tool_log_line("run_program", &input);
        assert_eq!(line, "run_program program=python.exe script=run.py args=6");
        assert!(!line.contains("nordlicht"));
        assert!(!line.contains("briefing"));
    }

    #[test]
    fn a_rewritten_file_counts_as_written() {
        let dir = tempfile::tempdir().unwrap();
        let report = dir.path().join("report.docx");
        std::fs::write(&report, b"v1").unwrap();
        let before = snapshot_files(dir.path());

        // Untouched: nothing to publish.
        assert!(files_written_since(&before, dir.path()).is_empty());

        // Same name, new content: it is written again.
        std::fs::write(&report, b"version two").unwrap();
        std::fs::write(dir.path().join("new.xlsx"), b"x").unwrap();
        let written = files_written_since(&before, dir.path());
        assert_eq!(written.len(), 2);
        assert!(written.iter().any(|p| p.ends_with("report.docx")));
        assert!(written.iter().any(|p| p.ends_with("new.xlsx")));
    }

    #[test]
    fn log_line_without_a_script_counts_every_argument() {
        let input = json!({ "command": "/usr/bin/soffice --headless --convert-to pdf memo.docx" });
        assert_eq!(
            tool_log_line("run_program", &input),
            "run_program program=soffice args=4"
        );
        assert_eq!(
            tool_log_line("read_file", &json!({ "path": "/home/u/Synaplan/out/a.md" })),
            "read_file path=/home/u/Synaplan/out/a.md"
        );
    }
}
