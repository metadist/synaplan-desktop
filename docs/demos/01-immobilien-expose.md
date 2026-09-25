# UC-D1 — Exposé aus dem Objektordner

**Film:** [uc-d1-immobilien-expose.mp4](videos/uc-d1-immobilien-expose.mp4)
**Language of the app:** German. **Person:** Katrin Weber, estate agent,
Weber Immobilien, Hamburg.

Script: `tests/demo/scenarios/immobilien-expose.ts`.

## The job

Thursday afternoon. A seller has just handed over a flat in Rahlstedt.
Four documents are already on the laptop: floor plan, energy certificate,
floor-area workbook, owner description. The exposé has to go out tonight,
and the viewing list needs an invitation for Saturday 3 October, 11:00.

Today that is an evening of copying figures between Word and Excel, then a
separate email and a calendar entry that may not match the exposé.

## What the desktop does

Project **Objekt Lindenstraße 12**, paired with the firm’s Synaplan
workspace. Skills on for this project: `docx`, `xlsx`, `email-draft`,
`calendar-event`, `chart`. Model shown: Claude Sonnet.

1. Katrin adds the four documents to the project’s knowledge folder. Each
   one moves from “sent” to **Ready**, which is the moment chat can use it.
2. She asks, in her own words, for a Word exposé (location, facts, fittings,
   energy, price) and an Excel sheet of the key figures. Purchase price
   689,000 €.
3. The first skill run on this computer shows the consent dialog once.
   She allows it.
4. The run lists the folder, reads the skill instructions, and runs the
   bundled Python scripts. Three files appear in the chat:
   - `Expose-Lindenstrasse-12.docx`
   - `Eckdaten-Lindenstrasse-12.xlsx`
   - `expose.md` (the text the Word file was built from)
5. The reply states the figures (142.3 m², energy class C, 4,842 €/m²) and
   marks two things it will not paper over: the owner text says underfloor
   heating throughout the ground floor, the floor-area sheet does not, and
   the energy certificate expires on 14 March 2027.
6. A second request writes the viewing invitation as an unsent `.eml` and
   the appointment as an `.ics`. The reply says plainly that this computer
   cannot write the event straight into Outlook; the calendar file is the
   way in.

**Show in folder** on any card reveals the file. That is the ten-second
path from “the assistant made this” to the file itself.

## Why this is faster

The figures are read once, from the documents she already has, into both
the exposé and the workbook. The invitation and the viewing use the same
date, so they cannot drift apart. The two conflicts stay visible instead
of being smoothed into a wrong listing.

## What this film does not claim

The `.eml` is a draft. Nobody is emailed. The `.ics` is a file she opens.
A connected Outlook calendar on the Synaplan website is a different path;
this desktop build does not take it.
