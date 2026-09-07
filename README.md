<div align="center">

# Synaplan Desktop

**Pair a computer with your Synaplan workspace and run Agent Skills locally.**

[Synaplan](https://github.com/metadist/synaplan) &nbsp;·&nbsp; [Website](https://www.synaplan.com) &nbsp;·&nbsp; [Live app](https://web.synaplan.com) &nbsp;·&nbsp; [Docs](https://docs.synaplan.com/desktop) &nbsp;·&nbsp; [Discord](https://discord.com/invite/kQB3eDjWfF) &nbsp;·&nbsp; [Synamail](https://github.com/metadist/Synamail)

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![GitHub](https://img.shields.io/badge/GitHub-metadist%2Fsynaplan--desktop-181717?logo=github&logoColor=white)](https://github.com/metadist/synaplan-desktop)
[![Docs](https://img.shields.io/badge/docs-docs.synaplan.com%2Fdesktop-6BA539)](https://docs.synaplan.com/desktop)

</div>

---

A small native app for **Windows, macOS, and Linux**. It pairs with your
[Synaplan](https://github.com/metadist/synaplan) workspace, chats through your
existing account, and runs installed [Agent Skills](https://agentskills.io/specification)
on this computer — under a folder allowlist, with no shell and no network.

It uses **only your Synaplan account**. There is no second AI subscription and
no vendor dashboard. The API key lives in the OS secret store (Windows Credential
Manager, macOS Keychain, Linux Secret Service), never in a config file.

> **Unsigned 1.0 preview.** Pair, chat, install or remove skills, check local
> tools, and run web-queued jobs on a running (optionally autostarted) app.
> **Build from source.** Signed public download comes after signing
> (notarization is still deferred). Unsigned installers are for internal
> testing only.

## What it is (and is not)

**It is** a Tauri 2 + Vue 3 desktop client. It talks to Synaplan over the
Anthropic-compatible Messages gateway (`/v1/messages`) and the desktop pairing
API. Enabled skills run locally as `{program, args[]}` against allowlisted
interpreters (Python, Node, LibreOffice).

**It is not** an Electron wrap of the Synaplan web app, and it never embeds
`web.synaplan.com` in a WebView. It is not a general-purpose shell host: skills
that need `bash`, `curl`, or the network are refused.

## Pair this computer

1. In Synaplan, open **Channels → Desktop** and choose *Pair this computer*
   (the instance needs **Desktop access** on — `DESKTOP_AGENT.ENABLED`).
2. In the app, enter the Synaplan address (for example
   `https://web.synaplan.com` or `http://localhost:8000` for a local install)
   and the short code.
3. The app stores a **scoped** key in the OS secret store. Disconnect the
   computer from the web UI at any time to revoke it.

## Docs

| Topic | Link |
| ----- | ---- |
| Overview and pairing | [docs.synaplan.com/desktop](https://docs.synaplan.com/desktop) |
| Skills | [docs.synaplan.com/desktop-skills](https://docs.synaplan.com/desktop-skills) |
| Folders and the out-box | [docs.synaplan.com/desktop-folders](https://docs.synaplan.com/desktop-folders) |
| Local tools (doctor) | [docs.synaplan.com/desktop-tools](https://docs.synaplan.com/desktop-tools) |
| In-repo skills catalog | [`docs/SKILLS.md`](docs/SKILLS.md) |

## Bundled skills

These ship with the app (`skills/bundled/`). Eleven need **Python 3** (standard
library only). **pptx** needs `python-pptx` and stays blocked until that
import works — the app never runs `pip`. See [`docs/BUNDLED_SKILLS.md`](docs/BUNDLED_SKILLS.md).

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
| **pptx** | PowerPoint deck (blocked until `python-pptx` is installed) |

## Install extra skills

On **Skills**, choose **From a folder**, **From a zip**, or **From a GitHub
address**, review the file list, then confirm. Many community Agent Skills
assume an unrestricted shell or network — Synaplan Desktop will refuse those.
See [`docs/SKILLS.md`](docs/SKILLS.md).

| OS | Skills directory |
| -- | ---------------- |
| Windows | `%LOCALAPPDATA%\Synaplan\Desktop\skills\` |
| macOS | `~/Library/Application Support/com.synaplan.desktop/skills/` |
| Linux | `$XDG_DATA_HOME/synaplan-desktop/skills/` or `~/.local/share/synaplan-desktop/skills/` |

## Developer quick start

Prerequisites are installed by the setup script for your OS:

```bash
# Linux
bash scripts/setup-linux.sh

# macOS
bash scripts/setup-macos.sh

# Windows (PowerShell)
./scripts/setup-windows.ps1
```

Then run the app:

```bash
npm run tauri dev
```

### Try it offline (no Synaplan server)

```bash
npm run mock-server          # starts http://localhost:8788
npm run tauri dev            # pair against http://localhost:8788 with any code
```

The mock accepts pairing, lists a mock model, and streams a short chat reply.

See [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) for the dev loop and
[`docs/PLATFORMS.md`](docs/PLATFORMS.md) for per-OS build prerequisites.

Run the full local gate (mirrors CI) before every commit:

```bash
make ci-local
```

## Architecture

- **`src/`** — Vue 3 + TypeScript UI. Talks to Rust only through
  `src/services/tauri.ts`.
- **`src-tauri/synaplan-core/`** — platform-independent, unit-tested Rust core
  (paths, secret store, pairing, SSE parsing, config, frozen contract types).
- **`src-tauri/synaplan-core/src/platform/`** — the only place OS differences
  live (`app_dirs`, `secret_store`).
- **`src-tauri/src/`** — the thin Tauri shell (commands + events).
- **`skills/bundled/`** — the twelve Agent Skills that ship with the app
  (eleven stdlib + a blocked-until-ready `pptx`).

## License

Apache License 2.0 — see [`LICENSE`](LICENSE). Copyright 2026 metadist GmbH.

## Status

Unsigned 1.0. Pairing, chat, skill install, doctor, tray, and the poll loop
work. **Build from source** with the setup scripts above. Signed public
installers are not published yet. On unsigned macOS, turning on “Start when I
sign in” may show a Login Items warning — that is expected.
