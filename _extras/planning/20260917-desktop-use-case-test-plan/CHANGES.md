# Required code changes

**Status:** Active. Each row is work the use-case runner demanded.
Do not tick a row by editing this file — tick it by turning the matching
`STATUS.md` check green, then move the row to § Done.

**Evidence:** three live runs against `http://localhost:8000` on 2026-09-17,
same paired admin, `--quick` matrix (Anthropic Claude Fable, Groq gpt-oss-120b,
OpenAI GPT-6 Astra, Google Gemini 3.5 Flash, xAI Grok 4.6, Mistral Large):

| Run | Id | Notes |
| --- | -- | ----- |
| 1 | `B6GTY` | Full suite. VECTORIZE listed `not_pulled` (later repaired). 2 passed / 9 failed in 250 s. |
| 2 | `TMR43` | VECTORIZE healthy. Aborted at the start of UC-06 (harness process killed). UC-00–05 match run 1. |
| 3 | `6GW9Q` | Full `--quick` rerun after the abort. Ledger in `STATUS.md`. |
| 4 | `V42D2` | 2026-10-23 `--full` against local `main`. synaplan `bb70055b055e773b4ed6b205c63b77e7b479aa72`, desktop `12e2510f035fc6776c34642e74007023d4119bd6`. 5 passed / 6 failed in 1533 s. Report `runs/20261023081938.json`. |

The three bugs from the Windows walk (Astra `max_tokens`, file with no
answer, gpt-oss-120b "looking online" with no sources) were **C1, C3, C2**.
C1, C2 and C3a are in **Done** on run `V42D2`. C3b is still open. They are
not Desktop-only — two of the three lived in the Synaplan Messages gateway
that Desktop already calls.

Repos: `synaplan/` (gateway, catalog, files) and `synaplan-desktop/`
(SSE parser, error copy, picker). No change here is a `store-required`
mobile path.

---

## How to read this

- **P0** — a person hits this on the first morning walk. Fix before any
  Desktop release that claims chat / files / Web.
- **P1** — same surfaces, next sprint. A model the picker offers still
  cannot finish the job.
- **P2** — measured, not guessed. Do not start these to "feel faster"
  without a new STATUS row.
- Every change names **repo**, **files**, **what**, **why**, and the
  **UC-xx** that turns green. A PR without that UC going green is not done.

Re-run after a batch:

```bash
cd /wwwroot/synaplan-desktop/src-tauri
cargo run -p synaplan-core --example usecases -- --quick
```

---

## P0 — the morning-walk bugs

### C3. Drop a file, ask, get no real answer

C3a is in **Done** (run `V42D2`). C3b is still open. C3c has no separate check on this run: UC-05 did not include Astra.

#### C3b. Office and PDF rows go `Ready` with no searchable text

| | |
| --- | --- |
| **Repo** | `synaplan` |
| **Files** | `backend/src/Service/File/FileProcessor.php`, `VectorizationService.php`; Desktop `src-tauri/synaplan-core/src/files.rs` only if the Files row must show a new state |
| **What** | A 1 KB generated `.docx` and a minimal text `.pdf` become `Ready` in seconds, then a fact question on that file is answered as "not in your project files". Either Tika/Docling extracted nothing, or empty text was still marked indexed. A file with zero extracted characters must be `Failed` with one sentence ("We could not read any text from this file") — never `Ready`. Confirm the extractor actually returns the planted sentence for the UC-06 fixtures (they are valid PDF 1.4 + OOXML). If the fixtures are too thin for Tika, thicken them in `examples/usecases/fixtures.rs` **and** keep a real Word/PDF in the corpus so we do not green-wash. |
| **Why** | UC-06 (run 1): 7/7 `Ready` in 7 s, handbook Markdown fact hit, `finance-*.docx` and `survey-*.pdf` miss. Run `V42D2` (2026-10-23): 7/7 accepted, 7 ready, 0 failed, 37.72 MB/min, and **0/3 facts** — the handbook markdown missed as well. The Files panel still says Ready when the fact is not answerable. Gate not green. |
| **Gate** | UC-06 facts from the `.docx` and the `.pdf` both hit. A deliberately empty PDF is `Failed`, not `Ready`. |

#### C3c. The model that cannot chat cannot answer a file

C1's gate is green on `V42D2`, and Astra answered UC-01 and wrote the UC-02 file. UC-05 on that run used Sonnet 5.5, gpt-oss-120b and mistral-large-4 only, so Astra on a file question is still unmeasured. No separate Desktop file-path change.

---

## P1 — same surfaces, next

### C4. Gemini tool turns drop `thought_signature`

| | |
| --- | --- |
| **Repo** | `synaplan` |
| **Files** | `backend/src/AI/Messages/Translator/GeminiMessagesTranslator.php` (`mapGeminiToAnthropic` ~202, `mapMessage` tool_use ~298) |
| **What** | Gemini 3.x requires `thought_signature` on every `functionCall` part of a follow-up. The translator strips it when mapping to Anthropic `tool_use` and never puts it back. Persist the signature on the tool_use block (extension field is fine) and replay it on the next Gemini request. |
| **Why** | UC-02 and UC-03 Gemini: first `write_file` / `web_search` succeeds, then `Function call is missing a thought_signature…` The person sees a Google API essay. Run `V42D2`: no `thought_signature` string in the failures. `google:gemini-2.5-flash-lite:chat` still failed UC-02 (file missing; the reply said it cannot create files). `google:gemini-3.7-flash:chat` answered UC-03 and used 5 gateway rounds (ceiling 4). Gate not green. |
| **Gate** | UC-02 Gemini writes `hello.md` and ends in text. UC-03 Gemini returns a URL or a named failure, never `thought_signature`. |

### C6. Provider jargon must not reach the bubble

| | |
| --- | --- |
| **Repo** | `synaplan-desktop` (display) and `synaplan` (do not forward raw upstream on `/v1/messages` when we can classify it) |
| **Files** | `synaplan-desktop/src/composables/useErrorText.ts`; `src-tauri/synaplan-core/src/messages.rs` (`ChatError::Server`); gateway error mapper |
| **What** | Known codes already localize. `ChatError::Server(String)` is the raw provider line. Classify at least: unsupported parameter, missing thought_signature, subscription tier, overloaded. Copy: "This model cannot answer right now. Pick another model." / "This model needs a higher plan at the provider. Pick another model." Never `max_tokens`, never a Google thought-signature URL. |
| **Why** | UC-10 Astra shows the `max_tokens` sentence. That is the same string the Windows walk hit. Run `V42D2`: UC-10 passed. The four failures are Hugging Face models and the person sees "Choose a model to chat with, then try again." Astra did not fail, so an Astra failure in plain language was not observed. Gate not fully met. |
| **Gate** | UC-10 Astra (and Gemini, once C4 is open) fail with plain language. |

### C7. Catalog says "available" for models the provider will refuse

| | |
| --- | --- |
| **Repo** | `synaplan` |
| **Files** | `backend/src/AI/Credential/ChatReadinessService.php` (`modelAvailability`); `backend/src/Service/Model/CapabilityCatalog.php` |
| **What** | Availability today is "provider has a key" (plus Ollama `not_pulled`). `mistral-large-latest` is listed available and 400s with `This model is not available in your subscription tier`. After a classified provider refusal, mark that **catalog key** unavailable with `unavailableReason` the picker already renders (`ModelSlotRow` disables `available=false`). Do not hide the whole Mistral provider. Same path can mark Astra unavailable until C1 lands, so the morning walk cannot pick a broken row. |
| **Why** | UC-01/02/03 Mistral. Run `V42D2`: 51 of 62 CHAT rows available. `mistral:mistral-large-4:chat` stayed available and UC-01 returned empty text in 3769 ms. `mistral-large-latest` and `mistral-medium-latest` answered `PONG`. Four Hugging Face rows are counted available and UC-01 shows `model_unavailable`. Gate not green. |
| **Gate** | UC-01 Mistral is either skipped as unavailable or answers `PONG`. The picker must not offer a row that UC-01 fails. |

### C8. PHP memory on en-bloc vectorize (watch)

| | |
| --- | --- |
| **Repo** | `synaplan` |
| **Files** | `backend/src/Service/File/VectorizationService.php`; backend/worker PHP `memory_limit` |
| **What** | While enabling real `ollama:bge-m3` embeddings, a 3 MB Markdown upload hit `OutOfMemoryError` in `VectorizationService` (run 2 investigation). Run 1 evaded this because VECTORIZE was `not_pulled` and files still flipped to `Ready` (that lie is C3b). Chunk embedding must stream; a single file must not load the whole corpus into one PHP array. If the worker needs a higher limit, set it explicitly and test with UC-06's 5 MB folder. |
| **Why** | A Desktop "drop a folder" that 500s on the third megabyte is the en-bloc story. Run `V42D2`: 7/7 accepted, 0 failed, 37.72 MB/min, ready in 7.9 s. Facts were 0/3, so the gate is not green. This runner does not read worker logs. |
| **Gate** | UC-06: 7/7 accepted, 0 failed, ≥ 1 MB/min, ≥ 2/3 facts. No PHP fatal in worker logs. |

---

## P2 — measured, do not start on a guess

### C9. Dictation feel on Windows

On 2026-09-17 UC-07 **passed** on this host: 31 s take → final text in **2.9 s** after
stop, similarity 0.99 ( Groq Whisper ). Of that 2.9 s, **2.8 s is the
correction pass**. The live commit was 43 ms. Run `V42D2` failed in 0 ms:
the catalog has no SOUND2TEXT default, so this run did not time dictation.

The Windows walk still called dictation "VERY slow". That is not a
failed budget here. Next measurement is on the Windows build, stopwatch
from the mic button, with the correction pass on and off. If Windows
stop→final is > 10 s, profile capture + `/v1/audio` + the second
session in `useDictation.ts`. Likely change: show the live commit as
the note immediately, run the correction pass in the background, and
replace the text when it arrives. Do not rewrite the audio stack
without a new UC-07 row from Windows.

### C10. Groq mixed server/client tool chatter

UC-04 Groq on run 3 died with `HarmonyError: Tools should have a name!`
after one `web_search` (run 1 only burned retries). Run `V42D2` UC-04
Groq passed: 8 steps, 0 `web_search` retry hints, 3 URLs, 27748 ms. No
`HarmonyError` in this run. If a later run fails again, the injected
`web_search` declaration must carry a name Groq Harmony accepts
(`GatewayToolCatalog` / `WebSearchTool::NAME`).

---

## Already green (do not "fix")

| Case | Evidence |
| ---- | -------- |
| UC-01 | On `V42D2`, Claude, Groq, Gemini, Grok and Astra answer `PONG`. The case still fails: see C7. |
| UC-02 | On `V42D2`, Claude, Groq, Grok and Astra write `hello.md`. `gemini-2.5-flash-lite` does not: see C4. |
| UC-04 | `V42D2`: 3/3 models wrote `sources.md` with ≥ 3 URLs (Sonnet 5.5, gpt-oss-120b, mistral-large-4). |
| UC-05 main / UC-05c / UC-05d | `V42D2` passed, including UC-05d after Ready. See Done C3a. |
| UC-08 | `V42D2`: create, write, read, search, append, delete, path refusal. |
| UC-09 | `V42D2`: the client is told the answer was cut off. See Done C5. |

---

## Suggested implementation order

C1, C2, C3a and C5 are in **Done** on run `V42D2`. Still open:

1. **C3b** — Ready files whose facts are not answered (UC-06 is 0/3).
2. **C4, C7** — one Gemini file write, and models the catalog calls available that do not answer.
3. **C6, C8** — remaining error copy, and the UC-06 fact half of the indexing gate.
4. **C9** only after a Windows UC-07 number. This host has no SOUND2TEXT default on `V42D2`.

Two PRs (house rule: Desktop `make ci-local` and Synaplan unfiltered gate
are never the same PR):

- `synaplan`: C1, C2a–c, C3a, C3b, C4, C7, C8.
- `synaplan-desktop`: C2d, C5, C6, and picker copy if C7 needs a new
  `unavailableReason`.

---

## Done

Moved here only when the row's own gate was green on run `V42D2`
(2026-10-23, `cargo run -p synaplan-core --example usecases -- --full`,
synaplan `bb70055b055e773b4ed6b205c63b77e7b479aa72`, desktop
`12e2510f035fc6776c34642e74007023d4119bd6`). A green gate does not mean
the whole case passed.

### C1. GPT-6 Astra rejects `max_tokens` on `/v1/messages`

UC-01 `openai:gpt-6-astra:chat` answered `PONG` (ttft 3651 ms). UC-02 wrote
`hello.md` (2 tool steps). UC-10 passed: no user-visible failure contains
`max_tokens` (the four failures are Hugging Face models with "Choose a model
to chat with, then try again.").

### C2. Web on → empty bubble (gpt-oss-120b and Claude)

UC-03 `anthropic:claude-sonnet-5-5:chat` and `groq:openai/gpt-oss-120b:chat`
each returned non-empty text containing `http`, `stop_reason=end_turn`, 3
gateway rounds. UC-04 passed for both (Sonnet: 4 URLs; Groq: 3 URLs). The
UC-03 case is still FAIL for other models (see C4 and C7).

### C3a. RAG context frozen on the first user turn

UC-05d: before upload hit=false; after Ready in the same thread hit=true
(`BADGE-V42D2-77`). UC-05 passed.

### C5. A cut-off answer looks finished

UC-09 "client is told the answer was cut off" is ok. 616 words,
`stop_reason=max_tokens`, `truncated=true`, model `groq:openai/gpt-oss-120b:chat`.
