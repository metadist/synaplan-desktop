# Local tools & skills

This is the developer view of how Synaplan Desktop runs **local skills** and the
**tools** they rely on (Python, Node.js, LibreOffice). The user-facing version is
[docs.synaplan.com/desktop-tools](https://docs.synaplan.com/desktop-tools).

> **Status:** the runtime described here is Sprint **B2** (skills runtime +
> confinement) and **B4** (the doctor + bundled `pptx`). Sprint B1 (pairing +
> chat + app shell) is done; the pieces below are the next work. This doc is the
> design so we build it "super cool *and* safe".

## The idea

A **skill** is an [Agent Skill](https://agentskills.io/specification): a folder
with a `SKILL.md` (name + description + instructions) and optional `scripts/` and
`assets/`. Enabled skills are advertised to the AI by name/description; when one
applies, the AI reads the full `SKILL.md` and calls tools (`Read`, `Write`,
`Bash`) that the desktop app executes **locally** — inside the user's folder
allowlist and using the local interpreters.

The magic users feel ("make a deck from these notes and save it to my Desktop")
comes from three boring, strict pieces working together: tool discovery
(the doctor), a binary allowlist, and path confinement.

## Rule zero: there is no shell

The single most important rule (master plan decision 29 / invariant **C12**):

- **Tools are `{ program, args[], workdir }`.** Never a command string.
- `sh -c`, `bash -c`, `cmd /c`, `powershell -Command`, `osascript -e` are
  **never constructed** — on any platform. `scripts/no-shell-guard.sh` fails CI
  if one appears in the sources.
- The ecosystem tool name `Bash` is kept for skill compatibility, but the
  incoming command is *tokenized* to `{program, args[]}` and anything with a
  shell metacharacter (`| & ; > < \` $( newline`) is refused, not sanitized.

So "PowerShell tools" specifically: **PowerShell and cmd are denied as execution
vectors.** A skill cannot ask us to run a `.ps1` through PowerShell. Windows
automation via COM/PowerShell is explicitly out of scope for v1. What a skill
*can* do on Windows is run an allowlisted **interpreter** (Python, Node) with
explicit arguments — which covers the portable skills we care about.

## The doctor (tool discovery) — `DC16` / `DC26`

The **Check this computer** screen resolves each tool to an absolute real path,
probes it (`--version` with a timeout — a binary that doesn't answer is
"missing"), and stores that path. A skill whose required tool is missing is
shown as blocked and never offered to the model.

Per-OS discovery (details in `docs/PLATFORMS.md` and the plan's
`13_cross_platform.md` §5):

| Tool | Windows | macOS | Linux |
| ---- | ------- | ----- | ----- |
| **Python** | `py -3` launcher, `python.exe` in Program Files / per-user; **ignore** the Microsoft Store `WindowsApps\python.exe` stub | `/usr/bin/python3` (Command Line Tools), Homebrew `python3` | distro `python3`; hint if `python3-venv` missing or PEP 668 externally-managed |
| **Node** | `node.exe` in `Program Files\nodejs`; note `npm`/`npx` are `.cmd` and need an explicit interpreter | Homebrew (`/opt/homebrew`, `/usr/local`) | distro `node` |
| **LibreOffice** | `soffice.com` under `C:\Program Files\LibreOffice\program\` | `/Applications/LibreOffice.app/Contents/MacOS/soffice` | distro `soffice`; Flatpak reported honestly |

`PATH` is attacker-influenced, so the **binary allowlist stores absolute real
paths**, never bare names. Discovery order: explicit config path → known install
locations → `PATH` (honouring `PATHEXT` on Windows).

## Execution model — `DC9` / `DC17`

When a tool runs:

1. `program` must resolve to an entry in the binary allowlist (an absolute real
   path), or to a script inside the invoking skill's own folder launched **by**
   an allowlisted interpreter.
2. `workdir` is the run scratch dir under the out-box, path-confined like any
   other path.
3. The **environment is constructed, not inherited** — start empty, add only a
   minimal `PATH` (allowlisted tool dirs), a scratch `HOME`/`TMPDIR`, `LANG`,
   `PYTHONIOENCODING=utf-8`. The Synaplan key, pairing code, `LD_PRELOAD`,
   `DYLD_INSERT_LIBRARIES`, `PYTHONPATH`, `NODE_OPTIONS`, … are never passed.
4. A **timeout kills the whole process tree** (Windows Job Object; POSIX process
   group + `SIGKILL`), so a hung grandchild can't linger.
5. Output is byte-capped and normalized to UTF-8 before it's shown or re-ingested.

## Path confinement — `DC6` / `DC24`

Every path (from the model, a script arg, a zip entry, or config) is
canonicalized, normalized per-OS, and **contained by path components** against
the allowlist — with deny globs applied last. The shared, table-driven corpus in
`tests/confinement/cases.toml` runs on all three OSes and covers symlink and
Windows junction escapes, UNC, drive-relative, 8.3 short names, alternate data
streams, reserved device names, `/private` firmlinks, NFC/NFD, and TOCTOU. See
the plan's `13_cross_platform.md` §3.

## Where the code lives (as we build it)

- Interpreter discovery: `src-tauri/synaplan-core/src/platform/` (a new
  `doctor` module) + a `#[tauri::command]` wrapper.
- Process spawn + env construction + tree-kill: `platform/process.rs`.
- Path confinement + the corpus: `platform/confinement.rs` +
  `tests/confinement/cases.toml`.
- The tool loop (`Read`/`Write`/`Bash` → `{program, args[]}`) sits in core and is
  driven from the chat/poll loops.

Keep all of this in `platform/` and core — the Vue side only ever sees results.
