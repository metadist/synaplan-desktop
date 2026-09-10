//! Filesystem-safe project slugs (local persistence §4).
//!
//! A slug is derived from the project name once, at create time, and is used
//! only for the human-visible folder under `projects_dir`. It is never the
//! sovereignty or RAG handle — that is the project `id`.

use unicode_normalization::UnicodeNormalization;

/// Maximum slug length in characters (all ASCII, so also bytes).
pub const MAX_SLUG_LEN: usize = 64;
/// The slug reserved for the built-in Personal project.
pub const PERSONAL_SLUG: &str = "personal";
/// Fallback when a name yields nothing usable.
const FALLBACK_SLUG: &str = "project";

/// Names that must not become a bare folder name (Windows devices, dots, and
/// the Personal slug). Compared case-insensitively.
const RESERVED: &[&str] = &[
    ".",
    "..",
    "con",
    "prn",
    "aux",
    "nul",
    "com1",
    "com2",
    "com3",
    "com4",
    "com5",
    "com6",
    "com7",
    "com8",
    "com9",
    "lpt1",
    "lpt2",
    "lpt3",
    "lpt4",
    "lpt5",
    "lpt6",
    "lpt7",
    "lpt8",
    "lpt9",
    PERSONAL_SLUG,
];

/// True if `slug` is a reserved folder name.
pub fn is_reserved(slug: &str) -> bool {
    let lower = slug.to_ascii_lowercase();
    RESERVED.iter().any(|r| *r == lower)
}

/// True if `slug` is a single filesystem-safe component: lowercase
/// `[a-z0-9-]`, no dots or separators, not reserved. Persisted records are
/// checked against this before any path is derived from them.
pub fn is_safe_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= MAX_SLUG_LEN
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && (slug == PERSONAL_SLUG || !is_reserved(slug))
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Turn a user-facing name into a lowercase ASCII `[a-z0-9-]` slug: strip
/// diacritics, collapse separators, cap the length, and avoid reserved names.
pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut pending_dash = false;
    for ch in name.nfd() {
        // NFD splits "é" into "e" + a combining mark; drop the mark.
        if unicode_normalization::char::is_combining_mark(ch) {
            continue;
        }
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(lower);
        } else {
            pending_dash = true;
        }
        if out.len() >= MAX_SLUG_LEN {
            break;
        }
    }
    let mut slug = out.trim_matches('-').to_string();
    if slug.is_empty() {
        slug = FALLBACK_SLUG.to_string();
    }
    if is_reserved(&slug) {
        slug.push_str("-project");
    }
    slug
}

/// Make `base` unique among `existing` by appending `-2`, `-3`, … as needed.
pub fn unique_slug<'a, I>(base: &str, existing: I) -> String
where
    I: IntoIterator<Item = &'a str>,
{
    let taken: Vec<&str> = existing.into_iter().collect();
    if !taken.contains(&base) {
        return base.to_string();
    }
    for n in 2u32.. {
        let suffix = format!("-{n}");
        let keep = MAX_SLUG_LEN.saturating_sub(suffix.len());
        let candidate = format!("{}{suffix}", &base[..base.len().min(keep)]);
        if !taken.contains(&candidate.as_str()) {
            return candidate;
        }
    }
    unreachable!("unique_slug loop always terminates")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_diacritics_and_collapses_separators() {
        assert_eq!(slugify("Küchen  Umbau 2026!"), "kuchen-umbau-2026");
        assert_eq!(slugify("  --Hello--World--  "), "hello-world");
        // "ß" has no ASCII decomposition, so it acts as a separator.
        assert_eq!(slugify("Ünïcödé Straße"), "unicode-stra-e");
    }

    #[test]
    fn empty_and_symbol_only_names_fall_back() {
        assert_eq!(slugify(""), "project");
        assert_eq!(slugify("!!!"), "project");
        assert_eq!(slugify("日本語"), "project");
    }

    #[test]
    fn caps_length_at_64() {
        let long = "a".repeat(200);
        assert_eq!(slugify(&long).len(), MAX_SLUG_LEN);
    }

    #[test]
    fn reserved_names_are_avoided() {
        assert_eq!(slugify("CON"), "con-project");
        assert_eq!(slugify("Personal"), "personal-project");
        assert!(is_reserved(".."));
        assert!(is_reserved("LPT1"));
        assert!(!is_reserved("console"));
    }

    #[test]
    fn safe_slugs_are_single_lowercase_components() {
        assert!(is_safe_slug("work"));
        assert!(is_safe_slug("kuchen-umbau-2"));
        assert!(is_safe_slug(PERSONAL_SLUG), "Personal's own slug is valid");
        assert!(!is_safe_slug(""));
        assert!(!is_safe_slug("../outside"));
        assert!(!is_safe_slug(".."));
        assert!(!is_safe_slug("a/b"));
        assert!(!is_safe_slug("a\\b"));
        assert!(!is_safe_slug(".hidden"));
        assert!(!is_safe_slug("Work"));
        assert!(!is_safe_slug("-lead"));
        assert!(!is_safe_slug("trail-"));
        assert!(!is_safe_slug(&"a".repeat(MAX_SLUG_LEN + 1)));
        assert!(is_safe_slug(&slugify("Küchen Umbau")));
    }

    #[test]
    fn unique_appends_counter() {
        let existing = ["work", "work-2"];
        assert_eq!(unique_slug("work", existing), "work-3");
        assert_eq!(unique_slug("home", existing), "home");
    }

    #[test]
    fn unique_respects_max_length() {
        let base = "b".repeat(MAX_SLUG_LEN);
        let taken = [base.as_str()];
        let next = unique_slug(&base, taken);
        assert!(next.len() <= MAX_SLUG_LEN);
        assert!(next.ends_with("-2"));
    }
}
