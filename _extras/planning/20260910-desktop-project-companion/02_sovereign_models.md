# Sovereign models — this project's models

**Status:** Draft 2026-09-10. **Binding.** A PR that ships projects
without this matrix, or that silently uses account / Assistant models
for desktop inference, is out of scope — stop and split.
**Depends on:** master-plan checklist rows 11–20, 14–16.
**Unlocks:** `PS3`, `PS4`, `PC6`, `PC7`, `PC9`, `PC12`.
**Repos:** `synaplan/` for the catalog + upload hints; `synaplan-desktop`
for persist + UI + every outbound call.
**Done:** the user can keep a project's data in a defined world by
picking models **per capability**, and the client actually sends those
picks.

This is the user-critical requirement of the epic. The rest of the
folder is how work is grouped. This file is how work **stays put**.

---

## 0. Why this file exists

Sovereignty here does **not** mean “the bytes never leave the laptop”.
Chat, dictation, embeddings, and image models still run on the **paired
Synaplan instance** and the providers that instance has configured
(Ollama on that box, an EU host, a US host, …).

Sovereignty **does** mean:

> Data from this project is processed only by the models you picked
> for this project.

A single chat-model dropdown cannot say that. Indexing a PDF with the
account `VECTORIZE` default, dictating with whatever `/v1/audio` picks,
or letting a bound Assistant swap in its own `models.chat` all break
the sentence — even if the UI still says “Ollama”.

**Project bindings are the sovereignty boundary.** Not the Assistant
recipe. Not `BCONFIG` / `DEFAULTMODEL`. Not the last model the web UI
saved.

---

## 1. Code to read first

| Path | Why |
| ---- | --- |
| `backend/src/Controller/OpenAICompatibleController.php` `listModels` | Flat `/v1/models`, `id` = providerId, no capability groups |
| `backend/src/Controller/ConfigController.php` grouped models | Has groups + `available` — **`messages:*` only** |
| `backend/src/Model/ModelCatalog.php` | `service:providerId:tag` keys; `findBidByKey` |
| `backend/src/Service/File/VectorizationService.php` | `getDefaultModel('VECTORIZE', $userId)` — today's index path |
| `backend/src/DTO/AgentDefinitionV1.php` | Assistant `models.chat` / `vision` / `vectorize` catalog keys |
| `src-tauri/synaplan-core/src/messages.rs` | `list_models` comment: capability filtering is impossible today |
| `src/services/tauri.ts` `ModelInfo` | `{ id, provider }` — not enough for the matrix |

---

## 2. The eight slots

Use Synaplan `DEFAULTMODEL` capability names in **code and this folder
only**. The UI never says them.

| Project slot (UI) | `DEFAULTMODEL` | Used for | v1 product behaviour |
| ----------------- | -------------- | -------- | -------------------- |
| **CHAT** | `CHAT` | Project chat / Assistant turns from desktop | Required before send. Client puts the binding in the `/v1/messages` body. |
| **VOICE** | `SOUND2TEXT` | Dictation `/v1/audio` | Required before dictate. Every session + one-shot uses it. |
| **SPEAK** | `TEXT2SOUND` | Read-aloud (later) | Picker **visible**. Unused in product v1 if desktop has no TTS. May be unset. |
| **VISION** | `PIC2TEXT` | Image / OCR understanding when a project file is an image | Used when a turn or file path needs it. May be unset until then. |
| **IMAGE** | `TEXT2PIC` | Image generation from project chat | Sent when the user asks to generate an image. May be unset. |
| **VIDEO** | `TEXT2VID` | Video generation | Picker **visible**. May be unset. |
| **EMBED** | `VECTORIZE` | Indexing project files (RAG) | **Must be selectable.** Upload/process sends this hint. Unset → do not index; explain why. |
| **DOCS** | `ANALYZE` | Document Q&A / processing of uploaded files | Sent as `analyze_model` when the process path analyzes. May be unset if that path is not used. |

**EMBED is not optional to offer.** If the panel can set CHAT but
indexing always uses the account default, sovereignty is a lie (C16).

SPEAK and VIDEO being unused or unset is honest. Hiding EMBED because
“vectorize is an operator setting on the web” is not.

---

## 3. What the user sees

### 3.1 One term

**This project's models.**

Not “defaults”, “capabilities”, “slots”, “DEFAULTMODEL”, “BTAG”,
“routing”, “provider id”.

Secondary line (all five locales), the sovereignty sentence:

> Data from this project is processed only by the models you picked
> here. Those models still run on your Synaplan workspace and the
> providers it is set up to use.

Short status chip when every *used* slot is set and consistent:
**Stay in this world.**

When a bound Assistant's recipe names a different chat / vision /
vectorize model than this project:

> This Assistant is set up to use other models. This project will
> keep using **this project's models**. You can pick a different
> Assistant, or change the models above.

Never auto-switch the project to match the Assistant. Never auto-switch
the turn to match the Assistant.

### 3.2 Provider + availability

Each row shows:

- Slot label (CHAT, VOICE, …) — user-facing names from
  [`05_ux_and_i18n.md`](./05_ux_and_i18n.md) §2 (e.g. **Chat**,
  **Dictation**, **Index files**, not `VECTORIZE`).
- Selected model display name (`providerId` or catalog display).
- **Provider** (`owned_by` / `service`: `ollama`, `openai`, …).
- **Availability** (ready / missing key / disabled) from the catalog.
- Unset action where allowed.

The point of provider + availability is so a non-technical user can
pick “all the Ollama ones” or “all the ones that are ready on this
workspace” without knowing tags.

A “use the same provider for every slot” helper is allowed. It must
only fill slots that have a selectable model from that provider. It
must not invent a pick.

### 3.3 Forbidden UI words (this panel especially)

`DEFAULTMODEL`, `BTAG`, `BMODELS`, `capability`, `VECTORIZE`,
`SOUND2TEXT`, `PIC2TEXT`, `TEXT2PIC`, `TEXT2VID`, `TEXT2SOUND`,
`ANALYZE`, `protocol`, `group_key`, `sk_`, Claude, Anthropic, MCP,
shell, Bash, Tauri, lease.

---

## 4. Identity: catalog key, not BID, not a second nickname

**Stick to one id. Prefer the catalog key.**

| Field | Value |
| ----- | ----- |
| Persist / send as model hint | `service:providerId:tag` (lowercased service; tag as catalog stores it) |
| Display | `providerId` (what people already see in `/v1/models`) plus provider |
| Fallback resolve | `ModelCatalog::findBidByKey` on the server |

Why not BID? Instance-local, changes on reseed, useless across docs.
Why not providerId alone? Two rows can share a providerId with
different tags (chat vs analyze). Why not `/v1/models` `id`? Same
collision, no capability.

Example key: `ollama:bge-m3:vectorize` displayed as `bge-m3`
(provider: Ollama, slot: Index files).

`/v1/messages` today accepts the OpenAI-style `model` string
(providerId). **Client rule:**

- Persist the catalog key on the project.
- When the gateway already accepts a catalog key, send the key.
- If a given endpoint still wants providerId (`/v1/messages` body
  today), send providerId **and** keep the key on the project. Do not
  persist only the providerId — a later catalog refresh cannot
  disambiguate.

`PS3` should document, per endpoint, whether the request field is the
catalog key or the providerId. Prefer teaching `/v1/messages` to
accept a catalog key *additively* (if the string contains two colons
or `findBidByKey` hits, use that). That is allowed and is **not**
`protocol: 2`.

---

## 5. Server catalog — `GET /v1/models/catalog`

### 5.1 Why a new route

| Route | Grouped? | Available? | Desktop key? |
| ----- | -------- | ---------- | ------------ |
| `GET /v1/models` | No | No | Yes (`desktop:messages`) |
| `GET /api/v1/config/models` | Yes | Yes | **No** (`messages:*`) |
| `GET /v1/audio/models` | VOICE only | Partial | Yes |

Do **not** put the grouped catalog on `/api/v1/config/models` and
widen the scope map. That is a re-pair-or-403 trap. Do **not** add
`desktop:agents`.

Ship **`GET /v1/models/catalog`** under the existing `/v1/` →
`desktop:messages` prefix.

### 5.2 Response shape (lock in `PS3` OpenAPI)

```json
{
  "object": "catalog",
  "capabilities": {
    "CHAT": [{ "id": "ollama:llama3.2:chat", "providerId": "llama3.2", "service": "ollama", "name": "Llama 3.2", "available": true, "unavailableReason": null }],
    "SOUND2TEXT": [],
    "TEXT2SOUND": [],
    "PIC2TEXT": [],
    "TEXT2PIC": [],
    "TEXT2VID": [],
    "VECTORIZE": [],
    "ANALYZE": []
  }
}
```

Rules:

- Keys of `capabilities` are the **exact** `DEFAULTMODEL` names in
  §2. Client maps them to UI slots. Do not send `SORT` / `MEM` /
  `PIC2PIC` / `IMG2VID` unless a later epic needs them — extra groups
  invite extra pickers.
- Only **selectable** models (same rules as the web picker:
  active + user-selectable). Do not list system-only sorter models.
- `id` = catalog key.
- `available` + `unavailableReason` copied from the config-models
  logic so “EU-hosted / key missing” is visible.
- `AGENTS.ENABLED` does not gate this route.
- Empty group = empty array, not omit. Client shows “No model
  available for this use”.

Reuse grouping from `ConfigController` (tag → capability), do not
invent a second map. VECTORIZE rows must appear even if the web UI
treats VECTORIZE as system-wide — the desktop project hint is a
**per-upload override**, not a rewrite of account defaults.

### 5.3 VOICE

`GET /v1/audio/models` remains. The Models panel prefers the catalog
`SOUND2TEXT` group. If catalog is down, VOICE may fall back to
`/v1/audio/models` **only** for display; persist still stores a
catalog key once known.

---

## 6. Project models win over Assistant models

A published Assistant (`AgentDefinitionV1`) has:

```text
models.chat / models.vision / models.vectorize  → catalog keys or null
```

On the **web**, a pinned agent applies those keys. On **desktop**,
that would leave the project's world without asking.

**Desktop inference rule (C15):**

| Call | Model source |
| ---- | ------------ |
| `POST /v1/messages` `model` | Project CHAT |
| Image-understand path | Project VISION |
| Image-generate path | Project IMAGE |
| Video-generate path | Project VIDEO (or refuse if unset) |
| `POST /v1/audio/*` `model` | Project VOICE |
| File vectorize hint | Project EMBED |
| File analyze hint | Project DOCS |

Headers / pin:

- `x-synaplan-agent-id` still selects the **recipe** (prompt, tools,
  knowledge *folders declared on the assistant*, behaviour).
- `x-synaplan-rag-group-key` still selects the **project** knowledge
  folder (`DESKTOP:{id}`).
- Server-side `AgentPinResolver` must **not** be allowed to replace
  the request body model with the recipe chat model on this path.
  If today's resolver overwrites, `PS2` stops that when the request
  is a desktop Messages call that already set `model`. Document the
  chosen mechanism in `PS2` (options flag `keepRequestModel: true`,
  or “body model always wins”).

Client warning: compare Assistant publicView model keys to the
project matrix. Any differing non-null key → the §3.1 warning.
Null on the recipe = “uses workspace default” → also warn, because
that default is not the project matrix.

Do not hide the warning behind an “advanced” toggle.

---

## 7. File index must take a hint (C16)

Today:

```text
VectorizationService → modelConfigService.getDefaultModel('VECTORIZE', $userId)
```

That is the account / system default. A project EMBED binding that
never leaves the laptop is theatre.

`PS4`: `POST /api/v1/files/upload` and the process path accept
optional `vectorize_model` and `analyze_model` (catalog keys).
`desktop:files` already allows the route — no new scope.

Validation:

- Unknown key → 400, do not fall back silently.
- Key not allowed for that capability → 400.
- Omitted hint → today's account default (web + old clients).
  Desktop **must not omit** when EMBED is set. When EMBED is unset,
  desktop **must not** upload with `process_level=vectorize`; store
  only or refuse with the Models empty-state.

---

## 8. Persist shape (client)

In `{config_dir}/projects/{id}.toml`:

```toml
[models]
chat      = "ollama:llama3.2:chat"          # catalog key or empty
voice     = "openai:whisper-1:sound2text"
speak     = ""
vision    = ""
image     = ""
video     = ""
embed     = "ollama:bge-m3:vectorize"
docs      = ""
```

TOML keys are the **UI slot ids** (lowercase), not `SOUND2TEXT`.
Comments in this folder may show both. The file on disk should be
readable without the PHP enum.

Personal migration: `chat = last_chat_model` if that string is
already a catalog key; if it is a providerId from today's picker,
store it in `chat` **and** a `chat_legacy_provider_id` so `PC6` can
rebind when the catalog arrives.

---

## 9. Unset / missing / unavailable

| State | Chat | Dictate | Index |
| ----- | ---- | ------- | ----- |
| Slot unset | Block send; ask to pick a Chat model | Block; ask to pick a Dictation model | Do not vectorize |
| Catalog says unavailable | Show reason; do not send | Same | Same |
| Catalog fetch failed | Keep last persisted keys; banner | Same | Same |
| Model removed from catalog | Treat as unavailable; do not silently pick another | Same | Same |

Never “helpfully” substitute the account default. Substitution is how
worlds leak.

---

## 10. Non-goals

- Changing web account `DEFAULTMODEL` from the desktop.
- Per-turn model override in v1 (the composer uses the project CHAT
  model; a one-off override is a later epic and must still be a
  model *from this project's allowed provider story* — do not add it
  now).
- Client-side filtering of `/v1/models` pretending to be a catalog.
- Putting grouped models behind `messages:*`.
- Applying Assistant `models.*` “just for this turn”.
- Operator-only VECTORIZE remaining the only index model for
  desktop-uploaded files.

---

## 11. Acceptance

- Checklist rows 11–20 ticked.
- `GET /v1/models/catalog` returns the eight groups with catalog-key
  `id`s (`PS3`).
- Upload honours `vectorize_model` / `analyze_model` (`PS4`).
- Project file stores eight keys (`PC6`).
- `send_chat` / `send_agent_chat` send project CHAT (`PC7`).
- File upload sends EMBED/DOCS (`PC9`).
- Dictation sends VOICE (`PC12`).
- Assistant bind shows the world warning; tests prove the body model
  is still the project CHAT key (`PC10` + `PS2`/`PS5`).
