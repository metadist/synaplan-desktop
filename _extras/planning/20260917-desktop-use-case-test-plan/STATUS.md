# Desktop use-case test status

Written by `cargo run -p synaplan-core --example usecases`. Do not edit the table by hand — see [README.md](./README.md) §5 for the ledger policy and [CHANGES.md](./CHANGES.md) for the work each failing row demands.

**Last run:** 2026-10-23T09:40:11Z · target `http://localhost:8000` · desktop `12e2510` · flags `--quick` · **9 passed, 4 failed, 0 skipped**

| Case | Journey | Result | Time | Detail |
| ---- | ------- | ------ | ---- | ------ |
| UC-00 | Open the app: models and workspace reachable | ❌ FAIL | 381 ms | 51 chat models available |
| UC-01 | Pick a chat model, say hello | ✅ PASS | 4.9 s | 7/7 models answered PONG |
| UC-02 | Ask the assistant to write a file | ✅ PASS | 35.7 s | 7/7 models wrote the file |
| UC-03 | Turn Web on, ask a current-events question | ✅ PASS | 85.4 s | 7/7 models answered with a source |
| UC-03-de | Turn Web on, ask the same question in German | ❌ FAIL | 114.0 s | 6/7 models answered with a source |
| UC-04 | Online research into a file | ✅ PASS | 50.2 s | 3/3 models produced sources.md |
| UC-04-de | Online research into a file, asked in German | ✅ PASS | 93.8 s | 3/3 models produced sources.md |
| UC-05 | Drop one file, ask about it | ✅ PASS | 21.4 s | 3/3 models answered; ready in 145 ms |
| UC-06 | Index a whole folder (megabytes) and ask | ❌ FAIL | 23.8 s | 7 files (4.96 MB) ready in 6.4 s, 0/3 facts answered |
| UC-07 | Dictate a note | ❌ FAIL | 0 ms | no dictation model |
| UC-08 | Write, find and delete notes (local) | ✅ PASS | 14 ms | create → write → read → search → append → delete |
| UC-09 | Ask for a long answer | ✅ PASS | 16.9 s | 2795 words, stop_reason Some("end_turn") |
| UC-10 | Something goes wrong: what does the person read? | ✅ PASS | 9.4 s | no model failed; unknown-model copy checked |

## Metrics (latest run)

| Case | Metric | Value |
| ---- | ------ | ----- |
| UC-00 | chat_models_available | 51 |
| UC-00 | chat_models_total | 62 |
| UC-00 | default_sound2text |  |
| UC-00 | default_text2sound |  |
| UC-00 | default_vectorize | ollama:bge-m3:vectorize |
| UC-01 | total_ms anthropic:claude-fable-5-1:chat | 3515 |
| UC-01 | total_ms anthropic:claude-sonnet-5-5:chat | 1191 |
| UC-01 | total_ms google:gemini-3.5-flash:chat | 1336 |
| UC-01 | total_ms groq:openai/gpt-oss-120b:chat | 530 |
| UC-01 | total_ms mistral:mistral-large-latest:chat | 1233 |
| UC-01 | total_ms openai:gpt-6-astra:chat | 1673 |
| UC-01 | total_ms xai:grok-4.6:chat | 1308 |
| UC-01 | ttft_ms anthropic:claude-fable-5-1:chat | 2386 |
| UC-01 | ttft_ms anthropic:claude-sonnet-5-5:chat | 1179 |
| UC-01 | ttft_ms google:gemini-3.5-flash:chat | 1328 |
| UC-01 | ttft_ms groq:openai/gpt-oss-120b:chat | 530 |
| UC-01 | ttft_ms mistral:mistral-large-latest:chat | 1228 |
| UC-01 | ttft_ms openai:gpt-6-astra:chat | 1673 |
| UC-01 | ttft_ms xai:grok-4.6:chat | 1307 |
| UC-02 | steps anthropic:claude-fable-5-1:chat | 1 |
| UC-02 | steps anthropic:claude-sonnet-5-5:chat | 1 |
| UC-02 | steps google:gemini-3.5-flash:chat | 1 |
| UC-02 | steps groq:openai/gpt-oss-120b:chat | 1 |
| UC-02 | steps mistral:mistral-large-latest:chat | 1 |
| UC-02 | steps openai:gpt-6-astra:chat | 2 |
| UC-02 | steps xai:grok-4.6:chat | 1 |
| UC-02 | total_ms anthropic:claude-fable-5-1:chat | 4704 |
| UC-02 | total_ms anthropic:claude-sonnet-5-5:chat | 4728 |
| UC-02 | total_ms google:gemini-3.5-flash:chat | 4196 |
| UC-02 | total_ms groq:openai/gpt-oss-120b:chat | 1200 |
| UC-02 | total_ms mistral:mistral-large-latest:chat | 8689 |
| UC-02 | total_ms openai:gpt-6-astra:chat | 7709 |
| UC-02 | total_ms xai:grok-4.6:chat | 4484 |
| UC-03 | rounds anthropic:claude-fable-5-1:chat | 3 |
| UC-03 | rounds anthropic:claude-sonnet-5-5:chat | 3 |
| UC-03 | rounds google:gemini-3.5-flash:chat | 2 |
| UC-03 | rounds groq:openai/gpt-oss-120b:chat | 2 |
| UC-03 | rounds mistral:mistral-large-latest:chat | 1 |
| UC-03 | rounds openai:gpt-6-astra:chat | 1 |
| UC-03 | rounds xai:grok-4.6:chat | 1 |
| UC-03 | total_ms anthropic:claude-fable-5-1:chat | 5729 |
| UC-03 | total_ms anthropic:claude-sonnet-5-5:chat | 7125 |
| UC-03 | total_ms google:gemini-3.5-flash:chat | 7206 |
| UC-03 | total_ms groq:openai/gpt-oss-120b:chat | 2583 |
| UC-03 | total_ms mistral:mistral-large-latest:chat | 3346 |
| UC-03 | total_ms openai:gpt-6-astra:chat | 7161 |
| UC-03 | total_ms xai:grok-4.6:chat | 8048 |
| UC-03-de | rounds anthropic:claude-fable-5-1:chat | 3 |
| UC-03-de | rounds anthropic:claude-sonnet-5-5:chat | 3 |
| UC-03-de | rounds google:gemini-3.5-flash:chat | 2 |
| UC-03-de | rounds groq:openai/gpt-oss-120b:chat | 2 |
| UC-03-de | rounds mistral:mistral-large-latest:chat | 1 |
| UC-03-de | rounds openai:gpt-6-astra:chat | 2 |
| UC-03-de | rounds xai:grok-4.6:chat | 4 |
| UC-03-de | total_ms anthropic:claude-fable-5-1:chat | 6901 |
| UC-03-de | total_ms anthropic:claude-sonnet-5-5:chat | 11587 |
| UC-03-de | total_ms google:gemini-3.5-flash:chat | 7349 |
| UC-03-de | total_ms groq:openai/gpt-oss-120b:chat | 2727 |
| UC-03-de | total_ms mistral:mistral-large-latest:chat | 5986 |
| UC-03-de | total_ms openai:gpt-6-astra:chat | 7757 |
| UC-03-de | total_ms xai:grok-4.6:chat | 19400 |
| UC-04 | steps anthropic:claude-fable-5-1:chat | 2 |
| UC-04 | steps anthropic:claude-sonnet-5-5:chat | 5 |
| UC-04 | steps groq:openai/gpt-oss-120b:chat | 3 |
| UC-04 | total_ms anthropic:claude-fable-5-1:chat | 14558 |
| UC-04 | total_ms anthropic:claude-sonnet-5-5:chat | 17068 |
| UC-04 | total_ms groq:openai/gpt-oss-120b:chat | 18555 |
| UC-04 | urls anthropic:claude-fable-5-1:chat | 5 |
| UC-04 | urls anthropic:claude-sonnet-5-5:chat | 4 |
| UC-04 | urls groq:openai/gpt-oss-120b:chat | 3 |
| UC-04-de | steps anthropic:claude-fable-5-1:chat | 3 |
| UC-04-de | steps anthropic:claude-sonnet-5-5:chat | 4 |
| UC-04-de | steps groq:openai/gpt-oss-120b:chat | 10 |
| UC-04-de | total_ms anthropic:claude-fable-5-1:chat | 21340 |
| UC-04-de | total_ms anthropic:claude-sonnet-5-5:chat | 17187 |
| UC-04-de | total_ms groq:openai/gpt-oss-120b:chat | 55290 |
| UC-04-de | urls anthropic:claude-fable-5-1:chat | 3 |
| UC-04-de | urls anthropic:claude-sonnet-5-5:chat | 3 |
| UC-04-de | urls groq:openai/gpt-oss-120b:chat | 3 |
| UC-05 | ready_ms | 145 |
| UC-05 | uc05b_pdf_ready | 1 |
| UC-05 | upload_ms | 301 |
| UC-06 | all_ready_ms | 6389 |
| UC-06 | corpus_files | 7 |
| UC-06 | corpus_mb | 4.96 |
| UC-06 | facts_hit | 0/3 |
| UC-06 | mb_per_min | 46.57 |
| UC-06 | ready_ms finance-9RDYA.docx | 529 |
| UC-06 | ready_ms handbook-9RDYA.md | 529 |
| UC-06 | ready_ms survey-9RDYA.pdf | 529 |
| UC-06 | ready_ms weekly-0-9RDYA.md | 529 |
| UC-06 | ready_ms weekly-1-9RDYA.md | 529 |
| UC-06 | ready_ms weekly-2-9RDYA.md | 529 |
| UC-06 | ready_ms weekly-3-9RDYA.md | 529 |
| UC-06 | upload_ms_total | 5851 |
| UC-09 | client_words | 2795 |
| UC-09 | model | groq:openai/gpt-oss-120b:chat |
| UC-09 | raw_stop_reason | end_turn |
| UC-10 | failing_models | 0 |

## Findings (failing checks, latest run)

- **UC-00** VECTORIZE default is available — ollama:bge-m3:vectorize is listed as unavailable (provider_unavailable) — every knowledge-folder upload indexes with it
- **UC-00** SOUND2TEXT default is available — no default advertised
- **UC-00** TEXT2SOUND default is available — no default advertised
- **UC-03-de** groq:openai/gpt-oss-120b:chat: answer with a source — stop_reason=Some("end_turn") rounds≈2 error=None client_text="I looked this up but could not turn the results into an answer. Please try again, or ask i…" total 2727 ms
- **UC-06** fact from handbook-9RDYA.md answered — anthropic:claude-sonnet-5-5:chat: "I couldn't find a Nordlicht warehouse access code for the Hamburg site in your project files. I searched twice, once for…"
- **UC-06** fact from finance-9RDYA.docx answered — anthropic:claude-sonnet-5-5:chat: "I couldn't find the cost centre in your project files. I searched twice and only got generic filler text. It has words l…"
- **UC-06** fact from survey-9RDYA.pdf answered — anthropic:claude-sonnet-5-5:chat: "I couldn't find a survey reference for an August 2026 customer satisfaction survey in your project files. I searched twi…"
- **UC-07** dictation model advertised — catalog has no SOUND2TEXT default

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
| 2026-09-17T12:15:02Z | 1746c70 | http://localhost:8000 | 1 | 1 | 0 | --verbose --only UC-03,UC-10 --models groq:openai/gpt-oss-120b:chat,mistral:mistral-large-latest:chat |
| 2026-09-17T12:15:10Z | 1746c70 | http://localhost:8000 | 1 | 0 | 0 | --verbose --only UC-10 --models mistral:mistral-large-latest:chat |
| 2026-09-17T12:23:33Z | 2e98094 | http://localhost:8000 | 0 | 1 | 0 | --quick --verbose --only UC-04 |
| 2026-09-17T12:26:26Z | 2e98094 | http://localhost:8000 | 1 | 0 | 0 | --quick --verbose --only UC-04 |
| 2026-09-18T07:08:42Z | e79b589 | http://localhost:8000 | 1 | 3 | 0 | --quick --verbose --only UC-00,UC-02,UC-03,UC-04 |
| 2026-10-23T08:19:38Z | 12e2510 | http://localhost:8000 | 5 | 6 | 0 | --full |
| 2026-10-23T09:40:11Z | 12e2510 | http://localhost:8000 | 9 | 4 | 0 | --quick |
