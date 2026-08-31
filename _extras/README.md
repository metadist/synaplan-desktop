# `_extras/` — vendored reference material

This directory holds **read-only reference copies** of material that lives in
other repositories, so a developer working in `synaplan-desktop` has the context
they need without switching repos.

## `planning/20260829-desktop-agent-client/`

A vendored copy of the desktop-agent-client **plan of record**, copied from the
`synaplan` repository at `_devextras/planning/20260829-desktop-agent-client/`
(source commit `cd8b1a5`).

- **This is a reference, not the source of truth.** The authoritative plan lives
  in the `synaplan` repo. If the two ever disagree, the `synaplan` copy wins.
- Read `00_master_plan.md` first, then the sprint file for the step you are on.
  Phase B (`DC*` steps) is this repository; Phase A (`DS*`) is the server and is
  already merged.
- Do not treat edits here as changing the plan — propose plan changes in the
  `synaplan` repo.

Refresh it when the plan advances:

```bash
cp -r ../synaplan/_devextras/planning/20260829-desktop-agent-client \
      _extras/planning/
```
