---
name: Synaplan Desktop
description: Tauri 2 + Vue 3 desktop client that pairs with a Synaplan workspace and runs Agent Skills locally
---

# Synaplan Desktop — Development Guide

A small, separate desktop app (Windows, macOS, Linux) that pairs with a Synaplan
workspace, chats with the user's Synaplan account through the Anthropic-compatible
Messages gateway, and (from Sprint B2 on) runs installed **Agent Skills** on the
user's own computer under a filesystem allowlist.

**Stack:** Tauri 2 + Rust (path confinement, process spawn, OS secret store) +
Vue 3 / TypeScript / Vite. **It is not** an Electron wrap of the Synaplan web
SPA, and it never embeds `web.synaplan.com` in a WebView.

**Plan of record:** the `synaplan` repository, at
`_devextras/planning/20260829-desktop-agent-client/`. This client is **Phase B**;
the whole server side (**Phase A**) is already merged and its job/check-in
contract is **frozen at `protocol: 1`**.

## Critical rules

### Language

- **Code, comments, commit messages: always English.** Chat responses follow the
  user's language.

### Git — allowed, but never on `main`

- Feature branches + PRs only. **Never** commit or push directly to `main`, never
  force-push `main`.
- [Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`,
  `refactor:`, `docs:`, `chore:`, `test:`, `ci:`.
- **Never** add AI attribution ("Generated with…", "Co-Authored-By: …").
- Merge conflicts: merge both sides manually; never `git checkout --ours/--theirs`
  on code; if unsure, ask.

### The pre-commit gate — `make ci-local`

`make ci-local` is the gate and mirrors CI 1:1 (green locally ⇒ green CI). It runs:

| Step | Checks |
| ---- | ------ |
| `npm run lint` | ESLint + Prettier |
| `npm run type-check` | `vue-tsc` |
| `npm run test` | Vitest (incl. i18n parity) |
| `scripts/no-shell-guard.sh` | **C12** — no shell is ever constructed |
| `cargo fmt --all --check` | Rust formatting |
| `cargo clippy … -D warnings` | Rust lints (core, then whole workspace) |
| `cargo test -p synaplan-core` | Rust unit tests |
| `npm run build` + `cargo build` | Frontend + debug app build |

**CI runs the unit tests and the debug build on all three OSes
(`ubuntu-latest`, `windows-latest`, `macos-latest`) on every PR.** A control
verified on one OS is an assumption on the other two (invariant C11). Never
weaken the three-runner matrix; if minutes are tight, drop the debug build on
Windows/macOS to push-only — never the tests.

### Architecture — where things live

- **`src-tauri/synaplan-core/`** — the platform-independent, unit-tested core
  (paths, secret store, pairing, SSE, config, the frozen contract types). No
  Tauri dependency, so it compiles and tests fast on all three OSes.
- **`src-tauri/synaplan-core/src/platform/`** — the **only** place OS differences
  live (`app_dirs`, `secret_store`). No `#[cfg(target_os = …)]` for paths or
  secrets anywhere else. A reviewer reads one directory to know what differs.
- **`src-tauri/src/`** — a thin Tauri shell: wires core functions to
  `#[tauri::command]`s and events. No business logic, no platform branch.
- **`src/`** — Vue 3 (`<script setup lang="ts">`), talks to Rust only through
  `src/services/tauri.ts`.

### Runtime config & secrets

- **No `VITE_*` env var for the Synaplan URL** — it is unknown at build time and
  comes from pairing at runtime. Dev-only flags (e.g. `VITE_SYNAPLAN_DEV_URL` to
  pre-fill the address field) are OK.
- The scoped API key lives in the OS secret store (`SecretStore`), **never** in
  the config file, the webview, the audit log, a crash report, or a subprocess
  env. On 401 it is deleted, not left behind.
- **Never commit** an `sk_…` key, a keychain dump, or a pairing code.

### The frozen contract (C9)

- `tests/fixtures/desktop-contract/` is vendored **byte-for-byte** from the
  `synaplan` server repo (source commit recorded in its README). Do not edit
  these files — a change is a `protocol: 2` decision on the server side. A
  checksum test fails the build if a byte drifts.
- A job's device-facing input is only `{skill, prompt, fileIds}` — structurally
  enforced by `deny_unknown_fields`. There is no field through which a shell
  string can reach the computer.

### No shell, ever (C12)

- Local tools (Sprint B2+) take `{program, args[], workdir}`. `sh -c`, `cmd /c`,
  `powershell -Command`, and `osascript -e` are **never constructed**.
  `scripts/no-shell-guard.sh` greps for them in CI.

### i18n

- All user-facing strings via `vue-i18n`. **Update all five locales in the same
  commit:** `en`, `de`, `es`, `fr`, `tr` (`src/i18n/`). `tests/unit/i18n/`
  enforces full key parity.
- Never put a forbidden term in primary UI copy (see the plan's `12_ux_and_i18n.md`
  §2.1): `Claude`, `Anthropic`, `MCP`, `shell`, `Bash`, `tool_use`, `sk_`,
  `Tauri`, `lease`, etc. Show **platform-native paths** in the UI, never `~/…`
  on Windows.

## Boundaries

### Ask first before

- Adding dependencies (npm / cargo) or a native plugin.
- Changing the CI matrix, bundle identifier (`com.synaplan.desktop`), the vendor
  path (`Synaplan\Desktop`), or the `app_dirs` layout — these are permanent once
  a user installs.
- Widening what a skill may do, or touching the pairing/secret/execution paths.

### Never do

- Commit to `main`, force-push `main`, or add AI attribution.
- Commit secrets, keys, pairing codes, `dist/`, `src-tauri/target/`, or
  `node_modules/`. Code-signing material lives in the gitignored `.signing/`
  directory (only its `README.md` + `secrets.env.example` are tracked) and is
  **never** committed.
- Copy the Synaplan Vue SPA into this repo or add a Claude Code dependency.
- Add a `#[cfg(target_os)]` path/secret branch outside `platform/`.
- Skip the pre-commit gate.

## Detailed documentation

- `docs/DEVELOPMENT.md` — local setup, dev loop, the mock server.
- `docs/PLATFORMS.md` — per-OS build prerequisites and the signing plan.
