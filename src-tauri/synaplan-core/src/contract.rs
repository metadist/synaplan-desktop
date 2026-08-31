//! Typed views of the frozen Synaplan Desktop `protocol: 1` contract, plus tests
//! that pin the vendored fixtures. The check-in / report *loop* is Sprint B5;
//! this module exists from B1 so the contract is locked into the client's type
//! system and CI the moment the fixtures are vendored (C9).

use serde::{Deserialize, Serialize};

/// The frozen protocol version. A device speaking anything else is answered with
/// an empty job list and a far `next_call_at`.
pub const PROTOCOL_VERSION: u32 = 1;

/// One leased job as delivered to the device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceJob {
    #[serde(rename = "jobId")]
    pub job_id: i64,
    #[serde(rename = "type")]
    pub job_type: String,
    pub input: DeviceJobInputRaw,
    #[serde(rename = "leaseToken")]
    pub lease_token: String,
    #[serde(rename = "leaseExpires")]
    pub lease_expires: i64,
    pub attempt: i64,
}

/// The device-facing `input` object (camelCase `fileIds`, matching the wire
/// format). `deny_unknown_fields` is the structural guarantee that no
/// `command`/`script`/`argv` key can reach the runner — a future server bug
/// therefore cannot become code execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceJobInputRaw {
    pub skill: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(rename = "fileIds", default)]
    pub file_ids: Vec<i64>,
}

/// The `agent_checkin` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckinResponse {
    pub protocol: u32,
    pub jobs: Vec<DeviceJob>,
    #[serde(rename = "next_call_at")]
    pub next_call_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    const CHECKIN_RESPONSE: &str =
        include_str!("../../../tests/fixtures/desktop-contract/checkin_response.json");
    const JOB_SKILL_RUN: &str =
        include_str!("../../../tests/fixtures/desktop-contract/job_skill_run.json");

    #[test]
    fn checkin_response_is_protocol_1_with_a_clean_job() {
        let resp: CheckinResponse = serde_json::from_str(CHECKIN_RESPONSE).unwrap();
        assert_eq!(resp.protocol, PROTOCOL_VERSION);
        assert_eq!(resp.jobs.len(), 1);
        let job = &resp.jobs[0];
        assert_eq!(job.job_type, "skill.run");
        assert_eq!(job.input.skill, "hello-files");
    }

    #[test]
    fn job_input_rejects_unknown_keys() {
        // The frozen fixture parses under deny_unknown_fields (only the three
        // allowed keys) — proving no executable key is present today.
        let job: DeviceJob = serde_json::from_str(JOB_SKILL_RUN).unwrap();
        assert_eq!(job.input.skill, "hello-files");

        // A hostile `command` key must be structurally rejected, so it can never
        // reach the runner even if a future server bug emits it.
        let hostile = serde_json::json!({
            "skill": "pptx", "prompt": "x", "fileIds": [], "command": "rm -rf /"
        });
        let strict: Result<DeviceJobInputRaw, _> = serde_json::from_value(hostile);
        assert!(
            strict.is_err(),
            "an extra key outside {{skill, prompt, fileIds}} must be rejected"
        );
    }

    /// Guard that the vendored fixtures have not drifted from the recorded
    /// source-commit checksums (client half of C9).
    #[test]
    fn vendored_fixtures_match_recorded_checksums() {
        let cases: &[(&str, &[u8], &str)] = &[
            (
                "checkin_request.json",
                include_bytes!("../../../tests/fixtures/desktop-contract/checkin_request.json"),
                "8e72b150e012ae6e6dfe6dc095bdadf8f0d33ef10e881ab1e0e141abe9b07637",
            ),
            (
                "checkin_response.json",
                include_bytes!("../../../tests/fixtures/desktop-contract/checkin_response.json"),
                "4de6c8d175a15d5c4ad1ce28e2f48939aa2a7271bef83fccc99e6ff97cd21bf9",
            ),
            (
                "job_skill_run.json",
                include_bytes!("../../../tests/fixtures/desktop-contract/job_skill_run.json"),
                "e5c99d7f411629c40da65b1b6b4c01fac92a23c98fed1ea8fdbc2e69fd32a037",
            ),
            (
                "enqueue_request.json",
                include_bytes!("../../../tests/fixtures/desktop-contract/enqueue_request.json"),
                "c5a90db9d43f3587f6eeb0e8b0fafad3013aaa96a0108fb205afa95fc91edebd",
            ),
            (
                "report_success.json",
                include_bytes!("../../../tests/fixtures/desktop-contract/report_success.json"),
                "e60e9f7f8861c05ea8dc726648fb227fbfaac32685c32de6c1317de2b2ac8a44",
            ),
            (
                "report_unknown_skill.json",
                include_bytes!(
                    "../../../tests/fixtures/desktop-contract/report_unknown_skill.json"
                ),
                "03ff8ab7d9245bba87971dd1923c4b9b9e7185a6ce5ba2103ef9b42e4403ff31",
            ),
        ];
        for (name, bytes, expected) in cases {
            let mut hasher = Sha256::new();
            hasher.update(bytes);
            let got = format!("{:x}", hasher.finalize());
            assert_eq!(
                &got, expected,
                "vendored fixture {name} drifted from the source commit"
            );
        }
    }
}
