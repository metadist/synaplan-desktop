# Development

## Prerequisites

Run the setup script for your OS once (installs the Tauri build prerequisites,
Rust, and JS dependencies):

| OS | Script |
| -- | ------ |
| Linux | `bash scripts/setup-linux.sh` |
| macOS | `bash scripts/setup-macos.sh` |
| Windows (PowerShell) | `./scripts/setup-windows.ps1` |

You need **Node 22+** and the **Rust stable** toolchain. Per-OS system libraries
are listed in [`PLATFORMS.md`](./PLATFORMS.md).

## Dev loop

```bash
make dev              # recommended: plaintext key + http://localhost:8000 pre-filled
npm run tauri dev     # Vite + Tauri only (on WSL the key file is still used automatically)
```

- The frontend runs on `http://localhost:1420` (fixed port; Tauri loads it).
- Rust changes rebuild the native binary; Vue changes hot-reload.

Frontend-only work (no native window) can use `npm run dev` and a browser, but
the `@tauri-apps/api` calls only resolve inside the Tauri window (or the mock
below).

## Run without a server (mock)

```bash
npm run mock-server   # http://localhost:8788: pair, models, streamed chat, 403 /admin
```

Pair the app against `http://localhost:8788` with any code. This is the same
idea as the server repo's `_devextras/testing/messages-gateway/`.

## Run against a real Synaplan

1. On the Synaplan instance, enable `DESKTOP_AGENT.ENABLED` (see the server
   repo's `docs/DESKTOP.md`).
2. In Synaplan: **Channels → Desktop → Pair this computer** → get a code.
3. In the app: enter the instance address + code.

For quick iteration, set `VITE_SYNAPLAN_DEV_URL` in a local `.env.local` to
pre-fill the address field (dev-only; the runtime address always comes from
pairing):

```
VITE_SYNAPLAN_DEV_URL=http://localhost:8000
```

Pair against the **backend** URL (`http://localhost:8000` for the dev stack),
not the Vite web UI on `:5173`. The instance needs `DESKTOP_AGENT.ENABLED` on;
enable it in dev with:

```sql
INSERT INTO BCONFIG (BOWNERID, BGROUP, BSETTING, BVALUE)
VALUES (0,'DESKTOP_AGENT','ENABLED','1')
ON DUPLICATE KEY UPDATE BVALUE='1';
```

and make sure the server migrations have run (`make -C backend migrate` in the
`synaplan` repo) — without the `BDESKTOPDEVICES` table, pairing 500s.

## Headless Linux / WSL (no system keyring)

The API key is stored in the OS secret store. Headless Linux and WSL usually
have **no Secret Service** (`org.freedesktop.secrets`). On Linux the app now
falls back by itself to a `0600` key file and shows a warning — you do not have
to set an environment variable just to pair. `make dev` still sets
`SYNAPLAN_DESKTOP_ALLOW_PLAINTEXT_KEY=1` and pre-fills `http://localhost:8000`.

The key lives at `$XDG_CONFIG_HOME/synaplan-desktop/key.plaintext` (or
`~/.config/synaplan-desktop/key.plaintext`). The pairing screen also has an
**API key** tab so you can paste a key from **Channels → API keys** without a
pairing code. This fallback is refused by the unattended poll loop (Sprint B5).
On a normal desktop (GNOME/KDE/macOS/Windows) the OS keyring is used.

> Pairing codes are **one-time**. If an attempt fails after the server accepted
> the code, generate a fresh one for the next try.

## The gate

```bash
make ci-local
```

Mirrors CI exactly. Individual pieces:

```bash
make lint            # ESLint + Prettier
make type-check      # vue-tsc
make test            # Vitest
make guard-no-shell  # C12 guard
make rust-fmt        # cargo fmt --check
make rust-lint       # clippy (core)
make rust-test       # cargo test (core)
make build           # frontend build + workspace clippy + cargo build
make format          # auto-fix JS + Rust formatting
```

Rust core tests run without the webview toolchain, so `cargo test -p
synaplan-core` is fast on any machine. The full app build (`cargo build` in
`src-tauri`) needs the per-OS system libraries.

## Icons

Placeholder icons are generated from a source PNG:

```bash
node scripts/generate-icon.mjs        # writes src-tauri/icons/source.png
npx tauri icon src-tauri/icons/source.png   # expands to the platform icon set
```

Real branding replaces these before GA (Sprint B6).

## Tests

- **Rust:** `src-tauri/synaplan-core/src/**` (`#[cfg(test)]` modules) — paths,
  secret store, URL validation, hostname sanitisation, SSE parsing, frozen
  contract + checksums.
- **Frontend:** `tests/unit/**` (Vitest) — i18n parity, the Tauri service
  wrappers, and the Pair/Chat views with a mocked Rust side.
- All tests run offline and point every home-ish env var at a tempdir; a
  developer's real installation is never touched.
