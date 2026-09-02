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

> **Preview.** Pairing, chat, bundled skills, the folder allowlist, and the
> local-tools check work today. Build from source — signed public installers are
> not out yet.

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
2. In the app, enter the Synaplan **API** address (for example
   `https://web.synaplan.com` or `http://localhost:8000` for a local install —
   not the Vite UI on `:5173` and not Keycloak on `:8080`) and the short code.
   On Linux/WSL you can also use the **API key** tab and paste a key.
3. The app stores a **scoped** key in the OS secret store (or a local file when
   no keyring is available). Disconnect the computer from the web UI at any
   time to revoke it.

## Docs

| Topic | Link |
| ----- | ---- |
| Overview and pairing | [docs.synaplan.com/desktop](https://docs.synaplan.com/desktop) |
| Skills | [docs.synaplan.com/desktop-skills](https://docs.synaplan.com/desktop-skills) |
| Folders and the out-box | [docs.synaplan.com/desktop-folders](https://docs.synaplan.com/desktop-folders) |
| Local tools (doctor) | [docs.synaplan.com/desktop-tools](https://docs.synaplan.com/desktop-tools) |
| In-repo skills catalog | [`docs/SKILLS.md`](docs/SKILLS.md) |

## Bundled skills

These ship with the app (`skills/bundled/`). They need **Python 3** (standard
library only) and write into the out-box.

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

## Install extra skills

Today you install a skill by **copying its folder** (it must contain
`SKILL.md`) into this computer's skills directory, then restarting the app or
reopening **Skills** and enabling the toggle. The folder name must match the
`name` in the `SKILL.md` frontmatter (lowercase, hyphens).

| OS | Skills directory |
| -- | ---------------- |
| Windows | `%LOCALAPPDATA%\Synaplan\Desktop\skills\` |
| macOS | `~/Library/Application Support/com.synaplan.desktop/skills/` |
| Linux | `$XDG_DATA_HOME/synaplan-desktop/skills/` or `~/.local/share/synaplan-desktop/skills/` |

A zip/Git installer in the UI is coming soon. Many community Agent Skills assume
an unrestricted shell or network — Synaplan Desktop will refuse those. See
[`docs/SKILLS.md`](docs/SKILLS.md) for the catalog, safety rules, and
compatible ideas.

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
- **`skills/bundled/`** — the eleven zero-setup Agent Skills that ship with
  the app.

## License

Apache License 2.0 — see [`LICENSE`](LICENSE). Copyright 2026 metadist GmbH.

## Status

Preview. Pairing, chat, and local skills work. **Build from source** with the
setup scripts above. Signed public installers are not published yet.
