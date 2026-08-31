# Platforms

Windows, macOS, and Linux are **all Tier 1**: a red platform blocks a release,
and unit tests (including, from Sprint B2, the path-confinement corpus) run on
all three in every PR.

| OS | Minimum | Architectures | Installer (Sprint B6) |
| -- | ------- | ------------- | --------------------- |
| Windows | 10 22H2 / 11 | x64 (arm64 build-only) | Signed MSI or NSIS |
| macOS | 13 Ventura | universal (arm64 + x64) | Signed + notarized `.dmg` |
| Linux | glibc 2.31+ (Ubuntu 22.04 baseline) | x64 (arm64 build-only) | AppImage + `.deb` |

Windows-on-ARM and Linux-on-ARM build in CI but are **not** part of the manual
acceptance matrix in v1.

## Build prerequisites

Handled by the setup scripts (`scripts/setup-*.{sh,ps1}`); listed here for
reference.

### Windows

- **MSVC C++ build tools** ("Desktop development with C++").
- **WebView2 runtime** (Evergreen; bundled into the installer for end users).
- Node 22+, Rust stable.

### macOS

- **Xcode Command Line Tools** (C toolchain + WebKit headers).
- Rust targets `aarch64-apple-darwin` and `x86_64-apple-darwin` for a universal
  release build.
- Node 22+.

### Linux (Ubuntu/Debian names; the script maps them for dnf/pacman)

- `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`,
  `patchelf`, `libxdo-dev`, `libssl-dev`, `libdbus-1-dev`, `pkg-config`,
  `build-essential`.
- Node 22+, Rust stable.

The platform-independent core (`synaplan-core`) needs only `libdbus-1-dev` +
`pkg-config` (for the Secret Service backend) and a C compiler on Linux — no
webview libraries — so its unit tests are cheap to run anywhere.

## Canonical directories

All paths come from one module (`synaplan-core::platform::app_dirs`); no other
code expands a home directory.

| Purpose | Windows | macOS | Linux |
| ------- | ------- | ----- | ----- |
| Config | `%APPDATA%\Synaplan\Desktop\config.toml` | `~/Library/Application Support/com.synaplan.desktop/config.toml` | `$XDG_CONFIG_HOME/synaplan-desktop/config.toml` |
| Skills | `%LOCALAPPDATA%\Synaplan\Desktop\skills\` | `~/Library/Application Support/com.synaplan.desktop/skills/` | `$XDG_DATA_HOME/synaplan-desktop/skills/` |
| Out-box | `%USERPROFILE%\Synaplan\out\` | `~/Synaplan/out/` | `~/Synaplan/out/` |
| Audit log | `%LOCALAPPDATA%\Synaplan\Desktop\logs\audit.log` | `~/Library/Logs/com.synaplan.desktop/audit.log` | `$XDG_STATE_HOME/synaplan-desktop/audit.log` |

The out-box deliberately sits in the user's home on all three so a generated file
is findable in Explorer/Finder. The bundle identifier `com.synaplan.desktop` and
the Windows vendor path `Synaplan\Desktop` are **permanent** — changing them
orphans an existing install.

## Secret storage

| OS | Backend |
| -- | ------- |
| Windows | Credential Manager (generic credential `Synaplan Desktop`, DPAPI) |
| macOS | login Keychain (item `com.synaplan.desktop`; ACL is bound to the signing identity — re-verify after signing lands in Sprint B6) |
| Linux | Secret Service (`libsecret` via GNOME Keyring / KWallet) |

Headless Linux has no Secret Service. The app fails with a named error unless
`SYNAPLAN_DESKTOP_ALLOW_PLAINTEXT_KEY=1` is set, which opts into a `0600`
plaintext file, warns at every start, and (from Sprint B5) is refused by the
unattended poll loop. A silent downgrade to plaintext is a bug.

## Signing & distribution (Sprint B6 — GA blocker)

Not built yet; recorded here so it is not discovered late:

- **Windows:** Authenticode (OV or EV) — unsigned means a SmartScreen wall.
- **macOS:** Developer ID + hardened runtime + **notarization + stapling** —
  unnotarized means Gatekeeper refuses to launch.
- **Linux:** detached GPG signature + published checksums.

Certificate procurement has weeks of lead time and is a Phase A-era task
(`P0`). No download link goes near the Synaplan UI/docs until an installer is
signed.

The actual certificates, keys, and passwords are kept **locally, never
committed**, in the gitignored `.signing/` directory — see
[`.signing/README.md`](../.signing/README.md) for the expected layout and how it
maps to the release workflow's GitHub Actions secrets (`DC28`).
