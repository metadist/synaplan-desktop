//! Skill loader (DC7): scans the skills directory for `SKILL.md` folders, parses
//! the frontmatter (`name`, `description`), and tracks enable/disable state in a
//! local `skills.json`. The bundled `hello-files` example is seeded on first use.

use std::collections::BTreeMap;
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
];

/// True if `name` is one of the skills shipped with the app.
fn is_bundled(name: &str) -> bool {
    BUNDLED_SKILLS.iter().any(|s| s.name == name)
}

/// A discovered skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub dir: String,
    pub bundled: bool,
    pub enabled: bool,
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
    let trimmed = md.trim_start();
    let rest = trimmed.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    let block = &rest[..end];

    let mut name = None;
    let mut description = None;
    for line in block.lines() {
        if let Some(v) = line.strip_prefix("name:") {
            name = Some(unquote(v.trim()));
        } else if let Some(v) = line.strip_prefix("description:") {
            description = Some(unquote(v.trim()));
        }
    }
    match (name, description) {
        (Some(n), Some(d)) if !n.is_empty() => Some((n, d)),
        _ => None,
    }
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
/// name is invalid, is skipped.
pub fn load_skills(skills_dir: &Path) -> Vec<Skill> {
    let _ = ensure_bundled(skills_dir);
    let enabled_map = load_enabled(skills_dir);

    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(skills_dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let md_path = dir.join("SKILL.md");
        if !md_path.is_file() {
            continue;
        }
        let md = std::fs::read_to_string(&md_path).unwrap_or_default();
        let Some((name, description)) = parse_frontmatter(&md) else {
            continue;
        };
        let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if name != dir_name || !is_valid_name(&name) {
            continue;
        }
        let bundled = is_bundled(&name);
        let enabled = enabled_map.get(&name).copied().unwrap_or(true);
        out.push(Skill {
            name,
            description,
            dir: dir.to_string_lossy().to_string(),
            bundled,
            enabled,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// The names of skills the model should be offered (enabled + valid).
pub fn enabled_skill_names(skills_dir: &Path) -> Vec<String> {
    load_skills(skills_dir)
        .into_iter()
        .filter(|s| s.enabled)
        .map(|s| s.name)
        .collect()
}

fn enabled_file(skills_dir: &Path) -> std::path::PathBuf {
    skills_dir.join("skills.json")
}

fn load_enabled(skills_dir: &Path) -> BTreeMap<String, bool> {
    std::fs::read_to_string(enabled_file(skills_dir))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Set the enabled flag for a skill and persist `skills.json`.
pub fn set_enabled(skills_dir: &Path, name: &str, enabled: bool) -> std::io::Result<()> {
    let mut map = load_enabled(skills_dir);
    map.insert(name.to_string(), enabled);
    std::fs::create_dir_all(skills_dir)?;
    let json = serde_json::to_string_pretty(&map).unwrap_or_else(|_| "{}".to_string());
    std::fs::write(enabled_file(skills_dir), json)
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
