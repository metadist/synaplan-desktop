//! `SecretStore` (DC23) — the Synaplan API key never touches the config file. It
//! lives in the OS secret store: Windows Credential Manager (DPAPI), macOS
//! login Keychain, or the Linux Secret Service (GNOME Keyring / KWallet).
//!
//! One trait, three native backends selected per target, plus an in-memory
//! double for CI (a headless runner has no Secret Service session) and an
//! explicitly opt-in **or missing-keyring** plaintext fallback for headless
//! Linux / WSL (cross-platform §4). A working OS keyring is never skipped. The
//! unattended poll loop (Sprint B5) refuses a plaintext key.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use thiserror::Error;

/// The account/user component of the secret-store entry (same on every OS).
const ACCOUNT: &str = "api-key";

/// The OS-visible service/label of the secret-store entry. Per cross-platform
/// §4: a generic credential named `Synaplan Desktop` on Windows, a Keychain
/// item `com.synaplan.desktop` on macOS.
#[cfg(target_os = "macos")]
const SERVICE: &str = "com.synaplan.desktop";
#[cfg(not(target_os = "macos"))]
const SERVICE: &str = "Synaplan Desktop";

/// The environment variable that opts into the Linux plaintext fallback.
pub const PLAINTEXT_OPT_IN_VAR: &str = "SYNAPLAN_DESKTOP_ALLOW_PLAINTEXT_KEY";

#[derive(Debug, Error)]
pub enum SecretStoreError {
    #[error("secret store backend error: {0}")]
    Backend(String),
    #[error(
        "no OS secret store is available on this machine; on headless Linux set {} =1 to allow a 0600 plaintext file fallback",
        PLAINTEXT_OPT_IN_VAR
    )]
    Unavailable,
    #[error("io error: {0}")]
    Io(String),
}

/// Store, read, and delete a single secret (the scoped Synaplan API key).
pub trait SecretStore: Send + Sync {
    /// Return the stored key, or `None` if nothing is stored.
    fn get(&self) -> Result<Option<String>, SecretStoreError>;
    /// Store (overwrite) the key.
    fn set(&self, key: &str) -> Result<(), SecretStoreError>;
    /// Delete the key (a no-op if there is nothing stored). Called on 401.
    fn delete(&self) -> Result<(), SecretStoreError>;
    /// A short backend name for diagnostics ("keyring", "memory", "plaintext").
    fn backend_name(&self) -> &'static str;
    /// True only for the insecure Linux plaintext fallback. The unattended poll
    /// loop (B5) refuses to run when this is true.
    fn is_plaintext(&self) -> bool {
        false
    }
    /// Absolute path of the plaintext key file, when this backend is the
    /// fallback. Used so the UI can point the developer at the file.
    fn plaintext_path(&self) -> Option<PathBuf> {
        None
    }
}

// ---------------------------------------------------------------------------
// Native OS keyring backend
// ---------------------------------------------------------------------------

/// The real OS-backed secret store (Credential Manager / Keychain / Secret
/// Service) via the `keyring` crate.
pub struct KeyringSecretStore;

impl KeyringSecretStore {
    pub fn new() -> Result<Self, SecretStoreError> {
        Ok(Self)
    }

    fn entry(&self) -> Result<keyring::Entry, SecretStoreError> {
        keyring::Entry::new(SERVICE, ACCOUNT).map_err(|e| SecretStoreError::Backend(e.to_string()))
    }
}

impl SecretStore for KeyringSecretStore {
    fn get(&self) -> Result<Option<String>, SecretStoreError> {
        match self.entry()?.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(keyring::Error::NoStorageAccess(_)) | Err(keyring::Error::PlatformFailure(_)) => {
                Err(SecretStoreError::Unavailable)
            }
            Err(e) => Err(SecretStoreError::Backend(e.to_string())),
        }
    }

    fn set(&self, key: &str) -> Result<(), SecretStoreError> {
        match self.entry()?.set_password(key) {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoStorageAccess(_)) | Err(keyring::Error::PlatformFailure(_)) => {
                Err(SecretStoreError::Unavailable)
            }
            Err(e) => Err(SecretStoreError::Backend(e.to_string())),
        }
    }

    fn delete(&self) -> Result<(), SecretStoreError> {
        match self.entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(keyring::Error::NoStorageAccess(_)) | Err(keyring::Error::PlatformFailure(_)) => {
                Err(SecretStoreError::Unavailable)
            }
            Err(e) => Err(SecretStoreError::Backend(e.to_string())),
        }
    }

    fn backend_name(&self) -> &'static str {
        "keyring"
    }
}

// ---------------------------------------------------------------------------
// In-memory double (tests + CI)
// ---------------------------------------------------------------------------

/// A process-local secret store used in CI, where a headless runner has no
/// Secret Service session. Never used in a shipped build.
#[derive(Default)]
pub struct InMemorySecretStore {
    value: Mutex<Option<String>>,
}

impl InMemorySecretStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecretStore for InMemorySecretStore {
    fn get(&self) -> Result<Option<String>, SecretStoreError> {
        Ok(self
            .value
            .lock()
            .expect("secret store mutex poisoned")
            .clone())
    }

    fn set(&self, key: &str) -> Result<(), SecretStoreError> {
        *self.value.lock().expect("secret store mutex poisoned") = Some(key.to_string());
        Ok(())
    }

    fn delete(&self) -> Result<(), SecretStoreError> {
        *self.value.lock().expect("secret store mutex poisoned") = None;
        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "memory"
    }
}

// ---------------------------------------------------------------------------
// Plaintext fallback (headless Linux, opt-in only)
// ---------------------------------------------------------------------------

/// A `0600` plaintext file fallback for headless Linux with no Secret Service.
/// Only ever selected when the user explicitly sets [`PLAINTEXT_OPT_IN_VAR`],
/// and it warns on construction. Refused by the unattended poll loop (B5).
pub struct PlaintextSecretStore {
    file: PathBuf,
}

impl PlaintextSecretStore {
    pub fn new(config_dir: &Path) -> Result<Self, SecretStoreError> {
        Ok(Self {
            file: config_dir.join("key.plaintext"),
        })
    }
}

impl SecretStore for PlaintextSecretStore {
    fn get(&self) -> Result<Option<String>, SecretStoreError> {
        match std::fs::read_to_string(&self.file) {
            Ok(v) => Ok(Some(v.trim().to_string()).filter(|s| !s.is_empty())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(SecretStoreError::Io(e.to_string())),
        }
    }

    fn set(&self, key: &str) -> Result<(), SecretStoreError> {
        if let Some(parent) = self.file.parent() {
            std::fs::create_dir_all(parent).map_err(|e| SecretStoreError::Io(e.to_string()))?;
        }
        std::fs::write(&self.file, key).map_err(|e| SecretStoreError::Io(e.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&self.file, std::fs::Permissions::from_mode(0o600))
                .map_err(|e| SecretStoreError::Io(e.to_string()))?;
        }
        Ok(())
    }

    fn delete(&self) -> Result<(), SecretStoreError> {
        match std::fs::remove_file(&self.file) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(SecretStoreError::Io(e.to_string())),
        }
    }

    fn backend_name(&self) -> &'static str {
        "plaintext"
    }

    fn is_plaintext(&self) -> bool {
        true
    }

    fn plaintext_path(&self) -> Option<PathBuf> {
        Some(self.file.clone())
    }
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/// Whether the plaintext fallback should be selected. Kept as a pure decision
/// (separate from reading the process environment) so it is testable without
/// mutating global state.
///
/// On Linux: the env opt-in, **or** a missing/broken Secret Service (typical
/// WSL / headless). A working keyring is never silently skipped.
/// Elsewhere: never — Credential Manager / Keychain are always used.
pub fn plaintext_selected(opt_in_flag: bool, keyring_available: bool) -> bool {
    cfg!(target_os = "linux") && (opt_in_flag || !keyring_available)
}

fn plaintext_opt_in_from_env() -> bool {
    std::env::var(PLAINTEXT_OPT_IN_VAR)
        .map(|v| v == "1")
        .unwrap_or(false)
}

fn keyring_is_available() -> bool {
    match KeyringSecretStore::new() {
        Ok(store) => store.get().is_ok(),
        Err(_) => false,
    }
}

fn open_plaintext_store(config_dir: &Path) -> Result<Box<dyn SecretStore>, SecretStoreError> {
    eprintln!(
        "WARNING: Synaplan Desktop will store the API key in a 0600 plaintext file under {}, \
         not the OS secret store. This is refused by the unattended poll loop. \
         Set {}=1 explicitly, or install a Secret Service, to choose this path on purpose.",
        config_dir.display(),
        PLAINTEXT_OPT_IN_VAR
    );
    Ok(Box::new(PlaintextSecretStore::new(config_dir)?))
}

/// Build the appropriate secret store for a real run. On Linux this honours the
/// opt-in plaintext fallback and also uses it when no Secret Service is
/// available (WSL / headless); everywhere else it is always the native OS store.
pub fn default_secret_store(config_dir: &Path) -> Result<Box<dyn SecretStore>, SecretStoreError> {
    let opt_in = plaintext_opt_in_from_env();
    let keyring_ok = keyring_is_available();
    if plaintext_selected(opt_in, keyring_ok) {
        return open_plaintext_store(config_dir);
    }
    let _ = config_dir; // unused on the native path
    Ok(Box::new(KeyringSecretStore::new()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_roundtrip() {
        let store = InMemorySecretStore::new();
        assert_eq!(store.get().unwrap(), None);
        store.set("sk_test_abc").unwrap();
        assert_eq!(store.get().unwrap().as_deref(), Some("sk_test_abc"));
        store.delete().unwrap();
        assert_eq!(store.get().unwrap(), None);
        assert!(!store.is_plaintext());
    }

    #[test]
    fn plaintext_is_never_selected_when_keyring_works_without_optin() {
        assert!(!plaintext_selected(false, true));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn plaintext_selected_on_linux_with_optin() {
        assert!(plaintext_selected(true, true));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn plaintext_selected_on_linux_when_keyring_missing() {
        assert!(plaintext_selected(false, false));
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn plaintext_never_selected_off_linux() {
        assert!(!plaintext_selected(true, false));
    }

    #[test]
    fn plaintext_store_roundtrip_and_flag() {
        let dir = tempfile::tempdir().unwrap();
        let store = PlaintextSecretStore::new(dir.path()).unwrap();
        assert_eq!(store.get().unwrap(), None);
        store.set("sk_test_plain").unwrap();
        assert_eq!(store.get().unwrap().as_deref(), Some("sk_test_plain"));
        assert!(store.is_plaintext());
        assert_eq!(
            store.plaintext_path().as_deref(),
            Some(dir.path().join("key.plaintext").as_path())
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.path().join("key.plaintext"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600, "plaintext key file must be 0600");
        }

        store.delete().unwrap();
        assert_eq!(store.get().unwrap(), None);
    }
}
