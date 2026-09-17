//! UC-02 agent turn that writes a file, UC-04 online research into a file.
//! Same loop as the Tauri `send_agent_chat` command: `build_system_prompt`,
//! the three file tools, `build_tool_policy` confinement, `run_agent_turn`
//! with `max_tokens 8192`, `dispatch_tool` on this computer.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use serde_json::{json, Value};
use synaplan_core::agent::{run_agent_turn, web_search_tool, AgentEvent, AgentTool};
use synaplan_core::agent_tools::{
    build_system_prompt, build_tool_policy, dispatch_tool, list_files_tool, read_file_tool,
    write_file_tool, WEB_SEARCH_PROMPT,
};
use synaplan_core::filesystem::FilesystemPolicy;
use synaplan_core::messages::{ChatError, TurnContext};

use crate::report::{snippet, Case, CaseResult};
use crate::Ctx;

/// What one agent turn did, as the run-activity UI would have shown it.
#[derive(Debug, Default)]
pub struct AgentRun {
    pub text: String,
    pub steps: Vec<String>,
    pub tool_rounds: usize,
    pub server_tool_retries: usize,
    pub error: Option<ChatError>,
    pub done: bool,
    pub total_ms: u64,
    pub outbox: PathBuf,
}

fn outbox_for(ctx: &Ctx, label: &str, model: &str) -> PathBuf {
    let safe: String = model
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    ctx.work_dir.join(format!("{label}-{safe}")).join("out")
}

/// Run one agentic turn with the desktop's tool surface (exec off) in a
/// scratch out-box.
pub async fn agent_turn(ctx: &Ctx, model: &str, label: &str, prompt: &str, web: bool) -> AgentRun {
    let outbox = outbox_for(ctx, label, model);
    let skills_dir = ctx.work_dir.join("skills");
    let _ = std::fs::create_dir_all(&outbox);
    let _ = std::fs::create_dir_all(&skills_dir);
    let mut run = AgentRun {
        outbox: outbox.clone(),
        ..AgentRun::default()
    };

    let mut fs_policy = FilesystemPolicy::default();
    fs_policy.ensure_outbox(&outbox);
    let policy = match build_tool_policy(&fs_policy, &skills_dir, &outbox, Vec::new()) {
        Ok(p) => p,
        Err(e) => {
            run.error = Some(ChatError::Server(format!("tool policy: {e}")));
            return run;
        }
    };
    let mut system = build_system_prompt(&[], &skills_dir, &outbox, &fs_policy.read, false);
    let mut tools: Vec<AgentTool> = vec![list_files_tool(), read_file_tool(), write_file_tool()];
    if web {
        tools.push(web_search_tool());
        system.push_str(WEB_SEARCH_PROMPT);
    }

    let turn = TurnContext::model(ctx.wire(model));
    let messages = vec![json!({ "role": "user", "content": prompt })];
    let cancel = AtomicBool::new(false);
    let started = Instant::now();
    let verbose = ctx.verbose;
    let result = run_agent_turn(
        &ctx.base,
        &ctx.key,
        &turn,
        &system,
        messages,
        &tools,
        &cancel,
        |name, input| dispatch_tool(&policy, &outbox, name, input),
        |event| match event {
            AgentEvent::Text(t) => run.text.push_str(&t),
            AgentEvent::ToolStart { name, input } => {
                run.tool_rounds += 1;
                let line = format!("{name} {}", snippet(&describe_input(&input), 80));
                if verbose {
                    println!("      → {line}");
                }
                run.steps.push(line);
            }
            AgentEvent::ToolEnd { name, result } => {
                if name == "web_search" && result.is_error {
                    run.server_tool_retries += 1;
                }
                if verbose {
                    println!(
                        "      ← {name} {} {}",
                        if result.is_error { "error" } else { "ok" },
                        result.summary
                    );
                }
            }
            AgentEvent::Done | AgentEvent::Cancelled => run.done = true,
        },
    )
    .await;
    if let Err(e) = result {
        run.error = Some(e);
    }
    run.total_ms = started.elapsed().as_millis() as u64;
    run
}

fn describe_input(input: &Value) -> String {
    match input {
        Value::Object(map) => map
            .iter()
            .map(|(k, v)| match v {
                Value::String(s) => format!("{k}={}", snippet(s, 40)),
                other => format!("{k}={other}"),
            })
            .collect::<Vec<_>>()
            .join(" "),
        other => other.to_string(),
    }
}

fn read_out(outbox: &Path, name: &str) -> Option<String> {
    let direct = outbox.join(name);
    if let Ok(s) = std::fs::read_to_string(&direct) {
        return Some(s);
    }
    // The model sometimes nests a folder; accept the file anywhere under out/.
    let mut stack = vec![outbox.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.file_name().is_some_and(|f| f == name) {
                return std::fs::read_to_string(p).ok();
            }
        }
    }
    None
}

/// UC-02: write `hello.md` into the out-box on every matrix model.
pub async fn uc02_write_file(ctx: &Ctx) -> CaseResult {
    let mut case = Case::new("UC-02", "Ask the assistant to write a file");
    let models = crate::with_default_first(ctx, ctx.models.clone());
    let mut good = 0usize;
    for model in &models {
        let run = agent_turn(
            ctx,
            model,
            "uc02",
            "Create a file named hello.md in the out-box that contains exactly the line: HELLO FROM SYNAPLAN. Then tell me the path.",
            false,
        )
        .await;
        let content = read_out(&run.outbox, "hello.md").unwrap_or_default();
        let wrote = content.to_uppercase().contains("HELLO FROM SYNAPLAN");
        let detail = match &run.error {
            Some(e) => format!(
                "error shown: \"{e}\" ({}) after {} steps",
                e.code(),
                run.tool_rounds
            ),
            None => format!(
                "{} tool steps, {} ms, file {}, reply \"{}\"",
                run.tool_rounds,
                run.total_ms,
                if wrote { "written" } else { "MISSING" },
                snippet(&run.text, 60)
            ),
        };
        let ok = case.check(
            format!("{model}: hello.md written and turn ends in text"),
            run.error.is_none()
                && wrote
                && run.done
                && !run.text.trim().is_empty()
                && run.tool_rounds <= 6,
            detail,
        );
        if ok {
            good += 1;
        }
        case.metric(format!("steps {model}"), run.tool_rounds as u64);
        case.metric(format!("total_ms {model}"), run.total_ms);
    }
    case.finish(format!("{good}/{} models wrote the file", models.len()))
}

/// UC-04: web research written to `sources.md` (three models).
pub async fn uc04_research_to_file(ctx: &Ctx) -> CaseResult {
    let mut case = Case::new("UC-04", "Online research into a file");
    let models: Vec<String> = crate::with_default_first(ctx, ctx.models.clone())
        .into_iter()
        .take(3)
        .collect();
    let mut good = 0usize;
    for model in &models {
        let run = agent_turn(
            ctx,
            model,
            "uc04",
            "Search the web for the current LTS version of Node.js and write a file sources.md into the out-box with the version number and at least three source URLs as full https:// links, one per line. Do not write the file until those links are in the contents. Then summarise in one sentence.",
            true,
        )
        .await;
        let content = read_out(&run.outbox, "sources.md").unwrap_or_default();
        let urls = content.lines().filter(|l| l.contains("http")).count();
        let detail = match &run.error {
            Some(e) => format!(
                "error shown: \"{e}\" ({}) after {} steps",
                e.code(),
                run.tool_rounds
            ),
            None => format!(
                "{} steps ({} web_search retry hints), {} URLs in sources.md, {} ms, reply \"{}\"",
                run.tool_rounds,
                run.server_tool_retries,
                urls,
                run.total_ms,
                snippet(&run.text, 60)
            ),
        };
        let ok = case.check(
            format!("{model}: sources.md with ≥3 URLs"),
            run.error.is_none() && urls >= 3 && run.done && !run.text.trim().is_empty(),
            detail,
        );
        if run.server_tool_retries > 2 {
            case.check(
                format!("{model}: mixed server/client steps stay rare"),
                false,
                format!(
                    "{} \"call web_search alone\" retries in one turn",
                    run.server_tool_retries
                ),
            );
        }
        if ok {
            good += 1;
        }
        case.metric(format!("steps {model}"), run.tool_rounds as u64);
        case.metric(format!("urls {model}"), urls as u64);
        case.metric(format!("total_ms {model}"), run.total_ms);
    }
    case.finish(format!(
        "{good}/{} models produced sources.md",
        models.len()
    ))
}
