//! # synaplan-core
//!
//! The platform-independent, unit-tested core of Synaplan Desktop. Everything
//! that can be tested without a webview or a live server lives here so it can be
//! exercised by `cargo test -p synaplan-core` on all three Tier-1 operating
//! systems (Linux, macOS, Windows) without the heavy Tauri/webkit toolchain.
//!
//! The thin `synaplan-desktop` Tauri crate only *wires* these functions to
//! `#[tauri::command]`s and events — it contains no business logic and no
//! platform branches.
//!
//! Module map:
//! - [`platform`] — the ONLY place OS differences live (`app_dirs`, secret store).
//! - [`config`] — the on-disk `config.toml` (base URL + device id; never the key).
//! - [`url`] — pairing address validation (https, or http only for loopback).
//! - [`hostname`] — device-name sanitisation for the pre-filled computer name.
//! - [`sse`] — a pure Anthropic-Messages SSE parser (streaming chat tokens).
//! - [`pairing`] — the `/api/v1/desktop/pair` exchange + key verification.
//! - [`messages`] — the streaming `/v1/messages` chat turn + `/v1/models`.

pub mod agent;
pub mod config;
pub mod contract;
pub mod filesystem;
pub mod hostname;
mod http;
pub mod messages;
pub mod pairing;
pub mod platform;
pub mod skills;
pub mod sse;
pub mod tools;
pub mod url;

pub use config::DesktopConfig;
pub use platform::app_dirs::{AppDirs, Env};
pub use platform::secret_store::{
    default_secret_store, plaintext_selected, InMemorySecretStore, SecretStore, SecretStoreError,
};
pub use sse::{ChatEvent, SseParser};
