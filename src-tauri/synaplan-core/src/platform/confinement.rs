//! Path confinement (DC6 / DC24) — the core security control for local skills.
//!
//! Every path a skill touches (from the model, a script argument, an archive
//! entry, or config) is validated here before any read or write. The algorithm
//! (cross-platform §3):
//!
//! 1. Reject empty paths, NUL / control characters, and `..` components.
//! 2. Reject per-OS hazards *before* canonicalization (Windows: UNC/device
//!    namespaces, drive-relative paths, alternate data streams, reserved device
//!    names, trailing dot/space, 8.3 short names).
//! 3. Canonicalize with the platform resolver (resolves POSIX symlinks, Windows
//!    junctions/reparse points, macOS firmlinks). For a not-yet-existing write
//!    target, canonicalize the deepest existing ancestor and append the tail.
//! 4. Normalize (Unicode NFC everywhere; case-fold on Windows and macOS,
//!    byte-exact on Linux) and contain by **path components** — never a string
//!    prefix, so `Documents2` cannot match the root `Documents`.
//! 5. Apply deny globs against the canonical path.
//! 6. Writes additionally require a write root.
//!
//! The corpus of escape cases lives in the `#[cfg(test)]` module and in
//! `tests/confinement/cases.toml`; it runs on all three OSes in CI.

use std::path::{Component, Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

/// Whether a path is being resolved for reading or writing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfinementError {
    #[error("empty path")]
    Empty,
    #[error("path contains an invalid character")]
    InvalidChar,
    #[error("path must be absolute and fully qualified")]
    NotAbsolute,
    #[error("path uses a rejected form for this platform")]
    PlatformHazard,
    #[error("path could not be resolved")]
    Unresolvable,
    #[error("path is outside the allowed folders")]
    OutsideRoots,
    #[error("path matches a deny rule")]
    Denied,
    #[error("writing is not allowed at this location")]
    NotWritable,
}

/// A resolver that confines paths to a set of read/write roots minus deny globs.
pub struct Confinement {
    read_roots: Vec<PathBuf>,
    write_roots: Vec<PathBuf>,
    deny: GlobSet,
}

impl Confinement {
    /// Build a confinement. Roots are canonicalized at construction (so a
    /// `/tmp/x` root matches the canonical `/private/tmp/x` on macOS). Roots
    /// that do not exist are skipped.
    pub fn new(
        read_roots: &[PathBuf],
        write_roots: &[PathBuf],
        deny_globs: &[String],
    ) -> Result<Self, ConfinementError> {
        let canon = |roots: &[PathBuf]| -> Vec<PathBuf> {
            roots
                .iter()
                .filter_map(|r| std::fs::canonicalize(r).ok())
                .collect()
        };
        let mut builder = GlobSetBuilder::new();
        for pattern in deny_globs {
            let glob =
                GlobBuilderCompat::build(pattern).map_err(|_| ConfinementError::InvalidChar)?;
            builder.add(glob);
        }
        let deny = builder.build().map_err(|_| ConfinementError::InvalidChar)?;
        Ok(Self {
            read_roots: canon(read_roots),
            write_roots: canon(write_roots),
            deny,
        })
    }

    /// Resolve `raw` for `access`, returning the canonical path if it is allowed.
    pub fn resolve(&self, raw: &str, access: Access) -> Result<PathBuf, ConfinementError> {
        if raw.is_empty() {
            return Err(ConfinementError::Empty);
        }
        if raw.contains('\0') || raw.chars().any(|c| c.is_control()) {
            return Err(ConfinementError::InvalidChar);
        }
        reject_platform_hazards(raw)?;

        let input = Path::new(raw);
        if !input.is_absolute() {
            return Err(ConfinementError::NotAbsolute);
        }
        // Reject `.`/`..` at the string level, so it also catches them inside a
        // Windows `\\?\` verbatim path (where `components()` does not resolve or
        // reliably classify them).
        if raw.split(['/', '\\']).any(|seg| seg == ".." || seg == ".") {
            return Err(ConfinementError::InvalidChar);
        }

        let canonical = match access {
            Access::Read => {
                std::fs::canonicalize(input).map_err(|_| ConfinementError::Unresolvable)?
            }
            Access::Write => canonicalize_existing_ancestor(input)?,
        };

        if self.deny.is_match(deny_key(&canonical)) {
            return Err(ConfinementError::Denied);
        }

        match access {
            Access::Read => {
                let allowed = self
                    .read_roots
                    .iter()
                    .chain(self.write_roots.iter())
                    .any(|r| is_within(r, &canonical));
                if allowed {
                    Ok(canonical)
                } else {
                    Err(ConfinementError::OutsideRoots)
                }
            }
            Access::Write => {
                if self.write_roots.iter().any(|r| is_within(r, &canonical)) {
                    Ok(canonical)
                } else {
                    Err(ConfinementError::NotWritable)
                }
            }
        }
    }
}

/// Build a deny glob that matches case-insensitively on Windows/macOS.
struct GlobBuilderCompat;
impl GlobBuilderCompat {
    fn build(pattern: &str) -> Result<Glob, globset::Error> {
        globset::GlobBuilder::new(pattern)
            .case_insensitive(cfg!(any(target_os = "windows", target_os = "macos")))
            .literal_separator(false)
            .build()
    }
}

/// Normalize a canonical path into the string deny globs are matched against:
/// strip the Windows `\\?\` prefix and use forward slashes so `**/.ssh/**`
/// works uniformly.
fn deny_key(path: &Path) -> String {
    let mut s = path.to_string_lossy().to_string();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        s = stripped.to_string();
    }
    s.replace('\\', "/")
}

/// Per-component comparison key: NFC everywhere, case-folded on Windows/macOS.
fn component_key(component: Component<'_>) -> String {
    let raw = component.as_os_str().to_string_lossy();
    let nfc: String = raw.nfc().collect();
    if cfg!(any(target_os = "windows", target_os = "macos")) {
        nfc.to_lowercase()
    } else {
        nfc
    }
}

/// Component-wise containment: `candidate` must be `root` or nested under it.
fn is_within(root: &Path, candidate: &Path) -> bool {
    let root_comps: Vec<String> = root.components().map(component_key).collect();
    let cand_comps: Vec<String> = candidate.components().map(component_key).collect();
    if cand_comps.len() < root_comps.len() {
        return false;
    }
    root_comps
        .iter()
        .zip(cand_comps.iter())
        .all(|(a, b)| a == b)
}

/// Canonicalize the deepest existing ancestor of `path`, then append the
/// remaining (non-existent) components — for write targets that do not exist yet.
fn canonicalize_existing_ancestor(path: &Path) -> Result<PathBuf, ConfinementError> {
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    let mut cur = path.to_path_buf();
    loop {
        if let Ok(canon) = std::fs::canonicalize(&cur) {
            let mut result = canon;
            for name in tail.iter().rev() {
                result.push(name);
            }
            return Ok(result);
        }
        match cur.file_name() {
            Some(name) => {
                tail.push(name.to_os_string());
                if !cur.pop() {
                    return Err(ConfinementError::Unresolvable);
                }
            }
            None => return Err(ConfinementError::Unresolvable),
        }
    }
}

/// Windows-only string hazards, rejected before canonicalization. No-op on other
/// platforms (where `:`, reserved names, etc. are legal filename characters).
#[cfg(target_os = "windows")]
fn reject_platform_hazards(raw: &str) -> Result<(), ConfinementError> {
    // `std::fs::canonicalize` emits extended-length verbatim paths (`\\?\C:\…`);
    // accept the drive form after stripping the prefix, but reject `\\?\UNC\…`,
    // the device namespace (`\\.\`), and plain UNC (`\\server\share`).
    let path = if let Some(rest) = raw.strip_prefix(r"\\?\") {
        if rest.len() >= 4 && rest[..4].eq_ignore_ascii_case("UNC\\") {
            return Err(ConfinementError::PlatformHazard);
        }
        rest
    } else if raw.starts_with("\\\\") || raw.starts_with("//") {
        // Plain UNC (\\server\share) and the device namespace (\\.\) both begin
        // with two backslashes and are rejected here.
        return Err(ConfinementError::PlatformHazard);
    } else {
        raw
    };

    // Strip an optional drive prefix ("C:") for the remaining checks.
    let bytes = path.as_bytes();
    let after_drive = if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        // Drive-relative ("C:foo") — not fully qualified.
        if bytes.len() >= 3 && bytes[2] != b'\\' && bytes[2] != b'/' {
            return Err(ConfinementError::PlatformHazard);
        }
        &path[2..]
    } else {
        path
    };

    // Alternate data streams: any ':' after the drive letter.
    if after_drive.contains(':') {
        return Err(ConfinementError::PlatformHazard);
    }

    for part in after_drive.split(['\\', '/']) {
        if part.is_empty() {
            continue;
        }
        // Trailing dot or space.
        if part.ends_with('.') || part.ends_with(' ') {
            return Err(ConfinementError::PlatformHazard);
        }
        // 8.3 short name form (DOCUME~1).
        let has_short = part
            .bytes()
            .enumerate()
            .any(|(i, b)| b == b'~' && part.as_bytes().get(i + 1).is_some_and(u8::is_ascii_digit));
        if has_short {
            return Err(ConfinementError::PlatformHazard);
        }
        // Reserved device names (stem before the first dot).
        let stem = part.split('.').next().unwrap_or(part).to_ascii_uppercase();
        let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (stem.starts_with("COM") && is_device_digit(&stem[3..]))
            || (stem.starts_with("LPT") && is_device_digit(&stem[3..]));
        if reserved {
            return Err(ConfinementError::PlatformHazard);
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn is_device_digit(rest: &str) -> bool {
    rest.len() == 1 && matches!(rest.as_bytes()[0], b'1'..=b'9')
}

#[cfg(not(target_os = "windows"))]
fn reject_platform_hazards(_raw: &str) -> Result<(), ConfinementError> {
    Ok(())
}

/// The default deny globs applied on top of the allowlist (sensitive files that
/// are never readable even inside an allowed folder).
pub fn default_deny_globs() -> Vec<String> {
    [
        "**/.ssh/**",
        "**/.env",
        "**/.env.*",
        "**/*.key",
        "**/*.pem",
        "**/id_rsa*",
        "**/.git/config",
        "**/.aws/**",
        "**/.kube/**",
        "**/.gnupg/**",
        "**/Library/Keychains/**",
        "**/AppData/**",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct Fixture {
        _dir: tempfile::TempDir,
        root: PathBuf,
        outside: PathBuf,
        confinement: Confinement,
    }

    fn setup() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let base = fs::canonicalize(dir.path()).unwrap();
        let root = base.join("root");
        let outside = base.join("outside");
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::create_dir_all(base.join("root2")).unwrap(); // sibling-prefix trap
        fs::write(root.join("sub").join("file.txt"), b"hi").unwrap();
        fs::write(outside.join("secret.txt"), b"secret").unwrap();
        fs::create_dir_all(root.join(".ssh")).unwrap();
        fs::write(root.join(".ssh").join("id_rsa"), b"key").unwrap();

        // Use a minimal deny list for the fixture: the default globs include
        // `**/AppData/**`, and Windows temp dirs live under AppData, which would
        // deny every fixture path. The .ssh rule still exercises deny matching.
        let confinement = Confinement::new(
            std::slice::from_ref(&root),
            std::slice::from_ref(&root),
            &["**/.ssh/**".to_string()],
        )
        .unwrap();
        Fixture {
            _dir: dir,
            root,
            outside,
            confinement,
        }
    }

    #[test]
    fn allows_read_inside_root() {
        let f = setup();
        let p = f.root.join("sub").join("file.txt");
        assert!(f
            .confinement
            .resolve(p.to_str().unwrap(), Access::Read)
            .is_ok());
    }

    #[test]
    fn denies_read_outside_root() {
        let f = setup();
        let p = f.outside.join("secret.txt");
        assert_eq!(
            f.confinement.resolve(p.to_str().unwrap(), Access::Read),
            Err(ConfinementError::OutsideRoots)
        );
    }

    #[test]
    fn denies_deny_glob_inside_root() {
        let f = setup();
        let p = f.root.join(".ssh").join("id_rsa");
        assert_eq!(
            f.confinement.resolve(p.to_str().unwrap(), Access::Read),
            Err(ConfinementError::Denied)
        );
    }

    #[test]
    fn sibling_prefix_is_not_contained() {
        let f = setup();
        // "<base>/root2" must not be treated as inside "<base>/root".
        let sibling = f.root.parent().unwrap().join("root2");
        assert_eq!(
            f.confinement
                .resolve(sibling.to_str().unwrap(), Access::Read),
            Err(ConfinementError::OutsideRoots)
        );
    }

    #[test]
    fn parent_dir_component_is_rejected() {
        let f = setup();
        // Build the input as a raw string with a `..` segment so the rejection is
        // deterministic on every platform (joining onto a Windows `\\?\` verbatim
        // path does not leave a literal `..`).
        let raw = format!("{}/../outside/secret.txt", f.root.to_string_lossy());
        assert_eq!(
            f.confinement.resolve(&raw, Access::Read),
            Err(ConfinementError::InvalidChar)
        );
    }

    #[test]
    fn write_inside_write_root_ok_outside_denied() {
        let f = setup();
        let new_inside = f.root.join("sub").join("new.txt");
        assert!(f
            .confinement
            .resolve(new_inside.to_str().unwrap(), Access::Write)
            .is_ok());
        let new_outside = f.outside.join("new.txt");
        assert_eq!(
            f.confinement
                .resolve(new_outside.to_str().unwrap(), Access::Write),
            Err(ConfinementError::NotWritable)
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escape_is_denied() {
        let f = setup();
        let link = f.root.join("escape");
        std::os::unix::fs::symlink(&f.outside, &link).unwrap();
        // root/escape/secret.txt canonicalizes to outside/secret.txt.
        let via_link = link.join("secret.txt");
        assert_eq!(
            f.confinement
                .resolve(via_link.to_str().unwrap(), Access::Read),
            Err(ConfinementError::OutsideRoots)
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_string_hazards_are_rejected() {
        let f = setup();
        let root = f.root.to_str().unwrap();
        for bad in [
            format!("{root}\\file.txt:evil"),   // ADS
            format!("{root}\\CON"),             // reserved
            format!("{root}\\report."),         // trailing dot
            format!("{root}\\DOCUME~1"),        // 8.3
            "\\\\server\\share\\x".to_string(), // UNC
            "C:relative".to_string(),           // drive-relative
        ] {
            assert_eq!(
                f.confinement.resolve(&bad, Access::Read),
                Err(ConfinementError::PlatformHazard),
                "expected hazard rejection for {bad}"
            );
        }
    }

    #[test]
    fn empty_and_control_chars_rejected() {
        let f = setup();
        assert_eq!(
            f.confinement.resolve("", Access::Read),
            Err(ConfinementError::Empty)
        );
        assert_eq!(
            f.confinement.resolve("/tmp/a\u{0007}b", Access::Read),
            Err(ConfinementError::InvalidChar)
        );
    }
}
