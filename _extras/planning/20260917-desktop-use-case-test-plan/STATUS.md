# Desktop use-case test status

Written by `cargo run -p synaplan-core --example usecases`. Do not edit the table by hand — see [README.md](./README.md) §5 for the ledger policy and [CHANGES.md](./CHANGES.md) for the work each failing row demands.

**Last run:** 2026-09-17T10:53:45Z · target `http://localhost:8000` · desktop `c893147` · flags `--quick --verbose --only UC-00,UC-01,UC-09,UC-10` · **3 passed, 1 failed, 0 skipped**

| Case | Journey | Result | Time | Detail |
| ---- | ------- | ------ | ---- | ------ |
| UC-00 | Open the app: models and workspace reachable | ✅ PASS | 65 ms | 44 chat models available |
| UC-01 | Pick a chat model, say hello | ❌ FAIL | 5.7 s | 4/6 models answered PONG |
| UC-09 | Ask for a long answer | ✅ PASS | 4.6 s | 645 words, stop_reason Some("max_tokens") |
| UC-10 | Something goes wrong: what does the person read? | ✅ PASS | 9.3 s | 2 model(s) failed — copy checked |

## Metrics (latest run)

| Case | Metric | Value |
| ---- | ------ | ----- |
| UC-00 | chat_models_available | 44 |
| UC-00 | chat_models_total | 54 |
| UC-00 | default_sound2text | groq:whisper-large-v3:sound2text |
| UC-00 | default_text2sound | google:gemini-3.1-flash-tts-preview:text2sound |
| UC-00 | default_vectorize | ollama:bge-m3:vectorize |
| UC-01 | total_ms anthropic:claude-fable-5-1:chat | 3973 |
| UC-01 | total_ms google:gemini-3.5-flash:chat | 1172 |
| UC-01 | total_ms groq:openai/gpt-oss-120b:chat | 366 |
| UC-01 | total_ms mistral:mistral-large-latest:chat | 257 |
| UC-01 | total_ms openai:gpt-6-astra:chat | 1247 |
| UC-01 | total_ms xai:grok-4.6:chat | 1687 |
| UC-01 | ttft_ms anthropic:claude-fable-5-1:chat | 3015 |
| UC-01 | ttft_ms google:gemini-3.5-flash:chat | 1165 |
| UC-01 | ttft_ms groq:openai/gpt-oss-120b:chat | 362 |
| UC-01 | ttft_ms mistral:mistral-large-latest:chat | null |
| UC-01 | ttft_ms openai:gpt-6-astra:chat | null |
| UC-01 | ttft_ms xai:grok-4.6:chat | 1646 |
| UC-09 | client_words | 645 |
| UC-09 | model | groq:openai/gpt-oss-120b:chat |
| UC-09 | raw_stop_reason | max_tokens |
| UC-10 | failing_models | 2 |

## Findings (failing checks, latest run)

- **UC-01** openai:gpt-6-astra:chat answers — error shown to the user: "Function tools with reasoning_effort are not supported for gpt-6-astra in /v1/chat/completions. To use function tools, use /v1/responses or set reasoning_effort to 'none'." (server)
- **UC-01** mistral:mistral-large-latest:chat answers — error shown to the user: "This model is not available in your subscription tier" (server)

## History

<!-- history: one row per run, appended by the runner -->
| Run (UTC) | Desktop | Target | Pass | Fail | Skip | Flags |
| --- | --- | --- | --- | --- | --- | --- |
| 2026-09-17T09:17:01Z | 21efbf0 | http://localhost:8000 | 2 | 9 | 0 | B6GTY --quick (VECTORIZE not_pulled) |
| 2026-09-17T09:31Z | 21efbf0 | http://localhost:8000 | — | — | — | TMR43 aborted at UC-06 |
| 2026-09-17T09:36Z | 21efbf0 | http://localhost:8000 | 3 | 8 | 0 | composite B6GTY+6GW9Q (this file) |
| 2026-09-17T10:15:44Z | 2379231 | http://localhost:8000 | 4 | 7 | 0 | --quick --verbose |
| 2026-09-17T10:53:45Z | c893147 | http://localhost:8000 | 3 | 1 | 0 | --quick --verbose --only UC-00,UC-01,UC-09,UC-10 |
