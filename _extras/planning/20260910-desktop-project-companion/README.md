# Synaplan Desktop — project companion

**Status:** Plan drafted 2026-09-10. Research only until the decision
checklist in [`00_master_plan.md`](./00_master_plan.md) is ticked. No
product code in this change.
**Product:** turn the already-shipped Synaplan Desktop client into a
**local-first project companion**. Each project on this computer owns
chats, Markdown notes, dictation, a knowledge folder on the paired
Synaplan instance, bound Assistants, a local-skill overlay, and — first
class — **this project's models**.
**Builds on:** [`20260829-desktop-agent-client`](../20260829-desktop-agent-client/README.md)
(pairing, scoped keys, `protocol: 1`, path confinement, secret store,
`app_dirs`). That epic is frozen. This folder does not reopen it.

> **The ask:** after pairing, the unit of work is a **Project** on this
> computer. Synaplan supplies models, Assistants, and vectorized
> knowledge. The computer supplies notes, local skills, and file I/O.
> The user must be able to say, per project, **which models process
> that project's data** — chat, voice, vision, images, video, embeddings,
> documents — so the data stays in a defined world.

This is **planning only**. Do not change product code from this folder.
Do not commit these files unless a human asks.

---

## Executive recommendation

**Yes — and it is a new epic on the existing client, not a rewrite, not
a wrap of the web SPA, and not a `protocol: 2`.**

The Phase B client already pairs, chats through `POST /v1/messages`,
runs installed Agent Skills under confinement, and uploads artifacts
with `desktop:files`. What it does not have is a **project**: persisted
threads, notes on disk, a knowledge folder, Assistant binding, or a
per-project model matrix.

Local-first does **not** need a Project entity on the server. It does
need a few **additive `/v1/` endpoints** so existing paired keys keep
working (no re-pair). Do **not** add `desktop:agents` to
`pairingScopes()`.

**Sovereignty is not a settings nicety.** A single “preferred chat
model” is not enough. If file indexing silently uses the account
`VECTORIZE` default, or an Assistant silently swaps in its own chat
model, the project has left the world the user picked. The binding
rule is in [`02_sovereign_models.md`](./02_sovereign_models.md) and is
not optional.

---

## Order of work — smallest server first, then the client

This epic is **not** a second “create the repo after the server”
story. The client already exists. Server steps are small, stay under
`/v1/`, and must land before the client slices that consume them.

**Phase S — `synaplan/` (`PS1`–`PS6`)**

1. List Assistants for a desktop key (`GET /v1/assistants`).
2. Pin an Assistant + a knowledge folder on `POST /v1/messages`
   (headers, Anthropic SSE unchanged).
3. Capability-grouped model catalog for a desktop key
   (`GET /v1/models/catalog`).
4. File upload/process accepts explicit vectorize / analyze model
   hints so **this project's models** apply to RAG.
5. Scope, catalog, pin, and upload-hint tests + OpenAPI.
6. `docs/DESKTOP.md` — desktop-only, no `protocol: 2`.

**Phase C — `synaplan-desktop` (`PC1`–`PC14`)**

1. Project core + `projects_dir` + Tauri commands.
2. Project-first shell (Chat / Notes / Files / Agents / Models).
3. Persistent chats; sovereign model panel; pass the project CHAT
   model on every turn.
4. Notes (Milkdown — ask first) and knowledge-folder uploads.
5. Assistant bind (project models win) + skill overlay + In/Out.
6. Dictation (sani-sis pattern, project VOICE model + language).
7. Computer / Doctor stay reachable; locale parity on every slice.

### Why this order

| Reason | Consequence |
| ------ | ----------- |
| Existing keys already have `desktop:messages` and `desktop:files` | New routes must live under `/v1/` or `/api/v1/files*` — no new pairing scope |
| Catalog and upload hints are what make sovereignty real | Shipping Assistants without them lets a recipe or the account default leave the project's world |
| `protocol: 1` stays frozen | Headers on `/v1/messages` are not a job-shape change |
| One ID per PR | A reviewer can revert a slice without collapsing the epic |
| Five locales in the same change | A Models panel that exists only in English is unfinished |

**Trade-off accepted:** Phase S ships endpoints the current client does
not call yet. That is cheaper than a re-pair, and cheaper than teaching
desktop to impersonate a cookie session.

Until Phase S merges, the client can still ship project CRUD, the new
shell, persisted chats, and notes. The Assistants picker, the Models
catalog, and knowledge-folder indexing wait on `PS1`–`PS4`.

---

## How to read this folder

| File | Role |
| ---- | ---- |
| [`00_master_plan.md`](./00_master_plan.md) | Decisions, architecture, two-repo split, non-goals. **Tick the checklist before any code.** |
| [`01_product_model.md`](./01_product_model.md) | Project as the local unit. What lives on disk vs on Synaplan. |
| [`02_sovereign_models.md`](./02_sovereign_models.md) | **Binding.** Per-project model matrix. Catalog. Project models win. |
| [`03_server_v1_extensions.md`](./03_server_v1_extensions.md) | `/v1/assistants`, message headers, catalog, upload hints. |
| [`04_local_persistence.md`](./04_local_persistence.md) | `projects_dir`, index, Personal migration, confinement. |
| [`05_ux_and_i18n.md`](./05_ux_and_i18n.md) | Shell, canonical terms, five locales, forbidden words. |
| [`06_notes_and_dictation.md`](./06_notes_and_dictation.md) | Milkdown notes + sani-sis STT (not the web upload-file path). |
| [`07_files_and_io.md`](./07_files_and_io.md) | Knowledge folder, `DESKTOP:{projectId}`, EMBED/DOCS hints. |
| [`08_assistants_and_skills.md`](./08_assistants_and_skills.md) | Assistant = server recipe. Skill = local folder. In/Out board. |
| [`09_testing_and_gates.md`](./09_testing_and_gates.md) | Gates for both repos. Binding. |
| [`10_work_breakdown.md`](./10_work_breakdown.md) | PR-sized steps (`PS*` server, `PC*` client). **This is the implementation order.** |

**Execute from [`10_work_breakdown.md`](./10_work_breakdown.md).** The
topic files say why. The breakdown says how big and what “done” means.

A Cursor plan may exist beside this folder. **This folder is
canonical.** If the two drift, update the Cursor plan to point here —
never the other way around.

---

## Two repositories

| Repo | What it owns | Touched in |
| ---- | ------------ | ---------- |
| `synaplan/` | `/v1/assistants`, message headers, `/v1/models/catalog`, file model hints, OpenAPI, `docs/DESKTOP.md` | **Phase S** (`PS1`–`PS6`) |
| `synaplan-desktop` | Project core, `projects_dir`, shell, chats, Models panel, notes, files UI, Assistant bind, skill overlay, dictation | **Phase C** (`PC1`–`PC14`) |

Do **not** put a Project table on the server. Do **not** wrap
`frontend/` or embed `web.synaplan.com`. Do **not** change
`pairingScopes()`, `protocol: 1`, the secret store, or the existing
`app_dirs` identifiers (bundle id, vendor path, out-box, skills,
config, audit).

The Vue UI talks to Rust **only** through `src/services/tauri.ts`.

---

## What “done” looks like

### Phase S (server)

1. A paired `desktop:messages` key can `GET /v1/assistants` (publicView,
   `AGENTS.ENABLED`) and cannot CRUD `/api/v1/agents`.
2. `POST /v1/messages` accepts `x-synaplan-agent-id` and
   `x-synaplan-rag-group-key`, wires them into `AgentPinResolver` +
   `MessageProcessor`, and still returns Anthropic SSE. The CHAT model
   still comes from the body.
3. `GET /v1/models/catalog` returns capability-grouped selectable
   models. `id` is the catalog key `service:providerId:tag`. Display
   uses `providerId` plus provider and availability.
4. File upload/process honours `vectorize_model` / `analyze_model`
   (catalog keys) so a project EMBED/DOCS binding is not a lie.
5. `ApiKeyScopeTest` + pin + catalog + upload-hint tests are green.
   OpenAPI annotations are complete.
6. `docs/DESKTOP.md` describes the desktop-only extensions. The job
   contract is still `protocol: 1`.

### Phase C (client)

A paired user on this computer can:

1. Open Synaplan Desktop and land in a **Personal** project (created on
   first launch from `last_chat_model` + the global enabled-skill set).
2. Create another project. Switch. Rename. Delete (with confirm).
3. Set **this project's models** for CHAT, VOICE, SPEAK, VISION, IMAGE,
   VIDEO, EMBED, DOCS — and see provider + availability so they can
   pick, for example, all Ollama or all EU-hosted.
4. Chat in that project. Every turn sends the project CHAT model. RAG
   uses that project's knowledge folder. A bound Assistant may shape
   the recipe; it must **not** silently replace the project models.
5. Write notes as Markdown on disk, dictate with the project VOICE
   model and the project language, upload files into the knowledge
   folder with the project EMBED/DOCS models.
6. Bind Assistants, enable a subset of installed skills for this
   project, and see In / Out in plain language.
7. Still reach **This computer** and **Check this computer** from the
   overflow/footer.

---

## Non-goals (this epic) — one screen

- A server-side Project entity or cross-device note sync.
- Changing `protocol: 1` (`{skill, prompt, fileIds}` only).
- Widening `pairingScopes()` or adding `desktop:agents`.
- Granting `agents:*` / `messages:*` / `rag:*` to paired keys.
- Wrapping the Synaplan SPA or embedding `web.synaplan.com`.
- A shell, `shell.exec`, or any constructed `sh -c` / `cmd /c`.
- Changing secret-store or `app_dirs` identifiers.
- A single “preferred model” instead of the eight-slot matrix.
- Silently using an Assistant’s own chat/vision/vectorize models.
- On-device Whisper, or the web ChatInput upload-file STT path.
- LLM smoothing of transcripts (`transkript_glaetten`) in v1.
- Full TipTap / Notion editor.
- Using SPEAK (TTS) in the product v1 (the picker is visible).

---

## Workflow for each step

1. Tick any open decision that the step depends on — including the
   sovereignty rows in the master-plan checklist.
2. Implement **one** step from the breakdown — `PS*` in `synaplan/`,
   `PC*` in `synaplan-desktop`.
3. Run the gate in [`09_testing_and_gates.md`](./09_testing_and_gates.md)
   for the repo you touched.
4. PR on a feature branch. Conventional Commits. No AI attribution.
   Never `main`.
