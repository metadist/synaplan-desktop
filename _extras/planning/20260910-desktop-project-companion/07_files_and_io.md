# Files and I/O — knowledge folder

**Status:** Draft 2026-09-10.
**Depends on:** checklist rows 2, 13, 16; `PS4`; [`02_sovereign_models.md`](./02_sovereign_models.md).
**Unlocks:** `PC9`, share-note, In board.
**Repos:** client `files.rs` + Files view; server hints already in
[`03_server_v1_extensions.md`](./03_server_v1_extensions.md).
**Done:** a dropped file lands in `DESKTOP:{projectId}`, is vectorized
with the project EMBED model (and analyzed with DOCS when that path
runs), and the UI shows status in plain language.

---

## 0. Why this file exists

Desktop already uploads skill artifacts with
`process_level=store` and **no** `group_key`
(`src-tauri/synaplan-core/src/files.rs`). That is correct for “here
is a pptx result”. It is wrong for “this PDF belongs to this
project and should be searchable in chat”.

If we add `group_key` but not `vectorize_model`, RAG uses the
account `VECTORIZE` default and the Models panel is a lie (C16).

---

## 1. Code to read first

| Path | Why |
| ---- | --- |
| `src-tauri/synaplan-core/src/files.rs` | Today's store-only upload |
| `backend/src/Controller/FileController.php` | `group_key`, `process_level`, list filters |
| `backend/src/Service/File/VectorizationService.php` | Account default today |
| `backend/src/Service/Message/Handler/ChatHandler.php` | `rag_group_key` |

---

## 2. Knowledge folder

| UI | Wire |
| -- | ---- |
| Knowledge folder | `group_key = DESKTOP:{projectId}` |

- `{projectId}` is the **stable local id**, not the slug.
- Not user-editable in v1.
- Never shown as `group_key` / `DESKTOP:` in primary copy. Details
  / doctor may show the technical string behind “Show details”.

Chat turns send `x-synaplan-rag-group-key: DESKTOP:{id}` (`PS2` /
`PC10`) so RAG searches this folder.

---

## 3. Upload (`PC9`)

Extend the Rust client (keep `upload_store` for artifacts):

```text
upload_project_file(base, key, path, project_id, vectorize_model, analyze_model)
  POST /api/v1/files/upload
  files[]
  process_level=vectorize
  source=api
  group_key=DESKTOP:{project_id}
  vectorize_model={catalog key}    // required if we vectorize
  analyze_model={catalog key}      // if set
```

Rules:

- EMBED unset → **do not** vectorize. Either refuse with “Pick a
  model for indexing files” or upload `process_level=store` and
  show “Saved in Synaplan but not searchable yet”. Prefer **refuse
  to claim it is in the knowledge folder** until EMBED is set.
- EMBED set → always send the hint. Never omit “to use the default”.
- DOCS set → send `analyze_model`. Unset → omit (analyze may use
  server default **only** if that path is not a sovereignty-critical
  read; if analyze feeds RAG text, treat DOCS like EMBED and refuse
  or send). Safer v1: if the process path will call ANALYZE, require
  DOCS or skip analyze.
- VISION: if the file is an image and a later turn needs OCR /
  understand, use project VISION — do not let the Assistant vision
  key win.
- 401 → existing disconnect path; delete key; do not leave a
  “uploading…” row forever.

List: `GET /api/v1/files` with `group_key` if the API supports it
(it does as a query param). Desktop key already may.

---

## 4. Status UI

Plain language, not vector-state enums:

| Internal | UI intent |
| -------- | --------- |
| local only | On this computer |
| uploaded / stored | Sent to Synaplan |
| extracting | Reading the file… |
| vectorizing | Indexing with this project's model… |
| vectorized | Ready for chat |
| error | Could not index — {plain reason} |

Failures: quota, model unavailable, assistants/files off, network.
Never dump PHP exception traces.

Optional share from Notes uses the same pipeline (`06` §2.1).

---

## 5. In / Out (files half)

**In:** queued / uploaded / indexed files; shared notes; last
successful index time.

**Out:** project `{slug}/out` artifacts; “Show in folder”; files
attached to a chat turn.

The board UI lives on Agents (`08`). Files view is the drop zone +
status list.

---

## 6. Confinement

Pickers start in user-chosen folders (existing allowlist). A file
read for upload is a **copy to Synaplan**, not a new write root.
Do not add the whole home directory.

Project `out/` is a write root for that project only.

---

## 7. Tests

| Case | Assert |
| ---- | ------ |
| Multipart includes group_key, process_level=vectorize, vectorize_model | Rust unit with captured form |
| EMBED unset → no vectorize upload | Unit |
| 401 | Unauthorized mapping |
| List filtered to DESKTOP:{id} | Unit / mock |

Server hint tests live in `PS4`/`PS5`.

---

## 8. Non-goals

- Syncing the local notes folder into Synaplan automatically.
- Editing `group_key` in the UI.
- Using `process_level=full` unless a later need appears.
- Uploading from JS `fetch` (key leak).
- Changing the skill-artifact `upload_store` default without an
  explicit project context.

---

## 9. Acceptance

- `files.rs` can store (old) and vectorize-into-project (new).
- Every project vectorize call carries EMBED (C16).
- Files view shows the §4 statuses.
- Chat RAG header uses the same `DESKTOP:{id}`.
