//! Shared HTTP client construction. The Synaplan address is *pinned*: the client
//! never follows a redirect (a redirect to another host is treated as an error),
//! so pairing and chat cannot be silently bounced to a different server.

use std::time::Duration;

/// Build the shared reqwest client used for pairing and chat.
pub(crate) fn client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(15))
        .user_agent(concat!("SynaplanDesktop/", env!("CARGO_PKG_VERSION")))
        .build()
}

/// Join a validated base URL and an absolute path (`/api/...`).
pub(crate) fn join(base_url: &str, path: &str) -> String {
    format!("{}{}", base_url.trim_end_matches('/'), path)
}
