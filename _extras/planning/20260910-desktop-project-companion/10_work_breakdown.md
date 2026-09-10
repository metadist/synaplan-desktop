# Work breakdown — PR-sized steps

**Status:** Draft 2026-09-10. The topic files say *what* and *why*.
This file says *how big*, *in what order*, and what “done” means.

Implement **one ID per PR** unless two S-sized steps are safer
together (say so in the PR).

| Prefix | Repo | Range |
| ------ | ---- | ----- |
| `PS` (Project Server) | `synaplan/` | `PS1`–`PS6` |
| `PC` (Project Client) | `synaplan-desktop` | `PC1`–`PC14` |
| `L` | docs / copy | `L1` |

**ID order is not always execution order** — the `Depends` column is.
Early client steps (`PC1`–`PC5`, `PC8`, `PC13`) do **not** wait for
Phase S. Sovereignty and bind steps do.

---

## 0. Status

Client steps `PC1`–`PC14` are implemented on the `feat/project-companion`
branch of `synaplan-desktop` (2026-09-10); the server steps `PS1`–`PS6` are
still open. Where a client step needs a server route that does not exist
yet, the client shows the honest 404 state (`assistants_disabled`, catalog
missing) instead of pretending. Tick a row when that step's PR **merges**.

| Phase | Area | Steps | State |
| ----- | ---- | ----- | ----- |
| — | Decision checklist [`00`](./00_master_plan.md) §0–§0.3 | rows 1–27 | **Not ticked** |
| — | Native-speaker terms | `L1` | Not started |
| S | Assistants list | `PS1` | Not started |
| S | Messages headers | `PS2` | Not started |
| S | Model catalog | `PS3` | Not started |
| S | Upload model hints | `PS4` | Not started |
| S | Test matrix + OpenAPI | `PS5` | Not started |
| S | `docs/DESKTOP.md` | `PS6` | Not started |
| C | Project core | `PC1` | Done on branch |
| C | `projects_dir` | `PC2` | Done on branch |
| C | Tauri seam | `PC3` | Done on branch |
| C | Shell / nav / i18n | `PC4` | Done on branch |
| C | Persistent chats | `PC5` | Done on branch |
| C | Sovereign Models panel | `PC6` | Done on branch; live catalog once `PS3` ships (404 state until then) |
| C | Send project CHAT | `PC7` | Done on branch |
| C | Notes + Milkdown | `PC8` | Done on branch; Milkdown (`@milkdown/kit`, approved 2026-09-10) with the slim toolbar |
| C | Knowledge folder upload | `PC9` | Done on branch; model hints are sent, server honours them once `PS4` ships |
| C | Assistant bind + headers | `PC10` | Done on branch; list is `assistants_disabled` until `PS1`, headers honoured once `PS2` ships |
| C | Skill overlay + In/Out | `PC11` | Done on branch |
| C | Dictation | `PC12` | Done on branch (existing `/v1/audio` routes) |
| C | Computer / Doctor reachable | `PC13` | Done on branch |
| C | Locale / tests each slice | `PC14` | **Rule on every `PC*`** — held on every commit of the branch |

---

## 1. Step-size rules

Same spirit as 2026-08-29:

| Rule | Test |
| ---- | ---- |
| One PR, one concern | Title without a pile of “and” |
| Never one PR across remotes | `PS*` xor `PC*` |
| New HTTP before the client that needs it | `PS3` before a live `PC6` |
| Every step ships tests | [`09_testing_and_gates.md`](./09_testing_and_gates.md) |
| Independently revertable | Product still coherent if only this PR reverts |
| Three acceptance bullets or it is not understood | Go back to the topic file |

Size: **S** a few files, **M** one subsystem, **L** split unless
justified.

### 1.1 Definition of done

See testing doc §6. Short version: unfiltered gate, tests, OpenAPI /
locales if needed, empty characterization diff on server, no `sk_`,
no `pairingScopes()` edit, no job-fixture edit.

### 1.2 Parallelism (unlike 2026-08-29)

The client **already exists**. Do not wait for all `PS*` to finish
before `PC1`. Do not ship `PC6`/`PC9`/`PC10`/`PC12` as if the server
routes existed when they do not — stub with honesty copy, or wait.

---

## 2. Cross-cutting

| ID | Step | Layer | Size | Depends | Acceptance |
| -- | ---- | ----- | ---- | ------- | ---------- |
| **L1** | Native-speaker pass on [`05`](./05_ux_and_i18n.md) §2 | Docs | S | — | DE/ES/FR/TR table ticked or corrected in that file before `PC4` copy freezes |
| **PC14** | Locale parity + Vitest + Rust tests **on each slice** | Gate | S | every `PC*` | Not a late mop-up PR. Each `PC*` PR already includes five locales if it touched copy, and `make ci-local`. A final audit PR is allowed only to shrink leftover English if `L1` changed terms |

---

## 3. Phase S — `synaplan/`

Topic: [`03_server_v1_extensions.md`](./03_server_v1_extensions.md),
[`02_sovereign_models.md`](./02_sovereign_models.md).

| ID | Step | Layer | Size | Depends | Acceptance |
| -- | ---- | ----- | ---- | ------- | ---------- |
| **PS1** | `GET /v1/assistants` + `GET /v1/assistants/{id}` = `publicView`; `AGENTS.ENABLED`; `desktop:messages` | BE | M | checklist 6, 3 | Flag off → 404; flag on + paired key → 200; same key 403 on `/api/v1/agents*`; no draft; OpenAPI |
| **PS2** | `POST /v1/messages` reads `x-synaplan-agent-id` + `x-synaplan-rag-group-key`; wires `AgentPinResolver` + `MessageProcessor`; Anthropic SSE; **body model wins** | BE | M | `PS1` (resolver reuse; can start in parallel if needed) | No headers = today's behaviour; headers pin + RAG folder; recipe chat key does **not** replace body `model` (C15); job fixtures unchanged (C9) |
| **PS3** | `GET /v1/models/catalog` capability-grouped; `id` = catalog key; provider + availability; eight groups | BE | M | checklist 14–15 | Paired key 200; `/api/v1/config/models` still `messages:*`; groups exact; selectable only; OpenAPI |
| **PS4** | File upload/process accepts `vectorize_model` / `analyze_model` (catalog keys) | BE | M | checklist 16 | Hint used instead of `getDefaultModel`; omit = old default; bad key 400; `desktop:files` ok; OpenAPI |
| **PS5** | Matrix: `ApiKeyScopeTest`, catalog, pin, upload hint; OpenAPI complete | Test | S | `PS1`–`PS4` | Testing doc §3 rows C13–C16 green; characterization empty |
| **PS6** | `docs/DESKTOP.md` — desktop-only machine API; **no protocol 2** | Docs | S | `PS2`, `PS3`, `PS4` | Doc states four scopes unchanged, job contract `protocol: 1`, catalog + headers + hints |

**Phase S exit:** a paired key can list Assistants, pin a turn, read
the catalog, and index with an explicit EMBED model — without
re-pair.

Suggested titles: see `03` §9.

---

## 4. Phase C — project core and shell (`synaplan-desktop`)

Topics: [`01`](./01_product_model.md), [`04`](./04_local_persistence.md),
[`05`](./05_ux_and_i18n.md).

| ID | Step | Layer | Size | Depends | Acceptance |
| -- | ---- | ----- | ---- | ------- | ---------- |
| **PC1** | Project core in `synaplan-core`: CRUD, index, slug, Personal migration + Rust tests | Core | M | checklist 1, 7 | `ensure_personal` once; inherits `last_chat_model` + enabled skills; slug rules; no server call |
| **PC2** | `AppDirs.projects_dir` (`~/Synaplan/projects`) — **only** in `platform/app_dirs.rs` | Core | S | `PC1` can land first if it takes dirs as args; prefer `PC2` before or with `PC1` | Three-OS expectations; grep finds no other home expansion (C17) |
| **PC3** | Tauri commands + `src/services/tauri.ts` wrappers/types for projects (no Vue shell yet) | Seam | M | `PC1`, `PC2` | Vue can list/create/switch only through `tauri.ts`; no `fs` plugin |
| **PC4** | Shell/nav/i18n: project switcher + Chat / Notes / Files / Agents / Models; five locales | UI | M | `PC3`, `L1` | Old four-item rail gone; pairing unchanged; light + dark; empty states from `01`/`05` |
| **PC5** | Persistent chats per project (list, load, save, new thread); keep streaming | UI + Core | M | `PC3`, existing chat | Restart restores threads; no key in JSON; stream still works |
| **PC13** | Computer + Doctor (+ skill *install*) remain reachable from overflow/footer | UI | S | `PC4` | Both views open; install still computer-level |

**Core+shell exit:** Personal exists; user can create a second
project and chat with persisted threads; machine screens are not
orphaned.

---

## 5. Phase C — sovereignty (`synaplan-desktop`)

Topic: [`02_sovereign_models.md`](./02_sovereign_models.md). **Do not
skip.**

| ID | Step | Layer | Size | Depends | Acceptance |
| -- | ---- | ----- | ---- | ------- | ---------- |
| **PC6** | Models panel: eight slots, persist bindings, fetch catalog, provider + availability, unset rules | UI + Core | M | `PC3`, `PS3` | EMBED selectable; SPEAK/VIDEO visible; no `DEFAULTMODEL` in UI; persist catalog keys; unavailable does not silently substitute (C15/C16 client half) |
| **PC7** | `send_chat` / `send_agent_chat` always send project CHAT | Core | S | `PC5`, `PC6` | Captured body `model` = project binding; send blocked if CHAT unset |

Until `PS3` merges, `PC6` may merge a **disabled** panel that says
the workspace has not sent the model list — not a fake catalog.

---

## 6. Phase C — notes, files, agents, voice

| ID | Step | Layer | Size | Depends | Acceptance |
| -- | ---- | ----- | ---- | ------- | ---------- |
| **PC8** | Notes + Milkdown + manager under `{projects_dir}/{slug}/notes`. **Ask before npm dep.** | UI + Core | M | `PC2`, `PC4`, checklist 25 | MD round-trip; toolbar = headings/bold/lists/links/code; reveal; optional share hooks `PC9` |
| **PC9** | Files: `group_key=DESKTOP:{id}`, `process_level=vectorize`, EMBED/DOCS hints, status list | Core + UI | M | `PS4`, `PC6`, `PC3` | Form fields asserted; EMBED unset refuses dishonest index; statuses from `07` §4 (C16) |
| **PC10** | Bind Assistants; headers on chat; **project models win**; world warning | UI + Core | M | `PS1`, `PS2`, `PC6`, `PC7` | Headers present; body model still project CHAT (C15); warning when recipe differs |
| **PC11** | Per-project skill overlay + In/Out board | UI + Core | M | `PC1`, `PC9` (In files), `PC4` | Overlay ∩ install; Personal copied enables; In/Out plain language; no job-shape change |
| **PC12** | Dictation session + one-shot (sani-sis); project VOICE + language; Rust HTTP; mic capability | UI + Core | M | `PC6`, `PC8` (caret) and Chat composer; checklist 21–24 | Session `commit_after_bytes=480000`; language + prompt + model; one-shot preferred; no JS `fetch` to `/v1/audio` (C18) |

**Notes dep:** the `PC8` PR description must record the ask. If the
dep is refused, ship textarea + the same manager, and say so.

---

## 7. Suggested calendar (not a commitment)

| Week | Steps | Repo | Note |
| ---- | ----- | ---- | ---- |
| 1 | `PS1`, `PS3` in parallel; `PC2`, `PC1`, `PC3` | both | Catalog and project core do not depend on each other |
| 1 | `L1` | docs | Before `PC4` strings freeze |
| 2 | `PS2`, `PS4`; `PC4`, `PC5`, `PC13` | both | Headers + upload hints; shell + chats |
| 3 | `PS5`, `PS6`; `PC6`, `PC7` | both | Sovereignty becomes real |
| 4 | `PC8`, `PC9` | desktop | Notes + knowledge folder |
| 5 | `PC10`, `PC11`, `PC12` | desktop | Bind, overlay, dictation |

If scope slips, **cut VIDEO/SPEAK usage** (already unused) and
**cut In/Out polish** before cutting the catalog, upload hints, or
the Models panel. Cutting `PS3`/`PS4`/`PC6` is cutting the epic.

---

## 8. What was easy to conflate (do not re-merge)

| Temptation | Why it is wrong | Keep split |
| ---------- | --------------- | ---------- |
| “Add `desktop:agents` and call `/api/v1/agents`” | Existing keys 403; forces re-pair | `PS1` under `/v1/` |
| “Use `/api/v1/config/models`” | `messages:*` | `PS3` catalog |
| “One preferred model is enough” | Index + STT leak the account default | Eight slots (`PC6`) |
| “The Assistant's models are the good ones” | Leaves the project's world | Project wins (`PS2`/`PC10`) |
| “Upload with group_key only” | VECTORIZE stays account-default | `PS4` + `PC9` |
| “Web ChatInput upload-file for dictation” | Ignores language/model/prompt | `PC12` / `/v1/audio` |
| “Hardcode `de` like sani-sis” | Product is multilingual | Project language |
| “Put `projects_dir` in `config.rs`” | Second home expander | `PC2` / `app_dirs` only |
| “Wrap the SPA for Assistants” | Forbidden | `GET /v1/assistants` |
| “Tweak protocol 1 to carry agentId” | Frozen contract | Headers on `/v1/messages` |
| “JS fetch for STT is simpler” | Key in the WebView | Rust HTTP |
| “Skip EMBED because web locks VECTORIZE” | Sovereignty is a lie | Picker + hint |
| “English Models panel first” | Parity gate | `PC14` on that PR |

---

## 9. Tick list for the human who starts coding

Print or copy. Tick in [`00_master_plan.md`](./00_master_plan.md) §0
as well.

- [ ] Rows 1–10 (local-first, frozen contract, no re-pair, no SPA wrap)
- [ ] Rows 11–20 (**sovereignty** — matrix, catalog, hints, project wins, copy)
- [ ] Rows 21–24 (dictation protocol)
- [ ] Rows 25–27 (Milkdown ask, shell, skill overlay)
- [ ] `L1` terms
- [ ] Next unfinished `Depends`-ready ID in this file
- [ ] Gate in [`09`](./09_testing_and_gates.md)
- [ ] Update §0 of this file when it merges
