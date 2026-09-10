# Assistants and Skills — bind, overlay, In/Out

**Status:** Draft 2026-09-10.
**Depends on:** checklist rows 3, 17, 27; `PS1`, `PS2`;
[`02_sovereign_models.md`](./02_sovereign_models.md) §6;
[`05_ux_and_i18n.md`](./05_ux_and_i18n.md).
**Unlocks:** `PC10`, `PC11`.
**Repos:** client; server list/pin already specified in `03`.
**Done:** a project can bind Assistants and enable a subset of
installed Skills; chat uses project models; the In/Out board is
understandable.

---

## 0. Why this file exists

Two products share no UI word. Collapsing them is how we get
“enable this Assistant on disk” and “run this Skill on the server”.

| UI | What | Where it runs |
| -- | ---- | ------------- |
| **Assistant** | Synaplan recipe (`publicView`) | Paired instance (prompt, tools, extra folders) |
| **Skill** | Local `SKILL.md` folder | This computer, confined |

Install of Skills stays **computer-level** (today's Skills page
flow). Enablement is a **per-project overlay**. Assistants are
never installed as folders.

---

## 1. Code to read first

| Path | Why |
| ---- | --- |
| `backend/src/Service/Agent/AgentSerializer.php` `publicView` | List payload |
| `backend/src/Service/Agent/AgentPinResolver.php` | Pin |
| `backend/src/DTO/AgentDefinitionV1.php` | Recipe `models.*` |
| `src/views/SkillsView.vue` | Install stays; enablement moves to overlay |
| `src-tauri/synaplan-core` skills load/enable | Global enable = install visibility; project overlay = may run here |
| `src-tauri/synaplan-core/src/messages.rs` | `send_chat` — add header variant `send_agent_chat` |

---

## 2. Bind Assistants (`PC10`)

1. `GET /v1/assistants` via Rust (`desktop:messages`).
2. Picker: name + short description. Flag off → honesty copy
   ([`05`](./05_ux_and_i18n.md) §4).
3. User sets **default** Assistant (or none). Several binds allowed;
   one default.
4. Persist ids on the project file.
5. On send: `x-synaplan-agent-id` = default (or the one chosen on
   that thread) + `x-synaplan-rag-group-key` = `DESKTOP:{id}`.
6. Body `model` = project CHAT (`PC7`). **Never** the recipe chat
   key.

### 2.1 World warning (C15)

Compare `publicView.models.{chat,vision,vectorize}` to the project
matrix:

- Any non-null key ≠ project slot → warn.
- Null key (“workspace default”) → warn (that default is not the
  project).

Copy: [`02`](./02_sovereign_models.md) §3.1. Do not silently apply
recipe models. Do not disable the Assistant — the recipe (prompt /
behaviour) is still useful; only the **models** stay on the project.

If `PS2` cannot stop the server from overwriting the body model,
**this client slice is blocked**. Do not ship a bind that leaks.

---

## 3. Skill overlay (`PC11`)

Computer-level:

- Install / remove / bundled list — overflow → This computer or
  today's Skills install UI moved there.
- Global “enabled” in `skills.json` still means “installed and
  allowed on this computer at all”.

Project-level:

- `enabled_skills: [name, …]` in the project toml.
- Agents view: toggles for each **installed** skill.
- The Messages tool catalog / skill preface for a turn in this
  project includes **only** the overlay ∩ installed ∩ computer-enabled.
- Web-queued `skill.run` (protocol 1) is unchanged: the device
  still refuses unknown/disabled **computer-level** names. A
  project overlay does not accept a job for a skill disabled on
  the computer. Jobs from the web are **computer-level** in v1
  (no project id on the job). Do not invent a job field (`C9`).

Personal migration copies today's global enabled set into the
overlay so behaviour matches pre-upgrade.

---

## 4. In / Out board

The Agents view is an **activity board**, not a log dump.

**In**

- Files in the knowledge folder and their index status (`07` §4).
- Notes marked shared.
- Last successful index time.
- “Assistants are off” / catalog failures as banners, not rows.

**Out**

- Files in `{projects_dir}/{slug}/out`.
- “Show in folder”.
- Last chat artifacts attached to a thread.

Failures in the language of [`05`](./05_ux_and_i18n.md). Quota,
vectorize failed, Assistant flag off.

Do not show lease ids, MCP tool names, or `sk_`.

---

## 5. Chat loop

Existing streaming + local tool loop stays. New work:

| Command | Adds |
| ------- | ---- |
| `send_chat` | Body model = project CHAT; optional RAG header even without Assistant |
| `send_agent_chat` | Same + `x-synaplan-agent-id` |

Both go through `tauri.ts`. Vue does not attach headers.

RAG without an Assistant: still send the knowledge-folder header so
project files are searchable. If the server ignores the header
without a pin, `PS2` must make the header sufficient alone — that
is the local-first case (no Assistant bound).

---

## 6. Tests

| Case | Assert |
| ---- | ------ |
| List assistants on mock | publicView parsed; draft absent |
| Send includes both headers + project model | Rust captured request |
| Overlay hides a computer-enabled skill | Preface / catalog |
| World warning when recipe chat ≠ project chat | Vue unit |
| Flag-off empty state | Vue unit |

---

## 7. Non-goals

- Editing Assistant recipes on the desktop.
- Installing Skills from the Agents board (install stays computer).
- Putting `projectId` on `protocol: 1` jobs.
- Auto-enabling every installed skill on every new project
  (except Personal migration).
- Using recipe `models.*` “just this once”.

---

## 8. Acceptance

- Bind + unbind persist.
- Headers + project CHAT on the wire (`PC10`).
- Overlay ∩ install (`PC11`).
- In/Out readable in five locales.
- Computer install UI still reachable (`PC13`).
