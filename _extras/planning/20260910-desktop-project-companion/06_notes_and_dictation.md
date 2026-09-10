# Notes and dictation

**Status:** Draft 2026-09-10.
**Depends on:** checklist rows 21–25; [`04_local_persistence.md`](./04_local_persistence.md);
[`02_sovereign_models.md`](./02_sovereign_models.md) VOICE slot.
**Unlocks:** `PC8`, `PC12`.
**Repos:** `synaplan-desktop`. Dictation HTTP is Rust; the WebView only
captures audio.
**Done:** Markdown notes on disk; dictation follows sani-sis, uses
**this project's** VOICE model and language, and never puts the API
key in JS.

---

## 0. Why this file exists

Notes are the local half of a project (files on *this* computer).
Dictation is how many people will fill them. The web chat already
has an upload-file STT path that **does not** honour language / model
/ prompt reliably. Copying that path would silently ignore the
project VOICE model — another sovereignty leak.

sani-sis already solved live + one-shot against Synaplan `/v1/audio`.
This epic copies that **protocol**, not the care-service product.

---

## 1. Code to read first

| Path | Why |
| ---- | --- |
| `/wwwroot/sani-sis/sani-sis/src/components/DiktatKnopf.vue` | Pause commit, dual capture, poll, full re-transcribe |
| `/wwwroot/sani-sis/sani-sis/src-tauri/src/synaplan.rs` | Rust HTTP, `commit_after_bytes` 480000, language + prompt |
| `backend/src/Controller/AudioTranscriptionController.php` | `/v1/audio/*` — already `desktop:messages` |
| `src/services/tauri.ts` | New invoke wrappers only here |

Server default auto-commit is **3 s** (`DEFAULT_COMMIT_AFTER_BYTES =
96000`). That cuts mid-word. The client **must** send
`commit_after_bytes: 480000` on session create.

---

## 2. Notes (`PC8`)

### 2.1 On disk

`{projects_dir}/{slug}/notes/*.md`

- Create: `YYYY-MM-DD-hhmm.md` or a slug from the first heading
  after save.
- UTF-8 Markdown. No HTML storage format.
- Title in the manager = first `# ` heading, else filename.
- Delete = confirm + unlink. Reveal in file manager = existing
  `reveal_path`.
- Search = filename + file text (simple substring in v1).
- Pin = optional local index; not a server flag.

Optional **Share with project knowledge**: confirm, then upload that
`.md` with `process_level=vectorize`, `group_key=DESKTOP:{id}`, and
the project EMBED/DOCS hints (`PC9`). Copy must say the note **leaves
this computer** and is indexed with this project's models.

### 2.2 Editor — Milkdown, ask first

**Ask / approve in the first notes PR before adding the npm
dependency.**

Proposed: [`@milkdown/vue`](https://milkdown.dev/) + commonmark
(crepe-slim if it stays small). Markdown is the file; the toolbar is
headings, bold/italic, lists, links, code. **No** Notion-like blocks,
comments, or database properties.

Why Milkdown: Vue 3, MD-in / MD-out, small. Why not TipTap: heavier
and invites a block editor. Why not a raw `<textarea>` only: we
promised a slim WYSIWYG; textarea is the fallback if the dep is
refused.

Caret must remain addressable for dictation insert.

### 2.3 Note manager

List + search + new + open + delete + reveal + optional share.
Do not build a second file tree of the whole `projects_dir`.

---

## 3. Dictation (`PC12`) — binding protocol

**Not** `/api/v1/messages/upload-file`. **Not** `/api/v1/files/upload`
for STT. **Not** on-device Whisper.

### 3.1 Always send

On `POST /v1/audio/transcriptions` **and** on session create:

| Field | Source |
| ----- | ------ |
| `model` | Project VOICE catalog key / providerId (same rule as CHAT) |
| `language` | Project `dictation_language` (ISO). **Not** hardcoded `de`. **Not** UI locale unless the user set them equal. |
| `prompt` | Generic note-taking prompt, locale-appropriate (five strings). **Not** the sani-sis care-service prompt. |

If VOICE is unset or unavailable: do not open the mic; send the user
to Models.

### 3.2 Live session

```text
POST /v1/audio/transcriptions/sessions
encoding:            pcm_s16le
sample_rate:         16000
channels:            1
commit_after_bytes:  480000    // 16 kHz * 2 bytes * 15 s
language / model / prompt: as §3.1
```

Auth: `Authorization: Bearer` + `X-API-Key` from **Rust**, key from
the secret store.

### 3.3 Client commit (Whisper needs a sentence)

From `DiktatKnopf.vue`:

| Constant | Value | Why |
| -------- | ----- | --- |
| Sample rate | 16000 | Server PCM |
| `PAUSE_MS` | 700 | End of phrase |
| `MIN_STUECK` | 48000 samples (3 s) | Shorter is not recognized well |
| `MAX_STUECK` | 240000 samples (15 s) | Cap if someone never pauses |
| Poll | ~1800 ms | Interim text |

Commit when `(pause && samples >= 48000) || samples >= 240000`.
Do **not** send silence. No chunk overlap.

### 3.4 Dual capture + prefer one-shot

1. Live PCM → session chunks → poll `GET` session ~1.8 s → interim
   text at the caret (notes) or composer (chat).
2. Parallel `MediaRecorder` blob of the **whole** take.
3. On stop: `POST /v1/audio/transcriptions` with the **full** blob,
   same language + model + prompt.
4. **Prefer the one-shot result** over the concatenated live text.
5. Then close the session.

No `transkript_glaetten` (sani-sis LLM smoothing) in v1.

### 3.5 Where the bytes go

```text
WebView                  Rust                         Synaplan
MediaRecorder  ----->    tauri command                /v1/audio/*
ScriptProcessor PCM ---> (key stays in secret store)
                         never echo key to JS
```

JS may hold PCM / blobs. JS may **not** hold the API key, base URL
+ key combos, or a home-rolled `fetch` to `/v1/audio`.

Tauri **microphone** capability is required. Deny → plain-language
error, no crash.

### 3.6 Models list for the VOICE slot

Prefer catalog `SOUND2TEXT` (`PS3`). `GET /v1/audio/models` is the
fallback listing. Every STT call still uses the **project** VOICE
binding, not “whatever the audio route would default to”.

---

## 4. Prompt (generic, five locales)

EN intent (write the real strings in i18n, not in Rust):

> Notes and spoken sentences. Keep punctuation. Do not summarise.
> Do not invent names. Transcribe only what was said.

DE/ES/FR/TR: same meaning. **Not** nursing / care / “Klienten”.

Do not put the project name in the prompt unless we later add it as
an explicit option (PII).

---

## 5. Insert rules

- Notes: insert at caret; do not replace the whole document on
  one-shot unless the take started with an empty selection and we
  tracked the interim range — prefer replacing only the interim
  span from this take.
- Chat: insert into the composer, not a sent user message.
- Busy flag disables send / second mic.

---

## 6. Tests

| Kind | Assert |
| ---- | ------ |
| Rust unit | Session body contains language, prompt, `commit_after_bytes=480000`, project model |
| Rust unit | One-shot form fields match |
| Rust unit | 401 → existing unauthorized path; key not in error string |
| Vue unit | Commit math: 2.9 s + pause does **not** commit; 3.0 s + pause does; 15 s commits without pause |
| Vue unit | Stop prefers one-shot text |
| Gate | No `fetch('/v1/audio` in `src/` JS |

Manual: one take in EN and one in DE on a real workspace.

---

## 7. Non-goals

- On-device Whisper / whisper.cpp in this epic.
- Web ChatInput upload-file STT.
- `transkript_glaetten`.
- Hardcoded German.
- Storing recordings.
- Speaker diarization.
- Using SPEAK to read the note back (picker only).

---

## 8. Acceptance

- Notes round-trip on disk (`PC8`).
- Milkdown added only after an explicit ask in that PR.
- Dictation matches §3; VOICE + language on every call (`PC12`).
- Key never in JS (review + grep).
- Five locale prompts.
