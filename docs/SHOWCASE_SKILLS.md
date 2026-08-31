# Showcase skills catalog

A curated set of Agent Skills to demonstrate Synaplan Desktop on **Windows and
macOS** (all also work on Linux). Each runs **locally**, under the folder
allowlist, with **no shell and no network** — a skill calls an allowlisted
interpreter (Python / Node) or LibreOffice with explicit arguments and writes
into the out-box.

> **Dependency reality (v1).** Skills that need only the interpreter's **standard
> library** or **LibreOffice** run out of the box. Skills that need a
> third-party package (Pillow, python-pptx, …) need that package present; v1 does
> not auto-run `pip install` (PEP 668). The planned per-skill virtualenv
> (`DC26`/follow-up) removes this caveat. Until then, prefer **Wave 1** for a
> zero-setup demo.

## Wave 1 — zero-setup (stdlib + LibreOffice), bundle-ready

| Skill | What it does | Needs | Win | mac |
| ----- | ------------ | ----- | :-: | :-: |
| **anytopdf** | Convert Word/Excel/PowerPoint/ODF/CSV/image to PDF | LibreOffice (`soffice --headless --convert-to pdf`) | ✅ | ✅ |
| **office-convert** | Convert between docx/odt, xlsx/ods, pptx/odp, etc. | LibreOffice | ✅ | ✅ |
| **csv-insights** | Profile a CSV → Markdown report (rows/cols, types, missing %, top values, basic stats) | Python stdlib | ✅ | ✅ |
| **format-convert** | JSON ⇄ TOML ⇄ CSV convert / validate / pretty-print | Python stdlib (`json`, `tomllib`, `csv`) | ✅ | ✅ |
| **zip-kit** | Create / extract / list `.zip` archives safely | Python stdlib (`zipfile`) | ✅ | ✅ |
| **checksums** | Generate / verify SHA-256 checksums for files | Python stdlib (`hashlib`) | ✅ | ✅ |
| **ics-calendar** | Build `.ics` calendar events from a description | Python stdlib | ✅ | ✅ |
| **text-stats** | Word/line/char counts, readability, keyword frequency for a text/markdown file | Python stdlib | ✅ | ✅ |

## Wave 2 — needs one package (great once per-skill venvs land)

| Skill | What it does | Needs | Win | mac |
| ----- | ------------ | ----- | :-: | :-: |
| **pptx** ⭐ | Build a PowerPoint deck from notes/outline/markdown (the bundled marquee; official Apache-2.0) | Python + `python-pptx` (or LibreOffice) | ✅ | ✅ |
| **docx** | Draft/format a Word document from markdown/notes | Python + `python-docx` | ✅ | ✅ |
| **xlsx-report** | CSV → formatted Excel workbook with a summary sheet + chart | Python + `openpyxl` | ✅ | ✅ |
| **pdf-toolkit** | Merge / split / rotate / extract text / add page numbers | Python + `pypdf` | ✅ | ✅ |
| **image-kit** | Resize / crop / convert / compress images; strip EXIF; batch a folder | Python + `Pillow` | ✅ | ✅ |
| **contact-sheet** | Thumbnail-grid contact sheet PNG from a folder of images | Python + `Pillow` | ✅ | ✅ |
| **photo-rename** | Rename photos by EXIF capture date (dry-run + confirm) | Python + `Pillow` | ✅ | ✅ |
| **chart-maker** | CSV/JSON series → bar/line/pie chart PNG | Python + `matplotlib` | ✅ | ✅ |
| **qr-code** | QR code PNG/SVG from text / URL / Wi-Fi / vCard | Python + `segno` (pure-Python) | ✅ | ✅ |
| **markdown-to-html** | Render a Markdown file to a styled standalone HTML file | Node + `markdown-it` (or Python `markdown`) | ✅ | ✅ |

⭐ = bundled first (Sprint B4).

## Recommended first showcase (5)

A compelling, zero-to-low-setup demo across file types:

1. **pptx** — "make a 5-slide deck about Q3 from these notes" (the wow moment).
2. **anytopdf** — "turn this .docx into a PDF" (LibreOffice, no pip).
3. **csv-insights** — "summarize sales.csv" → tidy Markdown report (stdlib).
4. **image-kit** — "resize every photo in this folder to 1080px and strip EXIF".
5. **qr-code** — "make a QR code for this URL" (instant, visual).

## What each skill must respect (contract)

- Only `{program, args[]}` execution via allowlisted interpreters — **never a
  shell**, never `curl`/`wget`/PowerShell.
- Read only from allowlisted folders; write only into the out-box.
- Declare `compatibility` for required tools so the doctor can block a skill
  whose runtime is missing instead of failing mid-run.

## Not in the showcase (and why)

- Anything needing the **network** (web scraping, API calls) — skills have no
  network; use Synaplan server tools / MCP instead.
- **Outlook / OS automation** (COM / AppleScript) — out of scope (use Synamail /
  M365). See the plan §12.
- Heavy toolchains (Chromium for Mermaid, ffmpeg video) — possible later, but not
  a first-wave, dependency-light showcase.

See also: [docs.synaplan.com/desktop-skills](https://docs.synaplan.com/desktop-skills)
and `docs/LOCAL_TOOLS.md`.
