#!/usr/bin/env python3
"""xlsx — write a real Excel workbook (.xlsx) from JSON or CSV, or read a
workbook back to JSON / CSV. Pure standard library (zipfile + XML), no openpyxl,
no Excel.

Usage:
    python3 run.py write <spec.json | input.csv> <output.xlsx>
    python3 run.py read  <input.xlsx> <output.json | output.csv>

Spec for write (JSON):
    {
      "sheets": [
        {
          "name": "Summary",
          "columns": ["Region", "Q1", "Q2"],
          "rows": [["North", 120, 130.5], ["South", 90, 88]],
          "totals": true                     (optional: adds a SUM row)
        }
      ]
    }

Cells that are numbers stay numbers; a string starting with "=" is written as a
formula (e.g. "=B2*1.19"). A CSV input becomes one sheet named after the file.
Reading a .csv output takes the first sheet only; .json contains every sheet.
"""
from __future__ import annotations

import csv
import json
import re
import sys
import zipfile
from pathlib import Path
from xml.etree import ElementTree as ET
from xml.sax.saxutils import escape

NS_MAIN = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
NS_REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
NS_PKG = "http://schemas.openxmlformats.org/package/2006/relationships"

CONTENT_TYPES = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>
{sheet_overrides}</Types>
"""

ROOT_RELS = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>
"""

STYLES = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<numFmts count="1"><numFmt numFmtId="164" formatCode="#,##0.00"/></numFmts>
<fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><sz val="11"/><color rgb="FFFFFFFF"/><name val="Calibri"/></font></fonts>
<fills count="3"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill><fill><patternFill patternType="solid"><fgColor rgb="FF2F5496"/><bgColor indexed="64"/></patternFill></fill></fills>
<borders count="2"><border><left/><right/><top/><bottom/><diagonal/></border><border><left/><right/><top style="thin"><color rgb="FF2F5496"/></top><bottom style="double"><color rgb="FF2F5496"/></bottom><diagonal/></border></borders>
<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
<cellXfs count="4">
<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
<xf numFmtId="0" fontId="1" fillId="2" borderId="0" xfId="0" applyFont="1" applyFill="1"><alignment vertical="center"/></xf>
<xf numFmtId="164" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/>
<xf numFmtId="164" fontId="0" fillId="0" borderId="1" xfId="0" applyNumberFormat="1" applyBorder="1"/>
</cellXfs>
<cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>
</styleSheet>
"""

STYLE_DEFAULT, STYLE_HEADER, STYLE_NUMBER, STYLE_TOTAL = 0, 1, 2, 3


# --------------------------------------------------------------------------- helpers


def col_letter(index: int) -> str:
    """0 -> A, 25 -> Z, 26 -> AA."""
    letters = ""
    index += 1
    while index > 0:
        index, rem = divmod(index - 1, 26)
        letters = chr(65 + rem) + letters
    return letters


def col_index(ref: str) -> int:
    letters = re.match(r"[A-Z]+", ref)
    if not letters:
        return 0
    n = 0
    for ch in letters.group(0):
        n = n * 26 + (ord(ch) - 64)
    return n - 1


def as_number(value):
    if isinstance(value, bool):
        return None
    if isinstance(value, (int, float)):
        return value
    if isinstance(value, str):
        s = value.strip().replace("\u202f", "").replace(" ", "")
        if re.fullmatch(r"-?\d+", s):
            return int(s)
        if re.fullmatch(r"-?\d+\.\d+", s):
            return float(s)
        # 1.234,56 (German) -> 1234.56
        if re.fullmatch(r"-?\d{1,3}(\.\d{3})+(,\d+)?", s) or re.fullmatch(r"-?\d+,\d+", s):
            try:
                return float(s.replace(".", "").replace(",", "."))
            except ValueError:
                return None
        if re.fullmatch(r"-?\d{1,3}(,\d{3})+(\.\d+)?", s):
            try:
                return float(s.replace(",", ""))
            except ValueError:
                return None
    return None


def safe_sheet_name(name: str, taken: set[str]) -> str:
    cleaned = re.sub(r"[\[\]:*?/\\]", " ", str(name or "Sheet")).strip()[:31] or "Sheet"
    candidate, n = cleaned, 2
    while candidate.lower() in taken:
        suffix = f" ({n})"
        candidate = cleaned[: 31 - len(suffix)] + suffix
        n += 1
    taken.add(candidate.lower())
    return candidate


# --------------------------------------------------------------------------- write


def sheet_xml(columns: list, rows: list[list], totals: bool) -> str:
    all_rows: list[list] = []
    if columns:
        all_rows.append(list(columns))
    all_rows.extend(list(r) for r in rows)
    width = max((len(r) for r in all_rows), default=1)

    numeric_cols = [
        any(as_number(r[c]) is not None for r in rows if c < len(r)) and all(
            r[c] in (None, "") or as_number(r[c]) is not None for r in rows if c < len(r)
        )
        for c in range(width)
    ]

    cells_xml: list[str] = []
    first_data_row = 2 if columns else 1
    last_data_row = first_data_row + len(rows) - 1

    for ri, row in enumerate(all_rows, start=1):
        parts = []
        for ci in range(width):
            value = row[ci] if ci < len(row) else None
            ref = f"{col_letter(ci)}{ri}"
            if value is None or value == "":
                continue
            if columns and ri == 1:
                parts.append(f'<c r="{ref}" s="{STYLE_HEADER}" t="inlineStr"><is><t>{escape(str(value))}</t></is></c>')
                continue
            if isinstance(value, str) and value.startswith("="):
                parts.append(f'<c r="{ref}" s="{STYLE_NUMBER}"><f>{escape(value[1:])}</f></c>')
                continue
            number = as_number(value)
            if number is not None:
                style = STYLE_NUMBER if isinstance(number, float) else STYLE_DEFAULT
                parts.append(f'<c r="{ref}" s="{style}"><v>{number}</v></c>')
            elif isinstance(value, bool):
                parts.append(f'<c r="{ref}" t="b"><v>{1 if value else 0}</v></c>')
            else:
                parts.append(f'<c r="{ref}" t="inlineStr"><is><t xml:space="preserve">{escape(str(value))}</t></is></c>')
        cells_xml.append(f'<row r="{ri}">{"".join(parts)}</row>')

    total_row = len(all_rows) + 1
    if totals and rows:
        parts = [f'<c r="A{total_row}" s="{STYLE_TOTAL}" t="inlineStr"><is><t>Total</t></is></c>']
        for ci in range(1, width):
            if numeric_cols[ci]:
                col = col_letter(ci)
                parts.append(
                    f'<c r="{col}{total_row}" s="{STYLE_TOTAL}"><f>SUM({col}{first_data_row}:{col}{last_data_row})</f></c>'
                )
        cells_xml.append(f'<row r="{total_row}">{"".join(parts)}</row>')

    # Column widths from the longest cell text (capped).
    cols_xml = []
    for ci in range(width):
        longest = max((len(str(r[ci])) for r in all_rows if ci < len(r) and r[ci] not in (None, "")), default=8)
        cols_xml.append(f'<col min="{ci + 1}" max="{ci + 1}" width="{min(max(longest + 2, 8), 60)}" customWidth="1"/>')

    dimension = f"A1:{col_letter(width - 1)}{max(total_row if totals and rows else len(all_rows), 1)}"
    views = (
        '<sheetViews><sheetView workbookViewId="0"><pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/></sheetView></sheetViews>'
        if columns
        else '<sheetViews><sheetView workbookViewId="0"/></sheetViews>'
    )
    autofilter = f'<autoFilter ref="A1:{col_letter(width - 1)}{max(len(all_rows), 1)}"/>' if columns and rows else ""
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        f'<worksheet xmlns="{NS_MAIN}" xmlns:r="{NS_REL}">'
        f'<dimension ref="{dimension}"/>{views}<sheetFormatPr defaultRowHeight="15"/>'
        f'<cols>{"".join(cols_xml)}</cols><sheetData>{"".join(cells_xml)}</sheetData>{autofilter}'
        '<pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/>'
        "</worksheet>"
    )


def write_workbook(sheets: list[dict], out: Path) -> None:
    taken: set[str] = set()
    named = [(safe_sheet_name(s.get("name", f"Sheet{i + 1}"), taken), s) for i, s in enumerate(sheets)]
    overrides = "".join(
        f'<Override PartName="/xl/worksheets/sheet{i + 1}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>\n'
        for i in range(len(named))
    )
    sheets_xml = "".join(
        f'<sheet name="{escape(name)}" sheetId="{i + 1}" r:id="rId{i + 1}"/>' for i, (name, _) in enumerate(named)
    )
    workbook = (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        f'<workbook xmlns="{NS_MAIN}" xmlns:r="{NS_REL}"><bookViews><workbookView/></bookViews>'
        f"<sheets>{sheets_xml}</sheets></workbook>"
    )
    rels = "".join(
        f'<Relationship Id="rId{i + 1}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{i + 1}.xml"/>'
        for i in range(len(named))
    )
    rels += f'<Relationship Id="rId{len(named) + 1}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>'
    workbook_rels = f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="{NS_PKG}">{rels}</Relationships>'

    out.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(out, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        zf.writestr("[Content_Types].xml", CONTENT_TYPES.format(sheet_overrides=overrides))
        zf.writestr("_rels/.rels", ROOT_RELS)
        zf.writestr("xl/workbook.xml", workbook)
        zf.writestr("xl/_rels/workbook.xml.rels", workbook_rels)
        zf.writestr("xl/styles.xml", STYLES)
        for i, (_, spec) in enumerate(named):
            rows = [list(r) if isinstance(r, (list, tuple)) else [r] for r in spec.get("rows", [])]
            zf.writestr(
                f"xl/worksheets/sheet{i + 1}.xml",
                sheet_xml(list(spec.get("columns", [])), rows, bool(spec.get("totals", False))),
            )


def sheets_from_input(src: Path) -> list[dict]:
    if src.suffix.lower() == ".csv":
        with src.open(newline="", encoding="utf-8-sig") as fh:
            sample = fh.read(4096)
            fh.seek(0)
            try:
                dialect = csv.Sniffer().sniff(sample, delimiters=",;\t|")
            except csv.Error:
                dialect = csv.excel
            rows = [row for row in csv.reader(fh, dialect)]
        if not rows:
            raise ValueError("the CSV is empty")
        return [{"name": src.stem, "columns": rows[0], "rows": rows[1:]}]
    data = json.loads(src.read_text(encoding="utf-8"))
    if isinstance(data, list):
        # A bare list of objects -> one sheet, keys as columns.
        if data and isinstance(data[0], dict):
            columns = list(data[0].keys())
            return [{"name": "Sheet1", "columns": columns, "rows": [[row.get(c) for c in columns] for row in data]}]
        return [{"name": "Sheet1", "columns": [], "rows": data}]
    if isinstance(data, dict) and "sheets" in data:
        return list(data["sheets"])
    if isinstance(data, dict) and "rows" in data:
        return [data]
    raise ValueError('spec needs {"sheets": [...]}, {"columns": [...], "rows": [...]} or a JSON array')


# --------------------------------------------------------------------------- read


def read_workbook(src: Path) -> list[dict]:
    m = f"{{{NS_MAIN}}}"
    with zipfile.ZipFile(src) as zf:
        names = set(zf.namelist())
        shared: list[str] = []
        if "xl/sharedStrings.xml" in names:
            root = ET.fromstring(zf.read("xl/sharedStrings.xml"))
            for si in root.findall(f"{m}si"):
                shared.append("".join(t.text or "" for t in si.iter(f"{m}t")))
        rels_root = ET.fromstring(zf.read("xl/_rels/workbook.xml.rels"))
        rel_targets = {}
        for rel in rels_root:
            target = rel.get("Target") or ""
            if target.startswith("/"):
                target = target[1:]
            elif not target.startswith("xl/"):
                target = "xl/" + target
            rel_targets[rel.get("Id")] = target
        workbook = ET.fromstring(zf.read("xl/workbook.xml"))
        sheets_out = []
        for sheet in workbook.iter(f"{m}sheet"):
            rid = sheet.get(f"{{{NS_REL}}}id")
            part = rel_targets.get(rid)
            if not part or part not in names:
                continue
            ws = ET.fromstring(zf.read(part))
            rows: list[list] = []
            for row in ws.iter(f"{m}row"):
                values: dict[int, object] = {}
                for c in row.findall(f"{m}c"):
                    ref = c.get("r") or ""
                    idx = col_index(ref)
                    kind = c.get("t")
                    v = c.find(f"{m}v")
                    text = v.text if v is not None else None
                    if kind == "s" and text is not None:
                        value: object = shared[int(text)] if int(text) < len(shared) else ""
                    elif kind == "inlineStr":
                        value = "".join(t.text or "" for t in c.iter(f"{m}t"))
                    elif kind == "b":
                        value = text == "1"
                    elif kind in ("str", "e"):
                        value = text or ""
                    elif text is None:
                        # A formula without a cached result (never opened in
                        # Excel): hand back the formula so nothing is lost.
                        formula = c.find(f"{m}f")
                        value = f"={formula.text}" if formula is not None and formula.text else None
                    else:
                        try:
                            value = int(text) if re.fullmatch(r"-?\d+", text) else float(text)
                        except ValueError:
                            value = text
                    values[idx] = value
                if values:
                    width = max(values) + 1
                    rows.append([values.get(i) for i in range(width)])
                else:
                    rows.append([])
            # Trim trailing empty rows.
            while rows and not any(v not in (None, "") for v in rows[-1]):
                rows.pop()
            sheets_out.append({"name": sheet.get("name") or part, "rows": rows})
    return sheets_out


# --------------------------------------------------------------------------- main


def main() -> int:
    if len(sys.argv) != 4 or sys.argv[1] not in ("write", "read"):
        print(
            "usage: run.py write <spec.json|input.csv> <output.xlsx> | run.py read <input.xlsx> <output.json|output.csv>",
            file=sys.stderr,
        )
        return 2
    mode, src, dst = sys.argv[1], Path(sys.argv[2]), Path(sys.argv[3])
    if not src.is_file():
        print(f"input not found: {src}", file=sys.stderr)
        return 2
    try:
        if mode == "write":
            if dst.suffix.lower() != ".xlsx":
                print("output must end with .xlsx", file=sys.stderr)
                return 2
            write_workbook(sheets_from_input(src), dst)
        else:
            if not zipfile.is_zipfile(src):
                print("input is not an .xlsx file", file=sys.stderr)
                return 2
            sheets = read_workbook(src)
            dst.parent.mkdir(parents=True, exist_ok=True)
            if dst.suffix.lower() == ".csv":
                with dst.open("w", newline="", encoding="utf-8") as fh:
                    writer = csv.writer(fh)
                    for row in (sheets[0]["rows"] if sheets else []):
                        writer.writerow(["" if v is None else v for v in row])
            else:
                dst.write_text(json.dumps({"sheets": sheets}, ensure_ascii=False, indent=1), encoding="utf-8")
    except (ValueError, KeyError, json.JSONDecodeError) as exc:
        print(f"could not process {src.name}: {exc}", file=sys.stderr)
        return 1
    print(dst)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
