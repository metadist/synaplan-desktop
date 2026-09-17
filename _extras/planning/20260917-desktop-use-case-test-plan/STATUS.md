# Desktop use-case test status

Written by `cargo run -p synaplan-core --example usecases`. Do not edit the table by hand — see [README.md](./README.md) §5 for the ledger policy and [CHANGES.md](./CHANGES.md) for the work each failing row demands.

**Last run:** 2026-09-17T12:13:14Z · target `http://localhost:8000` · desktop `0a76d66` · flags `--quick --verbose --only UC-01,UC-02,UC-03,UC-04,UC-05,UC-06,UC-09,UC-10` · **3 passed, 5 failed, 0 skipped**

| Case | Journey | Result | Time | Detail |
| ---- | ------- | ------ | ---- | ------ |
| UC-01 | Pick a chat model, say hello | ❌ FAIL | 7.4 s | 5/6 models answered PONG |
| UC-02 | Ask the assistant to write a file | ❌ FAIL | 27.6 s | 5/6 models wrote the file |
| UC-03 | Turn Web on, ask a current-events question | ❌ FAIL | 92.8 s | 4/6 models answered with a source |
| UC-04 | Online research into a file | ❌ FAIL | 44.2 s | 1/3 models produced sources.md |
| UC-05 | Drop one file, ask about it | ✅ PASS | 52.3 s | 3/3 models answered; ready in 61 ms |
| UC-06 | Index a whole folder (megabytes) and ask | ❌ FAIL | 5.3 min | 6 files (4.96 MB) ready in 305.1 s, 2/2 facts answered |
| UC-09 | Ask for a long answer | ✅ PASS | 4.7 s | 693 words, stop_reason Some("max_tokens") |
| UC-10 | Something goes wrong: what does the person read? | ✅ PASS | 8.0 s | 1 model(s) failed — copy checked |

## Metrics (latest run)

| Case | Metric | Value |
| ---- | ------ | ----- |
| UC-01 | total_ms anthropic:claude-fable-5-1:chat | 4411 |
| UC-01 | total_ms google:gemini-3.5-flash:chat | 1144 |
| UC-01 | total_ms groq:openai/gpt-oss-120b:chat | 441 |
| UC-01 | total_ms mistral:mistral-large-latest:chat | 240 |
| UC-01 | total_ms openai:gpt-6-astra:chat | 5605 |
| UC-01 | total_ms xai:grok-4.6:chat | 1812 |
| UC-01 | ttft_ms anthropic:claude-fable-5-1:chat | 3525 |
| UC-01 | ttft_ms google:gemini-3.5-flash:chat | 1140 |
| UC-01 | ttft_ms groq:openai/gpt-oss-120b:chat | 437 |
| UC-01 | ttft_ms mistral:mistral-large-latest:chat | null |
| UC-01 | ttft_ms openai:gpt-6-astra:chat | 5467 |
| UC-01 | ttft_ms xai:grok-4.6:chat | 1810 |
| UC-02 | steps anthropic:claude-fable-5-1:chat | 1 |
| UC-02 | steps google:gemini-3.5-flash:chat | 1 |
| UC-02 | steps groq:openai/gpt-oss-120b:chat | 1 |
| UC-02 | steps mistral:mistral-large-latest:chat | 0 |
| UC-02 | steps openai:gpt-6-astra:chat | 2 |
| UC-02 | steps xai:grok-4.6:chat | 1 |
| UC-02 | total_ms anthropic:claude-fable-5-1:chat | 9770 |
| UC-02 | total_ms google:gemini-3.5-flash:chat | 2942 |
| UC-02 | total_ms groq:openai/gpt-oss-120b:chat | 1274 |
| UC-02 | total_ms mistral:mistral-large-latest:chat | 248 |
| UC-02 | total_ms openai:gpt-6-astra:chat | 8843 |
| UC-02 | total_ms xai:grok-4.6:chat | 4549 |
| UC-03 | rounds anthropic:claude-fable-5-1:chat | 3 |
| UC-03 | rounds google:gemini-3.5-flash:chat | 2 |
| UC-03 | rounds groq:openai/gpt-oss-120b:chat | 2 |
| UC-03 | rounds mistral:mistral-large-latest:chat | 0 |
| UC-03 | rounds openai:gpt-6-astra:chat | 2 |
| UC-03 | rounds xai:grok-4.6:chat | 1 |
| UC-03 | total_ms anthropic:claude-fable-5-1:chat | 10922 |
| UC-03 | total_ms google:gemini-3.5-flash:chat | 8968 |
| UC-03 | total_ms groq:openai/gpt-oss-120b:chat | 2876 |
| UC-03 | total_ms mistral:mistral-large-latest:chat | 287 |
| UC-03 | total_ms openai:gpt-6-astra:chat | 7931 |
| UC-03 | total_ms xai:grok-4.6:chat | 14200 |
| UC-04 | steps anthropic:claude-fable-5-1:chat | 1 |
| UC-04 | steps groq:openai/gpt-oss-120b:chat | 0 |
| UC-04 | steps openai:gpt-6-astra:chat | 1 |
| UC-04 | total_ms anthropic:claude-fable-5-1:chat | 26430 |
| UC-04 | total_ms groq:openai/gpt-oss-120b:chat | 2973 |
| UC-04 | total_ms openai:gpt-6-astra:chat | 14781 |
| UC-04 | urls anthropic:claude-fable-5-1:chat | 5 |
| UC-04 | urls groq:openai/gpt-oss-120b:chat | 0 |
| UC-04 | urls openai:gpt-6-astra:chat | 0 |
| UC-05 | ready_ms | 61 |
| UC-05 | uc05b_pdf_ready | 1 |
| UC-05 | upload_ms | 2312 |
| UC-06 | all_ready_ms | 305103 |
| UC-06 | corpus_files | 7 |
| UC-06 | corpus_mb | 4.96 |
| UC-06 | facts_hit | 2/2 |
| UC-06 | mb_per_min | 0.98 |
| UC-06 | ready_ms finance-3RXVB.docx | 369 |
| UC-06 | ready_ms survey-3RXVB.pdf | 369 |
| UC-06 | ready_ms weekly-0-3RXVB.md | 369 |
| UC-06 | ready_ms weekly-1-3RXVB.md | 369 |
| UC-06 | ready_ms weekly-2-3RXVB.md | 369 |
| UC-06 | ready_ms weekly-3-3RXVB.md | 369 |
| UC-06 | upload_ms_total | 3961 |
| UC-09 | client_words | 693 |
| UC-09 | model | groq:openai/gpt-oss-120b:chat |
| UC-09 | raw_stop_reason | max_tokens |
| UC-10 | failing_models | 1 |

## Findings (failing checks, latest run)

- **UC-01** mistral:mistral-large-latest:chat answers — error shown to the user: "This model is not available in your subscription tier" (server)
- **UC-02** mistral:mistral-large-latest:chat: hello.md written and turn ends in text — error shown: "This model is not available in your subscription tier" (server) after 0 steps
- **UC-03** groq:openai/gpt-oss-120b:chat: answer with a source — stop_reason=Some("end_turn") rounds≈2 error=None client_text="The current Long‑Term Support (LTS) version of Node.js is **Node 24** (code‑named “Krypton…" total 2876 ms
- **UC-03** mistral:mistral-large-latest:chat: answer with a source — stop_reason=None rounds≈0 error=Some("This model is not available in your subscription tier") client_text="" total 287 ms
- **UC-04** groq:openai/gpt-oss-120b:chat: sources.md with ≥3 URLs — 0 steps (0 web_search retry hints), 0 URLs in sources.md, 2973 ms, reply "The current Active LTS version of Node.js is 24 (e.g., 24.21…"
- **UC-04** openai:gpt-6-astra:chat: sources.md with ≥3 URLs — 1 steps (0 web_search retry hints), 0 URLs in sources.md, 14781 ms, reply "I’ll record Node.js v24.13.0 (LTS), the newest LTS version s…"
- **UC-06** handbook-3RXVB.md upload accepted — upload HTTP 500 Internal Server Error (server)
- **UC-06** every file accepted — 6/7 uploaded in 3961 ms
- **UC-06** indexing throughput ≥ 1 MB/min — 0.98 MB/min for 4.96 MB

## History

<!-- history: one row per run, appended by the runner -->
| Run (UTC) | Desktop | Target | Pass | Fail | Skip | Flags |
| --- | --- | --- | --- | --- | --- | --- |
| 2026-09-17T09:17:01Z | 21efbf0 | http://localhost:8000 | 2 | 9 | 0 | B6GTY --quick (VECTORIZE not_pulled) |
| 2026-09-17T09:31Z | 21efbf0 | http://localhost:8000 | — | — | — | TMR43 aborted at UC-06 |
| 2026-09-17T09:36Z | 21efbf0 | http://localhost:8000 | 3 | 8 | 0 | composite B6GTY+6GW9Q (this file) |
| 2026-09-17T10:15:44Z | 2379231 | http://localhost:8000 | 4 | 7 | 0 | --quick --verbose |
| 2026-09-17T10:53:45Z | c893147 | http://localhost:8000 | 3 | 1 | 0 | --quick --verbose --only UC-00,UC-01,UC-09,UC-10 |
| 2026-09-17T11:15:07Z | 176a58d | http://localhost:8000 | 2 | 5 | 0 | --quick --verbose --only UC-02,UC-03,UC-04,UC-05,UC-06,UC-07,UC-08 |
| 2026-09-17T12:13:14Z | 0a76d66 | http://localhost:8000 | 3 | 5 | 0 | --quick --verbose --only UC-01,UC-02,UC-03,UC-04,UC-05,UC-06,UC-09,UC-10 |
