# Desktop use-case test status

Written by `cargo run -p synaplan-core --example usecases`. Do not edit the table by hand — see [README.md](./README.md) §5 for the ledger policy and [CHANGES.md](./CHANGES.md) for the work each failing row demands.

**Last run:** 2026-09-17T11:15:07Z · target `http://localhost:8000` · desktop `176a58d` · flags `--quick --verbose --only UC-02,UC-03,UC-04,UC-05,UC-06,UC-07,UC-08` · **2 passed, 5 failed, 0 skipped**

| Case | Journey | Result | Time | Detail |
| ---- | ------- | ------ | ---- | ------ |
| UC-02 | Ask the assistant to write a file | ❌ FAIL | 16.9 s | 3/6 models wrote the file |
| UC-03 | Turn Web on, ask a current-events question | ❌ FAIL | 68.6 s | 2/6 models answered with a source |
| UC-04 | Online research into a file | ❌ FAIL | 27.7 s | 1/3 models produced sources.md |
| UC-05 | Drop one file, ask about it | ❌ FAIL | 41.6 s | 2/3 models answered; ready in 58 ms |
| UC-06 | Index a whole folder (megabytes) and ask | ❌ FAIL | 17.2 min | 6 files (4.96 MB) ready in 1021.1 s, 2/2 facts answered |
| UC-07 | Dictate a note | ✅ PASS | 25.2 s | 29 s take → text in 1.2 s after stop, similarity 0.99 |
| UC-08 | Write, find and delete notes (local) | ✅ PASS | 4 ms | create → write → read → search → append → delete |

## Metrics (latest run)

| Case | Metric | Value |
| ---- | ------ | ----- |
| UC-02 | steps anthropic:claude-fable-5-1:chat | 1 |
| UC-02 | steps google:gemini-3.5-flash:chat | 1 |
| UC-02 | steps groq:openai/gpt-oss-120b:chat | 1 |
| UC-02 | steps mistral:mistral-large-latest:chat | 0 |
| UC-02 | steps openai:gpt-6-astra:chat | 0 |
| UC-02 | steps xai:grok-4.6:chat | 1 |
| UC-02 | total_ms anthropic:claude-fable-5-1:chat | 8974 |
| UC-02 | total_ms google:gemini-3.5-flash:chat | 2200 |
| UC-02 | total_ms groq:openai/gpt-oss-120b:chat | 1060 |
| UC-02 | total_ms mistral:mistral-large-latest:chat | 235 |
| UC-02 | total_ms openai:gpt-6-astra:chat | 685 |
| UC-02 | total_ms xai:grok-4.6:chat | 3744 |
| UC-03 | rounds anthropic:claude-fable-5-1:chat | 3 |
| UC-03 | rounds google:gemini-3.5-flash:chat | 1 |
| UC-03 | rounds groq:openai/gpt-oss-120b:chat | 2 |
| UC-03 | rounds mistral:mistral-large-latest:chat | 0 |
| UC-03 | rounds openai:gpt-6-astra:chat | 0 |
| UC-03 | rounds xai:grok-4.6:chat | 1 |
| UC-03 | total_ms anthropic:claude-fable-5-1:chat | 12142 |
| UC-03 | total_ms google:gemini-3.5-flash:chat | 2370 |
| UC-03 | total_ms groq:openai/gpt-oss-120b:chat | 2688 |
| UC-03 | total_ms mistral:mistral-large-latest:chat | 330 |
| UC-03 | total_ms openai:gpt-6-astra:chat | 492 |
| UC-03 | total_ms xai:grok-4.6:chat | 10768 |
| UC-04 | steps anthropic:claude-fable-5-1:chat | 1 |
| UC-04 | steps groq:openai/gpt-oss-120b:chat | 1 |
| UC-04 | steps openai:gpt-6-astra:chat | 0 |
| UC-04 | total_ms anthropic:claude-fable-5-1:chat | 22448 |
| UC-04 | total_ms groq:openai/gpt-oss-120b:chat | 4498 |
| UC-04 | total_ms openai:gpt-6-astra:chat | 720 |
| UC-04 | urls anthropic:claude-fable-5-1:chat | 5 |
| UC-04 | urls groq:openai/gpt-oss-120b:chat | 0 |
| UC-04 | urls openai:gpt-6-astra:chat | 0 |
| UC-05 | ready_ms | 58 |
| UC-05 | uc05b_pdf_ready | 1 |
| UC-05 | upload_ms | 242 |
| UC-06 | all_ready_ms | 1021142 |
| UC-06 | corpus_files | 7 |
| UC-06 | corpus_mb | 4.96 |
| UC-06 | facts_hit | 2/2 |
| UC-06 | mb_per_min | 0.29 |
| UC-06 | ready_ms finance-123H2.docx | 372 |
| UC-06 | ready_ms survey-123H2.pdf | 372 |
| UC-06 | ready_ms weekly-0-123H2.md | 372 |
| UC-06 | ready_ms weekly-1-123H2.md | 372 |
| UC-06 | ready_ms weekly-2-123H2.md | 372 |
| UC-06 | ready_ms weekly-3-123H2.md | 372 |
| UC-06 | upload_ms_total | 3807 |
| UC-07 | correction_pass_ms | 1108 |
| UC-07 | correction_windows | 1 |
| UC-07 | final_similarity | 0.99 |
| UC-07 | first_interim_ms | 1094 |
| UC-07 | live_commit_ms | 43 |
| UC-07 | live_phrase_commit_ms | [1041] |
| UC-07 | live_phrases | 1 |
| UC-07 | live_similarity | 0.99 |
| UC-07 | long_correction_pass_ms | 4634 |
| UC-07 | long_correction_windows | 4 |
| UC-07 | long_one_shot_ms | 2530 |
| UC-07 | long_take_seconds | 115 |
| UC-07 | one_shot_ms | 1027 |
| UC-07 | one_shot_similarity | 0.99 |
| UC-07 | session_open_ms | 30 |
| UC-07 | stop_to_final_ms | 1152 |
| UC-07 | take_seconds | 28.7 |
| UC-07 | tts_model | google:gemini-3.1-flash-tts-preview:text2sound |
| UC-07 | tts_ms | 14663 |
| UC-07 | voice_model | groq:whisper-large-v3:sound2text |

## Findings (failing checks, latest run)

- **UC-02** openai:gpt-6-astra:chat: hello.md written and turn ends in text — error shown: "Function tools with reasoning_effort are not supported for gpt-6-astra in /v1/chat/completions. To use function tools, use /v1/responses or set reasoning_effort to 'none'." (server) after 0 steps
- **UC-02** google:gemini-3.5-flash:chat: hello.md written and turn ends in text — error shown: "This model rejected the request. Try another model." (model_rejected) after 1 steps
- **UC-02** mistral:mistral-large-latest:chat: hello.md written and turn ends in text — error shown: "This model is not available in your subscription tier" (server) after 0 steps
- **UC-03** groq:openai/gpt-oss-120b:chat: answer with a source — stop_reason=Some("end_turn") rounds≈2 error=None client_text="The current LTS (Long‑Term Support) version of Node.js is **Node 24** (code‑named “Krypton…" total 2688 ms
- **UC-03** openai:gpt-6-astra:chat: answer with a source — stop_reason=None rounds≈0 error=Some("Function tools with reasoning_effort are not supported for gpt-6-astra in /v1/chat/completions. To u…") client_text="" total 492 ms
- **UC-03** google:gemini-3.5-flash:chat: answer with a source — stop_reason=None rounds≈1 error=Some("Invalid JSON payload received. Unknown name \"thoughtSignature\" at 'contents[1].parts[0].function_cal…") client_text="" total 2370 ms
- **UC-03** mistral:mistral-large-latest:chat: answer with a source — stop_reason=None rounds≈0 error=Some("This model is not available in your subscription tier") client_text="" total 330 ms
- **UC-04** groq:openai/gpt-oss-120b:chat: sources.md with ≥3 URLs — error shown: "Tool choice is none, but model called a tool" (server) after 1 steps
- **UC-04** openai:gpt-6-astra:chat: sources.md with ≥3 URLs — error shown: "Unsupported value: 'reasoning_effort' does not support 'none' with this model. Supported values are: 'low', 'medium', 'high', and 'xhigh'." (server) after 0 steps
- **UC-05** openai:gpt-6-astra:chat: answer names the fact — error shown: "Unsupported value: 'reasoning_effort' does not support 'none' with this model. Supported values are: 'low', 'medium', 'high', and 'xhigh'."
- **UC-06** handbook-123H2.md upload accepted — upload HTTP 500 Internal Server Error (server)
- **UC-06** every file accepted — 6/7 uploaded in 3807 ms
- **UC-06** indexing throughput ≥ 1 MB/min — 0.29 MB/min for 4.96 MB

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
