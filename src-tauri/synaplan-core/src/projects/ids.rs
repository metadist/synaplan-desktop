//! Local identifiers and timestamps for project records.
//!
//! Projects and chat threads need a stable id that is generated on this
//! computer and never reused. We emit a ULID-shaped string (26 Crockford
//! base32 characters: 48 bits of milliseconds + 80 random bits) without adding
//! a crate: the randomness comes from `std`'s per-instance `RandomState`
//! seeds, which is plenty for a local, single-user identifier space.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// Length of a ULID string.
pub const ID_LEN: usize = 26;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn random_u64(salt: u64) -> u64 {
    let mut hasher = RandomState::new().build_hasher();
    salt.hash(&mut hasher);
    COUNTER.fetch_add(1, Ordering::Relaxed).hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    hasher.finish()
}

/// Milliseconds since the Unix epoch (0 if the clock is before 1970).
pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// A new ULID-shaped identifier: time-ordered, unique on this computer.
pub fn new_id() -> String {
    encode_ulid(now_millis(), random_u64(1), random_u64(2))
}

fn encode_ulid(millis: u64, r1: u64, r2: u64) -> String {
    // 48 bits of time, then 80 random bits (64 from r1, 16 from r2).
    let time = (millis & 0x0000_FFFF_FFFF_FFFF) as u128;
    let rand = ((r1 as u128) << 16) | ((r2 & 0xFFFF) as u128);
    let value: u128 = (time << 80) | rand;
    let mut out = [b'0'; ID_LEN];
    for (i, slot) in out.iter_mut().enumerate() {
        let shift = (ID_LEN - 1 - i) * 5;
        let idx = ((value >> shift) & 0x1F) as usize;
        *slot = CROCKFORD[idx];
    }
    String::from_utf8_lossy(&out).to_string()
}

/// True if `id` is a string this module could have produced (or a similarly
/// shaped identifier) and is therefore safe to use as a file name component.
pub fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Current time as an ISO-8601 UTC string with second precision
/// (`2026-09-10T13:45:12Z`).
pub fn now_iso8601() -> String {
    iso8601_from_unix(now_millis() / 1000)
}

/// Format Unix seconds as ISO-8601 UTC. Uses the civil-from-days algorithm so
/// no date crate is needed.
pub fn iso8601_from_unix(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Howard Hinnant's `civil_from_days`: days since 1970-01-01 → (y, m, d).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_are_26_crockford_chars_and_unique() {
        let mut seen = HashSet::new();
        for _ in 0..2000 {
            let id = new_id();
            assert_eq!(id.len(), ID_LEN);
            assert!(id.bytes().all(|b| CROCKFORD.contains(&b)), "{id}");
            assert!(seen.insert(id), "duplicate id generated");
        }
    }

    #[test]
    fn ids_sort_by_time() {
        let a = encode_ulid(1_000, 0, 0);
        let b = encode_ulid(2_000, 0, 0);
        assert!(a < b);
    }

    #[test]
    fn safe_id_rejects_path_tricks() {
        assert!(is_safe_id(&new_id()));
        assert!(is_safe_id("01HZX-abc_9"));
        assert!(!is_safe_id(""));
        assert!(!is_safe_id("../x"));
        assert!(!is_safe_id("a/b"));
        assert!(!is_safe_id("a b"));
        assert!(!is_safe_id(&"a".repeat(65)));
    }

    #[test]
    fn iso8601_known_dates() {
        assert_eq!(iso8601_from_unix(0), "1970-01-01T00:00:00Z");
        // 2026-09-10T13:45:12Z
        assert_eq!(iso8601_from_unix(1_789_047_912), "2026-09-10T13:45:12Z");
        // Leap day.
        assert_eq!(iso8601_from_unix(1_709_164_800), "2024-02-29T00:00:00Z");
    }
}
