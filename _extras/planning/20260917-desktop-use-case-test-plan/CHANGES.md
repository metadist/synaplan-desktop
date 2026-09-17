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

The three bugs from the Windows walk (Astra `max_tokens`, file with no
answer, gpt-oss-120b "looking online" with no sources) are **C1, C3, C2**.
They are not Desktop-only — two of the three live in the Synaplan Messages
gateway that Desktop already calls.

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

### C1. GPT-6 Astra rejects `max_tokens` on `/v1/messages`

| | |
| --- | --- |
| **Repo** | `synaplan` |
| **Files** | `backend/src/AI/Messages/Translator/OpenAiMessagesTranslator.php` (`toOpenAiRequest`, ~175); reuse the heuristic in `backend/src/AI/Provider/OpenAIProvider.php` (`usesCompletionTokens` / `isGptReasoningFamily`) |
| **What** | The Messages gateway always copies the Anthropic field `max_tokens` into the Chat Completions body. GPT-6 / GPT-5 / o-series reject that name. The regular `OpenAIProvider` chat path already remaps (and for several models talks to the Responses API). The Desktop path never enters `OpenAIProvider` — it is the translator. Map `max_tokens` → `max_completion_tokens` for those models. If the catalog row says `meta.api = responses` (Astra does), do not keep hitting `/v1/chat/completions` — add or select a Responses translator the way `OpenAIProvider::chat()` already does. Keep the Anthropic *client* contract (`max_tokens` required in `MessagesGateway::prepare`). |
| **Why** | UC-01/02/03/04/05/10: every Astra turn dies before a token with `Unsupported parameter: 'max_tokens' is not supported with this model. Use 'max_completion_tokens' instead.` That is the first picker item a person can choose. |
| **Gate** | UC-01 Astra answers `PONG`. UC-02 writes `hello.md`. UC-10 no longer contains the string `max_tokens`. |
| **Do not** | Teach Desktop to send `max_completion_tokens`. The client speaks Anthropic Messages; the gateway translates. |

### C2. Web on → empty bubble (gpt-oss-120b and often Claude)

| | |
| --- | --- |
| **Repo** | `synaplan` first, then `synaplan-desktop` |
| **Files** | `backend/src/AI/Messages/Tools/GatewayToolLoop.php` (stream loop, ~273–330); `GatewayToolCatalog.php` (who owns `web_search`); `backend/src/AI/Messages/Translator/OpenAiMessagesTranslator.php` (server-tool strip); `synaplan-desktop/src-tauri/synaplan-core/src/sse.rs` |
| **What** | Three failures, one product sentence: *the person turns Web on and does not get an answer with a source.* |
| | **(a) Gateway must not end a web-search turn in silence.** After the last server-tool round, if no assistant text was emitted, write one terminal sentence: either the sources that came back, or "The web search finished without a usable answer. Try again, or turn Web off." Never `message_stop` with an empty message. That is UX rule U8. |
| | **(b) Cap search loops at two rounds, then force a wrap-up.** Groq gpt-oss-120b spent 6–8 gateway pings (ceiling is 4) and still produced `client_text=""`. The loop default is `mcp_max_iterations = 8`. A `web_search` that keeps calling itself is not progress. After two executed searches, inject a wrap-up turn ("answer now, cite the URLs you have") and stop. |
| | **(c) Anthropic thinking blocks must survive the loop.** Run 1 closed Claude with `messages.1.content.0.thinking: each thinking block must contain thinking`. Run 3 closed Claude with `stop_reason=end_turn` and still `client_text=""`. When the gateway appends `tool_result`, it must keep every `thinking` block the model sent, including the text. Empty or dropped thinking is an invalid follow-up. |
| | **(d) Desktop: empty `Done` is a failure, not success.** `SseParser` only promotes `text_delta` to `ChatEvent::Token`. Citations, thinking, and `server_tool_result` are dropped. If the stream ends with no token and no `error`, show a named recovery (`chat.searchReturnedNothing`) instead of a blank assistant row. If the gateway later emits citation blocks, parse them. |
| **Why** | This is "I am looking online for an answer" with nothing after it. The model (or an intermediate hidden round) said it would search; the client never received a source. xAI Grok 4.6 already passes UC-03 on the same stack — the search provider works. |
| **Gate** | UC-03: Claude and gpt-oss-120b return non-empty text containing `http`, `stop_reason=end_turn`, ≤ 4 pings. UC-04 stays green for those two. |

### C3. Drop a file, ask, get no real answer

Three independent causes. Fix all three; any one reproduces the walk.

#### C3a. RAG context is frozen on the first user turn for two hours

| | |
| --- | --- |
| **Repo** | `synaplan` |
| **Files** | `backend/src/AI/Messages/MessagesContextInjector.php` (`sessionBlock`, cache TTL 7200, query = `firstUserText`) |
| **What** | Session key = first user text + user id (+ rag group). The injected RAG block is computed **once** and replayed. If the person asks before the file exists, that empty result is cached for two hours. Asking again in the same thread after `Ready` still injects "no files". Desktop always sends `x-synaplan-rag-group-key`; when that header is present, **re-search every turn** (or bust the cache when the folder's file set / newest `Ready` changes). Do not cache an empty RAG hit for a desktop group key. |
| **Why** | UC-05d (runs 2 and 3): before upload the model correctly says it cannot see the file; after `Ready` in the **same** thread it still says it cannot. A new thread with the same question (UC-05 main) answers `ZETA-…`. That is exactly "I uploaded a file and got no real answer" when the person stays in the thread they already opened. |
| **Gate** | UC-05d after-Ready hit=true. |

#### C3b. Office and PDF rows go `Ready` with no searchable text

| | |
| --- | --- |
| **Repo** | `synaplan` |
| **Files** | `backend/src/Service/File/FileProcessor.php`, `VectorizationService.php`; Desktop `src-tauri/synaplan-core/src/files.rs` only if the Files row must show a new state |
| **What** | A 1 KB generated `.docx` and a minimal text `.pdf` become `Ready` in seconds, then a fact question on that file is answered as "not in your project files". Either Tika/Docling extracted nothing, or empty text was still marked indexed. A file with zero extracted characters must be `Failed` with one sentence ("We could not read any text from this file") — never `Ready`. Confirm the extractor actually returns the planted sentence for the UC-06 fixtures (they are valid PDF 1.4 + OOXML). If the fixtures are too thin for Tika, thicken them in `examples/usecases/fixtures.rs` **and** keep a real Word/PDF in the corpus so we do not green-wash. |
| **Why** | UC-06 (run 1): 7/7 `Ready` in 7 s, handbook Markdown fact hit, `finance-*.docx` and `survey-*.pdf` miss. The Files panel lied: Ready ≠ answerable. |
| **Gate** | UC-06 facts from the `.docx` and the `.pdf` both hit. A deliberately empty PDF is `Failed`, not `Ready`. |

#### C3c. The model that cannot chat cannot answer a file

C1 unblocks Astra on UC-05. Until C1 lands, the picker must not present
Astra as a working chat model (see C7). No separate Desktop file-path
change.

---

## P1 — same surfaces, next

### C4. Gemini tool turns drop `thought_signature`

| | |
| --- | --- |
| **Repo** | `synaplan` |
| **Files** | `backend/src/AI/Messages/Translator/GeminiMessagesTranslator.php` (`mapGeminiToAnthropic` ~202, `mapMessage` tool_use ~298) |
| **What** | Gemini 3.x requires `thought_signature` on every `functionCall` part of a follow-up. The translator strips it when mapping to Anthropic `tool_use` and never puts it back. Persist the signature on the tool_use block (extension field is fine) and replay it on the next Gemini request. |
| **Why** | UC-02 and UC-03 Gemini: first `write_file` / `web_search` succeeds, then `Function call is missing a thought_signature…` The person sees a Google API essay. |
| **Gate** | UC-02 Gemini writes `hello.md` and ends in text. UC-03 Gemini returns a URL or a named failure, never `thought_signature`. |

### C5. A cut-off answer looks finished

| | |
| --- | --- |
| **Repo** | `synaplan-desktop` |
| **Files** | `src-tauri/synaplan-core/src/sse.rs`; `src-tauri/synaplan-core/src/messages.rs`; chat UI + all five locales |
| **What** | Raw SSE already reports `stop_reason=max_tokens`. `SseParser` ignores `message_delta` and emits `ChatEvent::Done`. Add `ChatEvent::Truncated` (or a flag on Done). The bubble must say the answer was cut off and offer "Continue". The agent loop already has `CONTINUE_AFTER_CUTOFF` — plain chat does not. |
| **Why** | UC-09: 643 words, raw `max_tokens`, client received `Done`. |
| **Gate** | UC-09 "client is told the answer was cut off" = ok. |

### C6. Provider jargon must not reach the bubble

| | |
| --- | --- |
| **Repo** | `synaplan-desktop` (display) and `synaplan` (do not forward raw upstream on `/v1/messages` when we can classify it) |
| **Files** | `synaplan-desktop/src/composables/useErrorText.ts`; `src-tauri/synaplan-core/src/messages.rs` (`ChatError::Server`); gateway error mapper |
| **What** | Known codes already localize. `ChatError::Server(String)` is the raw provider line. Classify at least: unsupported parameter, missing thought_signature, subscription tier, overloaded. Copy: "This model cannot answer right now. Pick another model." / "This model needs a higher plan at the provider. Pick another model." Never `max_tokens`, never a Google thought-signature URL. |
| **Why** | UC-10 Astra shows the `max_tokens` sentence. That is the same string the Windows walk hit. |
| **Gate** | UC-10 Astra (and Gemini, once C4 is open) fail with plain language. |

### C7. Catalog says "available" for models the provider will refuse

| | |
| --- | --- |
| **Repo** | `synaplan` |
| **Files** | `backend/src/AI/Credential/ChatReadinessService.php` (`modelAvailability`); `backend/src/Service/Model/CapabilityCatalog.php` |
| **What** | Availability today is "provider has a key" (plus Ollama `not_pulled`). `mistral-large-latest` is listed available and 400s with `This model is not available in your subscription tier`. After a classified provider refusal, mark that **catalog key** unavailable with `unavailableReason` the picker already renders (`ModelSlotRow` disables `available=false`). Do not hide the whole Mistral provider. Same path can mark Astra unavailable until C1 lands, so the morning walk cannot pick a broken row. |
| **Why** | UC-01/02/03 Mistral. 44/54 CHAT rows look ready; at least one is not. |
| **Gate** | UC-01 Mistral is either skipped as unavailable or answers `PONG`. The picker must not offer a row that UC-01 fails. |

### C8. PHP memory on en-bloc vectorize (watch)

| | |
| --- | --- |
| **Repo** | `synaplan` |
| **Files** | `backend/src/Service/File/VectorizationService.php`; backend/worker PHP `memory_limit` |
| **What** | While enabling real `ollama:bge-m3` embeddings, a 3 MB Markdown upload hit `OutOfMemoryError` in `VectorizationService` (run 2 investigation). Run 1 evaded this because VECTORIZE was `not_pulled` and files still flipped to `Ready` (that lie is C3b). Chunk embedding must stream; a single file must not load the whole corpus into one PHP array. If the worker needs a higher limit, set it explicitly and test with UC-06's 5 MB folder. |
| **Why** | A Desktop "drop a folder" that 500s on the third megabyte is the en-bloc story. Confirm on run 3 STATUS (`UC-06` upload/ready rows). |
| **Gate** | UC-06: 7/7 accepted, 0 failed, ≥ 1 MB/min, ≥ 2/3 facts. No PHP fatal in worker logs. |

---

## P2 — measured, do not start on a guess

### C9. Dictation feel on Windows

UC-07 **passed** on this host: 31 s take → final text in **2.9 s** after
stop, similarity 0.99 ( Groq Whisper ). Of that 2.9 s, **2.8 s is the
correction pass**. The live commit was 43 ms.

The Windows walk still called dictation "VERY slow". That is not a
failed budget here. Next measurement is on the Windows build, stopwatch
from the mic button, with the correction pass on and off. If Windows
stop→final is > 10 s, profile capture + `/v1/audio` + the second
session in `useDictation.ts`. Likely change: show the live commit as
the note immediately, run the correction pass in the background, and
replace the text when it arrives. Do not rewrite the audio stack
without a new UC-07 row from Windows.

### C10. Groq mixed server/client tool chatter

UC-04 Groq sometimes spends several `web_search: call it alone, then
continue` retries and still writes `sources.md`. After C2(b) this should
collapse. If UC-04 still fails "mixed server/client steps stay rare",
teach `dispatch_tool` / the agent prompt that `web_search` is
server-owned and must not be paired with `read_file` of a URL in the
same step (`agent_tools.rs`, `WEB_SEARCH_PROMPT`).

---

## Already green (do not "fix")

| Case | Evidence |
| ---- | -------- |
| UC-00 | After `COMPOSE_PROFILES` includes `local-ai` and `bge-m3` is pulled, catalog + VECTORIZE + SOUND2TEXT + TEXT2SOUND are available (runs 2 and 3). |
| UC-01 | Claude, Groq, Gemini, Grok answer `PONG` well inside the 15 s first-token ceiling. |
| UC-02 | Claude, Groq, Grok write `hello.md` in one or two tool steps. |
| UC-04 | Claude (and usually Groq) write `sources.md` with ≥ 3 URLs. |
| UC-05 main / UC-05c | Fresh thread and second-turn small-talk retrieve the Markdown fact (Claude, Groq). |
| UC-07 | Dictation protocol + correction pass meet the 10 s ceiling on this host. |
| UC-08 | Local notes: create, write, read, search, append, delete, path refusal. |

---

## Suggested implementation order

1. **C1** — unblocks Astra on every case at once. Small, local to the OpenAI messages translator.
2. **C2 (a)(b)(c)** — unblocks Web for Groq and Claude. Product-visible.
3. **C3a** — unblocks "I just dropped a file in this chat".
4. **C6** — so any remaining provider error is readable while C4/C7 land.
5. **C4, C5, C3b, C7, C8** — same sprint if capacity allows; each is independently shippable.
6. **C9** only after a Windows UC-07 number.

Two PRs (house rule: Desktop `make ci-local` and Synaplan unfiltered gate
are never the same PR):

- `synaplan`: C1, C2a–c, C3a, C3b, C4, C7, C8.
- `synaplan-desktop`: C2d, C5, C6, and picker copy if C7 needs a new
  `unavailableReason`.

---

## Done

_(empty — move a row here when its STATUS check is green on a recorded run.)_
