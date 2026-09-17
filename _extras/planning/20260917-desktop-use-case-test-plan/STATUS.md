# Desktop use-case test status

Written by `cargo run -p synaplan-core --example usecases`. Do not edit the table by hand — see [README.md](./README.md) §5 for the ledger policy and [CHANGES.md](./CHANGES.md) for the work each failing row demands.

**Last run:** 2026-09-17T12:15:10Z · target `http://localhost:8000` · desktop `1746c70` · flags `--verbose --only UC-10 --models mistral:mistral-large-latest:chat` · **1 passed, 0 failed, 0 skipped**

| Case | Journey | Result | Time | Detail |
| ---- | ------- | ------ | ---- | ------ |
| UC-10 | Something goes wrong: what does the person read? | ✅ PASS | 244 ms | 1 model(s) failed — copy checked |

## Metrics (latest run)

| Case | Metric | Value |
| ---- | ------ | ----- |
| UC-10 | failing_models | 1 |

## Findings (failing checks, latest run)

- none

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
