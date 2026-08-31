//! Platform-specific code — the single directory a reviewer reads to know what
//! differs per OS (master plan §0.3 / decision 27, cross-platform §2, §4).
//!
//! Nothing outside this module may construct a home-relative path or branch on
//! the operating system for paths or secret storage.

pub mod app_dirs;
pub mod confinement;
pub mod secret_store;
