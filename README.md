# Synaplan Desktop

A small desktop app (Windows · macOS · Linux) that pairs with your Synaplan
workspace, lets you chat with your Synaplan account, and runs installed **Agent
Skills** on your own computer under a filesystem allowlist.

It uses **only your Synaplan account** — no Claude dashboard, no separate agent
host. It talks to Synaplan over the Anthropic-compatible Messages gateway
(`/v1/messages`) and the desktop pairing API. Your API key is stored in the OS
secret store (Windows Credential Manager · macOS Keychain · Linux Secret Service),
never in a config file.

> **Status:** early development (Phase B, Sprint B1). Pairing + streaming chat
> work; local skills, the skills manager, and the job poll loop come in later
> sprints. No signed installers yet — build from source (below).

## Quick start (developers)

Prerequisites are installed by the setup script for your OS:

```bash
# Linux
bash scripts/setup-linux.sh

# macOS
bash scripts/setup-macos.sh

# Windows (PowerShell)
./scripts/setup-windows.ps1
```

Then run the app:

```bash
npm run tauri dev
```

Pair it against a Synaplan instance that has **Desktop access** turned on
(`DESKTOP_AGENT.ENABLED`): open Synaplan in a browser → **Channels → Desktop** →
*Pair this computer* → type the address and code into the app.

### Try it offline (no server)

```bash
npm run mock-server          # starts http://localhost:8788
npm run tauri dev            # pair against http://localhost:8788 with any code
```

The mock server accepts pairing, lists a mock model, and streams a short chat
reply — enough to click through the UI without a live Synaplan.

## Development

See [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) for the dev loop and
[`docs/PLATFORMS.md`](docs/PLATFORMS.md) for per-OS build prerequisites.

Run the full local gate (mirrors CI) before every commit:

```bash
make ci-local
```

## Architecture

- **`src/`** — Vue 3 + TypeScript UI. Talks to Rust only through
  `src/services/tauri.ts`.
- **`src-tauri/synaplan-core/`** — platform-independent, unit-tested Rust core
  (paths, secret store, pairing, SSE parsing, config, frozen contract types).
- **`src-tauri/synaplan-core/src/platform/`** — the only place OS differences
  live (`app_dirs`, `secret_store`).
- **`src-tauri/src/`** — the thin Tauri shell (commands + events).

## License

Private / all rights reserved (for now).
