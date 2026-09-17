# Desktop use-case test status

Written by `cargo run -p synaplan-core --example usecases`. Do not edit the table by hand — see [README.md](./README.md) §5 for the ledger policy and [CHANGES.md](./CHANGES.md) for the work each failing row demands.

**Last run:** 2026-09-17T10:15:44Z · target `http://localhost:8000` · desktop `2379231` · flags `--quick --verbose` · **4 passed, 7 failed, 0 skipped**

| Case | Journey | Result | Time | Detail |
| ---- | ------- | ------ | ---- | ------ |
| UC-00 | Open the app: models and workspace reachable | ✅ PASS | 85 ms | 44 chat models available |
| UC-01 | Pick a chat model, say hello | ❌ FAIL | 5.8 s | 4/6 models answered PONG |
| UC-02 | Ask the assistant to write a file | ❌ FAIL | 17.7 s | 3/6 models wrote the file |
| UC-03 | Turn Web on, ask a current-events question | ❌ FAIL | 69.1 s | 1/6 models answered with a source |
| UC-04 | Online research into a file | ❌ FAIL | 30.5 s | 1/3 models produced sources.md |
| UC-05 | Drop one file, ask about it | ❌ FAIL | 10.7 min | upload failed |
| UC-06 | Index a whole folder (megabytes) and ask | ❌ FAIL | 27.6 min | 6 files (4.96 MB) ready in 1643.1 s, 2/2 facts answered |
| UC-07 | Dictate a note | ✅ PASS | 27.2 s | 31 s take → text in 1.1 s after stop, similarity 0.99 |
| UC-08 | Write, find and delete notes (local) | ✅ PASS | 4 ms | create → write → read → search → append → delete |
| UC-09 | Ask for a long answer | ❌ FAIL | 4.6 s | 616 words, stop_reason Some("max_tokens") |
| UC-10 | Something goes wrong: what does the person read? | ✅ PASS | 8.9 s | 2 model(s) failed — copy checked |

## Metrics (latest run)

| Case | Metric | Value |
| ---- | ------ | ----- |
| UC-00 | chat_models_available | 44 |
| UC-00 | chat_models_total | 54 |
| UC-00 | default_sound2text | groq:whisper-large-v3:sound2text |
| UC-00 | default_text2sound | google:gemini-3.1-flash-tts-preview:text2sound |
| UC-00 | default_vectorize | ollama:bge-m3:vectorize |
| UC-01 | total_ms anthropic:claude-fable-5-1:chat | 3648 |
| UC-01 | total_ms google:gemini-3.5-flash:chat | 1243 |
| UC-01 | total_ms groq:openai/gpt-oss-120b:chat | 447 |
| UC-01 | total_ms mistral:mistral-large-latest:chat | 632 |
| UC-01 | total_ms openai:gpt-6-astra:chat | 745 |
| UC-01 | total_ms xai:grok-4.6:chat | 2189 |
| UC-01 | ttft_ms anthropic:claude-fable-5-1:chat | 2810 |
| UC-01 | ttft_ms google:gemini-3.5-flash:chat | 1235 |
| UC-01 | ttft_ms groq:openai/gpt-oss-120b:chat | 444 |
| UC-01 | ttft_ms mistral:mistral-large-latest:chat | null |
| UC-01 | ttft_ms openai:gpt-6-astra:chat | null |
| UC-01 | ttft_ms xai:grok-4.6:chat | 2139 |
| UC-02 | steps anthropic:claude-fable-5-1:chat | 1 |
| UC-02 | steps google:gemini-3.5-flash:chat | 1 |
| UC-02 | steps groq:openai/gpt-oss-120b:chat | 2 |
| UC-02 | steps mistral:mistral-large-latest:chat | 0 |
| UC-02 | steps openai:gpt-6-astra:chat | 0 |
| UC-02 | steps xai:grok-4.6:chat | 1 |
| UC-02 | total_ms anthropic:claude-fable-5-1:chat | 8619 |
| UC-02 | total_ms google:gemini-3.5-flash:chat | 2474 |
| UC-02 | total_ms groq:openai/gpt-oss-120b:chat | 1805 |
| UC-02 | total_ms mistral:mistral-large-latest:chat | 492 |
| UC-02 | total_ms openai:gpt-6-astra:chat | 421 |
| UC-02 | total_ms xai:grok-4.6:chat | 3847 |
| UC-03 | rounds anthropic:claude-fable-5-1:chat | 3 |
| UC-03 | rounds google:gemini-3.5-flash:chat | 1 |
| UC-03 | rounds groq:openai/gpt-oss-120b:chat | 6 |
| UC-03 | rounds mistral:mistral-large-latest:chat | 0 |
| UC-03 | rounds openai:gpt-6-astra:chat | 0 |
| UC-03 | rounds xai:grok-4.6:chat | 2 |
| UC-03 | total_ms anthropic:claude-fable-5-1:chat | 8780 |
| UC-03 | total_ms google:gemini-3.5-flash:chat | 2879 |
| UC-03 | total_ms groq:openai/gpt-oss-120b:chat | 6255 |
| UC-03 | total_ms mistral:mistral-large-latest:chat | 357 |
| UC-03 | total_ms openai:gpt-6-astra:chat | 668 |
| UC-03 | total_ms xai:grok-4.6:chat | 19888 |
| UC-04 | steps anthropic:claude-fable-5-1:chat | 1 |
| UC-04 | steps groq:openai/gpt-oss-120b:chat | 1 |
| UC-04 | steps openai:gpt-6-astra:chat | 0 |
| UC-04 | total_ms anthropic:claude-fable-5-1:chat | 21205 |
| UC-04 | total_ms groq:openai/gpt-oss-120b:chat | 8752 |
| UC-04 | total_ms openai:gpt-6-astra:chat | 493 |
| UC-04 | urls anthropic:claude-fable-5-1:chat | 5 |
| UC-04 | urls groq:openai/gpt-oss-120b:chat | 0 |
| UC-04 | urls openai:gpt-6-astra:chat | 0 |
| UC-06 | all_ready_ms | 1643136 |
| UC-06 | corpus_files | 7 |
| UC-06 | corpus_mb | 4.96 |
| UC-06 | facts_hit | 2/2 |
| UC-06 | mb_per_min | 0.18 |
| UC-06 | ready_ms finance-6GW9Q.docx | 352 |
| UC-06 | ready_ms survey-6GW9Q.pdf | 352 |
| UC-06 | ready_ms weekly-0-6GW9Q.md | 352 |
| UC-06 | ready_ms weekly-1-6GW9Q.md | 352 |
| UC-06 | ready_ms weekly-2-6GW9Q.md | 352 |
| UC-06 | ready_ms weekly-3-6GW9Q.md | 352 |
| UC-06 | upload_ms_total | 659534 |
| UC-07 | correction_pass_ms | 1033 |
| UC-07 | correction_windows | 1 |
| UC-07 | final_similarity | 0.99 |
| UC-07 | first_interim_ms | 783 |
| UC-07 | live_commit_ms | 48 |
| UC-07 | live_phrase_commit_ms | [732,1532] |
| UC-07 | live_phrases | 2 |
| UC-07 | live_similarity | 0.99 |
| UC-07 | long_correction_pass_ms | 4624 |
| UC-07 | long_correction_windows | 4 |
| UC-07 | long_one_shot_ms | 1861 |
| UC-07 | long_take_seconds | 124 |
| UC-07 | one_shot_ms | 1278 |
| UC-07 | one_shot_similarity | 0.99 |
| UC-07 | session_open_ms | 27 |
| UC-07 | stop_to_final_ms | 1082 |
| UC-07 | take_seconds | 31.0 |
| UC-07 | tts_model | google:gemini-3.1-flash-tts-preview:text2sound |
| UC-07 | tts_ms | 15865 |
| UC-07 | voice_model | groq:whisper-large-v3:sound2text |
| UC-09 | client_words | 616 |
| UC-09 | model | groq:openai/gpt-oss-120b:chat |
| UC-09 | raw_stop_reason | max_tokens |
| UC-10 | failing_models | 2 |

## Findings (failing checks, latest run)

- **UC-01** openai:gpt-6-astra:chat answers — error shown to the user: "Unsupported parameter: 'max_tokens' is not supported with this model. Use 'max_completion_tokens' instead." (server)
- **UC-01** mistral:mistral-large-latest:chat answers — error shown to the user: "This model is not available in your subscription tier" (server)
- **UC-02** openai:gpt-6-astra:chat: hello.md written and turn ends in text — error shown: "Unsupported parameter: 'max_tokens' is not supported with this model. Use 'max_completion_tokens' instead." (server) after 0 steps
- **UC-02** google:gemini-3.5-flash:chat: hello.md written and turn ends in text — error shown: "Function call is missing a thought_signature in functionCall parts. This is required for tools to work correctly, and missing thought_signature may lead to degraded model performance. Additional data, function call `default_api:write_file` , position 2. Please refer to https://ai.google.dev/gemini-api/docs/thought-signatures for more details." (server) after 1 steps
- **UC-02** mistral:mistral-large-latest:chat: hello.md written and turn ends in text — error shown: "This model is not available in your subscription tier" (server) after 0 steps
- **UC-03** anthropic:claude-fable-5-1:chat: answer with a source — stop_reason=Some("end_turn") rounds≈3 error=None client_text="" total 8780 ms
- **UC-03** groq:openai/gpt-oss-120b:chat: answer with a source — stop_reason=Some("end_turn") rounds≈6 error=None client_text="" total 6255 ms
- **UC-03** groq:openai/gpt-oss-120b:chat: within budget — 6 gateway rounds, 6255 ms (ceiling 4 rounds / 60000 ms)
- **UC-03** openai:gpt-6-astra:chat: answer with a source — stop_reason=None rounds≈0 error=Some("Unsupported parameter: 'max_tokens' is not supported with this model. Use 'max_completion_tokens' in…") client_text="" total 668 ms
- **UC-03** google:gemini-3.5-flash:chat: answer with a source — stop_reason=None rounds≈1 error=Some("Function call is missing a thought_signature in functionCall parts. This is required for tools to wo…") client_text="" total 2879 ms
- **UC-03** mistral:mistral-large-latest:chat: answer with a source — stop_reason=None rounds≈0 error=Some("This model is not available in your subscription tier") client_text="" total 357 ms
- **UC-04** groq:openai/gpt-oss-120b:chat: sources.md with ≥3 URLs — error shown: "failed to template request: failed to render tokenized output: failed to render tokens with harmony: HarmonyError: EncodingError: Message=render failed: Tools should have a name!" (server) after 1 steps
- **UC-04** openai:gpt-6-astra:chat: sources.md with ≥3 URLs — error shown: "Unsupported parameter: 'max_tokens' is not supported with this model. Use 'max_completion_tokens' instead." (server) after 0 steps
- **UC-05** upload accepted — upload HTTP 500 Internal Server Error (server)
- **UC-06** handbook-6GW9Q.md upload accepted — upload HTTP 500 Internal Server Error (server)
- **UC-06** every file accepted — 6/7 uploaded in 659534 ms
- **UC-06** indexing throughput ≥ 1 MB/min — 0.18 MB/min for 4.96 MB
- **UC-09** client is told the answer was cut off — raw stop_reason=max_tokens after 616 words; stream_chat delivered Done with no truncation signal

## History

<!-- history: one row per run, appended by the runner -->
| Run (UTC) | Desktop | Target | Pass | Fail | Skip | Flags |
| --- | --- | --- | --- | --- | --- | --- |
| 2026-09-17T09:17:01Z | 21efbf0 | http://localhost:8000 | 2 | 9 | 0 | B6GTY --quick (VECTORIZE not_pulled) |
| 2026-09-17T09:31Z | 21efbf0 | http://localhost:8000 | — | — | — | TMR43 aborted at UC-06 |
| 2026-09-17T09:36Z | 21efbf0 | http://localhost:8000 | 3 | 8 | 0 | composite B6GTY+6GW9Q (this file) |
| 2026-09-17T10:15:44Z | 2379231 | http://localhost:8000 | 4 | 7 | 0 | --quick --verbose |
