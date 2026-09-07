//! Device poll loop (DC19 / DC20): check-in, validate, run, report.
//!
//! The loop itself is driven by the Tauri shell (sleep until `next_call_at`).
//! This module owns the per-tick logic so it can be unit-tested with fixtures
//! and never constructs a shell (C12) or reads extra job keys (C9).

use serde_json::json;

use crate::contract::{
    CheckinRequest, CheckinResponse, DeviceJob, ReportRequest, ERROR_LOCAL, ERROR_SKILL_DISABLED,
    ERROR_UNKNOWN_SKILL, ERROR_UNKNOWN_TYPE, JOB_TYPE_SKILL_RUN, PROTOCOL_VERSION,
};
use crate::skills::Skill;

/// Why a leased job must not run on this computer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobRefusal {
    pub error_code: String,
    pub message: String,
}

/// Last poll-tick status shown in the UI and the tray.
#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PollStatus {
    pub running: bool,
    pub last_checkin_unix: Option<i64>,
    pub next_call_at: Option<i64>,
    pub jobs_waiting: u32,
    pub last_error: Option<String>,
    pub plaintext_blocked: bool,
}

/// Decide whether this job may run. Reads only `{skill, prompt, fileIds}`.
pub fn classify_job(job: &DeviceJob, skills: &[Skill]) -> Result<(), JobRefusal> {
    if job.job_type != JOB_TYPE_SKILL_RUN {
        return Err(JobRefusal {
            error_code: ERROR_UNKNOWN_TYPE.into(),
            message: format!(
                "This computer does not run jobs of type '{}'.",
                job.job_type
            ),
        });
    }
    let Some(skill) = skills.iter().find(|s| s.name == job.input.skill) else {
        return Err(JobRefusal {
            error_code: ERROR_UNKNOWN_SKILL.into(),
            message: format!(
                "This computer does not have a skill named '{}'.",
                job.input.skill
            ),
        });
    };
    if !skill.enabled {
        return Err(JobRefusal {
            error_code: ERROR_SKILL_DISABLED.into(),
            message: format!("The skill '{}' is turned off.", skill.name),
        });
    }
    if !skill.allow_unattended {
        return Err(JobRefusal {
            error_code: ERROR_SKILL_DISABLED.into(),
            message: format!(
                "The skill '{}' is not allowed to run when nobody is at the keyboard.",
                skill.name
            ),
        });
    }
    if skill.blocked {
        return Err(JobRefusal {
            error_code: ERROR_LOCAL.into(),
            message: skill
                .blocked_reason
                .clone()
                .unwrap_or_else(|| format!("The skill '{}' cannot run here.", skill.name)),
        });
    }
    Ok(())
}

pub fn checkin_request(skills: &[Skill]) -> CheckinRequest {
    CheckinRequest::idle(
        skills
            .iter()
            .filter(|s| s.enabled && !s.blocked)
            .map(|s| s.name.clone())
            .collect(),
    )
}

pub fn parse_checkin_response(value: serde_json::Value) -> Result<CheckinResponse, String> {
    serde_json::from_value(value).map_err(|e| e.to_string())
}

pub fn refusal_report(lease_token: &str, refusal: &JobRefusal) -> ReportRequest {
    ReportRequest {
        lease_token: lease_token.to_string(),
        status: "failed".into(),
        error_code: Some(refusal.error_code.clone()),
        result: Some(json!({ "message": refusal.message })),
    }
}

pub fn success_report(lease_token: &str, summary: &str, file_ids: Vec<i64>) -> ReportRequest {
    ReportRequest {
        lease_token: lease_token.to_string(),
        status: "succeeded".into(),
        error_code: None,
        result: Some(json!({ "summary": summary, "fileIds": file_ids })),
    }
}

/// Seconds to sleep until `next_call_at`, never negative. Honour the server
/// schedule; do not invent a shorter interval.
pub fn sleep_secs(now_unix: i64, next_call_at: i64) -> u64 {
    next_call_at.saturating_sub(now_unix).max(0) as u64
}

/// Current Unix epoch seconds (injectable in tests via [`sleep_secs`]).
pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn protocol_ok(resp: &CheckinResponse) -> bool {
    resp.protocol == PROTOCOL_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::DeviceJobInputRaw;

    fn skill(name: &str, enabled: bool, unattended: bool, blocked: bool) -> Skill {
        Skill {
            name: name.into(),
            description: "d".into(),
            dir: "/tmp/x".into(),
            bundled: false,
            enabled,
            source: "folder".into(),
            license: None,
            compatibility_warning: false,
            allow_unattended: unattended,
            blocked,
            blocked_reason: if blocked {
                Some("Needs Python".into())
            } else {
                None
            },
            version: None,
            url: None,
            sha: None,
            needs_python: false,
            needs_node: false,
            needs_libreoffice: false,
            python_imports: vec![],
        }
    }

    fn job(kind: &str, skill_name: &str) -> DeviceJob {
        DeviceJob {
            job_id: 1,
            job_type: kind.into(),
            input: DeviceJobInputRaw {
                skill: skill_name.into(),
                prompt: "do it".into(),
                file_ids: vec![],
            },
            lease_token: "lt_x".into(),
            lease_expires: 1,
            attempt: 1,
        }
    }

    #[test]
    fn refuses_unknown_type() {
        let err = classify_job(&job("shell.exec", "hello-files"), &[]).unwrap_err();
        assert_eq!(err.error_code, ERROR_UNKNOWN_TYPE);
    }

    #[test]
    fn refuses_unknown_and_disabled() {
        let err = classify_job(&job(JOB_TYPE_SKILL_RUN, "pptx"), &[]).unwrap_err();
        assert_eq!(err.error_code, ERROR_UNKNOWN_SKILL);
        let skills = vec![skill("hello-files", false, true, false)];
        let err = classify_job(&job(JOB_TYPE_SKILL_RUN, "hello-files"), &skills).unwrap_err();
        assert_eq!(err.error_code, ERROR_SKILL_DISABLED);
    }

    #[test]
    fn refuses_without_unattended_and_when_blocked() {
        let skills = vec![skill("hello-files", true, false, false)];
        let err = classify_job(&job(JOB_TYPE_SKILL_RUN, "hello-files"), &skills).unwrap_err();
        assert_eq!(err.error_code, ERROR_SKILL_DISABLED);
        let skills = vec![skill("hello-files", true, true, true)];
        let err = classify_job(&job(JOB_TYPE_SKILL_RUN, "hello-files"), &skills).unwrap_err();
        assert_eq!(err.error_code, ERROR_LOCAL);
    }

    #[test]
    fn accepts_enabled_unattended_ready_skill() {
        let skills = vec![skill("hello-files", true, true, false)];
        assert!(classify_job(&job(JOB_TYPE_SKILL_RUN, "hello-files"), &skills).is_ok());
    }

    #[test]
    fn extra_command_key_never_reaches_classify() {
        let hostile = serde_json::json!({
            "skill": "hello-files", "prompt": "x", "fileIds": [], "command": "rm -rf /"
        });
        assert!(serde_json::from_value::<DeviceJobInputRaw>(hostile).is_err());
    }

    #[test]
    fn sleep_honours_next_call_at() {
        assert_eq!(sleep_secs(100, 180), 80);
        assert_eq!(sleep_secs(200, 180), 0);
    }

    #[test]
    fn frozen_checkin_request_shape() {
        let req = CheckinRequest::idle(vec!["hello-files".into(), "pptx".into()]);
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["protocol"], 1);
        assert_eq!(v["agentKind"], "synaplan-desktop");
        assert_eq!(v["capabilities"][0], "skill.run");
    }
}
