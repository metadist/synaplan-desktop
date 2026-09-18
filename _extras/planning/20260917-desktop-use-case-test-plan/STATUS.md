# Desktop use-case test status

Written by `cargo run -p synaplan-core --example usecases`. Do not edit the table by hand — see [README.md](./README.md) §5 for the ledger policy and [CHANGES.md](./CHANGES.md) for the work each failing row demands.

**Last run:** 2026-09-18T07:08:42Z · target `http://localhost:8000` · desktop `e79b589` · flags `--quick --verbose --only UC-00,UC-02,UC-03,UC-04` · **1 passed, 3 failed, 0 skipped**

| Case | Journey | Result | Time | Detail |
| ---- | ------- | ------ | ---- | ------ |
| UC-00 | Open the app: models and workspace reachable | ✅ PASS | 119 ms | 44 chat models available |
| UC-02 | Ask the assistant to write a file | ❌ FAIL | 28.1 s | 5/6 models wrote the file |
| UC-03 | Turn Web on, ask a current-events question | ❌ FAIL | 76.3 s | 4/6 models answered with a source |
| UC-04 | Online research into a file | ❌ FAIL | 59.0 s | 2/3 models produced sources.md |

## Metrics (latest run)

| Case | Metric | Value |
| ---- | ------ | ----- |
| UC-00 | chat_models_available | 44 |
| UC-00 | chat_models_total | 54 |
| UC-00 | default_sound2text | groq:whisper-large-v3:sound2text |
| UC-00 | default_text2sound | google:gemini-3.1-flash-tts-preview:text2sound |
| UC-00 | default_vectorize | ollama:bge-m3:vectorize |
| UC-02 | steps anthropic:claude-fable-5-1:chat | 1 |
| UC-02 | steps google:gemini-3.5-flash:chat | 1 |
| UC-02 | steps groq:openai/gpt-oss-120b:chat | 1 |
| UC-02 | steps mistral:mistral-large-latest:chat | 0 |
| UC-02 | steps openai:gpt-6-astra:chat | 2 |
| UC-02 | steps xai:grok-4.6:chat | 1 |
| UC-02 | total_ms anthropic:claude-fable-5-1:chat | 8941 |
| UC-02 | total_ms google:gemini-3.5-flash:chat | 3228 |
| UC-02 | total_ms groq:openai/gpt-oss-120b:chat | 1094 |
| UC-02 | total_ms mistral:mistral-large-latest:chat | 307 |
| UC-02 | total_ms openai:gpt-6-astra:chat | 10323 |
| UC-02 | total_ms xai:grok-4.6:chat | 4178 |
| UC-03 | rounds anthropic:claude-fable-5-1:chat | 3 |
| UC-03 | rounds google:gemini-3.5-flash:chat | 2 |
| UC-03 | rounds groq:openai/gpt-oss-120b:chat | 2 |
| UC-03 | rounds mistral:mistral-large-latest:chat | 0 |
| UC-03 | rounds openai:gpt-6-astra:chat | 1 |
| UC-03 | rounds xai:grok-4.6:chat | 1 |
| UC-03 | total_ms anthropic:claude-fable-5-1:chat | 10031 |
| UC-03 | total_ms google:gemini-3.5-flash:chat | 7508 |
| UC-03 | total_ms groq:openai/gpt-oss-120b:chat | 2890 |
| UC-03 | total_ms mistral:mistral-large-latest:chat | 276 |
| UC-03 | total_ms openai:gpt-6-astra:chat | 4619 |
| UC-03 | total_ms xai:grok-4.6:chat | 16549 |
| UC-04 | steps anthropic:claude-fable-5-1:chat | 2 |
| UC-04 | steps groq:openai/gpt-oss-120b:chat | 1 |
| UC-04 | steps openai:gpt-6-astra:chat | 1 |
| UC-04 | total_ms anthropic:claude-fable-5-1:chat | 28569 |
| UC-04 | total_ms groq:openai/gpt-oss-120b:chat | 4148 |
| UC-04 | total_ms openai:gpt-6-astra:chat | 26240 |
| UC-04 | urls anthropic:claude-fable-5-1:chat | 5 |
| UC-04 | urls groq:openai/gpt-oss-120b:chat | 5 |
| UC-04 | urls openai:gpt-6-astra:chat | 0 |

## Findings (failing checks, latest run)

- **UC-02** mistral:mistral-large-latest:chat: hello.md written and turn ends in text — error shown: "This model rejected the request. Try another model." (model_rejected) after 0 steps
- **UC-03** groq:openai/gpt-oss-120b:chat: answer with a source — stop_reason=Some("end_turn") rounds≈2 error=None client_text="I looked this up but could not turn the results into an answer. Please try again, or ask i…" total 2890 ms
- **UC-03** mistral:mistral-large-latest:chat: answer with a source — stop_reason=None rounds≈0 error=Some("This model is not available in your subscription tier") client_text="" total 276 ms
- **UC-04** openai:gpt-6-astra:chat: sources.md with ≥3 URLs — 1 steps (0 web_search retry hints), 0 URLs in sources.md, 26240 ms, reply "I’ll save the latest LTS release found in the search results…"

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
