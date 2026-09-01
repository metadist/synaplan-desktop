# Skills catalog

Synaplan Desktop runs [Agent Skills](https://agentskills.io/specification) on
this computer: a folder with a `SKILL.md` (name, description, instructions) and
optional `scripts/` and `assets/`. Enabled skills are advertised to the
assistant; when one applies, it reads the full `SKILL.md` and calls local tools.

User-facing walkthrough: [docs.synaplan.com/desktop-skills](https://docs.synaplan.com/desktop-skills).

## Folder layout

The folder name **must** match the `name` in the `SKILL.md` frontmatter
(lowercase, hyphens):

```text
my-skill/
  SKILL.md          # required — YAML frontmatter + instructions
  scripts/          # optional — the programs the skill documents
  assets/           # optional
```

```yaml
---
name: my-skill
description: One or two sentences the assistant uses to decide when to apply this skill.
---
```

## Where to put a skill

| OS | Skills directory |
| -- | ---------------- |
| Windows | `%LOCALAPPDATA%\Synaplan\Desktop\skills\` |
| macOS | `~/Library/Application Support/com.synaplan.desktop/skills/` |
| Linux | `$XDG_DATA_HOME/synaplan-desktop/skills/` or, if unset, `~/.local/share/synaplan-desktop/skills/` |

Bundled skills live in this repository under `skills/bundled/` and are copied
into that directory when the app starts. Your own folders sit next to them.

## Install today (folder copy)

A zip/Git installer in the UI is **coming soon**. Until then:

1. Copy a folder that contains `SKILL.md` (and any scripts it references) into
   the skills directory above.
2. Restart the app, or leave **Skills** and open it again.
3. Enable the toggle. Disabled skills are never offered to the assistant.

Treat a skill you did not write as untrusted until you have read its
`SKILL.md` and scripts.

## Safety

Synaplan Desktop is stricter than a typical Agent Skills host:

- Tools are `{program, args[]}`. There is **no shell** (`sh -c`, `cmd /c`,
  `powershell -Command`, …) and **no network** (`curl`, `wget`, …).
- A skill may run only allowlisted interpreters the [doctor](https://docs.synaplan.com/desktop-tools)
  found: **Python**, **Node**, **LibreOffice**.
- Reads stay inside folders you allow; writes go to the
  [out-box](https://docs.synaplan.com/desktop-folders).
- Review `SKILL.md` before you enable a skill. Many community skills assume an
  unrestricted shell — those calls are **refused**, not sanitized.

See [`LOCAL_TOOLS.md`](LOCAL_TOOLS.md) for the execution model.

## Bundled skills

These eleven ship with the app (`skills/bundled/`). They need **Python 3**
(standard library — no `pip install`) and write into the out-box.

| Skill | What it creates |
| ----- | --------------- |
| **csv-insights** | Markdown profile of a CSV (rows, fill rates, numeric and category summaries) |
| **email-draft** | Unsent `.eml` draft for Outlook, Apple Mail, or Thunderbird |
| **web-report** | Standalone HTML page from Markdown or plain text |
| **slides** | Self-contained HTML presentation (arrow keys; no PowerPoint) |
| **chart** | Bar or line chart from a CSV, as SVG inside a standalone HTML page |
| **data-table** | Searchable, sortable HTML table from a CSV |
| **calendar-event** | `.ics` invite for Outlook, Apple Calendar, or Google Calendar |
| **vcard** | `.vcf` contact card for any address book |
| **json-csv** | JSON array of objects ⇄ CSV |
| **invoice** | Print-ready HTML invoice from a JSON spec |
| **hello-files** | Tiny example that writes `hello.txt` into the out-box |

They are the gold-standard examples for this runtime: local files only,
`{program, args[]}`, no shell, no network.

## Installable OSS skills

Public Agent Skills that stay on **local files** can work here if they never
need a shell or the network. Be honest about the ecosystem: most community
skills were written for hosts that run `Bash` and fetch URLs. Synaplan Desktop
will refuse those tool calls.

**Start here**

- Official spec and examples: [agentskills.io](https://agentskills.io)
- This repo's bundled set: [`skills/bundled/`](../skills/bundled/) — copy the
  pattern, not a skill that shells out
- [anthropics/skills](https://github.com/anthropics/skills) document skills
  (docx / pptx / xlsx / pdf) are useful references, with caveats: they may
  need extra `pip` packages, and some invoke Bash. Those steps will not run
  here until they are rewritten as `{program, args[]}` against Python, Node,
  or LibreOffice

**Compatible to write (or install as a folder)**

These Wave-1 ideas from [`SHOWCASE_SKILLS.md`](SHOWCASE_SKILLS.md) fit the
runtime today (stdlib or LibreOffice, no pip, no shell). They are not bundled
yet — write them, or drop a finished folder into the skills directory.

| Skill | What it would do | Needs |
| ----- | ---------------- | ----- |
| **anytopdf** | Convert Word / Excel / PowerPoint / ODF / CSV / image to PDF | LibreOffice (`soffice --headless --convert-to pdf`) |
| **office-convert** | Convert between docx/odt, xlsx/ods, pptx/odp, and similar | LibreOffice |
| **zip-kit** | Create, extract, or list `.zip` archives safely | Python stdlib (`zipfile`) |
| **checksums** | Generate or verify SHA-256 checksums | Python stdlib (`hashlib`) |
| **text-stats** | Word / line / char counts, readability, keyword frequency | Python stdlib |

Skills that need a third-party package (Pillow, python-pptx, matplotlib, …)
can be installed the same way **if** that package is already on the machine.
v1 does not run `pip install` for you.

**Will not run as-is**

- Anything that scrapes the web, calls an HTTP API, or uses `curl` / `wget`
- Outlook / OS automation (COM, AppleScript, PowerShell)
- Skills whose only documented tool is `Bash` with pipes, redirects, or
  command substitution

Use Synaplan server tools or MCP for work that needs the network.

## See also

- [docs.synaplan.com/desktop-skills](https://docs.synaplan.com/desktop-skills)
- [`SHOWCASE_SKILLS.md`](SHOWCASE_SKILLS.md) — longer catalog by dependency wave
- [`LOCAL_TOOLS.md`](LOCAL_TOOLS.md) — doctor, binary allowlist, no-shell rule
