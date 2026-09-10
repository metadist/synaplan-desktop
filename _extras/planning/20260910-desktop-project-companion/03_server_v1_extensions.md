# Server v1 extensions (no re-pair, no protocol 2)

**Status:** Draft 2026-09-10.
**Depends on:** master-plan checklist rows 4, 6, 11–18; topic
[`02_sovereign_models.md`](./02_sovereign_models.md).
**Unlocks:** client `PC6`, `PC7`, `PC9`, `PC10`, `PC12`.
**Repos:** `synaplan/` only (`PS1`–`PS6`).
**Done:** a paired key, without new scopes, can list Assistants, pin a
turn, read a capability catalog, and index a file with an explicit
model.

---

## 0. Why this sprint family exists

Local-first projects do not need a server entity. They do need four
gaps closed so Assistants + catalog + knowledge-folder indexing work
with **today's** paired key:

| Need | Today with a paired key |
| ---- | ----------------------- |
| List Assistants | `/api/v1/agents*` needs `agents:*` → 403 |
| Pin Assistant + RAG folder on chat | `POST /v1/messages` is a passthrough — no `agentId`, no `rag_group_key` |
| Capability-grouped models | `/v1/models` is flat; `/api/v1/config/models` needs `messages:*` |
| Index with **this project's** EMBED/DOCS | Upload accepts `group_key` + `process_level`; vectorize uses account `VECTORIZE` |

All four stay under `/v1/` or existing `/api/v1/files*`. **Do not**
add `desktop:agents` to `ApiKeyScope::pairingScopes()`. Old keys would
miss it; every existing install would have to re-pair.

**Do not** change `protocol: 1` job fixtures. Headers on Messages are
not a job field.

---

## 1. Code to read first

| Path | Why |
| ---- | --- |
| `backend/src/Security/ApiKeyScope.php` | `pairingScopes()`, `/v1/` → `desktop:messages`, `/api/v1/files` → `desktop:files`, `/api/v1/config/models` → `messages:*` |
| `backend/tests/Unit/Security/ApiKeyScopeTest.php` | Extend; do not replace |
| `backend/src/Controller/AgentController.php` | Session CRUD — do not open this to desktop keys |
| `backend/src/Service/Agent/AgentSerializer.php` `publicView` | Reuse; never draft |
| `backend/src/Service/Agent/AgentConfig.php` | `AGENTS.ENABLED` |
| `backend/src/Service/Agent/AgentPinResolver.php` | Web pin path to reuse |
| `backend/src/Service/Message/MessageProcessor.php` | `rag_group_key` already honoured |
| `backend/src/Controller/OpenAICompatibleController.php` | `/v1/messages`, `/v1/models` |
| `backend/src/Controller/ConfigController.php` | Grouping + availability to reuse |
| `backend/src/Controller/FileController.php` | Multipart fields |
| `backend/src/Service/File/VectorizationService.php` | Account VECTORIZE default |
| `backend/src/Model/ModelCatalog.php` | `findBidByKey` |
| `docs/DESKTOP.md` | Extend; state `protocol: 1` unchanged |

---

## 2. `PS1` — `GET /v1/assistants`

### 2.1 Routes

| Method | Path | Auth |
| ------ | ---- | ---- |
| `GET` | `/v1/assistants` | API key or session; `desktop:messages` sufficient |
| `GET` | `/v1/assistants/{id}` | Same |

### 2.2 Behaviour

- If `AGENTS.ENABLED` is off for the user: **404** with a stable
  machine-readable code (e.g. `assistants_disabled`). Pick 404 to
  match other flag-off desktop routes; document it; test it.
- If on: list Assistants the user may **run** — owner + published /
  gallery that `AgentAccess` already allows for read. Shape =
  `AgentSerializer::publicView` (no draft JSON).
- `{id}` 404 if missing or not accessible.
- **No** POST/PATCH/DELETE under `/v1/assistants`. Creating or
  editing Assistants stays on `/api/v1/agents` + `agents:*`.

### 2.3 publicView must be enough to warn

Desktop needs enough of the recipe to compare worlds — at least
`id`, `name`, `description`, and `models.chat` / `vision` /
`vectorize` (catalog keys or null) from the published definition.
If today's `publicView` strips `models`, add a **reader-safe**
subset. Never the draft, never secrets, never tool tokens.

### 2.4 Acceptance

- Paired key: 200 list when flag on; 404 when flag off.
- Same key: 403 on `GET /api/v1/agents` and all mutating agent
  routes (`C14`).
- OpenAPI on both routes. No new scope constant.

---

## 3. `PS2` — Messages headers

### 3.1 Headers (not body, not job)

| Header | Meaning |
| ------ | ------- |
| `x-synaplan-agent-id` | Integer Assistant id. Ignored if missing/invalid/flag off (same as web). |
| `x-synaplan-rag-group-key` | Knowledge folder. Desktop sends `DESKTOP:{projectId}`. |

Keep names prefixed `x-synaplan-` so they cannot be confused with
Anthropic fields. Do **not** add `agentId` to the JSON body in a way
that breaks Anthropic clients. Headers are opt-in.

### 3.2 Wiring

1. Messages gateway reads the two headers into processor options
   (`agentId`, `rag_group_key`) — the same keys web
   `StreamController` already uses.
2. `AgentPinResolver` builds the `RuntimeProfile` (recipe).
3. `MessageProcessor` / `ChatHandler` apply `rag_group_key` so RAG
   searches that folder (plus whatever the recipe already allowed).
4. Response remains **Anthropic SSE**. No new event type required
   for v1.

### 3.3 Model must not flip (C15)

The CHAT model stays in the **body** (`model`). The desktop client
sends the project CHAT binding (`PC7`).

If pinning an agent currently overwrites the selected model with
`definition.models.chat`, **stop that on this path** when the
request already named a model. Body wins. Document it. Test:

- Pin agent A whose recipe chat key is X.
- Body model is Y (project).
- Completion / log / resolver sees **Y**.

RAG folder in the header still applies even when the Assistant also
lists folders — project folder is the desktop knowledge folder; do
not drop it.

### 3.4 Acceptance

- No header → behaviour identical to today (characterization /
  gateway tests still green).
- Both headers → pin + RAG group observed in unit tests with fakes.
- Extra unknown `x-synaplan-*` headers ignored.
- Job fixtures in `tests/fixtures/desktop-contract/` **byte-identical**
  (`C9`).

---

## 4. `PS3` — `GET /v1/models/catalog`

See [`02_sovereign_models.md`](./02_sovereign_models.md) §5 for the
locked JSON shape.

Implementation notes:

- New action on the OpenAI-compatible controller **or** a tiny
  dedicated controller mounted under `/v1/models/catalog`. Prefer
  next to `listModels` so auth/firewall stays obvious.
- Reuse ConfigController grouping + availability. Do not copy-paste
  a third tag map — extract if needed (small private helper is fine).
- `id` = `ModelCatalog` key `service:providerId:tag`.
- Groups: exactly `CHAT`, `SOUND2TEXT`, `TEXT2SOUND`, `PIC2TEXT`,
  `TEXT2PIC`, `TEXT2VID`, `VECTORIZE`, `ANALYZE`.
- Selectable + active only.
- Paired key 200. Session cookie 200 (harmless). No `messages:*`.

Optional additive: `/v1/messages` resolves `model` via
`findBidByKey` when the string looks like a catalog key. Test both
providerId (old clients) and catalog key (desktop).

---

## 5. `PS4` — upload / process model hints

### 5.1 Fields

On `POST /api/v1/files/upload` (multipart, already used by desktop)
and on the process-file path:

| Field | Value | Capability |
| ----- | ----- | ---------- |
| `vectorize_model` | catalog key | `VECTORIZE` |
| `analyze_model` | catalog key | `ANALYZE` |

JSON process endpoints may take the same names in the body.

### 5.2 Behaviour

- Omitted → today's `getDefaultModel('VECTORIZE', $userId)` (web
  unchanged).
- Present + valid → `VectorizationService` (and analyze) use that
  BID. Do not also write a new account default.
- Present + unknown / wrong capability → **400**, no silent fallback.
- `group_key` and `process_level` already exist; desktop will send
  `DESKTOP:{id}` and `vectorize`.

Premium / locked VECTORIZE on the **account default** page is a web
concern. A desktop hint is a per-file override the user already
chose on that computer. Do not reject the hint solely because the
web VECTORIZE row is operator-locked — if that conflicts with an
existing billing rule, `PS4` must call it out in the PR and pick the
stricter honest behaviour (refuse with a clear error vs accept).
Default proposal: **accept the hint** if the user may use that
model for inference at all (it is in the catalog as selectable).

### 5.3 Acceptance

- Test: upload with `vectorize_model` = catalog key of a selectable
  VECTORIZE model → process uses that BID (assert on a seam /
  argument), not `getDefaultModel`.
- Test: garbage key → 400.
- Test: paired `desktop:files` key may send the fields.
- OpenAPI updated; frontend Zod regen only if web schemas see the
  new optional fields (additive — should not break).

---

## 6. `PS5` — tests and OpenAPI

Not a dumping ground for tests that belong on `PS1`–`PS4`. Each of
those PRs ships its own tests. `PS5` is the **matrix and polish**:

| Case | Where |
| ---- | ----- |
| Pairing scopes still exactly four strings | `ApiKeyScopeTest` |
| `/v1/assistants` allowed; `/api/v1/agents` denied | Scope + controller |
| Catalog allowed on paired key; `/api/v1/config/models` still denied | Scope |
| Pin does not override body model | Messages / pin unit |
| RAG group key reaches ChatHandler options | Existing processor tests extended |
| Upload hint used; omit hint = old default | File upload / vectorize |
| OpenAPI documents all new fields and the catalog schema | Nelmio + `generate-schemas` if needed |
| Characterization snapshots untouched | Do not re-record |

---

## 7. `PS6` — docs

Update `synaplan/docs/DESKTOP.md`:

- New subsection **Desktop project companion (machine API)** —
  assistants, headers, catalog, file hints.
- Explicit: **job contract remains `protocol: 1`**. No new job
  type. No `desktop:agents`.
- Explicit: these routes are for Synaplan Desktop; they are not a
  public Agents CRUD.

Do not write a second contract file. Do not mention Claude / MCP in
a way that belongs in user UI.

---

## 8. Non-goals

- `BDESKTOPPROJECTS` or any Project table.
- `desktop:agents` or `pairingScopes()` edits.
- Changing `/mcp` `agent_checkin` shape.
- Letting desktop keys PATCH Assistants.
- Moving grouped models to `/api/v1/config/models` for desktop.
- `protocol: 2`.

---

## 9. Suggested PR titles

```
feat(desktop): list assistants on GET /v1/assistants
feat(desktop): pin assistant and rag folder on POST /v1/messages
feat(desktop): add GET /v1/models/catalog for paired keys
feat(files): accept vectorize_model and analyze_model on upload
test(desktop): paired-key catalog pin and upload-hint matrix
docs(desktop): project companion machine API
```
