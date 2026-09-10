# Local persistence

**Status:** Draft 2026-09-10.
**Depends on:** checklist rows 1, 4, 5, 7, 17; [`01_product_model.md`](./01_product_model.md).
**Unlocks:** `PC1`–`PC5`, notes, files-on-disk.
**Repos:** `synaplan-desktop` only.
**Done:** projects, chats, and notes survive restart; the API key is
never in those files; `projects_dir` exists only in `app_dirs`.

---

## 0. Why this file exists

The current client stores `config.toml` (including `last_chat_model`)
and skills metadata. Chat is **in memory**. There is no project
directory. This epic adds a projects home next to the out-box and a
metadata tree next to config — without renaming the four existing
`app_dirs` roots (C17).

---

## 1. Code to read first

| Path | Why |
| ---- | --- |
| `src-tauri/synaplan-core/src/platform/app_dirs.rs` | **Only** place a new directory may be born |
| `src-tauri/synaplan-core/src/config.rs` | `last_chat_model` migration source |
| `src-tauri/synaplan-core/src/platform/confinement.rs` | `default_deny_globs()` still apply |
| `src-tauri/src/commands/mod.rs` | Thin commands; logic stays in `synaplan-core` |

---

## 2. `projects_dir` (`PC2`)

Add a fifth resolved path **beside** the out-box, still under the
user-visible `Synaplan` folder:

| OS | Path (shorthand) |
| -- | ---------------- |
| Windows | `%USERPROFILE%\Synaplan\projects\` |
| macOS | `~/Synaplan/projects/` |
| Linux | `~/Synaplan/projects/` |

UI always shows the **platform-native** string from Rust. Vue never
concatenates `~/`.

```text
AppDirs {
    config_dir,     // unchanged
    skills_dir,     // unchanged
    outbox_dir,     // unchanged  ~/Synaplan/out
    audit_dir,      // unchanged
    projects_dir,   // NEW        ~/Synaplan/projects
}
```

Rules:

- Field added **only** in `platform/app_dirs.rs` (+ tests per OS).
- Grep in CI / PR review: no `Synaplan/projects` string outside
  `app_dirs` and this planning folder.
- Bundle id `com.synaplan.desktop` and vendor `Synaplan\Desktop`
  unchanged.
- Tests build `AppDirs` from a temp `Env` (existing pattern). Setting
  only `HOME` is not enough on Windows.

Computer-level out-box (`~/Synaplan/out`) stays for skill artifacts
that are not project-scoped (web-queued jobs). Project skill runs
that the user started **inside** a project write to
`{projects_dir}/{slug}/out/` as an extra write root for that project
only (`PC11`).

---

## 3. Layout

```text
{config_dir}/
  config.toml
  projects/
    index.json
    {id}.toml
    {id}/
      chats/
        {chatId}.json

{projects_dir}/
  {slug}/
    notes/
      *.md
    out/
```

`index.json` (v1):

```json
{
  "version": 1,
  "personal_id": "<id>",
  "migration": { "personal_v1": true },
  "order": ["<id>", "<id>"],
  "active_id": "<id>"
}
```

`{id}.toml` — see [`01_product_model.md`](./01_product_model.md) §2
and [`02_sovereign_models.md`](./02_sovereign_models.md) §8. No API
key. No pairing code. No `sk_`.

Chat JSON:

```json
{
  "id": "<chatId>",
  "title": "…",
  "created_at": "…",
  "updated_at": "…",
  "model": "<catalog key or providerId sent>",
  "assistant_id": null,
  "messages": [
    { "role": "user", "content": "…" },
    { "role": "assistant", "content": "…" }
  ],
  "artifacts": []
}
```

Notes: raw Markdown files. Title = first `# heading` or filename.
No sidecar required in v1; optional `notes/index.json` for pin/order
is allowed if it stays local.

---

## 4. Slug rules (`PC1`)

- Lowercase ASCII `[a-z0-9-]`, collapse spaces, strip diacritics.
- Max 64 chars. Empty → `project`.
- Unique among existing slugs. Collision → `-2`, `-3`.
- Reserved: `.`, `..`, Windows device names (`con`, `prn`, …).
- `personal` reserved for the Personal project.

Directory create is `create_dir_all` on notes + out. Never follow
symlinks out of `projects_dir` (confinement).

---

## 5. Personal migration (`PC1`)

When `projects/index.json` is missing:

1. New id, slug `personal`, `kind = personal`.
2. `models.chat` ← `last_chat_model` (see sovereign §8 for legacy
   providerId).
3. `enabled_skills` ← currently enabled skill names.
4. Create disk folders.
5. `active_id` = Personal.
6. Write index + toml atomically (temp + rename).

Idempotent: if the marker is set, do nothing. If the file is
corrupt, do not overwrite notes; surface an error and offer a new
empty index **without** deleting `{projects_dir}`.

---

## 6. Confinement

Path checks stay canonicalize → contain.

| Root | Access |
| ---- | ------ |
| `{projects_dir}/{slug}/notes` | Read/write for that project's note commands |
| `{projects_dir}/{slug}/out` | Write target for that project's local skills |
| `{config_dir}/projects/{id}` | App-only; not offered as a user write folder |
| Anything else under `projects_dir` | Not readable via another project's commands |

`default_deny_globs()` still apply (no writing `.env` into notes as
an escape — deny globs win).

Reveal-in-folder is allowed for notes and project out. Reveal of
`config_dir` is not a v1 button.

---

## 7. Rust module sketch (`PC1`)

`synaplan-core` module `projects` (names indicative):

| fn | Role |
| -- | ---- |
| `load_index` / `save_index` | index.json |
| `list_projects` | order from index |
| `create_project(name, language)` | id, slug, dirs, toml |
| `get_project` / `update_project` | metadata + models |
| `delete_project` | metadata; optional trash notes |
| `ensure_personal` | migration |
| `set_active` | index.active_id |
| `list_chats` / `load_chat` / `save_chat` | `PC5` may live here or `projects::chats` |

All paths come from `AppDirs`. Unit tests use temp `Env`.

The Tauri shell (`PC3`) exposes these as commands. Vue uses only
`tauri.ts` wrappers. No `fs` plugin from JS onto `projects_dir`.

---

## 8. What we will not persist

- API keys, pairing codes, tokens.
- Raw PCM / dictation blobs after the transcript is inserted (do not
  keep a secret audio archive in v1).
- Server file bytes (they live on Synaplan). A local “uploaded from”
  path is optional metadata.
- Assistant drafts.

---

## 9. Non-goals

- SQLite. JSON + TOML + Markdown is enough.
- Encrypting notes at rest (OS user account is the boundary).
- Sync / iCloud special cases beyond existing known-folder honesty.
- Changing `outbox_dir` to live under `projects_dir`.

---

## 10. Acceptance

- `AppDirs` tests on three OS expectations include `projects_dir`.
- `ensure_personal` tested: inherits model + skills; second call no-op.
- Slug collision and reserved-name tests.
- A save/load chat round-trip test (`PC5`).
- Grep: no home-path expansion outside `platform/`.
- Fixture files contain no `sk_`.
