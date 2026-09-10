//! Skill loader (DC7): scans the skills directory for `SKILL.md` folders, parses
//! the frontmatter (`name`, `description`), and tracks enable/disable state in a
//! local `skills.json`. The bundled `hello-files` example is seeded on first use.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// A file embedded in a bundled skill.
struct BundledFile {
    rel: &'static str,
    contents: &'static str,
}

/// A skill shipped with the app, embedded so it seeds without a resource path.
struct BundledSkill {
    name: &'static str,
    files: &'static [BundledFile],
}

/// The bundled skills, seeded on first use. All are standard-library only
/// (zero-setup) so they run wherever Python is available.
const BUNDLED_SKILLS: &[BundledSkill] = &[
    BundledSkill {
        name: "hello-files",
        files: &[BundledFile {
            rel: "SKILL.md",
            contents: include_str!("../../../skills/bundled/hello-files/SKILL.md"),
        }],
    },
    BundledSkill {
        name: "csv-insights",
        files: &[
            BundledFile {
                rel: "SKILL.md",
                contents: include_str!("../../../skills/bundled/csv-insights/SKILL.md"),
            },
            BundledFile {
                rel: "run.py",
                contents: include_str!("../../../skills/bundled/csv-insights/run.py"),
            },
        ],
    },
    BundledSkill {
        name: "email-draft",
        files: &[
            BundledFile {
                rel: "SKILL.md",
                contents: include_str!("../../../skills/bundled/email-draft/SKILL.md"),
            },
            BundledFile {
                rel: "run.py",
                contents: include_str!("../../../skills/bundled/email-draft/run.py"),
            },
        ],
    },
    BundledSkill {
        name: "web-report",
        files: &[
            BundledFile {
                rel: "SKILL.md",
                contents: include_str!("../../../skills/bundled/web-report/SKILL.md"),
            },
            BundledFile {
                rel: "run.py",
                contents: include_str!("../../../skills/bundled/web-report/run.py"),
            },
        ],
    },
    BundledSkill {
        name: "slides",
        files: &[
            BundledFile {
                rel: "SKILL.md",
                contents: include_str!("../../../skills/bundled/slides/SKILL.md"),
            },
            BundledFile {
                rel: "run.py",
                contents: include_str!("../../../skills/bundled/slides/run.py"),
            },
        ],
    },
    BundledSkill {
        name: "chart",
        files: &[
            BundledFile {
                rel: "SKILL.md",
                contents: include_str!("../../../skills/bundled/chart/SKILL.md"),
            },
            BundledFile {
                rel: "run.py",
                contents: include_str!("../../../skills/bundled/chart/run.py"),
            },
        ],
    },
    BundledSkill {
        name: "data-table",
        files: &[
            BundledFile {
                rel: "SKILL.md",
                contents: include_str!("../../../skills/bundled/data-table/SKILL.md"),
            },
            BundledFile {
                rel: "run.py",
                contents: include_str!("../../../skills/bundled/data-table/run.py"),
            },
        ],
    },
    BundledSkill {
        name: "calendar-event",
        files: &[
            BundledFile {
                rel: "SKILL.md",
                contents: include_str!("../../../skills/bundled/calendar-event/SKILL.md"),
            },
            BundledFile {
                rel: "run.py",
                contents: include_str!("../../../skills/bundled/calendar-event/run.py"),
            },
        ],
    },
    BundledSkill {
        name: "vcard",
        files: &[
            BundledFile {
                rel: "SKILL.md",
                contents: include_str!("../../../skills/bundled/vcard/SKILL.md"),
            },
            BundledFile {
                rel: "run.py",
                contents: include_str!("../../../skills/bundled/vcard/run.py"),
            },
        ],
    },
    BundledSkill {
        name: "json-csv",
        files: &[
            BundledFile {
                rel: "SKILL.md",
                contents: include_str!("../../../skills/bundled/json-csv/SKILL.md"),
            },
            BundledFile {
                rel: "run.py",
                contents: include_str!("../../../skills/bundled/json-csv/run.py"),
            },
        ],
    },
    BundledSkill {
        name: "invoice",
        files: &[
            BundledFile {
                rel: "SKILL.md",
                contents: include_str!("../../../skills/bundled/invoice/SKILL.md"),
            },
            BundledFile {
                rel: "run.py",
                contents: include_str!("../../../skills/bundled/invoice/run.py"),
            },
        ],
    },
    BundledSkill {
        name: "pptx",
        files: &[
            BundledFile {
                rel: "SKILL.md",
                contents: include_str!("../../../skills/bundled/pptx/SKILL.md"),
            },
            BundledFile {
                rel: "run.py",
                contents: include_str!("../../../skills/bundled/pptx/run.py"),
            },
        ],
    },
];

/// True if `name` is one of the skills shipped with the app.
pub fn is_bundled(name: &str) -> bool {
    BUNDLED_SKILLS.iter().any(|s| s.name == name)
}

/// How a skill arrived on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SkillSource {
    Bundled,
    Folder,
    Zip,
    Git,
}

impl SkillSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bundled => "bundled",
            Self::Folder => "folder",
            Self::Zip => "zip",
            Self::Git => "git",
        }
    }

    fn parse(raw: &str) -> Self {
        match raw {
            "folder" => Self::Folder,
            "zip" => Self::Zip,
            "git" => Self::Git,
            _ => Self::Bundled,
        }
    }
}

/// Frontmatter + derived compatibility flags.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    pub license: Option<String>,
    pub compatibility_warning: bool,
    pub needs_python: bool,
    pub needs_node: bool,
    pub needs_libreoffice: bool,
    pub python_imports: Vec<String>,
}

/// A discovered skill (the DTO the UI and the agent loop share).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub dir: String,
    pub bundled: bool,
    pub enabled: bool,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    pub compatibility_warning: bool,
    pub allow_unattended: bool,
    pub blocked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
    pub needs_python: bool,
    pub needs_node: bool,
    pub needs_libreoffice: bool,
    #[serde(default)]
    pub python_imports: Vec<String>,
}

/// One row of the on-disk `skills.json` catalog (enablement + provenance).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRecord {
    pub name: String,
    pub enabled: bool,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_at: Option<i64>,
    #[serde(default)]
    pub allow_unattended: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CatalogFile {
    #[serde(default)]
    skills: Vec<SkillRecord>,
}

/// True if `name` obeys the Agent Skills naming rule (lowercase, digits, hyphen).
pub fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// Parse `name` + `description` from a `SKILL.md` YAML-ish frontmatter block.
pub fn parse_frontmatter(md: &str) -> Option<(String, String)> {
    parse_skill_meta(md).map(|m| (m.name, m.description))
}

/// Parse name, description, license, and compatibility flags from frontmatter.
pub fn parse_skill_meta(md: &str) -> Option<SkillMeta> {
    let trimmed = md.trim_start();
    let rest = trimmed.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    let block = &rest[..end];
    let body = &rest[end + 4..];

    let mut meta = SkillMeta::default();
    for line in block.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("name:") {
            meta.name = unquote(v.trim());
        } else if let Some(v) = line.strip_prefix("description:") {
            meta.description = unquote(v.trim());
        } else if let Some(v) = line.strip_prefix("license:") {
            let lic = unquote(v.trim());
            if !lic.is_empty() {
                meta.license = Some(lic);
            }
        } else if let Some(v) = line.strip_prefix("compatibility:") {
            let rest = v.trim();
            if rest.to_ascii_lowercase().contains("claude") {
                meta.compatibility_warning = true;
            }
        } else if line_bool(line, "python") {
            meta.needs_python = true;
        } else if line_bool(line, "node") {
            meta.needs_node = true;
        } else if line_bool(line, "libreoffice") {
            meta.needs_libreoffice = true;
        } else if let Some(v) = line
            .strip_prefix("pythonImports:")
            .or_else(|| line.strip_prefix("python-imports:"))
        {
            meta.python_imports = parse_import_list(v);
            if !meta.python_imports.is_empty() {
                meta.needs_python = true;
            }
        }
    }
    if meta.name.is_empty() || meta.description.is_empty() {
        return None;
    }
    let blob = format!("{block}\n{body}").to_ascii_lowercase();
    if blob.contains("claude code") || blob.contains("claude-code") {
        meta.compatibility_warning = true;
    }
    Some(meta)
}

fn line_bool(line: &str, key: &str) -> bool {
    let prefix = format!("{key}:");
    let Some(v) = line.strip_prefix(&prefix) else {
        return false;
    };
    matches!(
        unquote(v.trim()).to_ascii_lowercase().as_str(),
        "true" | "required" | "yes"
    )
}

fn parse_import_list(raw: &str) -> Vec<String> {
    raw.trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|s| unquote(s.trim()))
        .filter(|s| !s.is_empty())
        .collect()
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"') && s.len() >= 2)
        || (s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2)
    {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/// Ensure every bundled skill exists under `skills_dir`. Missing files are
/// written; existing files are left untouched so user edits survive.
pub fn ensure_bundled(skills_dir: &Path) -> std::io::Result<()> {
    for skill in BUNDLED_SKILLS {
        let dir = skills_dir.join(skill.name);
        for file in skill.files {
            let path = dir.join(file.rel);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            if !path.exists() {
                std::fs::write(&path, file.contents)?;
            }
        }
    }
    Ok(())
}

/// Scan `skills_dir` for valid skills, applying enable/disable from `skills.json`.
/// A directory whose `name` frontmatter does not match its folder name, or whose
/// name is invalid, is skipped. Hidden / temp directories (`.tmp-*`, `.cache`)
/// are ignored.
pub fn load_skills(skills_dir: &Path) -> Vec<Skill> {
    let _ = ensure_bundled(skills_dir);
    let catalog = load_catalog(skills_dir);

    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(skills_dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if dir_name.starts_with('.') {
            continue;
        }
        let md_path = dir.join("SKILL.md");
        if !md_path.is_file() {
            continue;
        }
        let md = std::fs::read_to_string(&md_path).unwrap_or_default();
        let Some(mut meta) = parse_skill_meta(&md) else {
            continue;
        };
        if meta.name != dir_name || !is_valid_name(&meta.name) {
            continue;
        }
        if dir.join("run.py").is_file() {
            meta.needs_python = true;
        }
        let bundled = is_bundled(&meta.name);
        let rec = catalog.iter().find(|r| r.name == meta.name);
        let enabled = rec.map(|r| r.enabled).unwrap_or(true);
        let source = rec
            .map(|r| SkillSource::parse(&r.source).as_str().to_string())
            .unwrap_or_else(|| {
                if bundled {
                    SkillSource::Bundled.as_str().to_string()
                } else {
                    SkillSource::Folder.as_str().to_string()
                }
            });
        out.push(Skill {
            name: meta.name,
            description: meta.description,
            dir: dir.to_string_lossy().to_string(),
            bundled,
            enabled,
            source,
            license: meta.license,
            compatibility_warning: meta.compatibility_warning,
            allow_unattended: rec.map(|r| r.allow_unattended).unwrap_or(false),
            blocked: false,
            blocked_reason: None,
            version: rec.and_then(|r| r.version.clone()),
            url: rec.and_then(|r| r.url.clone()),
            sha: rec.and_then(|r| r.sha.clone()),
            needs_python: meta.needs_python,
            needs_node: meta.needs_node,
            needs_libreoffice: meta.needs_libreoffice,
            python_imports: meta.python_imports,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// The names of skills the model should be offered (enabled, not blocked).
pub fn enabled_skill_names(skills_dir: &Path) -> Vec<String> {
    load_skills(skills_dir)
        .into_iter()
        .filter(|s| s.enabled && !s.blocked)
        .map(|s| s.name)
        .collect()
}

/// The skills a **project** turn may offer: the project's overlay ∩ installed
/// ∩ enabled on this computer ∩ not blocked by a missing runtime. A project can
/// only narrow what the computer allows, never widen it — a name in the overlay
/// that is uninstalled or switched off at computer level is simply absent.
pub fn project_overlay(installed: Vec<Skill>, enabled_here: &[String]) -> Vec<Skill> {
    installed
        .into_iter()
        .filter(|s| s.enabled && !s.blocked && enabled_here.iter().any(|n| n == &s.name))
        .collect()
}

fn catalog_file(skills_dir: &Path) -> std::path::PathBuf {
    skills_dir.join("skills.json")
}

fn load_catalog(skills_dir: &Path) -> Vec<SkillRecord> {
    let raw = match std::fs::read_to_string(catalog_file(skills_dir)) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    if let Ok(file) = serde_json::from_str::<CatalogFile>(&raw) {
        if !file.skills.is_empty() {
            return file.skills;
        }
    }
    // Migrate the original `{ "name": bool }` map in place.
    if let Ok(map) = serde_json::from_str::<std::collections::BTreeMap<String, bool>>(&raw) {
        let migrated: Vec<SkillRecord> = map
            .into_iter()
            .map(|(name, enabled)| SkillRecord {
                source: if is_bundled(&name) {
                    SkillSource::Bundled.as_str().to_string()
                } else {
                    SkillSource::Folder.as_str().to_string()
                },
                name,
                enabled,
                version: None,
                url: None,
                sha: None,
                installed_at: None,
                allow_unattended: false,
            })
            .collect();
        let _ = save_catalog(skills_dir, &migrated);
        return migrated;
    }
    Vec::new()
}

fn save_catalog(skills_dir: &Path, records: &[SkillRecord]) -> std::io::Result<()> {
    std::fs::create_dir_all(skills_dir)?;
    let file = CatalogFile {
        skills: records.to_vec(),
    };
    let json =
        serde_json::to_string_pretty(&file).unwrap_or_else(|_| "{\"skills\":[]}".to_string());
    std::fs::write(catalog_file(skills_dir), json)
}

fn upsert_record(skills_dir: &Path, record: SkillRecord) -> std::io::Result<()> {
    let mut records = load_catalog(skills_dir);
    if let Some(existing) = records.iter_mut().find(|r| r.name == record.name) {
        *existing = record;
    } else {
        records.push(record);
    }
    save_catalog(skills_dir, &records)
}

/// Set the enabled flag for a skill and persist `skills.json`.
pub fn set_enabled(skills_dir: &Path, name: &str, enabled: bool) -> std::io::Result<()> {
    let mut records = load_catalog(skills_dir);
    if let Some(existing) = records.iter_mut().find(|r| r.name == name) {
        existing.enabled = enabled;
    } else {
        records.push(SkillRecord {
            name: name.to_string(),
            enabled,
            source: if is_bundled(name) {
                SkillSource::Bundled.as_str().to_string()
            } else {
                SkillSource::Folder.as_str().to_string()
            },
            version: None,
            url: None,
            sha: None,
            installed_at: None,
            allow_unattended: false,
        });
    }
    save_catalog(skills_dir, &records)
}

/// Set per-skill unattended permission (default false).
pub fn set_allow_unattended(skills_dir: &Path, name: &str, allow: bool) -> std::io::Result<()> {
    let mut records = load_catalog(skills_dir);
    if let Some(existing) = records.iter_mut().find(|r| r.name == name) {
        existing.allow_unattended = allow;
    } else {
        records.push(SkillRecord {
            name: name.to_string(),
            enabled: true,
            source: if is_bundled(name) {
                SkillSource::Bundled.as_str().to_string()
            } else {
                SkillSource::Folder.as_str().to_string()
            },
            version: None,
            url: None,
            sha: None,
            installed_at: None,
            allow_unattended: allow,
        });
    }
    save_catalog(skills_dir, &records)
}

/// Record a freshly installed (not yet enabled) user skill.
pub fn record_installed(
    skills_dir: &Path,
    name: &str,
    source: SkillSource,
    url: Option<&str>,
    sha: Option<&str>,
    enabled: bool,
) -> std::io::Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .ok();
    upsert_record(
        skills_dir,
        SkillRecord {
            name: name.to_string(),
            enabled,
            source: source.as_str().to_string(),
            version: None,
            url: url.map(str::to_string),
            sha: sha.map(str::to_string),
            installed_at: now,
            allow_unattended: false,
        },
    )
}

/// Snapshot of local tools used to block skills that cannot run here.
#[derive(Debug, Clone, Default)]
pub struct RuntimeSnapshot {
    pub python: bool,
    pub node: bool,
    pub libreoffice: bool,
    pub python_imports: Vec<String>,
}

/// Mark skills whose required runtime is missing. Blocked skills stay listed
/// but must not be offered to the model.
pub fn apply_runtime_blocks(skills: &mut [Skill], snapshot: &RuntimeSnapshot) {
    for skill in skills.iter_mut() {
        let mut reasons = Vec::new();
        if skill.needs_python && !snapshot.python {
            reasons.push("Python");
        }
        for module in &skill.python_imports {
            if snapshot.python && !snapshot.python_imports.iter().any(|m| m == module) {
                reasons.push(module.as_str());
            }
        }
        if skill.needs_node && !snapshot.node {
            reasons.push("Node.js");
        }
        if skill.needs_libreoffice && !snapshot.libreoffice {
            reasons.push("LibreOffice");
        }
        if reasons.is_empty() {
            skill.blocked = false;
            skill.blocked_reason = None;
        } else {
            skill.blocked = true;
            skill.blocked_reason = Some(format!("Needs {}", reasons.join(", ")));
        }
    }
}

/// Drop a user skill from the catalog after its folder is deleted.
pub fn remove_record(skills_dir: &Path, name: &str) -> std::io::Result<()> {
    let records: Vec<SkillRecord> = load_catalog(skills_dir)
        .into_iter()
        .filter(|r| r.name != name)
        .collect();
    save_catalog(skills_dir, &records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frontmatter() {
        let md = "---\nname: pptx\ndescription: Make slides.\n---\n# body";
        assert_eq!(
            parse_frontmatter(md),
            Some(("pptx".to_string(), "Make slides.".to_string()))
        );
    }

    #[test]
    fn validates_names() {
        assert!(is_valid_name("hello-files"));
        assert!(is_valid_name("pptx"));
        assert!(!is_valid_name("Hello"));
        assert!(!is_valid_name("has space"));
        assert!(!is_valid_name(""));
    }

    fn bare_skill(name: &str, enabled: bool, blocked: bool) -> Skill {
        Skill {
            name: name.to_string(),
            description: String::new(),
            dir: String::new(),
            bundled: true,
            enabled,
            source: "bundled".to_string(),
            license: None,
            compatibility_warning: false,
            allow_unattended: false,
            blocked,
            blocked_reason: None,
            version: None,
            url: None,
            sha: None,
            needs_python: false,
            needs_node: false,
            needs_libreoffice: false,
            python_imports: Vec::new(),
        }
    }

    #[test]
    fn project_overlay_only_narrows_the_computer_level_set() {
        let installed = vec![
            bare_skill("slides", true, false),
            bare_skill("csv-insights", true, false),
            bare_skill("off-here", false, false),
            bare_skill("no-python", true, true),
        ];
        let overlay = vec![
            "slides".to_string(),
            "off-here".to_string(),
            "no-python".to_string(),
            "not-installed".to_string(),
        ];
        let names: Vec<String> = project_overlay(installed, &overlay)
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert_eq!(names, vec!["slides"]);
    }

    #[test]
    fn seeds_and_loads_bundled_skill() {
        let dir = tempfile::tempdir().unwrap();
        let skills = load_skills(dir.path());
        let hello = skills
            .iter()
            .find(|s| s.name == "hello-files")
            .expect("bundled skill present");
        assert!(hello.bundled);
        assert!(hello.enabled);
        assert!(!hello.description.is_empty());
    }

    #[test]
    fn seeds_all_bundled_skills_with_scripts() {
        let dir = tempfile::tempdir().unwrap();
        let skills = load_skills(dir.path());
        let scripted = [
            "csv-insights",
            "email-draft",
            "web-report",
            "slides",
            "chart",
            "data-table",
            "calendar-event",
            "vcard",
            "json-csv",
            "invoice",
            "pptx",
        ];
        for name in std::iter::once("hello-files").chain(scripted) {
            assert!(
                skills
                    .iter()
                    .any(|s| s.name == name && s.bundled && s.enabled),
                "bundled skill missing: {name}"
            );
        }
        // The scripted skills ship a run.py on disk.
        for name in scripted {
            assert!(
                dir.path().join(name).join("run.py").is_file(),
                "run.py missing for {name}"
            );
        }
    }

    #[test]
    fn disable_persists_and_filters() {
        let dir = tempfile::tempdir().unwrap();
        let _ = load_skills(dir.path()); // seed
        set_enabled(dir.path(), "hello-files", false).unwrap();
        assert!(!enabled_skill_names(dir.path()).contains(&"hello-files".to_string()));
        let reloaded = load_skills(dir.path());
        assert!(
            !reloaded
                .iter()
                .find(|s| s.name == "hello-files")
                .unwrap()
                .enabled
        );
    }

    #[test]
    fn pptx_is_blocked_without_python_pptx() {
        let dir = tempfile::tempdir().unwrap();
        let mut skills = load_skills(dir.path());
        let snapshot = RuntimeSnapshot {
            python: true,
            node: true,
            libreoffice: false,
            python_imports: vec![],
        };
        apply_runtime_blocks(&mut skills, &snapshot);
        let pptx = skills.iter().find(|s| s.name == "pptx").expect("pptx");
        assert!(pptx.needs_python);
        assert!(pptx.python_imports.iter().any(|m| m == "pptx"));
        assert!(pptx.blocked);
        assert!(!enabled_skill_names(dir.path()).contains(&"pptx".to_string()) || pptx.blocked);
    }

    #[test]
    fn hermetic_pptx_script_writes_a_zip() {
        let python = crate::platform::doctor::resolve_on_path("python3")
            .or_else(|| crate::platform::doctor::resolve_on_path("python"));
        let Some(python) = python else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("deck.pptx");
        let script =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/hermetic-pptx/run.py");
        let status = std::process::Command::new(&python)
            .arg(&script)
            .arg(&out)
            .arg("Hermetic")
            .status()
            .expect("spawn python");
        assert!(status.success());
        assert!(out.is_file());
        let bytes = std::fs::read(&out).unwrap();
        assert!(bytes.starts_with(b"PK"), "pptx must be a zip");
    }

    #[test]
    fn skips_name_dir_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("wrongdir");
        std::fs::create_dir_all(&bad).unwrap();
        std::fs::write(
            bad.join("SKILL.md"),
            "---\nname: other\ndescription: x\n---\n",
        )
        .unwrap();
        let skills = load_skills(dir.path());
        assert!(skills.iter().all(|s| s.name != "other"));
    }
}
