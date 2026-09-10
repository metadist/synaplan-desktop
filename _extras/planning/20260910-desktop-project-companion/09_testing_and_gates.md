# Testing and gates (both repositories)

This is the quality contract for [`00_master_plan.md`](./00_master_plan.md).
A slice is not done when a project switcher appears on one laptop. It
is done when this file’s gate is green and the breakdown status row
is ready to tick.

The 2026-08-29 testing doc still binds the **client**
([`../20260829-desktop-agent-client/09_testing_and_documentation.md`](../20260829-desktop-agent-client/09_testing_and_documentation.md)):
three OS runners, confinement corpus, no-shell grep, secret-store
rules. This file adds what **this** epic must prove.

---

## 1. Principles

1. **Two repos, two gates, never in the same PR.** `PS*` → Synaplan
   unfiltered gate. `PC*` → desktop `make ci-local`.
2. **Unfiltered gate = CI.** `--filter` is diagnostic only.
3. **Deterministic and offline in CI.** No live LLM, no real STT
   provider, no real microphone. Fixture HTTP + tempdirs.
4. **`protocol: 1` fixtures are input, not output.** Byte-identical
   (`C9`). A fixture edit is a different epic.
5. **Characterization snapshots unchanged** on `synaplan/` (`C3` /
   `C19`). Do not re-record.
6. **Five locales** in the same change (`PC14` on every UI slice).
7. **OpenAPI → Zod** on new/changed HTTP fields. Catalog and file
   hints are additive.
8. **Mobile impact** on new `synaplan/` PHP paths: `backend-only` in
   `.github/mobile-impact-policy.json` if the path is new.
9. **No secrets in fixtures.** Keys = `sk_test_…`; URLs =
   `https://synaplan.test`.
10. **Sovereignty is tested, not demonstrated.** Catalog shape, body
    model vs recipe, upload hint vs `getDefaultModel`, STT model +
    language — each has an automated assertion (`C15`, `C16`).
11. **API key never in JS.** Grep `src/` for `/v1/audio` `fetch` and
    for `sk_` .
12. **`projects_dir` only in `platform/app_dirs.rs`.** Grep the
    home-path string.

---

## 2. Mandatory gates

### 2.1 `synaplan/` (every `PS*` PR)

From `synaplan/` with Docker up:

```bash
make lint \
  && make -C backend phpstan \
  && make test \
  && docker compose exec -T frontend npm run check:types \
  && make -C frontend test
```

If OpenAPI changed:

```bash
make -C frontend generate-schemas
docker compose exec -T frontend npm run check:types
```

Planning-only files under `_extras/planning/` (this folder lives in
**desktop**) do not run the PHP gate. Server PRs that only touch
`docs/DESKTOP.md` still run whatever Synaplan docs lint the repo
uses; they do not skip PHP if PHP changed.

E2E on Synaplan: **not** required for desktop-only `/v1/` additions
unless a web page consumes them. `PS4` optional fields on upload are
web-visible — run Synaplan E2E only if an existing upload spec
breaks (it should not: additive).

### 2.2 `synaplan-desktop/` (every `PC*` PR)

```bash
make ci-local
```

Includes: lint/format, `vue-tsc`, Vitest, Rust tests, no-shell
guard (C12), debug build. CI still runs unit tests on Linux,
Windows, and macOS.

`PC14` is not a separate “run locales once at the end” step. **Every**
`PC*` that touches UI updates five locales and keeps the parity test
green.

Ask before adding Milkdown (`PC8`) or any extra audio crate (`PC12`
should not need one).

---

## 3. Compatibility regression (this epic)

| Inv. | Test | Where |
| ---- | ---- | ----- |
| C9 | Desktop job fixtures byte-identical | Checksum / vendor test — do not edit |
| C12 | No `sh -c` / `cmd /c` / `powershell -Command` / `osascript -e` | Existing grep guard |
| C13 | Paired key reaches `/v1/assistants`, `/v1/models/catalog`, `/v1/audio/*` | `ApiKeyScopeTest` + controller |
| C14 | Paired key **cannot** `/api/v1/agents*` | Same |
| C15 | Pinned agent does not replace body `model` | `PS2`/`PS5` + client capture |
| C16 | Upload with `vectorize_model` uses that BID | `PS4`/`PS5` |
| C17 | `projects_dir` only from `app_dirs` | Rust tests + grep |
| C18 | No key in JS, notes, chats, audit | Grep + review |
| C19 | Routing characterization empty diff; widget namespace clean | Existing suites |

---

## 4. Test matrix by step

| Step | Repo | Automated | Manual |
| ---- | ---- | --------- | ------ |
| `PS1` | synaplan | Flag on/off, publicView, 403 on `/api/v1/agents` | — |
| `PS2` | synaplan | Headers → options; body model wins; no-header identical | — |
| `PS3` | synaplan | Groups, catalog-key `id`, paired key 200, selectable only | — |
| `PS4` | synaplan | Hint used; omit = old default; bad key 400 | — |
| `PS5` | synaplan | Full matrix §3 | — |
| `PS6` | synaplan | Docs only | Read `DESKTOP.md` for `protocol: 1` sentence |
| `PC1` | desktop | CRUD, slug, Personal migration | — |
| `PC2` | desktop | `projects_dir` per OS | — |
| `PC3` | desktop | Commands + `tauri.ts` types | — |
| `PC4` | desktop | Nav views, i18n parity | Light + dark shell |
| `PC5` | desktop | Chat save/load | One restart |
| `PC6` | desktop | Persist eight keys; catalog parse | Comprehension gate (`05` §6) |
| `PC7` | desktop | Captured `model` = project CHAT | — |
| `PC8` | desktop | Note write/read | Milkdown caret |
| `PC9` | desktop | Form fields + EMBED refuse | Upload one PDF on a dev instance |
| `PC10` | desktop | Headers + world warning | — |
| `PC11` | desktop | Overlay ∩ install | — |
| `PC12` | desktop | Session body; commit math; no JS fetch | One EN + one DE take |
| `PC13` | desktop | Overflow routes to computer/doctor | — |
| `PC14` | desktop | Parity test on every UI PR | — |

---

## 5. Documentation table (same PR as the code)

| Doc | Owner | When |
| --- | ----- | ---- |
| `docs/DESKTOP.md` machine API | synaplan | `PS6` (and snippets in `PS1`–`PS4` if you prefer not to wait) |
| This planning folder status lines | desktop | When a step merges |
| `AGENTS.md` / `docs/DEVELOPMENT.md` note on projects dir | desktop | `PC2` |
| Five locale files | desktop | Every `PC*` with copy |

User-facing `docs/**` rides with the code PR.

---

## 6. Definition of done (every `PS*` / `PC*` step)

1. Gate of the touched repo is green (unfiltered).
2. New branches have tests; sovereignty paths have a regression test.
3. OpenAPI + schemas if HTTP changed.
4. Five locales if copy changed.
5. PR lists which invariants the diff can touch (`C9`, `C13`–`C18`).
6. Characterization diff is empty (`PS*` only).
7. Docs in the same PR when behaviour is user- or contract-visible.
8. No `sk_` or pairing codes in the diff.
9. `PS*`: no `pairingScopes()` change; no job-fixture change.
10. `PC*`: no `synaplan/` file in the diff.
11. `PC*`: no new `#[cfg(target_os)]` outside `platform/`; green on
    the OS matrix CI already requires.
12. `PC8`: npm dep asked for, not sneaked in.

---

## 7. Local traps

- Synaplan: `var/cache/test` permissions after `compose down`
  (house `AGENTS.md`).
- Desktop: redirect **every** home-ish env var in tests (`04` §2).
- Dictation tests must not open a real mic in CI.
- Catalog tests must not call `/api/v1/config/models` with a paired
  key and call that “good enough”.
- Windows + macOS still run Rust project-path tests — a Linux-only
  `projects_dir` assertion is incomplete (`C17`).
