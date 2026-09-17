//! Results → `runs/<ts>.json`, `runs/latest.json`, and the human ledger
//! `STATUS.md` (latest table rewritten, history row appended). Nothing here
//! ever receives the API key.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Outcome {
    Pass,
    Fail,
    Skip,
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Outcome::Pass => "PASS",
            Outcome::Fail => "FAIL",
            Outcome::Skip => "SKIP",
        })
    }
}

/// One assertion with the evidence a reader needs to believe it.
#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub id: String,
    pub name: String,
    pub outcome: Outcome,
    pub duration_ms: u64,
    pub detail: String,
    pub checks: Vec<Check>,
    pub metrics: BTreeMap<String, Value>,
}

/// Builder used by every case: `check(...)` accumulates, `finish()` decides.
pub struct Case {
    id: &'static str,
    name: &'static str,
    started: Instant,
    checks: Vec<Check>,
    metrics: BTreeMap<String, Value>,
    skipped: Option<String>,
}

impl Case {
    pub fn new(id: &'static str, name: &'static str) -> Self {
        Self {
            id,
            name,
            started: Instant::now(),
            checks: Vec::new(),
            metrics: BTreeMap::new(),
            skipped: None,
        }
    }

    pub fn check(&mut self, name: impl Into<String>, ok: bool, detail: impl Into<String>) -> bool {
        let name = name.into();
        let detail = detail.into();
        println!(
            "    [{}] {}{}",
            if ok { "ok" } else { "FAIL" },
            name,
            if detail.is_empty() {
                String::new()
            } else {
                format!(" — {detail}")
            }
        );
        self.checks.push(Check { name, ok, detail });
        ok
    }

    pub fn metric(&mut self, name: impl Into<String>, value: impl Into<Value>) {
        self.metrics.insert(name.into(), value.into());
    }

    pub fn skip(&mut self, reason: impl Into<String>) {
        self.skipped = Some(reason.into());
    }

    pub fn finish(self, detail: impl Into<String>) -> CaseResult {
        let outcome = if self.skipped.is_some() {
            Outcome::Skip
        } else if self.checks.iter().all(|c| c.ok) {
            Outcome::Pass
        } else {
            Outcome::Fail
        };
        let mut detail: String = detail.into();
        if let Some(reason) = self.skipped {
            detail = if detail.is_empty() {
                reason
            } else {
                format!("{detail} ({reason})")
            };
        }
        CaseResult {
            id: self.id.to_string(),
            name: self.name.to_string(),
            outcome,
            duration_ms: self.started.elapsed().as_millis() as u64,
            detail,
            checks: self.checks,
            metrics: self.metrics,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub runner: &'static str,
    pub started_at: String,
    pub finished_at: String,
    pub target: String,
    pub git_sha: String,
    pub flags: String,
    pub cases: Vec<CaseResult>,
}

impl Report {
    pub fn new(
        started_at: String,
        finished_at: String,
        target: String,
        git_sha: String,
        flags: String,
        cases: Vec<CaseResult>,
    ) -> Self {
        Self {
            runner: "usecases v1",
            started_at,
            finished_at,
            target,
            git_sha,
            flags,
            cases,
        }
    }

    pub fn counts(&self) -> (usize, usize, usize) {
        let mut pass = 0;
        let mut fail = 0;
        let mut skip = 0;
        for c in &self.cases {
            match c.outcome {
                Outcome::Pass => pass += 1,
                Outcome::Fail => fail += 1,
                Outcome::Skip => skip += 1,
            }
        }
        (pass, fail, skip)
    }

    /// Write the JSON run file, refresh `latest.json`, rewrite `STATUS.md`.
    pub fn write(&self, dir: &Path) -> std::io::Result<PathBuf> {
        let runs = dir.join("runs");
        std::fs::create_dir_all(&runs)?;
        let stamp: String = self
            .finished_at
            .chars()
            .filter(|c| c.is_ascii_digit())
            .take(14)
            .collect();
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let path = runs.join(format!("{stamp}.json"));
        std::fs::write(&path, &json)?;
        std::fs::write(runs.join("latest.json"), &json)?;
        self.write_status(&dir.join("STATUS.md"))?;
        Ok(path)
    }

    fn write_status(&self, path: &Path) -> std::io::Result<()> {
        const MARKER: &str = "<!-- history: one row per run, appended by the runner -->";
        let previous = std::fs::read_to_string(path).unwrap_or_default();
        let history_rows: Vec<&str> = previous
            .split_once(MARKER)
            .map(|(_, tail)| tail)
            .unwrap_or("")
            .lines()
            .filter(|l| l.starts_with("| 20"))
            .collect();

        let (pass, fail, skip) = self.counts();
        let mut out = String::new();
        out.push_str("# Desktop use-case test status\n\n");
        out.push_str(
            "Written by `cargo run -p synaplan-core --example usecases`. Do not edit the table by hand — \
             see [README.md](./README.md) §5 for the ledger policy and [CHANGES.md](./CHANGES.md) for the \
             work each failing row demands.\n\n",
        );
        out.push_str(&format!(
            "**Last run:** {} · target `{}` · desktop `{}` · flags `{}` · **{} passed, {} failed, {} skipped**\n\n",
            self.finished_at,
            self.target,
            self.git_sha,
            if self.flags.is_empty() { "(none)" } else { &self.flags },
            pass,
            fail,
            skip
        ));
        out.push_str("| Case | Journey | Result | Time | Detail |\n| ---- | ------- | ------ | ---- | ------ |\n");
        for c in &self.cases {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                c.id,
                c.name,
                match c.outcome {
                    Outcome::Pass => "✅ PASS",
                    Outcome::Fail => "❌ FAIL",
                    Outcome::Skip => "⏭ SKIP",
                },
                human_duration(c.duration_ms),
                md_cell(&c.detail)
            ));
        }

        out.push_str(
            "\n## Metrics (latest run)\n\n| Case | Metric | Value |\n| ---- | ------ | ----- |\n",
        );
        for c in &self.cases {
            for (k, v) in &c.metrics {
                out.push_str(&format!(
                    "| {} | {} | {} |\n",
                    c.id,
                    k,
                    md_cell(&value_cell(v))
                ));
            }
        }

        out.push_str("\n## Findings (failing checks, latest run)\n\n");
        let mut any = false;
        for c in &self.cases {
            for ch in c.checks.iter().filter(|ch| !ch.ok) {
                any = true;
                out.push_str(&format!("- **{}** {} — {}\n", c.id, ch.name, ch.detail));
            }
        }
        if !any {
            out.push_str("- none\n");
        }

        out.push_str("\n## History\n\n");
        out.push_str(&format!("{MARKER}\n"));
        out.push_str("| Run (UTC) | Desktop | Target | Pass | Fail | Skip | Flags |\n| --- | --- | --- | --- | --- | --- | --- |\n");
        for row in history_rows {
            out.push_str(row);
            out.push('\n');
        }
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            self.finished_at,
            self.git_sha,
            self.target,
            pass,
            fail,
            skip,
            md_cell(if self.flags.is_empty() {
                "(none)"
            } else {
                &self.flags
            })
        ));
        std::fs::write(path, out)
    }
}

fn human_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms} ms")
    } else if ms < 120_000 {
        format!("{:.1} s", ms as f64 / 1000.0)
    } else {
        format!("{:.1} min", ms as f64 / 60_000.0)
    }
}

fn md_cell(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

fn value_cell(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Shared answer normaliser: letters and digits only, upper-case, so a
/// non-breaking hyphen or bold markers do not hide a correct answer.
pub fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// First `n` chars of a string for a report cell, single line.
pub fn snippet(s: &str, n: usize) -> String {
    let one: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.chars().count() <= n {
        one
    } else {
        format!("{}…", one.chars().take(n).collect::<String>())
    }
}
