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

**Plan of record:** vendored in this repo at
[`_extras/planning/20260829-desktop-agent-client/`](_extras/planning/20260829-desktop-agent-client/)
(source of truth is the `synaplan` repo). This client is **Phase B**; the whole
server side (**Phase A**) is already merged and its job/check-in contract is
**frozen at `protocol: 1`**.

**User docs:** [docs.synaplan.com/desktop](https://docs.synaplan.com/desktop)
(overview, skills, folders, local tools).

---

## Start here (60-second orientation)

New to this repo — or driving it with an AI assistant ("vibe coding")? Read this,
then skim [`_extras/planning/…/00_master_plan.md`](_extras/planning/20260829-desktop-agent-client/00_master_plan.md).

**What the app is:** a Tauri window that (1) *pairs* with a Synaplan instance,
(2) *chats* through the instance's Messages gateway, and (3) — soon — *runs local
Agent Skills* under a strict folder allowlist. The security model is the product;
treat it as load-bearing, not a detail.

**Where things live (the mental model):**

| You want to change… | Go to… |
| ------------------- | ------ |
| The UI | `src/` (Vue). Talk to Rust **only** through `src/services/tauri.ts`. |
| Behavior needing the OS (files, processes, secrets, network) | `src-tauri/synaplan-core/` (pure, unit-tested), then add a thin `#[tauri::command]` in `src-tauri/src/commands/`. |
| Anything OS-specific (paths, secret store, later path confinement) | **only** `src-tauri/synaplan-core/src/platform/`. |
| User-facing text | all five locales in `src/i18n/` (a test enforces key parity). |

**The loop that keeps you safe:** small change → `make ci-local` → commit on a
branch → PR. Green locally ⇒ green CI.

## Working with an AI assistant (vibe coding)

This repo is friendly to AI-assisted development. A few habits keep it clean and
reviewable:

- **Small, reviewable steps.** One concern per PR — the plan is already sliced
  into `DC*` steps, so follow them. Don't let an assistant rewrite half the app
  in one commit.
- **The gate is the proof.** Always run `make ci-local` before committing; if the
  assistant says "done", that's what confirms it. Never commit red.
- **Don't invent the server contract.** Pairing / job / messages shapes are fixed
  by the `synaplan` server. The frozen fixtures in
  `tests/fixtures/desktop-contract/` are the source of truth — conform to them,
  never edit them to make code pass.
- **Keep platform code in one place.** A `#[cfg(target_os = …)]` for paths or
  secrets outside `platform/` is a bug — move it.
- **No secrets, ever.** Not the API key (OS secret store), not signing material
  (`.signing/` is gitignored). If an assistant tries to log or commit a key,
  stop it.
- **Ask before adding dependencies** (npm or cargo) and say so in the PR.
- **When unsure, read the plan** in `_extras/planning/` — it explains *why* a
  rule exists (especially the no-shell and path-confinement rules).

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
- `docs/LOCAL_TOOLS.md` — how local skills use Python/Node/LibreOffice: the
  doctor, the binary allowlist, and the no-shell execution model.
- `docs/SHOWCASE_SKILLS.md` — the curated catalog of demo skills (Win/mac), in
  waves by dependency, with the bundle priority.
- `_extras/planning/` — the vendored plan of record (read `00_master_plan.md`).
