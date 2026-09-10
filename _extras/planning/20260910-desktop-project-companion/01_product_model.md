# Product model — Project as the local unit

**Status:** Draft 2026-09-10.
**Depends on:** master-plan checklist rows 1–10, 26–27.
**Unlocks:** persistence (`04`), shell (`05`), chats, notes, files, Agents.
**Repos:** mostly `synaplan-desktop`. The server never stores a Project.
**Done:** a reviewer can answer “what is a project?” and “what is *not*
a project?” without reading code.

---

## 0. Why this file exists

The 2026-08-29 client is **computer-scoped**: one pairing, one in-memory
chat, one global enabled-skill set, one `last_chat_model`. That is a
good pairing story and a bad work story. People already keep more than
one piece of work on one laptop. This epic makes **Project** the unit
they switch, not “the app”.

A Project is **not** a Synaplan user, not a Synaplan Assistant, not a
knowledge folder, and not a Skill. Those are things a project *uses*.

---

## 1. Code to read first

| Path | Why |
| ---- | --- |
| `src-tauri/synaplan-core/src/config.rs` | `last_chat_model`, global enabled skills — migration source |
| `src-tauri/synaplan-core/src/platform/app_dirs.rs` | Add `projects_dir` here only |
| `src/stores/ui.ts` | `View = 'chat' \| 'skills' \| 'computer' \| 'doctor'` — replace |
| `src/views/ChatView.vue` | In-memory transcript — becomes per-project |
| `src/services/tauri.ts` | The only JS→Rust door |
| `src-tauri/synaplan-core/src/files.rs` | Today's `process_level=store`, no `group_key` |

---

## 2. What a Project is

A Project is a **local record** plus a **local directory** plus a
**pointer** at a Synaplan knowledge folder.

### 2.1 Identity

| Field | Rules |
| ----- | ----- |
| `id` | Stable ULID / UUID generated on this computer. Never reused. This is the `{projectId}` in `DESKTOP:{projectId}`. |
| `slug` | Filesystem-safe, unique among projects on this computer. Derived from the name; collision → `-2`, `-3`. Shown in paths. |
| `name` | User-facing. Rename does **not** change `id`. Slug may follow a rename only if the directory can be renamed atomically and no file is open; otherwise keep the slug and show the new name. Prefer **keep slug** in v1 (simpler, no broken bookmarks). |
| `created_at` / `updated_at` | ISO-8601 in the project file. |
| `dictation_language` | ISO 639-1 (optionally `de-DE`). Independent of UI locale. |
| `default_assistant_id` | Optional Synaplan Assistant id. |
| `assistant_ids` | Optional extra binds (several allowed; one default). |
| `enabled_skills` | Overlay: skill **names** that may run in this project. Absent name → not enabled here, even if installed. |
| `models` | The eight-slot matrix. See [`02_sovereign_models.md`](./02_sovereign_models.md). |
| `knowledge_folder` | Always `DESKTOP:{id}`. Not user-editable in v1. |

`id` is the sovereignty and RAG handle. `slug` is only for humans and
disk. Do not put the slug in `group_key` — a rename would orphan
vectors.

### 2.2 What lives on this computer

| Thing | Where | Survives sign-out? |
| ----- | ----- | ------------------ |
| Index of projects | `{config_dir}/projects/index.json` | Yes |
| Project metadata + bindings | `{config_dir}/projects/{id}.toml` | Yes |
| Chat threads | `{config_dir}/projects/{id}/chats/{chatId}.json` | Yes |
| Notes (user-visible Markdown) | `{projects_dir}/{slug}/notes/*.md` | Yes |
| Project out folder | `{projects_dir}/{slug}/out/` | Yes |
| In / Out activity (derived) | Recomputed from files API + local out + chat artifacts | Partial |

Sign-out **deletes the API key** (existing rule). It does **not** delete
projects, notes, or chats. The next pair on the same computer sees the
same projects. Copy must say that chats and notes stay on this
computer; they are not “in your Synaplan account”.

### 2.3 What lives on Synaplan

| Thing | Handle | Scope |
| ----- | ------ | ----- |
| Vectorized files + optional shared notes | `group_key = DESKTOP:{projectId}` | That account's files |
| Assistant recipes | numeric id | Account / gallery publicView |
| Model catalog | catalog keys | What the instance will run |
| Inference, STT, embeddings | the project bindings | Paid / limited by the account |

There is **no** `BDESKTOPPROJECTS` table in this epic.

---

## 3. Computer-level vs project-level

| Concern | Level | UI home |
| ------- | ----- | ------- |
| Pairing, revoke, instance URL | Computer | Pair screen + footer |
| Secret / API key | Computer | Secret store — never UI |
| Skill *install* / remove / bundled | Computer | Overflow → Skills install (today's `SkillsView` install flow) |
| Skill *enablement* | Project | Agents |
| Global deny globs, execution consent | Computer | This computer |
| Doctor / Check this computer | Computer | Overflow / footer |
| Allowlisted extra folders | Computer | This computer (unchanged) |
| Chat threads, notes, knowledge folder | Project | Chat / Notes / Files |
| This project's models | Project | Models |
| Bound Assistants | Project | Agents |
| Dictation language | Project | Models or project settings (one place — Models is fine) |

Do not invent a second “default model” on the computer after Personal
exists. `last_chat_model` remains as a migration seed and as a
fallback write when the user changes Personal's CHAT slot (optional;
do not require it).

---

## 4. First-launch migration

On first launch after this epic lands (detect: `projects/index.json`
missing):

1. Create project `id` = new ULID, `slug` = `personal`, `name` =
   **Personal** (i18n at display time; stored name may be `Personal`
   and translated in the switcher via a `kind: personal` flag so a
   German UI shows **Persönlich** without renaming the folder).
2. Copy `config.toml` `last_chat_model` into `models.chat` if set.
   Other slots start **unset**.
3. Copy the global enabled-skill set into `enabled_skills`.
4. Create `{projects_dir}/personal/notes/` and `…/out/`.
5. Do **not** invent a fake chat transcript. The empty Chat view is
   the current empty state, now inside Personal.
6. Write a `migration.personal_v1 = true` marker on the index so this
   runs once.

If the user has never paired, still create Personal — notes do not
need a key. Catalog fetch waits for pair.

Deleting Personal: allowed only if at least one other project exists,
and only through `useDialog` with a danger confirm. Never auto-delete.

---

## 5. Lifecycle

### Create

Name + dictation language. Models may be copied from “use the same
models as {project}” or left unset until the Models panel. Do not
block create on the catalog (offline / unpaired).

### Switch

Switching project flushes any in-memory composer draft into that
project's “draft” slot (optional v1: discard with a confirm if dirty).
Chat view loads that project's thread list. Notes view lists that
slug's `notes/`.

### Delete

Confirm. Delete metadata + chats under `config_dir`. Offer “also move
the notes folder to Trash” vs “leave the files on disk”. Do **not**
silently delete Synaplan files in `DESKTOP:{id}` in v1 — say “Files
already sent to Synaplan stay in that knowledge folder. Remove them
from Synaplan on the web if you want them gone.” Honest > tidy.

### Unpair / re-pair

Projects stay. After re-pair to the **same** account, knowledge folder
and Assistants still make sense. After re-pair to a **different**
account, Assistants ids and file ids are foreign — mark binds
stale, do not call them. Notes and chats remain.

---

## 6. Empty states

| Surface | Empty copy intent (EN) |
| ------- | ---------------------- |
| No projects (should not happen after migration) | Create a project to keep chats and notes together. |
| New project chat | Start a chat in {name}. Answers use this project's models. |
| Notes | Write a note, or dictate. Files stay on this computer. |
| Files | Add files to this project's knowledge folder. They are sent to your Synaplan workspace and indexed with the model you picked for this project. |
| Agents | Bind an Assistant from your workspace, or enable a Skill installed on this computer. |
| Models | Choose the models that may process this project's data. |

---

## 7. Non-goals

- Syncing a project to another computer.
- Nested projects or folders-of-projects.
- Sharing a project id with another Synaplan user.
- Making `group_key` user-editable.
- Importing a Synaplan web “workspace” or widget as a project.
- Treating an Assistant as a project.

---

## 8. Acceptance (this file)

A `PC1` implementation matches §2–§5:

- Unique `id` + unique `slug`.
- Personal migration once, inheriting chat model + enabled skills.
- No server round-trip to create a project.
- Knowledge folder string is exactly `DESKTOP:{id}`.
- Rename does not change `id` (v1: does not change `slug` either).
