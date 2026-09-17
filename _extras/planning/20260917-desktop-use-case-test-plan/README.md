# Desktop use-case test plan (2026-09-17)

**Status:** Active. This folder is the binding test contract for every Synaplan
Desktop release from now on. [`STATUS.md`](./STATUS.md) is the ledger the runner
writes; [`CHANGES.md`](./CHANGES.md) is the list of code changes the runs
demanded.
**Trigger:** the 2026-09-17 morning walk on the Windows build found three bugs
within minutes — GPT-6 Astra answered with a `max_tokens` provider error, an
uploaded file produced no real answer, and gpt-oss-120b said "I am looking
online for an answer" and never showed a result. The 2026-09-16 release
verification ([`../../testing/20260916-release-use-cases/`](../../testing/20260916-release-use-cases/README.md))
had exercised three *project* stories but not the everyday paths (model
switch, plain file question, web search in plain chat, dictation timing,
bulk indexing). This plan closes that gap and makes the check repeatable.
**Related:** UX contract `synaplan/_devextras/planning/202609_ux_user_flows.md`
(U1–U12), [`../20260910-desktop-project-companion/09_testing_and_gates.md`](../20260910-desktop-project-companion/09_testing_and_gates.md).

---

## 1. Principles

1. **Use cases, not screens.** Every case is a thing a person does on the
   Desktop (pick a model and ask, drop a file and ask, turn Web on and ask,
   dictate a note, index a folder). A green unit test is not a green use case.
2. **The production core is the test subject.** The runner is a Rust example
   inside `synaplan-core` and calls the same functions the Tauri commands
   call (`messages::stream_chat`, `agent::run_agent_turn`,
   `files::upload_project_file`, `dictation::*`, `ProjectStore`). No
   re-implementation of the wire shape. When the runner sees an empty
   answer, the user would have seen an empty answer.
3. **Contract view beside the client view.** For streaming turns the runner
   also captures the raw SSE (`stop_reason`, block types, ping count) so a
   failure says *where* it happened: upstream, gateway tool loop, or the
   client parser.
4. **Honest outcome is an assertion.** A turn that ends without terminal
   text, a truncation the user is not told about, a provider error shown
   raw — each is a failing check (UX rule U8), not a note.
5. **Budgets are numbers.** Time-to-first-token, time-to-final-transcript,
   MB/min indexed — every performance case has a target and an acceptable
   ceiling, both recorded per run so a regression is visible in
   `STATUS.md` history.
6. **Every run leaves a record.** `runs/<timestamp>.json` (machine) and
   `STATUS.md` (human), key never included. Findings are collected from the
   failing checks and copied into `CHANGES.md` by hand with an owner and a
   repo.
7. **Runs cost money and minutes.** `--quick` uses six representative chat
   models; `--full` walks every available CHAT model. The full matrix is a
   release gate, the quick set is the pre-PR gate.

---

## 2. How to run

Prerequisites: the `synaplan/` dev stack up (`docker compose up -d`), a paired
desktop key for the target user, `ffmpeg` on the host (dictation case only).

```bash
# 1. Pair a harness computer (writes /tmp/synaplan-desktop-harness.creds, mode 0600).
#    Same user as the desktop you are checking; the knowledge folder is per user.
cd /wwwroot/synaplan && python3 _devextras/testing/desktop/pair.py admin@synaplan.com '<password>' 'Use-case harness'
#    (or ./pair.sh when jq is installed)

# 2. Run the plan from the desktop repo.
cd /wwwroot/synaplan-desktop/src-tauri
cargo run -p synaplan-core --example usecases -- --quick            # pre-PR gate (~5 min)
cargo run -p synaplan-core --example usecases -- --full             # release gate (~20 min)
cargo run -p synaplan-core --example usecases -- --only UC-03,UC-05 # one story
cargo run -p synaplan-core --example usecases -- --models groq:openai/gpt-oss-120b:chat,openai:gpt-6-astra:chat
cargo run -p synaplan-core --example usecases -- --help
```

Credentials come from `--creds <file>` (default `/tmp/synaplan-desktop-harness.creds`,
`KEY=VALUE` lines `DESKTOP_KEY`, `API_BASE_URL`) or from the environment
(`SYNAPLAN_DESKTOP_KEY`, `SYNAPLAN_BASE_URL`). The key is never written to a
report or printed.

The runner writes `runs/<UTC timestamp>.json`, refreshes `runs/latest.json`
and rewrites [`STATUS.md`](./STATUS.md) (latest table + appended history
row). Uploaded harness files are deleted from the workspace at the end unless
`--keep-files` is given.

---

## 3. The use cases

Every case names its journey (what the person does), the exact client path
it exercises, the checks, and the budget. IDs are stable; the runner reports
by ID.

| ID | Journey | Client path | Gate |
| -- | ------- | ----------- | ---- |
| UC-00 | Open the app: models and workspace are reachable | `catalog::fetch_catalog` | every run |
| UC-01 | Pick a chat model, say hello | `messages::stream_chat` (plain chat body, `max_tokens 1024`) | quick: 6 models · full: every available CHAT model |
| UC-02 | Ask the assistant to write a file | `agent::run_agent_turn` + `agent_tools` (list/read/write) | quick set |
| UC-03 | Turn **Web** on, ask a current-events question in plain chat | `stream_chat` with `web_search` server tool + raw SSE capture | quick set |
| UC-04 | Online research into a file | `run_agent_turn` with web + write_file, `WEB_SEARCH_PROMPT` | 3 models |
| UC-05 | Drop one file, ask about it | `files::upload_project_file` → `list_project_files` poll → `stream_chat` with `rag_group_key` | 3 models + timing traps |
| UC-06 | Index a whole folder (megabytes) and ask | upload en bloc → poll all → 3 fact questions | every run |
| UC-07 | Dictate a note | TTS fixture → `dictation::open_session/append_chunk/commit_session` (client protocol) + `transcribe` | every run |
| UC-08 | Write, find, rename and delete notes | `ProjectStore` notes API (local) | every run |
| UC-09 | Ask for a long answer | `stream_chat` + raw `stop_reason` | every run |
| UC-10 | Something goes wrong: what does the person read? | `stream_chat` error text for a failing model and an unknown model | every run |

### UC-00 Preflight

Fetch `/v1/models/catalog`. Checks: 200; ≥1 available CHAT model; the
`VECTORIZE` default is **available** (an unavailable index model means every
knowledge-folder upload fails silently later); SOUND2TEXT default available.

### UC-01 Model matrix — plain chat

For each model: `stream_chat` with `[{"role":"user","content":"Reply with the single word PONG."}]`,
`max_tokens 1024`, `stream true` — the exact `send_chat` body. Checks: no
error; non-empty text; text contains `PONG`; time to first token ≤ 15 s,
total ≤ 60 s. **Why:** a model that the picker offers and that cannot answer
"hello" is the first thing a new user hits. Astra's `max_tokens` error lives
here.

### UC-02 Agent turn — write a file

System prompt from `build_system_prompt` (no skills, exec off), tools
`list_files` / `read_file` / `write_file`, out-box in a temp folder. Prompt:
"Create a file named `hello.md` in the out-box that contains exactly the line
`HELLO FROM SYNAPLAN`." Checks: turn ends with `Done`; the file exists;
content contains the line; ≤ 6 tool rounds. **Why:** every "write a Word /
Excel" tile starts with this loop; the `max_tokens 8192` agent body is
exercised per model.

### UC-03 Web search in plain chat

Body = `send_chat` with `tools:[{"type":"web_search_20250305","name":"web_search"}]`.
Question: "Use web search: what is the current LTS version of Node.js? Answer
in two sentences and cite the URL you used." Checks (client view): text
non-empty, contains `http`. Checks (contract view, raw SSE): no `error`
event; final `stop_reason` is `end_turn` (never `tool_use` — the client
cannot serve a server tool); `ping` count (= gateway tool rounds) ≤ 4;
total ≤ 60 s. **Why:** the "I am looking online for an answer" bug: the
gateway loop spent its 8 rounds and handed an unserved `web_search` back.

### UC-04 Online research into a file

`run_agent_turn` with `web_search_tool()` + write tools, `WEB_SEARCH_PROMPT`
appended. Prompt: "Search the web for the current LTS version of Node.js and
write `sources.md` into the out-box with the version and at least three
source URLs." Checks: `sources.md` exists, ≥ 3 `http` lines; turn ends with
text; no `server_tool_result` retry loop longer than 2.

### UC-05 One file, one question

Upload `fact-<run>.md` (three unique facts) with the `send_chat` project
fields, poll `list_project_files` until `Ready` (≤ 120 s), then ask the fact
question in a fresh thread with `x-synaplan-rag-group-key`. Normalise the
answer (drop non-alphanumerics) before matching. Three models.

Timing traps (documented, currently expected to fail — see CHANGES):

- **UC-05b early ask.** Upload a PDF (slow extraction), ask *immediately*,
  then wait for `Ready` and ask again **in the same thread** (same first
  message). The second answer must contain the fact.
- **UC-05c second-turn question.** With ≥ 8 other files in the folder, start
  a thread with "Hello, I added some files." and ask the fact in turn two.
  The answer must contain the fact.

### UC-06 Vectorization en bloc

Generated corpus per run: one 3 MB Markdown (structured sections, 30 planted
facts), four 500 KB Markdown, one `.docx` (built in-process), one `.pdf`
(built in-process). Upload sequentially like the client; poll until all
`Ready`. Then three fact questions (big file, docx, pdf). Budgets: every
file `Ready` ≤ 10 min; indexed ≥ 1 MB/min; no file `Failed`; ≥ 2 of 3 facts
answered. Reports per-file upload time and time-to-ready.

### UC-07 Dictation

Fixture: a 100-word English paragraph rendered with the workspace's SPEAK
default via `/v1/audio/speech`, decoded with `ffmpeg` to 16 kHz mono
`s16le`. Then the exact client protocol from `useDictation.ts`: session with
`commit_after_bytes` 40 s, PCM posted as ≥ 5 s phrases with `commit=true`,
poll every 1.8 s, `commit_session` on stop, then the **correction pass**
(new session, 35 s windows, commit each, final commit), and separately the
one-shot `transcribe`. Measures: time to first interim text; stop → final
text; correction pass alone; one-shot alone; similarity (word-level) of the
final text vs the fixture. Budgets: stop → final **≤ 5 s target, 10 s
ceiling**; similarity ≥ 0.75. **Why:** "dictation is VERY slow" needs a
number and a phase.

### UC-08 Notes (local)

Temp `ProjectStore`; `ensure_personal`; create project; `create_note`;
`write_note` Markdown; `read_note` round-trip; `list_notes`; `search_notes`;
append a dictated sentence; `delete_note`; invalid names refused. Runs
without network; a failure here is a data-loss bug.

### UC-09 Long answer

Prompt for a 1 500-word essay with the desktop's `max_tokens 1024`. Raw
`stop_reason` is recorded. Check: **if** the raw stream ends with
`max_tokens`, the client must receive a truncation signal (today it receives
`Done`). Currently expected to fail — see CHANGES.

### UC-10 Error copy

Two turns that fail on purpose: an unknown model (expects the friendly
"Choose a model…" text) and a model whose provider rejects the body (expects
plain language — no parameter names, no `Unsupported parameter`). The check
is on the string the UI would show.

---

## 4. Budgets (single table)

| Metric | Target | Ceiling | Case |
| ------ | ------ | ------- | ---- |
| Time to first token, plain chat | 3 s | 15 s | UC-01 |
| Plain chat total | 10 s | 60 s | UC-01 |
| Web-search turn total | 20 s | 60 s | UC-03 |
| Gateway tool rounds per web turn | 1–2 | 4 | UC-03 |
| Small file `Ready` | 5 s | 120 s | UC-05 |
| Corpus (≈ 5 MB, 7 files) all `Ready` | 3 min | 10 min | UC-06 |
| Indexing throughput | 3 MB/min | 1 MB/min | UC-06 |
| Dictation stop → final | 5 s | 10 s | UC-07 |
| Dictation similarity | 0.9 | 0.75 | UC-07 |

---

## 5. Status ledger policy

- `STATUS.md` is rewritten by the runner: one row per case, `PASS` / `FAIL`
  / `SKIP`, duration, one-line detail. Below the table, **Findings** lists
  every failing check with its evidence line. **History** appends one row
  per run (date, git SHA of the desktop repo, target host, pass/fail
  counts, runner flags).
- A case that is *expected* to fail until a listed change lands is still
  reported as `FAIL`; the expectation is recorded in `CHANGES.md`, not in
  the runner. When the change lands, the row turns green without editing
  the plan.
- `runs/*.json` are committed for release runs (tagged in the History
  column), pruned for scratch runs.

---

## 6. Exit bullets (UX contract §6)

1. A new user picks any model the picker offers and gets an answer — no raw
   provider text. (UC-01, UC-10)
2. A dropped file is answered from within ten seconds of showing `Ready`,
   in the thread the person is already in. (UC-05, UC-06)
3. Web on → the answer carries sources; the turn never ends in silence.
   (UC-03, UC-04)
4. A dictated take of 40 s is text within 5 s of pressing stop. (UC-07)
5. Every failure the person can see names what happened and what to do.
   (UC-09, UC-10)
