# Desktop use-case test status

Written by hand from the 2026-09-17 live runs after the runner process
was killed mid-UC-06 (run 2) and again while run 3 was still in UC-05.
Do not treat this as a substitute for a later `usecases` rewrite — see
[README.md](./README.md) §5 and [CHANGES.md](./CHANGES.md).

**Last composite:** 2026-09-17T09:36Z · target `http://localhost:8000` · desktop `21efbf0` · flags `--quick --verbose` · **3 passed, 8 failed, 0 skipped**

Sources: run `B6GTY` (full, 250 s) for UC-06–10; run `6GW9Q` (in progress at UC-05) for UC-00–04.

| Case | Journey | Result | Time | Detail |
| ---- | ------- | ------ | ---- | ------ |
| UC-00 | Open the app: models and workspace reachable | ✅ PASS | ~2 s | 44/54 CHAT available; VECTORIZE `ollama:bge-m3:vectorize`; SOUND2TEXT + TEXT2SOUND ok (`6GW9Q`) |
| UC-01 | Pick a chat model, say hello | ❌ FAIL | 5.8 s | 4/6 PONG. Astra: `max_tokens` / `max_completion_tokens`. Mistral: subscription tier (`6GW9Q`) |
| UC-02 | Ask the assistant to write a file | ❌ FAIL | 17.7 s | 3/6 wrote hello.md (Claude, Groq, Grok). Astra max_tokens; Gemini thought_signature; Mistral tier (`6GW9Q`) |
| UC-03 | Turn Web on, ask a current-events question | ❌ FAIL | 69.1 s | 1/6 with a source (Grok). Claude + Groq: `end_turn` and **empty client text**. Astra / Gemini / Mistral as above (`6GW9Q`) |
| UC-04 | Online research into a file | ❌ FAIL | 30.5 s | 1/3 wrote sources.md (Claude). Groq: Harmony `Tools should have a name!`. Astra max_tokens (`6GW9Q`) |
| UC-05 | Drop one file, ask about it | ❌ FAIL | 21–42 s | Fresh-thread Markdown works on Claude + Groq. Astra dies. UC-05d same-thread after Ready misses (`TMR43` / `B6GTY`) |
| UC-06 | Index a whole folder (megabytes) and ask | ❌ FAIL | 34.1 s | 7/7 Ready in 7.0 s, 42 MB/min, **1/3 facts** (handbook hit; docx + pdf miss) (`B6GTY`) |
| UC-07 | Dictate a note | ✅ PASS | 32.4 s | 31 s take → text in 2.9 s after stop, similarity 0.99 (`B6GTY`) |
| UC-08 | Write, find, rename and delete notes | ✅ PASS | 0.0 s | create → write → read → search → append → delete (`B6GTY`) |
| UC-09 | Ask for a long answer | ❌ FAIL | 4.7 s | raw `stop_reason=max_tokens` after 643 words; client got `Done` (`B6GTY`) |
| UC-10 | Something goes wrong: what does the person read? | ❌ FAIL | 7.4 s | unknown model is friendly; Astra shows the raw `max_tokens` sentence (`B6GTY`) |

## Metrics (latest complete numbers)

| Case | Metric | Value |
| ---- | ------ | ----- |
| UC-01 | Astra | provider 400 `max_tokens` |
| UC-03 | Grok | 19.9 s, 2 rounds, URL cited |
| UC-03 | Groq | 6.3 s, 6 rounds, empty text |
| UC-03 | Claude | 8.8 s, 3 rounds, empty text |
| UC-05 | small file Ready | 36–55 ms |
| UC-06 | corpus | 4.96 MB / 7 files / 7.0 s Ready |
| UC-06 | facts | 1/3 |
| UC-07 | stop → final | 2881 ms (correction 2837 ms) |
| UC-07 | similarity | 0.99 |
| UC-09 | words / stop | 643 / max_tokens |

## Findings (failing checks)

- **UC-01** openai:gpt-6-astra:chat answers — `Unsupported parameter: 'max_tokens'… Use 'max_completion_tokens' instead.` → **C1**
- **UC-01** mistral:mistral-large-latest:chat answers — subscription tier → **C7**
- **UC-02** Gemini thought_signature after first `write_file` → **C4**
- **UC-03** Claude and Groq finish `end_turn` with empty client text; Groq 6 rounds > ceiling 4 → **C2**
- **UC-04** Groq HarmonyError `Tools should have a name!` → **C2 / C10**
- **UC-05d** same thread after Ready still misses the fact (RAG cache on first user turn, 2 h) → **C3a**
- **UC-06** docx and pdf `Ready` but facts not answered → **C3b**
- **UC-09** client not told the answer was cut off → **C5**
- **UC-10** Astra failure is raw provider text → **C6**

## History

<!-- history: one row per run, appended by the runner -->
| Run (UTC) | Desktop | Target | Pass | Fail | Skip | Flags |
| --- | --- | --- | --- | --- | --- | --- |
| 2026-09-17T09:17:01Z | 21efbf0 | http://localhost:8000 | 2 | 9 | 0 | B6GTY --quick (VECTORIZE not_pulled) |
| 2026-09-17T09:31Z | 21efbf0 | http://localhost:8000 | — | — | — | TMR43 aborted at UC-06 |
| 2026-09-17T09:36Z | 21efbf0 | http://localhost:8000 | 3 | 8 | 0 | composite B6GTY+6GW9Q (this file) |
