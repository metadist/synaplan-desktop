# Vendored Synaplan Desktop contract fixtures (`protocol: 1`)

These JSON files are **vendored byte-for-byte** from the Synaplan server
repository. They are the frozen wire shapes of the desktop job / check-in
contract (Sprint A3, `DS18`) and are the single source of truth this client
builds its unit tests against, so the client can be developed and tested
without a live server (DC3).

**Do not edit these files.** Any change to them is a `protocol: 2` decision on
the server side, with a migration plan (invariant C9). On the client side they
are test *input*, never something we tweak for convenience. `tests/unit` and
`src-tauri/synaplan-core` assert against them; a checksum guard fails the build
if a byte changes.

## Source

- **Repository:** `github.com/metadist/synaplan`
- **Path:** `_devextras/testing/desktop/fixtures/`
- **Commit:** `f29e543e9d115d89b43dc330b721ce3ad669c713`
  (`feat(desktop): implement desktop agent features and integration (#1647)`)

## SHA-256 (recorded from the source commit — the checksum guard compares these)

```
8e72b150e012ae6e6dfe6dc095bdadf8f0d33ef10e881ab1e0e141abe9b07637  checkin_request.json
4de6c8d175a15d5c4ad1ce28e2f48939aa2a7271bef83fccc99e6ff97cd21bf9  checkin_response.json
e5c99d7f411629c40da65b1b6b4c01fac92a23c98fed1ea8fdbc2e69fd32a037  job_skill_run.json
c5a90db9d43f3587f6eeb0e8b0fafad3013aaa96a0108fb205afa95fc91edebd  enqueue_request.json
e60e9f7f8861c05ea8dc726648fb227fbfaac32685c32de6c1317de2b2ac8a44  report_success.json
03ff8ab7d9245bba87971dd1923c4b9b9e7185a6ce5ba2103ef9b42e4403ff31  report_unknown_skill.json
```

## The two rules a client must never break

1. **`protocol: 1`** is carried by both the check-in request and response. A
   device speaking an unknown protocol is answered with an empty job list and a
   far `next_call_at` — never a guess.
2. **A job's device-facing `input` is ONLY `{skill, prompt, fileIds}`.** Any
   other key (`command`, `script`, `argv`, …) is dropped by the server and MUST
   be ignored by the device. There is no field through which a shell string can
   reach the computer. These fixtures are used from Sprint B5's poll loop.
