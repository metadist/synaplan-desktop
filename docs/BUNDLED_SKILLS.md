# Bundled skills

The skills in `skills/bundled/` ship inside Synaplan Desktop. They are copied
into the per-user skills directory on first launch. On every later launch the
app compares each bundled file with the fingerprint it seeded last time
(`.bundled-seeds.json` in the skills folder): an **untouched** copy follows
the app update, a copy the user **edited** is left alone. An install that
predates the fingerprints gets the update too, with the old file kept next to
it as `<name>.before-update`.

## License

Every bundled skill is **Apache-2.0**, written for this app. None of them is
Anthropic’s official document skill (that tree is proprietary and shells out;
we do not vendor it). Users may still install a community skill themselves
through **Skills → From a zip / From a GitHub address**.

## Catalog

All fourteen skills need only **Python 3** (standard library). They write into
the out-box. The app never runs `pip install`.

| Skill | Result |
| ----- | ------ |
| **hello-files** | Example `hello.txt` |
| **csv-insights** | Markdown profile of a CSV |
| **email-draft** | Unsent `.eml` draft |
| **web-report** | Standalone HTML page |
| **slides** | Self-contained HTML deck |
| **chart** | Bar/line chart as SVG in HTML |
| **data-table** | Searchable HTML table from a CSV |
| **calendar-event** | `.ics` invite |
| **vcard** | `.vcf` contact card |
| **json-csv** | JSON ⇄ CSV |
| **invoice** | Print-ready HTML invoice |
| **docx** | Real Word document from Markdown (headings, lists, tables, pictures, native charts); reads `.docx` back to Markdown |
| **xlsx** | Real Excel workbook from JSON/CSV (sheets, styled header, formulas, SUM row); reads `.xlsx` back to JSON/CSV |
| **pptx** | Real PowerPoint deck from a Markdown outline (title, bullets, pictures, native charts); reads `.pptx` back to Markdown |

The three Office skills build the OOXML packages themselves (`zipfile` +
XML), so they work on a plain Python install and never depend on Word, Excel
or PowerPoint being present. `chart` blocks in **docx** and **pptx** become
native DrawingML charts the user can restyle in Office. All three also
**read** their format, which is how a project can merge or analyse existing
Office files locally (read → reason → write).

CI runs each Office script for real (`office_skills_write_real_zip_packages`)
and still proves the tiny stdlib writer in `tests/fixtures/hermetic-pptx/`.

## Safety

- `{program, args[]}` only — no shell, no network.
- Writes stay in the out-box.
- A blocked skill stays visible on the Skills page and is never offered to
  the model.
