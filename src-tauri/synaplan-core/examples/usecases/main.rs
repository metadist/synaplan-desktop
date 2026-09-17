//! Use-case runner for Synaplan Desktop — the executable half of
//! `_extras/planning/20260917-desktop-use-case-test-plan/README.md`.
//!
//! It drives the **production core** (`messages::stream_chat`,
//! `agent::run_agent_turn`, `files::upload_project_file`, `dictation::*`,
//! `ProjectStore`) against a live, paired Synaplan workspace and records what a
//! person would have seen, plus the raw gateway contract next to it. Every
//! case has stable ID `UC-xx`; results land in `runs/<ts>.json` and
//! `STATUS.md` in the plan folder. The API key is never printed or written.
//!
//! ```text
//! cargo run -p synaplan-core --example usecases -- --quick
//! cargo run -p synaplan-core --example usecases -- --only UC-03,UC-05
//! cargo run -p synaplan-core --example usecases -- --pair admin@example.com 'secret'
//! ```

mod fixtures;
mod raw;
mod report;
mod uc_agent;
mod uc_chat;
mod uc_dictation;
mod uc_files;
mod uc_notes;
mod uc_web;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use synaplan_core::catalog::ModelCatalog;
use synaplan_core::projects::ids;

use report::{CaseResult, Report};

/// Six representative chat models: the workspace default family, the cheap
/// fast one, the reasoning one, and one each from Google, xAI, Mistral. The
/// `--full` run walks every available CHAT entry instead.
const QUICK_MODELS: &[&str] = &[
    "groq:openai/gpt-oss-120b:chat",
    "anthropic:claude-fable-5-1:chat",
    "openai:gpt-6-astra:chat",
    "google:gemini-3.5-flash:chat",
    "xai:grok-4.6:chat",
    "mistral:mistral-large-latest:chat",
];

const DEFAULT_CREDS: &str = "/tmp/synaplan-desktop-harness.creds";

/// One case: borrows the context, yields its result.
type CaseFn = fn(&Ctx) -> futures_util::future::BoxFuture<'_, CaseResult>;
const ALL_CASES: &[&str] = &[
    "UC-00", "UC-01", "UC-02", "UC-03", "UC-04", "UC-05", "UC-06", "UC-07", "UC-08", "UC-09",
    "UC-10",
];

/// Everything a case needs. Cloned into tasks; the key lives only here.
#[derive(Clone)]
pub struct Ctx {
    pub base: String,
    pub key: String,
    pub run_id: String,
    pub project_id: String,
    pub models: Vec<String>,
    pub catalog: Option<ModelCatalog>,
    pub work_dir: PathBuf,
    pub keep_files: bool,
    pub verbose: bool,
}

impl Ctx {
    /// The wire model id the desktop sends for a catalog key.
    pub fn wire(&self, catalog_key: &str) -> String {
        synaplan_core::projects::wire_model_id(catalog_key, None)
            .unwrap_or_else(|| catalog_key.to_string())
    }

    pub fn log(&self, line: &str) {
        println!("    {line}");
    }
}

struct Args {
    creds: PathBuf,
    report_dir: PathBuf,
    only: Option<BTreeSet<String>>,
    models: Option<Vec<String>>,
    full: bool,
    keep_files: bool,
    verbose: bool,
    pair: Option<(String, String, String)>,
    help: bool,
}

fn usage() -> &'static str {
    "usecases — Synaplan Desktop use-case runner\n\n\
     --creds <file>        KEY=VALUE file with DESKTOP_KEY and API_BASE_URL (default /tmp/synaplan-desktop-harness.creds)\n\
     --report-dir <dir>    plan folder to write runs/ and STATUS.md into (default: this repo's plan folder)\n\
     --only UC-01,UC-05    run only these cases\n\
     --models a,b          catalog keys for the model matrices (default: the quick set)\n\
     --quick               six representative chat models (default)\n\
     --full                every available CHAT model in the catalog\n\
     --keep-files          leave uploaded harness files in the workspace\n\
     --verbose             print answers and tool steps\n\
     --pair <email> <password> [name]   pair a harness computer and write the creds file, then exit\n\
     --help\n\n\
     Environment overrides: SYNAPLAN_DESKTOP_KEY, SYNAPLAN_BASE_URL."
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        creds: PathBuf::from(DEFAULT_CREDS),
        report_dir: default_report_dir(),
        only: None,
        models: None,
        full: false,
        keep_files: false,
        verbose: false,
        pair: None,
        help: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--creds" => args.creds = PathBuf::from(it.next().ok_or("--creds needs a path")?),
            "--report-dir" => {
                args.report_dir = PathBuf::from(it.next().ok_or("--report-dir needs a path")?)
            }
            "--only" => {
                let list = it.next().ok_or("--only needs a list")?;
                let set: BTreeSet<String> = list
                    .split(',')
                    .map(|s| s.trim().to_uppercase())
                    .filter(|s| !s.is_empty())
                    .collect();
                if let Some(unknown) = set.iter().find(|id| !ALL_CASES.contains(&id.as_str())) {
                    return Err(format!(
                        "unknown case {unknown}; known: {}",
                        ALL_CASES.join(", ")
                    ));
                }
                args.only = Some(set);
            }
            "--models" => {
                let list = it.next().ok_or("--models needs a list")?;
                args.models = Some(
                    list.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                );
            }
            "--quick" => args.full = false,
            "--full" => args.full = true,
            "--keep-files" => args.keep_files = true,
            "--verbose" | "-v" => args.verbose = true,
            "--pair" => {
                let email = it.next().ok_or("--pair needs <email> <password>")?;
                let password = it.next().ok_or("--pair needs <email> <password>")?;
                let name = it
                    .next()
                    .unwrap_or_else(|| "Use-case harness (WSL)".to_string());
                args.pair = Some((email, password, name));
            }
            "--help" | "-h" => args.help = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(args)
}

/// `<repo>/_extras/planning/20260917-desktop-use-case-test-plan`, derived from
/// the crate location so the runner works from any cwd.
fn default_report_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../_extras/planning/20260917-desktop-use-case-test-plan")
}

fn load_creds(path: &Path) -> Result<(String, String), String> {
    let mut key = std::env::var("SYNAPLAN_DESKTOP_KEY").ok();
    let mut base = std::env::var("SYNAPLAN_BASE_URL").ok();
    if let Ok(text) = std::fs::read_to_string(path) {
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            match k.trim() {
                "DESKTOP_KEY" if key.is_none() => key = Some(v.trim().to_string()),
                "API_BASE_URL" if base.is_none() => base = Some(v.trim().to_string()),
                _ => {}
            }
        }
    }
    let key = key.filter(|k| !k.is_empty()).ok_or_else(|| {
        format!(
            "no DESKTOP_KEY in {} and SYNAPLAN_DESKTOP_KEY unset — run with --pair first",
            path.display()
        )
    })?;
    let base = base
        .filter(|b| !b.is_empty())
        .unwrap_or_else(|| "http://localhost:8000".to_string());
    Ok((base.trim_end_matches('/').to_string(), key))
}

fn git_sha() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn selected(only: &Option<BTreeSet<String>>, id: &str) -> bool {
    match only {
        Some(set) => set.contains(id),
        None => true,
    }
}

/// A short run id usable inside planted facts (letters + digits only).
fn short_run_id() -> String {
    let id = ids::new_id();
    id.chars()
        .rev()
        .take(5)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

#[tokio::main]
async fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}\n\n{}", usage());
            std::process::exit(2);
        }
    };
    if args.help {
        println!("{}", usage());
        return;
    }
    if let Some((email, password, name)) = &args.pair {
        let base = std::env::var("SYNAPLAN_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8000".to_string());
        match raw::pair(&base, email, password, name, &args.creds).await {
            Ok(device) => println!(
                "paired device {device} for {email}; credentials in {}",
                args.creds.display()
            ),
            Err(e) => {
                eprintln!("pairing failed: {e}");
                std::process::exit(1);
            }
        }
        return;
    }

    let (base, key) = match load_creds(&args.creds) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    };

    let run_id = short_run_id();
    let work_dir = std::env::temp_dir().join(format!("synaplan-usecases-{run_id}"));
    let _ = std::fs::create_dir_all(&work_dir);
    let started = Instant::now();
    let started_at = ids::now_iso8601();
    println!("Synaplan Desktop use cases — run {run_id} against {base}");
    println!("work dir: {}", work_dir.display());

    let mut ctx = Ctx {
        base: base.clone(),
        key,
        run_id: run_id.clone(),
        project_id: format!("HARNESS{}", ids::new_id()),
        models: Vec::new(),
        catalog: None,
        work_dir,
        keep_files: args.keep_files,
        verbose: args.verbose,
    };

    // UC-00 always runs first: it decides the model list for the matrices.
    let mut results: Vec<CaseResult> = Vec::new();
    let (preflight, cat) = uc_chat::uc00_preflight(&ctx).await;
    ctx.catalog = cat;
    ctx.models = pick_models(&ctx, args.models.clone(), args.full);
    println!(
        "  models for matrices ({}): {}",
        ctx.models.len(),
        ctx.models.join(", ")
    );
    if selected(&args.only, "UC-00") {
        results.push(preflight);
    }

    let plan: Vec<(&str, CaseFn)> = vec![
        ("UC-01", |c| Box::pin(uc_chat::uc01_model_matrix(c))),
        ("UC-02", |c| Box::pin(uc_agent::uc02_write_file(c))),
        ("UC-03", |c| Box::pin(uc_web::uc03_web_search_plain(c))),
        ("UC-04", |c| Box::pin(uc_agent::uc04_research_to_file(c))),
        ("UC-05", |c| Box::pin(uc_files::uc05_single_file(c))),
        ("UC-06", |c| Box::pin(uc_files::uc06_en_bloc(c))),
        ("UC-07", |c| Box::pin(uc_dictation::uc07_dictation(c))),
        ("UC-08", |c| Box::pin(uc_notes::uc08_notes(c))),
        ("UC-09", |c| Box::pin(uc_chat::uc09_long_answer(c))),
        ("UC-10", |c| Box::pin(uc_chat::uc10_error_copy(c))),
    ];
    for (id, run) in plan {
        if !selected(&args.only, id) {
            continue;
        }
        println!("\n== {id}");
        let result = run(&ctx).await;
        println!(
            "   -> {} in {:.1}s{}",
            result.outcome,
            result.duration_ms as f64 / 1000.0,
            if result.detail.is_empty() {
                String::new()
            } else {
                format!(" — {}", result.detail)
            }
        );
        results.push(result);
    }

    if !ctx.keep_files {
        uc_files::cleanup(&ctx).await;
    }

    let flags: Vec<String> = std::env::args().skip(1).collect();
    let report = Report::new(
        started_at,
        ids::now_iso8601(),
        base,
        git_sha(),
        flags.join(" "),
        results,
    );
    match report.write(&args.report_dir) {
        Ok(path) => println!("\nreport: {}", path.display()),
        Err(e) => eprintln!("\ncould not write report: {e}"),
    }
    let (pass, fail, skip) = report.counts();
    println!(
        "{pass} passed, {fail} failed, {skip} skipped in {:.0}s",
        started.elapsed().as_secs_f64()
    );
    if fail > 0 {
        std::process::exit(1);
    }
}

/// `--models` wins; `--full` = every available CHAT entry; else the quick set
/// filtered to what the catalog actually offers as available.
fn pick_models(ctx: &Ctx, explicit: Option<Vec<String>>, full: bool) -> Vec<String> {
    if let Some(list) = explicit {
        return list;
    }
    let Some(cat) = &ctx.catalog else {
        return QUICK_MODELS.iter().map(|s| s.to_string()).collect();
    };
    let available: Vec<String> = cat
        .slots
        .get("chat")
        .into_iter()
        .flatten()
        .filter(|e| e.available)
        .map(|e| e.id.clone())
        .collect();
    if full {
        return available;
    }
    let quick: Vec<String> = QUICK_MODELS
        .iter()
        .filter(|m| available.iter().any(|a| a == *m))
        .map(|s| s.to_string())
        .collect();
    if quick.is_empty() {
        available.into_iter().take(6).collect()
    } else {
        quick
    }
}

/// Ensure `list` contains the workspace default chat model first (when known).
pub fn with_default_first(ctx: &Ctx, mut list: Vec<String>) -> Vec<String> {
    if let Some(default) = ctx
        .catalog
        .as_ref()
        .and_then(|c| c.defaults.get("CHAT"))
        .cloned()
    {
        list.retain(|m| m != &default);
        list.insert(0, default);
    }
    list
}
