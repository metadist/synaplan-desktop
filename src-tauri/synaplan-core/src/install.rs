//! Skill install / remove (DC12 / DC13): folder, zip, and GitHub zipball.
//!
//! Archives are untrusted. Every entry is validated before anything is written,
//! and the install is atomic: extract to a sibling temp directory, validate,
//! then rename into place. A failure deletes the temp tree and leaves the
//! skills directory unchanged.

use std::collections::HashSet;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use thiserror::Error;
use unicode_normalization::UnicodeNormalization;
use zip::ZipArchive;

use crate::skills::{self, is_bundled, is_valid_name, parse_skill_meta, SkillMeta, SkillSource};

const MAX_ENTRIES: usize = 256;
const MAX_ENTRY_BYTES: u64 = 8 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 32 * 1024 * 1024;
const MAX_REL_PATH: usize = 240;

const RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("{0}")]
    InvalidPackage(String),
    #[error("{0}")]
    UnsafeArchive(String),
    #[error("{0}")]
    Io(String),
    #[error("included skills cannot be removed")]
    BundledImmutable,
    #[error("that address is not allowed")]
    UrlRejected,
    #[error("could not download the skill")]
    Download,
}

impl InstallError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidPackage(_) => "invalid_package",
            Self::UnsafeArchive(_) => "unsafe_archive",
            Self::Io(_) => "install",
            Self::BundledImmutable => "bundled_immutable",
            Self::UrlRejected => "invalid_url",
            Self::Download => "download",
        }
    }
}

impl From<io::Error> for InstallError {
    fn from(e: io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

/// Metadata shown in the supply-chain confirm dialog before files are written
/// (folder/zip) or after a zipball is downloaded to a cache (URL).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPreview {
    pub name: String,
    pub description: String,
    pub license: Option<String>,
    pub files: Vec<String>,
    pub compatibility_warning: bool,
    pub source: String,
    pub needs_python: bool,
    pub needs_node: bool,
    pub needs_libreoffice: bool,
    pub python_imports: Vec<String>,
    /// Cached zip path for a URL preview; the confirm step installs from it.
    pub cache_path: Option<String>,
}

/// A parsed `https://github.com/owner/repo` (optionally `/tree/{ref}/{subdir}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubSkillRef {
    pub owner: String,
    pub repo: String,
    pub git_ref: String,
    pub subdir: Option<String>,
}

/// Parse a GitHub HTTPS URL. Rejects `file://`, `git://`, SSH, and non-https
/// (except loopback, used by tests).
pub fn parse_github_url(raw: &str) -> Result<GitHubSkillRef, InstallError> {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("file://")
        || lower.starts_with("git://")
        || lower.starts_with("ssh://")
        || trimmed.contains('@') && !trimmed.starts_with("http")
    {
        return Err(InstallError::UrlRejected);
    }

    let url = url::Url::parse(trimmed).map_err(|_| InstallError::UrlRejected)?;
    let https = url.scheme() == "https";
    let loopback_http = url.scheme() == "http"
        && url
            .host_str()
            .is_some_and(|h| h == "127.0.0.1" || h == "localhost" || h == "::1");
    if !https && !loopback_http {
        return Err(InstallError::UrlRejected);
    }
    if url.scheme() == "https" && url.host_str() != Some("github.com") {
        return Err(InstallError::UrlRejected);
    }

    let mut segs: Vec<String> = url
        .path()
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_end_matches(".git").to_string())
        .collect();
    if segs.len() < 2 {
        return Err(InstallError::UrlRejected);
    }
    let owner = segs.remove(0);
    let repo = segs.remove(0);
    if owner.is_empty() || repo.is_empty() {
        return Err(InstallError::UrlRejected);
    }

    let (git_ref, subdir) = if segs.first().map(String::as_str) == Some("tree") && segs.len() >= 2 {
        let git_ref = segs[1].clone();
        let sub = if segs.len() > 2 {
            Some(segs[2..].join("/"))
        } else {
            None
        };
        (git_ref, sub)
    } else {
        ("main".to_string(), None)
    };

    Ok(GitHubSkillRef {
        owner,
        repo,
        git_ref,
        subdir,
    })
}

/// Preview a folder that already contains `SKILL.md`. Does not copy.
pub fn preview_folder(folder: &Path) -> Result<InstallPreview, InstallError> {
    let meta = read_folder_meta(folder)?;
    let files = list_folder_files(folder)?;
    Ok(preview_from_meta(meta, files, SkillSource::Folder, None))
}

/// Preview a zip without extracting it onto the skills directory.
pub fn preview_zip(zip_path: &Path) -> Result<InstallPreview, InstallError> {
    let bytes = fs::read(zip_path).map_err(|e| InstallError::Io(e.to_string()))?;
    preview_zip_bytes(&bytes, None)
}

/// Download a GitHub zipball to `{skills_dir}/.cache/` and preview it.
pub async fn preview_url(url: &str, skills_dir: &Path) -> Result<InstallPreview, InstallError> {
    let parsed = parse_github_url(url)?;
    let (sha, bytes) = download_github_zipball(&parsed).await?;
    let cache_dir = skills_dir.join(".cache");
    fs::create_dir_all(&cache_dir)?;
    let cache_path = cache_dir.join(format!("{sha}.zip"));
    fs::write(&cache_path, &bytes)?;
    let mut preview = preview_zip_bytes(&bytes, parsed.subdir.as_deref())?;
    preview.source = SkillSource::Git.as_str().to_string();
    preview.cache_path = Some(cache_path.to_string_lossy().to_string());
    Ok(preview)
}

/// Copy a folder into `skills_dir/{name}/` (not a move). Enabled only after the
/// caller flips the catalog flag.
pub fn install_folder(
    folder: &Path,
    skills_dir: &Path,
    source: SkillSource,
    url: Option<&str>,
    sha: Option<&str>,
) -> Result<String, InstallError> {
    let preview = preview_folder(folder)?;
    validate_name(&preview.name)?;
    copy_folder_atomic(folder, skills_dir, &preview.name)?;
    record_install(skills_dir, &preview, source, url, sha)?;
    Ok(preview.name)
}

/// Extract a zip into `skills_dir/{name}/`.
pub fn install_zip(
    zip_path: &Path,
    skills_dir: &Path,
    source: SkillSource,
    url: Option<&str>,
    sha: Option<&str>,
    subdir: Option<&str>,
) -> Result<String, InstallError> {
    let bytes = fs::read(zip_path).map_err(|e| InstallError::Io(e.to_string()))?;
    install_zip_bytes(&bytes, skills_dir, source, url, sha, subdir)
}

/// Download (or reuse a cached zip) and install.
pub async fn install_url(url: &str, skills_dir: &Path) -> Result<String, InstallError> {
    let parsed = parse_github_url(url)?;
    let (sha, bytes) = download_github_zipball(&parsed).await?;
    install_zip_bytes(
        &bytes,
        skills_dir,
        SkillSource::Git,
        Some(url),
        Some(&sha),
        parsed.subdir.as_deref(),
    )
}

/// Install from an already-downloaded zip (URL confirm path).
pub fn install_cached_zip(
    cache_path: &Path,
    skills_dir: &Path,
    url: Option<&str>,
    sha: Option<&str>,
) -> Result<String, InstallError> {
    install_zip(cache_path, skills_dir, SkillSource::Git, url, sha, None)
}

/// Remove a user-installed skill. Bundled skills are immutable.
pub fn remove_skill(skills_dir: &Path, name: &str) -> Result<(), InstallError> {
    if is_bundled(name) {
        return Err(InstallError::BundledImmutable);
    }
    if !is_valid_name(name) {
        return Err(InstallError::InvalidPackage("invalid skill name".into()));
    }
    let target = skills_dir.join(name);
    if target.exists() {
        fs::remove_dir_all(&target)?;
    }
    skills::remove_record(skills_dir, name).map_err(|e| InstallError::Io(e.to_string()))?;
    Ok(())
}

/// Strip `com.apple.quarantine` after the user confirms the install. Uses the
/// libc xattr API — never a shell.
pub fn strip_quarantine(dir: &Path) {
    #[cfg(target_os = "macos")]
    {
        walk_strip_quarantine(dir);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = dir;
    }
}

#[cfg(target_os = "macos")]
fn walk_strip_quarantine(dir: &Path) {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if let Ok(c_path) = CString::new(path.as_os_str().as_bytes()) {
            let name = CString::new("com.apple.quarantine").expect("static");
            unsafe {
                libc::removexattr(c_path.as_ptr(), name.as_ptr(), 0);
            }
        }
        if path.is_dir() {
            walk_strip_quarantine(&path);
        }
    }
}

fn preview_from_meta(
    meta: SkillMeta,
    files: Vec<String>,
    source: SkillSource,
    cache_path: Option<String>,
) -> InstallPreview {
    InstallPreview {
        name: meta.name,
        description: meta.description,
        license: meta.license,
        files,
        compatibility_warning: meta.compatibility_warning,
        source: source.as_str().to_string(),
        needs_python: meta.needs_python,
        needs_node: meta.needs_node,
        needs_libreoffice: meta.needs_libreoffice,
        python_imports: meta.python_imports,
        cache_path,
    }
}

fn record_install(
    skills_dir: &Path,
    preview: &InstallPreview,
    source: SkillSource,
    url: Option<&str>,
    sha: Option<&str>,
) -> Result<(), InstallError> {
    skills::record_installed(
        skills_dir,
        &preview.name,
        source,
        url,
        sha,
        /* enabled */ false,
    )
    .map_err(|e| InstallError::Io(e.to_string()))
}

fn validate_name(name: &str) -> Result<(), InstallError> {
    if !is_valid_name(name) {
        return Err(InstallError::InvalidPackage(
            "the skill name is not valid".into(),
        ));
    }
    Ok(())
}

fn read_folder_meta(folder: &Path) -> Result<SkillMeta, InstallError> {
    let md = folder.join("SKILL.md");
    if !md.is_file() {
        return Err(InstallError::InvalidPackage(
            "the folder must contain a SKILL.md file".into(),
        ));
    }
    reject_links(folder)?;
    let text = fs::read_to_string(&md)?;
    let meta = parse_skill_meta(&text).ok_or_else(|| {
        InstallError::InvalidPackage("SKILL.md is missing a name and description".into())
    })?;
    validate_name(&meta.name)?;
    let dir_name = folder
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    if dir_name != meta.name {
        return Err(InstallError::InvalidPackage(
            "the folder name must match the skill name in SKILL.md".into(),
        ));
    }
    Ok(meta)
}

fn list_folder_files(folder: &Path) -> Result<Vec<String>, InstallError> {
    let mut files = Vec::new();
    collect_rel_files(folder, folder, &mut files, 0)?;
    files.sort();
    Ok(files)
}

fn collect_rel_files(
    root: &Path,
    dir: &Path,
    out: &mut Vec<String>,
    depth: usize,
) -> Result<(), InstallError> {
    if depth > 8 {
        return Err(InstallError::UnsafeArchive(
            "the folder is nested too deeply".into(),
        ));
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let ft = entry.file_type()?;
        if ft.is_symlink() {
            return Err(InstallError::UnsafeArchive(
                "the folder contains a link, which is not allowed".into(),
            ));
        }
        if ft.is_dir() {
            collect_rel_files(root, &path, out, depth + 1)?;
        } else {
            let rel = path
                .strip_prefix(root)
                .map_err(|_| InstallError::UnsafeArchive("path escaped the folder".into()))?;
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

fn reject_links(root: &Path) -> Result<(), InstallError> {
    fn walk(dir: &Path) -> Result<(), InstallError> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_symlink() {
                return Err(InstallError::UnsafeArchive(
                    "the folder contains a link, which is not allowed".into(),
                ));
            }
            let path = entry.path();
            if path.is_dir() {
                walk(&path)?;
            }
        }
        Ok(())
    }
    walk(root)
}

fn copy_folder_atomic(src: &Path, skills_dir: &Path, name: &str) -> Result<(), InstallError> {
    fs::create_dir_all(skills_dir)?;
    let tmp = unique_tmp(skills_dir, name);
    let dest = skills_dir.join(name);
    let result = (|| {
        copy_tree(src, &tmp)?;
        assert_contained(&tmp, &tmp)?;
        replace_dir(&tmp, &dest)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&tmp);
    }
    result
}

fn copy_tree(src: &Path, dest: &Path) -> Result<(), InstallError> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        if entry.file_type()?.is_symlink() {
            return Err(InstallError::UnsafeArchive(
                "the folder contains a link, which is not allowed".into(),
            ));
        }
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if from.is_dir() {
            copy_tree(&from, &to)?;
        } else {
            write_no_exec(&from, &to)?;
        }
    }
    Ok(())
}

fn write_no_exec(from: &Path, to: &Path) -> Result<(), InstallError> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = fs::read(from)?;
    write_bytes_no_exec(to, &bytes)
}

fn write_bytes_no_exec(to: &Path, bytes: &[u8]) -> Result<(), InstallError> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o644)
            .open(to)?;
        f.write_all(bytes)?;
    }
    #[cfg(not(unix))]
    {
        fs::write(to, bytes)?;
    }
    Ok(())
}

fn unique_tmp(skills_dir: &Path, name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    skills_dir.join(format!(".tmp-{name}-{nanos}"))
}

fn replace_dir(tmp: &Path, dest: &Path) -> Result<(), InstallError> {
    if dest.exists() {
        fs::remove_dir_all(dest)?;
    }
    fs::rename(tmp, dest).map_err(|e| InstallError::Io(e.to_string()))
}

fn preview_zip_bytes(bytes: &[u8], subdir: Option<&str>) -> Result<InstallPreview, InstallError> {
    let planned = plan_zip_entries(bytes, subdir)?;
    let md = planned
        .iter()
        .find(|e| e.rel == "SKILL.md")
        .ok_or_else(|| {
            InstallError::InvalidPackage(
                "the zip must contain a SKILL.md inside the skill folder".into(),
            )
        })?;
    let text = String::from_utf8_lossy(&md.data).to_string();
    let meta = parse_skill_meta(&text).ok_or_else(|| {
        InstallError::InvalidPackage("SKILL.md is missing a name and description".into())
    })?;
    validate_name(&meta.name)?;
    let files: Vec<String> = planned.iter().map(|e| e.rel.clone()).collect();
    Ok(preview_from_meta(meta, files, SkillSource::Zip, None))
}

fn install_zip_bytes(
    bytes: &[u8],
    skills_dir: &Path,
    source: SkillSource,
    url: Option<&str>,
    sha: Option<&str>,
    subdir: Option<&str>,
) -> Result<String, InstallError> {
    let planned = plan_zip_entries(bytes, subdir)?;
    let md = planned
        .iter()
        .find(|e| e.rel == "SKILL.md")
        .ok_or_else(|| {
            InstallError::InvalidPackage(
                "the zip must contain a SKILL.md inside the skill folder".into(),
            )
        })?;
    let text = String::from_utf8_lossy(&md.data).to_string();
    let meta = parse_skill_meta(&text).ok_or_else(|| {
        InstallError::InvalidPackage("SKILL.md is missing a name and description".into())
    })?;
    validate_name(&meta.name)?;

    fs::create_dir_all(skills_dir)?;
    let tmp = unique_tmp(skills_dir, &meta.name);
    let dest = skills_dir.join(&meta.name);
    let result = (|| {
        fs::create_dir_all(&tmp)?;
        for entry in &planned {
            let path = tmp.join(&entry.rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            write_bytes_no_exec(&path, &entry.data)?;
        }
        assert_contained(&tmp, &tmp)?;
        replace_dir(&tmp, &dest)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&tmp);
        return result.map(|_| meta.name);
    }

    let preview = preview_from_meta(
        meta.clone(),
        planned.iter().map(|e| e.rel.clone()).collect(),
        source,
        None,
    );
    record_install(skills_dir, &preview, source, url, sha)?;
    Ok(meta.name)
}

struct PlannedEntry {
    rel: String,
    data: Vec<u8>,
}

/// Validate every zip entry and return the files that will be written, with
/// paths relative to the skill root (`SKILL.md` at `.`).
fn plan_zip_entries(bytes: &[u8], subdir: Option<&str>) -> Result<Vec<PlannedEntry>, InstallError> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|_| InstallError::InvalidPackage("not a zip file".into()))?;
    if archive.len() > MAX_ENTRIES {
        return Err(InstallError::UnsafeArchive(
            "the zip has too many files".into(),
        ));
    }

    let names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();
    let skill_prefix = find_skill_prefix(&names, subdir)?;

    let mut planned = Vec::new();
    let mut fold_keys: HashSet<String> = HashSet::new();
    let mut total: u64 = 0;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|_| InstallError::InvalidPackage("could not read a zip entry".into()))?;
        let raw_name = file.name().to_string();
        validate_entry_name(&raw_name)?;

        if let Some(mode) = file.unix_mode() {
            if mode & 0o170000 == 0o120000 || mode & 0o170000 == 0o100000 && file.is_symlink() {
                return Err(InstallError::UnsafeArchive(
                    "the zip contains a link, which is not allowed".into(),
                ));
            }
            if file.is_symlink() {
                return Err(InstallError::UnsafeArchive(
                    "the zip contains a link, which is not allowed".into(),
                ));
            }
        }
        if file.is_symlink() {
            return Err(InstallError::UnsafeArchive(
                "the zip contains a link, which is not allowed".into(),
            ));
        }

        if file.is_dir() || raw_name.ends_with('/') {
            continue;
        }

        let rel = strip_prefix(&raw_name, &skill_prefix).ok_or_else(|| {
            InstallError::InvalidPackage("the zip layout is not a single skill folder".into())
        })?;
        if rel.is_empty() {
            continue;
        }
        validate_rel_path(&rel)?;

        let key = fold_key(&rel);
        if !fold_keys.insert(key) {
            return Err(InstallError::UnsafeArchive(
                "the zip contains two files that would overwrite each other".into(),
            ));
        }

        let size = file.size();
        if size > MAX_ENTRY_BYTES {
            return Err(InstallError::UnsafeArchive(
                "a file in the zip is too large".into(),
            ));
        }
        total = total.saturating_add(size);
        if total > MAX_TOTAL_BYTES {
            return Err(InstallError::UnsafeArchive("the zip is too large".into()));
        }

        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .map_err(|_| InstallError::InvalidPackage("could not read a zip entry".into()))?;
        if data.len() as u64 > MAX_ENTRY_BYTES {
            return Err(InstallError::UnsafeArchive(
                "a file in the zip is too large".into(),
            ));
        }
        planned.push(PlannedEntry { rel, data });
    }

    if !planned.iter().any(|e| e.rel == "SKILL.md") {
        return Err(InstallError::InvalidPackage(
            "the zip must contain {name}/SKILL.md, not a bare SKILL.md at the root".into(),
        ));
    }
    Ok(planned)
}

fn find_skill_prefix(names: &[String], subdir: Option<&str>) -> Result<String, InstallError> {
    let skill_mds: Vec<&str> = names
        .iter()
        .map(String::as_str)
        .filter(|n| n.ends_with("SKILL.md") || n.ends_with("SKILL.MD"))
        .collect();

    if let Some(sub) = subdir {
        let needle = format!("{}/SKILL.md", sub.trim_matches('/'));
        if let Some(full) = skill_mds
            .iter()
            .find(|n| n.replace('\\', "/").ends_with(&needle) || n.replace('\\', "/") == needle)
        {
            let norm = full.replace('\\', "/");
            return Ok(norm.trim_end_matches("SKILL.md").to_string());
        }
        return Err(InstallError::InvalidPackage(
            "the zip does not contain SKILL.md in the requested folder".into(),
        ));
    }

    // Prefer `{name}/SKILL.md` (one directory deep), then `{wrapper}/{name}/SKILL.md`
    // as GitHub zipballs produce.
    let mut prefixes = HashSet::new();
    for n in &skill_mds {
        let norm = n.replace('\\', "/");
        if let Some(p) = norm.strip_suffix("SKILL.md") {
            prefixes.insert(p.to_string());
        }
    }
    if prefixes.len() == 1 {
        let p = prefixes.into_iter().next().unwrap();
        if p.is_empty() {
            return Err(InstallError::InvalidPackage(
                "the zip must contain {name}/SKILL.md, not a bare SKILL.md at the root".into(),
            ));
        }
        return Ok(p);
    }
    if prefixes.is_empty() {
        return Err(InstallError::InvalidPackage(
            "the zip does not contain a SKILL.md".into(),
        ));
    }
    Err(InstallError::InvalidPackage(
        "the zip contains more than one skill; install a zip with a single skill folder".into(),
    ))
}

fn strip_prefix(raw: &str, prefix: &str) -> Option<String> {
    let norm = raw.replace('\\', "/");
    let prefix = prefix.trim_start_matches("./");
    let rest = if prefix.is_empty() {
        norm
    } else {
        norm.strip_prefix(prefix)?.to_string()
    };
    let rest = rest.trim_start_matches('/').to_string();
    Some(rest)
}

fn validate_entry_name(name: &str) -> Result<(), InstallError> {
    if name.contains('\\') {
        return Err(InstallError::UnsafeArchive(
            "the zip uses backslashes in a file name, which is not allowed".into(),
        ));
    }
    if name.starts_with('/') || name.starts_with('\\') {
        return Err(InstallError::UnsafeArchive(
            "the zip contains an absolute path".into(),
        ));
    }
    if name.contains(':') {
        return Err(InstallError::UnsafeArchive(
            "the zip contains a reserved file name".into(),
        ));
    }
    if name.split('/').any(|seg| seg == ".." || seg == ".") {
        return Err(InstallError::UnsafeArchive(
            "the zip contains a path that would write outside the skill folder".into(),
        ));
    }
    Ok(())
}

fn validate_rel_path(rel: &str) -> Result<(), InstallError> {
    if rel.len() > MAX_REL_PATH {
        return Err(InstallError::UnsafeArchive(
            "a file path in the zip is too long".into(),
        ));
    }
    for seg in rel.split('/') {
        if seg.is_empty() {
            continue;
        }
        if seg == ".." || seg == "." {
            return Err(InstallError::UnsafeArchive(
                "the zip contains a path that would write outside the skill folder".into(),
            ));
        }
        if seg.ends_with(' ') || seg.ends_with('.') {
            return Err(InstallError::UnsafeArchive(
                "the zip contains a reserved file name".into(),
            ));
        }
        let stem = seg.split('.').next().unwrap_or(seg);
        if RESERVED_NAMES.iter().any(|r| stem.eq_ignore_ascii_case(r)) {
            return Err(InstallError::UnsafeArchive(
                "the zip contains a reserved file name".into(),
            ));
        }
    }
    Ok(())
}

fn fold_key(rel: &str) -> String {
    rel.nfc().collect::<String>().to_lowercase()
}

fn assert_contained(root: &Path, dir: &Path) -> Result<(), InstallError> {
    let root_c = fs::canonicalize(root).map_err(|e| InstallError::Io(e.to_string()))?;
    fn walk(root: &Path, dir: &Path) -> Result<(), InstallError> {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            let canon = fs::canonicalize(&path).map_err(|e| InstallError::Io(e.to_string()))?;
            if !canon.starts_with(root) {
                return Err(InstallError::UnsafeArchive(
                    "a written file escaped the skill folder".into(),
                ));
            }
            if path.is_dir() {
                walk(root, &path)?;
            }
        }
        Ok(())
    }
    walk(&root_c, dir)
}

async fn download_github_zipball(
    parsed: &GitHubSkillRef,
) -> Result<(String, Vec<u8>), InstallError> {
    let sha = resolve_github_sha(parsed).await?;
    let url = format!(
        "https://codeload.github.com/{}/{}/zip/{sha}",
        parsed.owner, parsed.repo
    );
    let bytes = download_bytes(&url).await?;
    Ok((sha, bytes))
}

async fn resolve_github_sha(parsed: &GitHubSkillRef) -> Result<String, InstallError> {
    if is_full_sha(&parsed.git_ref) {
        return Ok(parsed.git_ref.clone());
    }
    let url = format!(
        "https://api.github.com/repos/{}/{}/commits/{}",
        parsed.owner, parsed.repo, parsed.git_ref
    );
    let client = github_client()?;
    let resp = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| InstallError::Download)?;
    if !resp.status().is_success() {
        return Err(InstallError::Download);
    }
    let body: serde_json::Value = resp.json().await.map_err(|_| InstallError::Download)?;
    body.get("sha")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or(InstallError::Download)
}

fn is_full_sha(s: &str) -> bool {
    s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit())
}

async fn download_bytes(url: &str) -> Result<Vec<u8>, InstallError> {
    let client = github_client()?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|_| InstallError::Download)?;
    if !resp.status().is_success() {
        return Err(InstallError::Download);
    }
    let bytes = resp.bytes().await.map_err(|_| InstallError::Download)?;
    if bytes.len() as u64 > MAX_TOTAL_BYTES {
        return Err(InstallError::UnsafeArchive(
            "the download is too large".into(),
        ));
    }
    Ok(bytes.to_vec())
}

fn github_client() -> Result<reqwest::Client, InstallError> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            let host = attempt.url().host_str().unwrap_or("");
            let allowed = matches!(
                host,
                "github.com"
                    | "api.github.com"
                    | "codeload.github.com"
                    | "objects.githubusercontent.com"
            );
            if allowed {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .connect_timeout(std::time::Duration::from_secs(15))
        .user_agent(concat!("SynaplanDesktop/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|_| InstallError::Download)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &b in data {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    /// Write a stored (uncompressed) zip with the exact entry names given —
    /// including names the `zip` crate would refuse to produce.
    fn write_raw_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let mut local = Vec::new();
        let mut central = Vec::new();
        for (name, data) in entries {
            let name_b = name.as_bytes();
            let crc = crc32(data);
            let offset = local.len() as u32;
            local.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            local.extend_from_slice(&20u16.to_le_bytes());
            local.extend_from_slice(&0u16.to_le_bytes());
            local.extend_from_slice(&0u16.to_le_bytes());
            local.extend_from_slice(&0u16.to_le_bytes());
            local.extend_from_slice(&0u16.to_le_bytes());
            local.extend_from_slice(&crc.to_le_bytes());
            local.extend_from_slice(&(data.len() as u32).to_le_bytes());
            local.extend_from_slice(&(data.len() as u32).to_le_bytes());
            local.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
            local.extend_from_slice(&0u16.to_le_bytes());
            local.extend_from_slice(name_b);
            local.extend_from_slice(data);

            central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&crc.to_le_bytes());
            central.extend_from_slice(&(data.len() as u32).to_le_bytes());
            central.extend_from_slice(&(data.len() as u32).to_le_bytes());
            central.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u32.to_le_bytes());
            central.extend_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(name_b);
        }
        let cd_offset = local.len() as u32;
        let cd_size = central.len() as u32;
        let mut out = File::create(path).unwrap();
        out.write_all(&local).unwrap();
        out.write_all(&central).unwrap();
        out.write_all(&0x0605_4b50u32.to_le_bytes()).unwrap();
        out.write_all(&0u16.to_le_bytes()).unwrap();
        out.write_all(&0u16.to_le_bytes()).unwrap();
        out.write_all(&(entries.len() as u16).to_le_bytes())
            .unwrap();
        out.write_all(&(entries.len() as u16).to_le_bytes())
            .unwrap();
        out.write_all(&cd_size.to_le_bytes()).unwrap();
        out.write_all(&cd_offset.to_le_bytes()).unwrap();
        out.write_all(&0u16.to_le_bytes()).unwrap();
    }

    const SKILL_MD: &str =
        "---\nname: sample-skill\ndescription: A test skill.\nlicense: Apache-2.0\n---\n# sample\n";

    fn valid_entries() -> Vec<(&'static str, &'static [u8])> {
        vec![
            ("sample-skill/SKILL.md", SKILL_MD.as_bytes()),
            ("sample-skill/run.py", b"print('ok')\n"),
        ]
    }

    #[test]
    fn github_url_parser_table() {
        let ok = parse_github_url("https://github.com/acme/widget").unwrap();
        assert_eq!(
            ok,
            GitHubSkillRef {
                owner: "acme".into(),
                repo: "widget".into(),
                git_ref: "main".into(),
                subdir: None,
            }
        );
        let nested =
            parse_github_url("https://github.com/acme/widget.git/tree/abc123/skills/pptx").unwrap();
        // `.git` is stripped from the repo segment only when it is the repo name.
        let tree =
            parse_github_url("https://github.com/acme/widget/tree/v1.2/skills/pptx").unwrap();
        assert_eq!(tree.git_ref, "v1.2");
        assert_eq!(tree.subdir.as_deref(), Some("skills/pptx"));
        let _ = nested;

        for bad in [
            "file:///tmp/x",
            "git://github.com/acme/widget",
            "ssh://git@github.com/acme/widget",
            "git@github.com:acme/widget.git",
            "http://example.com/acme/widget",
            "https://gitlab.com/acme/widget",
        ] {
            assert!(parse_github_url(bad).is_err(), "should reject {bad}");
        }
        assert!(parse_github_url("http://127.0.0.1:9/acme/widget").is_ok());
    }

    #[test]
    fn valid_zip_installs_atomically() {
        let root = tempfile::tempdir().unwrap();
        let skills = root.path().join("skills");
        fs::create_dir_all(&skills).unwrap();
        let zip_path = root.path().join("ok.zip");
        write_raw_zip(&zip_path, &valid_entries());

        let name = install_zip(&zip_path, &skills, SkillSource::Zip, None, None, None).unwrap();
        assert_eq!(name, "sample-skill");
        assert!(skills.join("sample-skill/SKILL.md").is_file());
        assert!(skills.join("sample-skill/run.py").is_file());
        let loaded = skills::load_skills(&skills);
        let skill = loaded.iter().find(|s| s.name == "sample-skill").unwrap();
        assert!(!skill.enabled, "install must not enable before confirm");
        assert!(!skill.bundled);
    }

    #[test]
    fn zip_slip_dotdot_is_rejected() {
        let root = tempfile::tempdir().unwrap();
        let skills = root.path().join("skills");
        fs::create_dir_all(&skills).unwrap();
        let zip_path = root.path().join("evil.zip");
        write_raw_zip(
            &zip_path,
            &[
                ("sample-skill/SKILL.md", SKILL_MD.as_bytes()),
                ("sample-skill/../../evil.txt", b"nope"),
            ],
        );
        let err = install_zip(&zip_path, &skills, SkillSource::Zip, None, None, None).unwrap_err();
        assert!(matches!(err, InstallError::UnsafeArchive(_)));
        assert!(!root.path().join("evil.txt").exists());
        assert!(!skills.join("sample-skill").exists());
    }

    #[test]
    fn zip_backslash_separator_is_rejected() {
        let root = tempfile::tempdir().unwrap();
        let skills = root.path().join("skills");
        fs::create_dir_all(&skills).unwrap();
        let zip_path = root.path().join("bs.zip");
        write_raw_zip(
            &zip_path,
            &[
                ("sample-skill/SKILL.md", SKILL_MD.as_bytes()),
                ("sample-skill\\..\\..\\evil.txt", b"nope"),
            ],
        );
        let err = install_zip(&zip_path, &skills, SkillSource::Zip, None, None, None).unwrap_err();
        assert!(matches!(err, InstallError::UnsafeArchive(_)));
        assert!(!skills.join("sample-skill").exists());
    }

    #[test]
    fn zip_ads_colon_is_rejected() {
        let root = tempfile::tempdir().unwrap();
        let skills = root.path().join("skills");
        fs::create_dir_all(&skills).unwrap();
        let zip_path = root.path().join("ads.zip");
        write_raw_zip(
            &zip_path,
            &[
                ("sample-skill/SKILL.md", SKILL_MD.as_bytes()),
                ("sample-skill/a.txt:b", b"ads"),
            ],
        );
        assert!(install_zip(&zip_path, &skills, SkillSource::Zip, None, None, None).is_err());
    }

    #[test]
    fn zip_reserved_and_trailing_dot_rejected() {
        let root = tempfile::tempdir().unwrap();
        let skills = root.path().join("skills");
        fs::create_dir_all(&skills).unwrap();
        for name in [
            "sample-skill/NUL",
            "sample-skill/CON.txt",
            "sample-skill/foo.",
        ] {
            let zip_path = root.path().join("r.zip");
            write_raw_zip(
                &zip_path,
                &[("sample-skill/SKILL.md", SKILL_MD.as_bytes()), (name, b"x")],
            );
            assert!(
                install_zip(&zip_path, &skills, SkillSource::Zip, None, None, None).is_err(),
                "should reject {name}"
            );
        }
    }

    #[test]
    fn zip_case_collision_rejected() {
        let root = tempfile::tempdir().unwrap();
        let skills = root.path().join("skills");
        fs::create_dir_all(&skills).unwrap();
        let zip_path = root.path().join("case.zip");
        write_raw_zip(
            &zip_path,
            &[
                ("sample-skill/SKILL.md", SKILL_MD.as_bytes()),
                ("sample-skill/Readme.txt", b"a"),
                ("sample-skill/readme.txt", b"b"),
            ],
        );
        assert!(install_zip(&zip_path, &skills, SkillSource::Zip, None, None, None).is_err());
    }

    #[test]
    fn zip_root_skill_md_rejected() {
        let root = tempfile::tempdir().unwrap();
        let skills = root.path().join("skills");
        fs::create_dir_all(&skills).unwrap();
        let zip_path = root.path().join("root.zip");
        write_raw_zip(&zip_path, &[("SKILL.md", SKILL_MD.as_bytes())]);
        let err = install_zip(&zip_path, &skills, SkillSource::Zip, None, None, None).unwrap_err();
        assert!(matches!(err, InstallError::InvalidPackage(_)));
        assert_eq!(fs::read_dir(&skills).unwrap().count(), 0);
    }

    #[test]
    fn zip_bomb_entry_count_rejected() {
        let root = tempfile::tempdir().unwrap();
        let skills = root.path().join("skills");
        fs::create_dir_all(&skills).unwrap();
        let zip_path = root.path().join("bomb.zip");
        let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
        entries.push(("sample-skill/SKILL.md".into(), SKILL_MD.as_bytes().to_vec()));
        for i in 0..300 {
            entries.push((format!("sample-skill/f{i}.txt"), b"x".to_vec()));
        }
        let refs: Vec<(&str, &[u8])> = entries
            .iter()
            .map(|(n, d)| (n.as_str(), d.as_slice()))
            .collect();
        write_raw_zip(&zip_path, &refs);
        assert!(install_zip(&zip_path, &skills, SkillSource::Zip, None, None, None).is_err());
        assert!(!skills.join("sample-skill").exists());
    }

    #[test]
    fn failed_install_leaves_skills_dir_untouched() {
        let root = tempfile::tempdir().unwrap();
        let skills = root.path().join("skills");
        fs::create_dir_all(&skills).unwrap();
        fs::write(skills.join("marker"), b"keep").unwrap();
        let zip_path = root.path().join("bad.zip");
        write_raw_zip(
            &zip_path,
            &[
                ("sample-skill/SKILL.md", SKILL_MD.as_bytes()),
                ("../outside.txt", b"x"),
            ],
        );
        assert!(install_zip(&zip_path, &skills, SkillSource::Zip, None, None, None).is_err());
        assert!(skills.join("marker").is_file());
        assert!(!skills.join("sample-skill").exists());
        assert!(!root.path().join("outside.txt").exists());
    }

    #[test]
    fn folder_install_copies_and_remove_works() {
        let root = tempfile::tempdir().unwrap();
        let skills = root.path().join("skills");
        let src = root.path().join("sample-skill");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("SKILL.md"), SKILL_MD).unwrap();
        fs::write(src.join("notes.txt"), b"hi").unwrap();

        let name = install_folder(&src, &skills, SkillSource::Folder, None, None).unwrap();
        assert_eq!(name, "sample-skill");
        assert!(src.join("SKILL.md").is_file(), "copy, not move");
        assert!(skills.join("sample-skill/notes.txt").is_file());

        remove_skill(&skills, "sample-skill").unwrap();
        assert!(!skills.join("sample-skill").exists());
    }

    #[test]
    fn cannot_remove_bundled() {
        let dir = tempfile::tempdir().unwrap();
        let _ = skills::load_skills(dir.path());
        let err = remove_skill(dir.path(), "hello-files").unwrap_err();
        assert!(matches!(err, InstallError::BundledImmutable));
        assert!(dir.path().join("hello-files/SKILL.md").is_file());
    }

    #[cfg(unix)]
    #[test]
    fn folder_symlink_rejected() {
        let root = tempfile::tempdir().unwrap();
        let skills = root.path().join("skills");
        let src = root.path().join("sample-skill");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("SKILL.md"), SKILL_MD).unwrap();
        let outside = root.path().join("secret.txt");
        fs::write(&outside, b"secret").unwrap();
        std::os::unix::fs::symlink(&outside, src.join("link.txt")).unwrap();
        assert!(install_folder(&src, &skills, SkillSource::Folder, None, None).is_err());
        assert!(!skills.join("sample-skill").exists());
    }
}
