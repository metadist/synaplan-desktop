# Bundled skills

The skills in `skills/bundled/` ship inside Synaplan Desktop. They are copied
into the per-user skills directory on first launch. Existing copies are left
alone so local edits survive.

## License

Every bundled skill is **Apache-2.0**, written for this app. None of them is
Anthropic’s official document skill (that tree is proprietary and shells out;
we do not vendor it). Users may still install a community skill themselves
through **Skills → From a zip / From a GitHub address**.

## Catalog

Eleven skills need only **Python 3** (standard library). They write into the
out-box.

| Skill | Result |
| ----- | ------ |
| **hello-files** | Example `hello.txt` |
| **csv-insights** | Markdown profile of a CSV |
| **email-draft** | Unsent `.eml` draft |
| **web-report** | Standalone HTML page |
| **slides** | Self-contained HTML deck (no PowerPoint, no extra packages) |
| **chart** | Bar/line chart as SVG in HTML |
| **data-table** | Searchable HTML table from a CSV |
| **calendar-event** | `.ics` invite |
| **vcard** | `.vcf` contact card |
| **json-csv** | JSON ⇄ CSV |
| **invoice** | Print-ready HTML invoice |

**pptx** is also bundled. It is original to this app (Apache-2.0) and uses
`python-pptx` only. The doctor **blocks** it until `python -c "import pptx"`
succeeds. The app never runs `pip install`. Use **slides** when you want a
deck with no extra packages.

CI proves a zip-shaped `.pptx` can be written with the stdlib-only script in
`tests/fixtures/hermetic-pptx/` (no LibreOffice, no `python-pptx` on the
runner). The bundled skill stays blocked on those images.

## Safety

- `{program, args[]}` only — no shell, no network.
- Writes stay in the out-box.
- A blocked skill stays visible on the Skills page and is never offered to
  the model.
