//! Local project artifacts: supported types, safe names, and the out-folder
//! write used after AI generation or a skill run.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Extensions Synaplan will store (`FileStorageService::ALLOWED_EXTENSIONS`).
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "pdf", "docx", "doc", "xlsx", "xls", "pptx", "ppt", "txt", "md", "csv", "odt", "ods", "odp",
    "odg", "odf", "rtf", "pages", "numbers", "key", "ics", "jpg", "jpeg", "png", "gif", "webp",
    "heic", "heif", "mp3", "mp4", "wav", "ogg", "m4a", "webm", "mov", "avi", "mkv", "jar",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatArtifact {
    pub path: String,
    pub name: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_id: Option<i64>,
}

impl ChatArtifact {
    pub fn new(path: impl AsRef<Path>, file_id: Option<i64>) -> Self {
        let path = path.as_ref();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "file".into());
        Self {
            kind: kind_from_path(path).to_string(),
            path: path.to_string_lossy().into_owned(),
            name,
            file_id,
        }
    }
}

pub fn is_supported(ext: &str) -> bool {
    let e = ext.trim_start_matches('.').to_ascii_lowercase();
    SUPPORTED_EXTENSIONS.contains(&e.as_str())
}

pub fn kind_from_path(path: &Path) -> &'static str {
    kind_from_extension(
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default(),
    )
}

pub fn kind_from_extension(ext: &str) -> &'static str {
    match ext.trim_start_matches('.').to_ascii_lowercase().as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "heic" | "heif" => "image",
        "mp3" | "wav" | "ogg" | "m4a" => "audio",
        "mp4" | "webm" | "mov" | "avi" | "mkv" => "video",
        "pdf" | "docx" | "doc" | "xlsx" | "xls" | "pptx" | "ppt" | "txt" | "md" | "csv" | "odt"
        | "ods" | "odp" | "odg" | "odf" | "rtf" | "pages" | "numbers" | "key" | "ics" => "document",
        _ => "file",
    }
}

pub fn extension_for_mime(mime: &str, fallback: &str) -> String {
    let mapped = match mime {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "audio/mpeg" | "audio/mp3" => "mp3",
        "audio/wav" | "audio/x-wav" => "wav",
        "audio/ogg" => "ogg",
        "audio/mp4" => "m4a",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        _ => "",
    };
    if mapped.is_empty() {
        fallback.trim_start_matches('.').to_ascii_lowercase()
    } else {
        mapped.to_string()
    }
}

/// Single path component, no separators, capped.
pub fn safe_stem(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if matches!(ch, '-' | '_' | ' ') && !out.ends_with('-') {
            out.push('-');
        }
        if out.len() >= 48 {
            break;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "file".into()
    } else {
        trimmed
    }
}

pub fn unique_out_path(out_dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let ext = ext.trim_start_matches('.').to_ascii_lowercase();
    let stem = safe_stem(stem);
    let candidate = out_dir.join(format!("{stem}.{ext}"));
    if !candidate.exists() {
        return candidate;
    }
    for n in 2..1000 {
        let p = out_dir.join(format!("{stem}-{n}.{ext}"));
        if !p.exists() {
            return p;
        }
    }
    out_dir.join(format!("{stem}-{}.{}", now_stamp(), ext))
}

pub fn write_bytes(
    out_dir: &Path,
    stem: &str,
    ext: &str,
    bytes: &[u8],
) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(out_dir)?;
    let path = unique_out_path(out_dir, stem, ext);
    std::fs::write(&path, bytes)?;
    Ok(path)
}

/// Copy `src` into `out_dir` when it is not already there. Returns the path
/// that belongs to the project out folder.
pub fn copy_into_out(out_dir: &Path, src: &Path) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(out_dir)?;
    if is_inside(out_dir, src) {
        return Ok(src.to_path_buf());
    }
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = src.extension().and_then(|s| s.to_str()).unwrap_or("bin");
    let dest = unique_out_path(out_dir, stem, ext);
    std::fs::copy(src, &dest)?;
    Ok(dest)
}

fn is_inside(root: &Path, path: &Path) -> bool {
    let Ok(root) = root.canonicalize() else {
        return false;
    };
    let Ok(path) = path.canonicalize() else {
        return false;
    };
    path.starts_with(&root)
}

fn now_stamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_types_cover_office_and_media() {
        assert!(is_supported("png"));
        assert!(is_supported(".PDF"));
        assert!(is_supported("mp3"));
        assert!(is_supported("docx"));
        assert!(!is_supported("exe"));
        assert_eq!(kind_from_extension("png"), "image");
        assert_eq!(kind_from_extension("mp3"), "audio");
        assert_eq!(kind_from_extension("mp4"), "video");
        assert_eq!(kind_from_extension("docx"), "document");
    }

    #[test]
    fn write_and_unique_names() {
        let dir = tempfile::tempdir().unwrap();
        let a = write_bytes(dir.path(), "Cat Picture!", "png", b"one").unwrap();
        let b = write_bytes(dir.path(), "Cat Picture!", "png", b"two").unwrap();
        assert_eq!(a.file_name().unwrap(), "cat-picture.png");
        assert_eq!(b.file_name().unwrap(), "cat-picture-2.png");
        assert_ne!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
        let copied = copy_into_out(dir.path(), &a).unwrap();
        assert_eq!(copied, a);
    }
}
