//! Device-name sanitisation. The pairing screen pre-fills the computer name from
//! the OS hostname, which can contain characters the server device list should
//! not render raw (control characters, angle brackets, etc.). We keep only a
//! conservative set and cap the length.

/// The maximum device-name length (matches the server's `BNAME VARCHAR(128)`,
/// kept comfortably under it).
pub const MAX_DEVICE_NAME_LEN: usize = 64;

const FALLBACK: &str = "My computer";

/// Turn a raw hostname (or user-typed name) into a safe device name.
///
/// Allowed characters: letters, digits, space, `-`, `_`, `.`. Everything else is
/// dropped, runs of whitespace collapse to a single space, the result is
/// trimmed and capped, and an empty result falls back to a friendly default.
pub fn sanitize_device_name(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len().min(MAX_DEVICE_NAME_LEN));
    let mut last_was_space = false;
    for ch in raw.chars() {
        let keep = ch.is_alphanumeric() || matches!(ch, '-' | '_' | '.');
        if keep {
            out.push(ch);
            last_was_space = false;
        } else if ch.is_whitespace() && !last_was_space && !out.is_empty() {
            out.push(' ');
            last_was_space = true;
        }
        // else: drop the character (or a leading/duplicate space) entirely.
    }

    let trimmed = out.trim();
    let capped: String = trimmed.chars().take(MAX_DEVICE_NAME_LEN).collect();
    let capped = capped.trim().to_string();
    if capped.is_empty() {
        FALLBACK.to_string()
    } else {
        capped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_a_normal_hostname() {
        assert_eq!(
            sanitize_device_name("Annas-MacBook-Pro.local"),
            "Annas-MacBook-Pro.local"
        );
    }

    #[test]
    fn drops_dangerous_characters() {
        assert_eq!(sanitize_device_name("desk<top>"), "desktop");
        assert_eq!(sanitize_device_name("a\u{0000}b"), "ab");
        assert_eq!(
            sanitize_device_name("name/with\\slashes"),
            "namewithslashes"
        );
    }

    #[test]
    fn collapses_whitespace() {
        assert_eq!(sanitize_device_name("  Jan's   laptop  "), "Jans laptop");
    }

    #[test]
    fn falls_back_when_empty() {
        assert_eq!(sanitize_device_name(""), FALLBACK);
        assert_eq!(sanitize_device_name("   \t "), FALLBACK);
        assert_eq!(sanitize_device_name("<>"), FALLBACK);
    }

    #[test]
    fn caps_length() {
        let long = "x".repeat(200);
        assert_eq!(
            sanitize_device_name(&long).chars().count(),
            MAX_DEVICE_NAME_LEN
        );
    }

    #[test]
    fn keeps_unicode_letters() {
        assert_eq!(sanitize_device_name("Büro-Rechner"), "Büro-Rechner");
    }
}
