# Business use-case demos

Three recorded walks of the real Synaplan Desktop interface. Each one is a
job a person actually has on a weekday — a listing, a contract, a quote —
and shows how the desktop client gets them to a file they can open, send, or
teach from.

The pictures are not mockups. Playwright drives the Vue app in Chromium.
A scripted stand-in (`tests/demo/fake-tauri.js`) answers the Tauri commands
the app already calls, so the window, the German and English copy, the
consent dialog, the run steps and the file cards are the production UI.
The assistant text and the files are scripted: no model is called, no
customer data leaves the machine, and the same recording can be made again.

| ID | Who | Language | Film |
| -- | --- | -------- | ---- |
| [UC-D1](01-immobilien-expose.md) | Estate agent, Hamburg | German | [mp4](videos/uc-d1-immobilien-expose.mp4) |
| [UC-D2](02-contract-review.md) | Solicitor, London | English | [mp4](videos/uc-d2-contract-review.mp4) |
| [UC-D3](03-trade-quote.md) | Electrician, Reading | English | [mp4](videos/uc-d3-trade-quote.mp4) |

## What “faster” means in these three jobs

The desktop does not replace the person’s judgement. It removes the
retyping. The source material already exists as files and notes on the
computer; the skills read it there and write ordinary files into the
project’s `out/` folder.

| Job | Without the desktop | With the desktop |
| --- | ------------------- | ---------------- |
| Listing | Copy figures out of four owner documents into a Word template and a spreadsheet by hand, then write the viewing email and the calendar entry separately. | One request reads the four documents and writes the exposé and the key-figures workbook. A second writes the unsent invitation and the calendar file. |
| Contract | Read a 41-clause draft against a firm playbook, keep a risk table in a separate spreadsheet, then brief trainees and draft the client email from memory. | One request writes the memo and the risk register from the draft, the playbook and the call note. A second turns the three serious points into a trainee deck. A third writes the cover email and the call. |
| Quote | Transcribe a van note into a spreadsheet, look up every line in the price list, then type the email, the contact and the diary entry. | The dictated note and the price-list CSV become an itemised workbook and a printable quote. A second request writes the email, the contact card and the install date. |

## Honest limits, visible in the films

- **Outlook is a file, not a login.** The desktop writes `.eml`, `.ics` and
  `.vcf` that Outlook, Apple Mail and Calendar open. It does not write into
  a connected mailbox. The estate-agent reply says that in so many words.
  Putting the event into a connected Outlook calendar is a platform job;
  see the desktop/platform split in the Synaplan server docs.
- **Numbers come from the files.** The quote prices are lookups in the
  firm’s CSV. Where the source is ambiguous (underfloor heating vs the
  floor-area sheet, a hob that might need a heavier cable), the reply marks
  it instead of inventing a figure.
- **A missing page stays missing.** The solicitor’s memo says schedule 2 is
  not in the draft.
- **The first skill run asks once.** “Run skills on this computer?” is the
  real consent dialog. Allowing it is a click; declining still lets the
  turn write text files only.
- **Run steps are in English.** The tool lines (“Read docx/SKILL.md”,
  “Ran python3”) come from the app itself, in every locale. The chat
  around them follows the project language.

## Record them again

Needs Node 22+ and the JS dependencies (`npm ci`). ffmpeg is only needed
for the `.mp4` step. No Rust build, no paired account.

```bash
npm run demo:record          # Chromium, three films, a few minutes
npm run demo:videos          # .webm → H.264 .mp4 next to these docs
```

One film: `npm run demo:record -- --grep UC-D1` (or `UC-D2`, `UC-D3`).

The storyboard for each film lives next to the test:
`tests/demo/scenarios/`. Change the prompts or the scripted replies there;
the docs in this folder describe the same walk in prose.
