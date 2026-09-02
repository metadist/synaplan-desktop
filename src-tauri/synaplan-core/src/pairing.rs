//! The pairing exchange (`POST /api/v1/desktop/pair`) and key verification
//! (`GET /v1/models`). The scoped key returned here is stored in the OS secret
//! store by the caller — never written to disk.

use serde::Deserialize;
use thiserror::Error;

use crate::http;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PairError {
    #[error("This code is wrong or has expired. Create a new one in Synaplan.")]
    InvalidCode,
    #[error("Desktop access is turned off on that Synaplan instance.")]
    FeatureDisabled,
    #[error("Too many attempts. Please wait a moment and try again.")]
    RateLimited,
    #[error("This computer was disconnected. Pair again.")]
    Unauthorized,
    #[error("Could not reach that Synaplan address. Check the address and your connection.")]
    Network,
    #[error("The server returned an unexpected response ({0}).")]
    Unexpected(String),
}

/// A machine-friendly error code the UI maps to a localized message.
impl PairError {
    pub fn code(&self) -> &'static str {
        match self {
            PairError::InvalidCode => "invalid_code",
            PairError::FeatureDisabled => "feature_disabled",
            PairError::RateLimited => "rate_limited",
            PairError::Unauthorized => "unauthorized",
            PairError::Network => "network",
            PairError::Unexpected(_) => "unexpected",
        }
    }
}

/// The result of a successful pairing (or pasted-key recovery).
#[derive(Debug, Clone)]
pub struct PairedDevice {
    pub device_id: Option<i64>,
    pub api_base_url: String,
    pub key: String,
}

#[derive(Debug, Deserialize)]
struct PairResponse {
    #[serde(default)]
    success: bool,
    #[serde(rename = "deviceId")]
    device_id: Option<i64>,
    key: Option<String>,
    #[serde(rename = "apiBaseUrl")]
    api_base_url: Option<String>,
}

/// Exchange a pairing code for a scoped API key + device id.
pub async fn pair(
    base_url: &str,
    code: &str,
    device_name: &str,
) -> Result<PairedDevice, PairError> {
    let client = http::client().map_err(|_| PairError::Network)?;
    let url = http::join(base_url, "/api/v1/desktop/pair");
    let body = serde_json::json!({
        "code": code,
        "deviceName": device_name,
        "capabilities": ["skill.run"],
    });

    let resp = client
        .post(url)
        .json(&body)
        .send()
        .await
        .map_err(|_| PairError::Network)?;

    match resp.status().as_u16() {
        200 | 201 => {
            let parsed: PairResponse = resp
                .json()
                .await
                .map_err(|e| PairError::Unexpected(e.to_string()))?;
            let key = parsed
                .key
                .filter(|k| !k.is_empty())
                .ok_or_else(|| PairError::Unexpected("no key in response".to_string()))?;
            if !parsed.success {
                return Err(PairError::Unexpected("server reported failure".to_string()));
            }
            Ok(PairedDevice {
                device_id: parsed.device_id,
                api_base_url: parsed.api_base_url.unwrap_or_else(|| base_url.to_string()),
                key,
            })
        }
        400 => Err(PairError::InvalidCode),
        401 => Err(PairError::Unauthorized),
        404 => Err(PairError::FeatureDisabled),
        429 => Err(PairError::RateLimited),
        other => Err(PairError::Unexpected(other.to_string())),
    }
}

/// Verify a (pasted) scoped key works against an instance by listing models.
/// Used by the recovery "paste a key" path.
pub async fn verify_key(base_url: &str, key: &str) -> Result<(), PairError> {
    let client = http::client().map_err(|_| PairError::Network)?;
    let url = http::join(base_url, "/v1/models");
    let resp = client
        .get(url)
        .header("x-api-key", key)
        .header("authorization", format!("Bearer {key}"))
        .send()
        .await
        .map_err(|_| PairError::Network)?;

    match resp.status().as_u16() {
        200 => Ok(()),
        401 | 403 => Err(PairError::Unauthorized),
        404 => Err(PairError::FeatureDisabled),
        other => Err(PairError::Unexpected(other.to_string())),
    }
}
