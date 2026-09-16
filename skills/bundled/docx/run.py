#!/usr/bin/env python3
"""docx — write a real Word document (.docx) from Markdown, or read a .docx back
to Markdown. Pure standard library (zipfile + XML), no python-docx, no Word.

Usage:
    python3 run.py write <input.md>   <output.docx>
    python3 run.py read  <input.docx> <output.md>

Markdown supported on write:
    # Title            -> document title (first H1 only)
    ## / ### Heading   -> Heading 1 / Heading 2
    plain paragraphs, **bold**, *italic*, `code`
    - bullet / 1. numbered lists
    | a | b | tables (first row = header)
    ![caption](image.png|.jpg)   -> embedded picture (path relative to the .md)
    ```chart ... ```   -> a native, editable Word chart. Block body:
        type: bar | line | pie
        title: Chart title
        categories: A, B, C
        series: Name: 1, 2, 3        (one line per series)
    ---                -> page break
"""
from __future__ import annotations

import re
import struct
import sys
import zipfile
import zlib
from pathlib import Path
from xml.etree import ElementTree as ET
from xml.sax.saxutils import escape

EMU_PER_CM = 360000
PAGE_TEXT_WIDTH_CM = 16.0  # A4 minus 2.5 cm margins
PALETTE = ["5B8CFF", "A55BFF", "22C55E", "FF8A5B", "F2C14E", "4ECDC4"]

NS_W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
NS_R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"

CONTENT_TYPES = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Default Extension="png" ContentType="image/png"/>
<Default Extension="jpeg" ContentType="image/jpeg"/>
<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
<Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
<Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/>
{chart_overrides}</Types>
"""

ROOT_RELS = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>
"""

STYLES = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Calibri" w:hAnsi="Calibri" w:cs="Calibri"/><w:sz w:val="22"/><w:lang w:val="en-US"/></w:rPr></w:rPrDefault>
<w:pPrDefault><w:pPr><w:spacing w:after="160" w:line="276" w:lineRule="auto"/></w:pPr></w:pPrDefault></w:docDefaults>
<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style>
<w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/><w:basedOn w:val="Normal"/><w:qFormat/><w:pPr><w:spacing w:after="240"/></w:pPr><w:rPr><w:sz w:val="52"/><w:b/><w:color w:val="1F3864"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Subtitle"><w:name w:val="Subtitle"/><w:basedOn w:val="Normal"/><w:qFormat/><w:rPr><w:sz w:val="26"/><w:color w:val="5B6577"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:before="360" w:after="120"/><w:outlineLvl w:val="0"/></w:pPr><w:rPr><w:sz w:val="32"/><w:b/><w:color w:val="2F5496"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:basedOn w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:before="240" w:after="80"/><w:outlineLvl w:val="1"/></w:pPr><w:rPr><w:sz w:val="26"/><w:b/><w:color w:val="2F5496"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="Heading3"><w:name w:val="heading 3"/><w:basedOn w:val="Normal"/><w:qFormat/><w:pPr><w:keepNext/><w:spacing w:before="200" w:after="60"/><w:outlineLvl w:val="2"/></w:pPr><w:rPr><w:sz w:val="24"/><w:b/><w:color w:val="1F3864"/></w:rPr></w:style>
<w:style w:type="paragraph" w:styleId="ListBullet"><w:name w:val="List Bullet"/><w:basedOn w:val="Normal"/><w:pPr><w:numPr><w:numId w:val="1"/></w:numPr><w:spacing w:after="60"/></w:pPr></w:style>
<w:style w:type="paragraph" w:styleId="ListNumber"><w:name w:val="List Number"/><w:basedOn w:val="Normal"/><w:pPr><w:numPr><w:numId w:val="2"/></w:numPr><w:spacing w:after="60"/></w:pPr></w:style>
<w:style w:type="paragraph" w:styleId="Caption"><w:name w:val="caption"/><w:basedOn w:val="Normal"/><w:pPr><w:jc w:val="center"/></w:pPr><w:rPr><w:i/><w:sz w:val="18"/><w:color w:val="5B6577"/></w:rPr></w:style>
<w:style w:type="table" w:default="1" w:styleId="TableNormal"><w:name w:val="Normal Table"/><w:tblPr><w:tblCellMar><w:left w:w="108" w:type="dxa"/><w:right w:w="108" w:type="dxa"/></w:tblCellMar></w:tblPr></w:style>
<w:style w:type="table" w:styleId="TableGrid"><w:name w:val="Table Grid"/><w:basedOn w:val="TableNormal"/><w:tblPr><w:tblBorders><w:top w:val="single" w:sz="4" w:space="0" w:color="BFBFBF"/><w:left w:val="single" w:sz="4" w:space="0" w:color="BFBFBF"/><w:bottom w:val="single" w:sz="4" w:space="0" w:color="BFBFBF"/><w:right w:val="single" w:sz="4" w:space="0" w:color="BFBFBF"/><w:insideH w:val="single" w:sz="4" w:space="0" w:color="BFBFBF"/><w:insideV w:val="single" w:sz="4" w:space="0" w:color="BFBFBF"/></w:tblBorders></w:tblPr></w:style>
<w:style w:type="character" w:styleId="Code"><w:name w:val="Code"/><w:rPr><w:rFonts w:ascii="Consolas" w:hAnsi="Consolas"/><w:sz w:val="20"/><w:shd w:val="clear" w:color="auto" w:fill="F2F4F8"/></w:rPr></w:style>
</w:styles>
"""

NUMBERING = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:abstractNum w:abstractNumId="0"><w:multiLevelType w:val="hybridMultilevel"/>
<w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="bullet"/><w:lvlText w:val="&#8226;"/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr><w:rPr><w:rFonts w:ascii="Symbol" w:hAnsi="Symbol" w:hint="default"/></w:rPr></w:lvl>
<w:lvl w:ilvl="1"><w:start w:val="1"/><w:numFmt w:val="bullet"/><w:lvlText w:val="&#8211;"/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="1440" w:hanging="360"/></w:pPr></w:lvl>
</w:abstractNum>
<w:abstractNum w:abstractNumId="1"><w:multiLevelType w:val="hybridMultilevel"/>
<w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1."/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr></w:lvl>
<w:lvl w:ilvl="1"><w:start w:val="1"/><w:numFmt w:val="lowerLetter"/><w:lvlText w:val="%2."/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="1440" w:hanging="360"/></w:pPr></w:lvl>
</w:abstractNum>
<w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num>
<w:num w:numId="2"><w:abstractNumId w:val="1"/></w:num>
</w:numbering>
"""

DOCUMENT = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture" xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
<w:body>
{body}
<w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1417" w:right="1417" w:bottom="1417" w:left="1417" w:header="708" w:footer="708" w:gutter="0"/></w:sectPr>
</w:body>
</w:document>
"""


# --------------------------------------------------------------------------- helpers


def emu_cm(cm: float) -> int:
    return int(round(cm * EMU_PER_CM))


def image_size(data: bytes) -> tuple[int, int] | None:
    """Pixel size of a PNG or JPEG from its header bytes."""
    if data[:8] == b"\x89PNG\r\n\x1a\n" and data[12:16] == b"IHDR":
        w, h = struct.unpack(">II", data[16:24])
        return w, h
    if data[:2] == b"\xff\xd8":
        i = 2
        while i + 9 < len(data):
            if data[i] != 0xFF:
                i += 1
                continue
            marker = data[i + 1]
            if marker in (0xC0, 0xC1, 0xC2):
                h, w = struct.unpack(">HH", data[i + 5 : i + 9])
                return w, h
            seg_len = struct.unpack(">H", data[i + 2 : i + 4])[0]
            i += 2 + seg_len
    return None


def inline_runs(text: str) -> str:
    """Turn **bold**, *italic* and `code` into w:r runs."""
    out: list[str] = []
    pattern = re.compile(r"(\*\*[^*]+\*\*|\*[^*]+\*|`[^`]+`)")
    for part in pattern.split(text):
        if not part:
            continue
        if part.startswith("**") and part.endswith("**"):
            out.append(run(part[2:-2], "<w:b/>"))
        elif part.startswith("`") and part.endswith("`"):
            out.append(run(part[1:-1], '<w:rStyle w:val="Code"/>'))
        elif part.startswith("*") and part.endswith("*"):
            out.append(run(part[1:-1], "<w:i/>"))
        else:
            out.append(run(part, ""))
    return "".join(out)


def run(text: str, rpr: str) -> str:
    rpr_xml = f"<w:rPr>{rpr}</w:rPr>" if rpr else ""
    return f'<w:r>{rpr_xml}<w:t xml:space="preserve">{escape(text)}</w:t></w:r>'


def paragraph(text: str, style: str = "", extra_ppr: str = "") -> str:
    ppr = ""
    if style or extra_ppr:
        style_xml = f'<w:pStyle w:val="{style}"/>' if style else ""
        ppr = f"<w:pPr>{style_xml}{extra_ppr}</w:pPr>"
    return f"<w:p>{ppr}{inline_runs(text)}</w:p>"


def table(rows: list[list[str]]) -> str:
    cols = max(len(r) for r in rows) if rows else 0
    if cols == 0:
        return ""
    width = int(PAGE_TEXT_WIDTH_CM * 567)  # twips
    col_w = width // cols
    grid = "".join(f'<w:gridCol w:w="{col_w}"/>' for _ in range(cols))
    out = [
        '<w:tbl><w:tblPr><w:tblStyle w:val="TableGrid"/>'
        f'<w:tblW w:w="{width}" w:type="dxa"/><w:tblLook w:val="04A0"/></w:tblPr>'
        f"<w:tblGrid>{grid}</w:tblGrid>"
    ]
    for ri, row in enumerate(rows):
        cells = list(row) + [""] * (cols - len(row))
        out.append("<w:tr>")
        for cell in cells:
            shade = '<w:shd w:val="clear" w:color="auto" w:fill="E8EEF8"/>' if ri == 0 else ""
            text = f"**{cell}**" if ri == 0 and cell and not cell.startswith("**") else cell
            out.append(
                f'<w:tc><w:tcPr><w:tcW w:w="{col_w}" w:type="dxa"/>{shade}</w:tcPr>'
                f'<w:p><w:pPr><w:spacing w:after="40"/></w:pPr>{inline_runs(text)}</w:p></w:tc>'
            )
        out.append("</w:tr>")
    out.append("</w:tbl>")
    out.append('<w:p><w:pPr><w:spacing w:after="120"/></w:pPr></w:p>')
    return "".join(out)


def picture(rid: str, docpr_id: int, px_w: int, px_h: int, caption: str) -> str:
    width_cm = min(PAGE_TEXT_WIDTH_CM, px_w / 96 * 2.54)
    height_cm = width_cm * px_h / px_w if px_w else width_cm * 0.6
    cx, cy = emu_cm(width_cm), emu_cm(height_cm)
    pic = (
        f'<w:p><w:pPr><w:jc w:val="center"/><w:spacing w:after="60"/></w:pPr><w:r><w:drawing>'
        f'<wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="{cx}" cy="{cy}"/>'
        f'<wp:docPr id="{docpr_id}" name="Picture {docpr_id}" descr="{escape(caption)}"/>'
        f'<a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">'
        f'<pic:pic><pic:nvPicPr><pic:cNvPr id="{docpr_id}" name="Picture {docpr_id}"/><pic:cNvPicPr/></pic:nvPicPr>'
        f'<pic:blipFill><a:blip r:embed="{rid}"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>'
        f'<pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr>'
        f"</pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>"
    )
    if caption:
        pic += paragraph(caption, "Caption")
    return pic


def chart_frame(rid: str, docpr_id: int) -> str:
    cx, cy = emu_cm(PAGE_TEXT_WIDTH_CM), emu_cm(9.0)
    return (
        f'<w:p><w:pPr><w:jc w:val="center"/></w:pPr><w:r><w:drawing>'
        f'<wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="{cx}" cy="{cy}"/>'
        f'<wp:docPr id="{docpr_id}" name="Chart {docpr_id}"/>'
        f'<a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">'
        f'<c:chart r:id="{rid}"/></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>'
    )


# --------------------------------------------------------------------------- charts


def parse_chart_block(lines: list[str]) -> dict:
    spec: dict = {"type": "bar", "title": "", "categories": [], "series": []}
    for raw in lines:
        line = raw.strip()
        if not line or ":" not in line:
            continue
        key, value = line.split(":", 1)
        key, value = key.strip().lower(), value.strip()
        if key == "type":
            spec["type"] = value.lower() if value.lower() in ("bar", "line", "pie") else "bar"
        elif key == "title":
            spec["title"] = value
        elif key == "categories":
            spec["categories"] = [c.strip() for c in value.split(",") if c.strip()]
        elif key == "series":
            name, _, nums = value.partition(":")
            values = []
            for n in nums.split(","):
                n = n.strip().replace("%", "")
                try:
                    values.append(float(n))
                except ValueError:
                    values.append(0.0)
            spec["series"].append((name.strip() or f"Series {len(spec['series']) + 1}", values))
    return spec


def chart_xml(spec: dict) -> str:
    """A native DrawingML chart with literal data (no embedded workbook needed)."""
    cats = spec["categories"] or [str(i + 1) for i in range(len(spec["series"][0][1]) if spec["series"] else 0)]
    kind = spec["type"]

    def num(v: float) -> str:
        return f"{v:g}"

    def cat_lit() -> str:
        pts = "".join(f'<c:pt idx="{i}"><c:v>{escape(c)}</c:v></c:pt>' for i, c in enumerate(cats))
        return f'<c:cat><c:strLit><c:ptCount val="{len(cats)}"/>{pts}</c:strLit></c:cat>'

    def val_lit(values: list[float]) -> str:
        pts = "".join(f'<c:pt idx="{i}"><c:v>{num(v)}</c:v></c:pt>' for i, v in enumerate(values[: len(cats)]))
        return (
            f'<c:val><c:numLit><c:formatCode>General</c:formatCode><c:ptCount val="{len(cats)}"/>{pts}</c:numLit></c:val>'
        )

    sers = []
    for i, (name, values) in enumerate(spec["series"]):
        color = PALETTE[i % len(PALETTE)]
        tx = f"<c:tx><c:v>{escape(name)}</c:v></c:tx>"
        if kind == "bar":
            sppr = f'<c:spPr><a:solidFill><a:srgbClr val="{color}"/></a:solidFill></c:spPr>'
            sers.append(
                f'<c:ser><c:idx val="{i}"/><c:order val="{i}"/>{tx}{sppr}<c:invertIfNegative val="0"/>{cat_lit()}{val_lit(values)}</c:ser>'
            )
        elif kind == "line":
            sppr = f'<c:spPr><a:ln w="28575" cap="rnd"><a:solidFill><a:srgbClr val="{color}"/></a:solidFill><a:round/></a:ln></c:spPr>'
            marker = f'<c:marker><c:symbol val="circle"/><c:size val="6"/><c:spPr><a:solidFill><a:srgbClr val="{color}"/></a:solidFill></c:spPr></c:marker>'
            sers.append(
                f'<c:ser><c:idx val="{i}"/><c:order val="{i}"/>{tx}{sppr}{marker}{cat_lit()}{val_lit(values)}<c:smooth val="0"/></c:ser>'
            )
        else:  # pie: one series, one colour per slice
            dpts = "".join(
                f'<c:dPt><c:idx val="{j}"/><c:bubble3D val="0"/><c:spPr><a:solidFill><a:srgbClr val="{PALETTE[j % len(PALETTE)]}"/></a:solidFill></c:spPr></c:dPt>'
                for j in range(len(cats))
            )
            dlbls = '<c:dLbls><c:showLegendKey val="0"/><c:showVal val="0"/><c:showCatName val="0"/><c:showSerName val="0"/><c:showPercent val="1"/><c:showBubbleSize val="0"/><c:showLeaderLines val="1"/></c:dLbls>'
            sers.append(f'<c:ser><c:idx val="{i}"/><c:order val="{i}"/>{tx}{dpts}{dlbls}{cat_lit()}{val_lit(values)}</c:ser>')
            break

    if kind == "bar":
        plot = (
            '<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/><c:varyColors val="0"/>'
            + "".join(sers)
            + '<c:gapWidth val="150"/><c:axId val="10"/><c:axId val="20"/></c:barChart>'
        )
    elif kind == "line":
        plot = (
            '<c:lineChart><c:grouping val="standard"/><c:varyColors val="0"/>'
            + "".join(sers)
            + '<c:marker val="1"/><c:axId val="10"/><c:axId val="20"/></c:lineChart>'
        )
    else:
        plot = '<c:pieChart><c:varyColors val="1"/>' + "".join(sers) + '<c:firstSliceAng val="0"/></c:pieChart>'

    axes = ""
    if kind != "pie":
        axes = (
            '<c:catAx><c:axId val="10"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/>'
            '<c:numFmt formatCode="General" sourceLinked="1"/><c:majorTickMark val="out"/><c:minorTickMark val="none"/><c:tickLblPos val="nextTo"/>'
            '<c:crossAx val="20"/><c:crosses val="autoZero"/><c:auto val="1"/><c:lblAlgn val="ctr"/><c:lblOffset val="100"/><c:noMultiLvlLbl val="0"/></c:catAx>'
            '<c:valAx><c:axId val="20"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="l"/><c:majorGridlines/>'
            '<c:numFmt formatCode="General" sourceLinked="1"/><c:majorTickMark val="out"/><c:minorTickMark val="none"/><c:tickLblPos val="nextTo"/>'
            '<c:crossAx val="10"/><c:crosses val="autoZero"/><c:crossBetween val="between"/></c:valAx>'
        )

    title = ""
    if spec["title"]:
        title = (
            '<c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr sz="1400" b="1"/></a:pPr>'
            f'<a:r><a:rPr lang="en-US" sz="1400" b="1"/><a:t>{escape(spec["title"])}</a:t></a:r></a:p></c:rich></c:tx>'
            '<c:overlay val="0"/></c:title><c:autoTitleDeleted val="0"/>'
        )
    else:
        title = '<c:autoTitleDeleted val="1"/>'

    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        '<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">'
        '<c:roundedCorners val="0"/><c:chart>'
        + title
        + "<c:plotArea><c:layout/>"
        + plot
        + axes
        + '</c:plotArea><c:legend><c:legendPos val="b"/><c:overlay val="0"/></c:legend><c:plotVisOnly val="1"/><c:dispBlanksAs val="gap"/>'
        "</c:chart></c:chartSpace>"
    )


# --------------------------------------------------------------------------- write


class DocBuilder:
    def __init__(self, base_dir: Path) -> None:
        self.base_dir = base_dir
        self.body: list[str] = []
        self.rels: list[str] = [
            '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>',
            '<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering" Target="numbering.xml"/>',
        ]
        self.media: list[tuple[str, bytes]] = []
        self.charts: list[str] = []
        self.next_rid = 3
        self.next_docpr = 1
        self.warnings: list[str] = []

    def rid(self) -> str:
        r = f"rId{self.next_rid}"
        self.next_rid += 1
        return r

    def docpr(self) -> int:
        self.next_docpr += 1
        return self.next_docpr

    def add_image(self, src: str, caption: str) -> None:
        path = Path(src)
        if not path.is_absolute():
            path = self.base_dir / path
        try:
            data = path.read_bytes()
        except OSError:
            self.warnings.append(f"image not found, skipped: {src}")
            self.body.append(paragraph(f"[missing image: {src}]", "Caption"))
            return
        size = image_size(data)
        if size is None:
            self.warnings.append(f"not a PNG/JPEG, skipped: {src}")
            self.body.append(paragraph(f"[unsupported image: {src}]", "Caption"))
            return
        ext = "png" if data[:4] == b"\x89PNG" else "jpeg"
        name = f"image{len(self.media) + 1}.{ext}"
        self.media.append((name, data))
        rid = self.rid()
        self.rels.append(
            f'<Relationship Id="{rid}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/{name}"/>'
        )
        self.body.append(picture(rid, self.docpr(), size[0], size[1], caption))

    def add_chart(self, spec: dict) -> None:
        if not spec["series"]:
            self.warnings.append("chart block without series, skipped")
            return
        self.charts.append(chart_xml(spec))
        n = len(self.charts)
        rid = self.rid()
        self.rels.append(
            f'<Relationship Id="{rid}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="charts/chart{n}.xml"/>'
        )
        self.body.append(chart_frame(rid, self.docpr()))

    def write(self, out: Path) -> None:
        overrides = "".join(
            f'<Override PartName="/word/charts/chart{i + 1}.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>\n'
            for i in range(len(self.charts))
        )
        rels = (
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
            + "".join(self.rels)
            + "</Relationships>"
        )
        out.parent.mkdir(parents=True, exist_ok=True)
        with zipfile.ZipFile(out, "w", compression=zipfile.ZIP_DEFLATED) as zf:
            zf.writestr("[Content_Types].xml", CONTENT_TYPES.format(chart_overrides=overrides))
            zf.writestr("_rels/.rels", ROOT_RELS)
            zf.writestr("word/document.xml", DOCUMENT.format(body="\n".join(self.body)))
            zf.writestr("word/styles.xml", STYLES)
            zf.writestr("word/numbering.xml", NUMBERING)
            zf.writestr("word/_rels/document.xml.rels", rels)
            for name, data in self.media:
                zf.writestr(f"word/media/{name}", data)
            for i, xml in enumerate(self.charts):
                zf.writestr(f"word/charts/chart{i + 1}.xml", xml)


def markdown_to_docx(md: str, base_dir: Path, out: Path) -> list[str]:
    doc = DocBuilder(base_dir)
    lines = md.splitlines()
    i = 0
    seen_title = False
    para: list[str] = []

    def flush_para() -> None:
        if para:
            doc.body.append(paragraph(" ".join(s.strip() for s in para)))
            para.clear()

    while i < len(lines):
        line = lines[i]
        stripped = line.strip()

        if stripped.startswith("```"):
            flush_para()
            lang = stripped[3:].strip().lower()
            block: list[str] = []
            i += 1
            while i < len(lines) and not lines[i].strip().startswith("```"):
                block.append(lines[i])
                i += 1
            i += 1
            if lang == "chart":
                doc.add_chart(parse_chart_block(block))
            else:
                code_rpr = '<w:rStyle w:val="Code"/>'
                for code_line in block:
                    doc.body.append(
                        f'<w:p><w:pPr><w:spacing w:after="0"/></w:pPr>{run(code_line, code_rpr)}</w:p>'
                    )
                doc.body.append('<w:p><w:pPr><w:spacing w:after="120"/></w:pPr></w:p>')
            continue

        if not stripped:
            flush_para()
            i += 1
            continue

        if stripped == "---":
            flush_para()
            doc.body.append('<w:p><w:r><w:br w:type="page"/></w:r></w:p>')
            i += 1
            continue

        heading = re.match(r"^(#{1,6})\s+(.*)$", stripped)
        if heading:
            flush_para()
            level, text = len(heading.group(1)), heading.group(2).strip()
            if level == 1 and not seen_title:
                doc.body.append(paragraph(text, "Title"))
                seen_title = True
            else:
                style = {1: "Heading1", 2: "Heading1", 3: "Heading2"}.get(level, "Heading3")
                doc.body.append(paragraph(text, style))
            i += 1
            continue

        image = re.match(r"^!\[([^\]]*)\]\(([^)]+)\)$", stripped)
        if image:
            flush_para()
            doc.add_image(image.group(2).strip(), image.group(1).strip())
            i += 1
            continue

        if stripped.startswith("|"):
            flush_para()
            rows: list[list[str]] = []
            while i < len(lines) and lines[i].strip().startswith("|"):
                cells = [c.strip() for c in lines[i].strip().strip("|").split("|")]
                if not all(re.fullmatch(r":?-{2,}:?", c) for c in cells if c):
                    rows.append(cells)
                i += 1
            doc.body.append(table(rows))
            continue

        bullet = re.match(r"^(\s*)[-*+]\s+(.*)$", line)
        numbered = re.match(r"^(\s*)\d+[.)]\s+(.*)$", line)
        if bullet or numbered:
            flush_para()
            m = bullet or numbered
            assert m is not None
            level = 1 if len(m.group(1).replace("\t", "    ")) >= 2 else 0
            style = "ListBullet" if bullet else "ListNumber"
            num_id = 1 if bullet else 2
            ppr = f'<w:numPr><w:ilvl w:val="{level}"/><w:numId w:val="{num_id}"/></w:numPr>'
            doc.body.append(paragraph(m.group(2).strip(), style, ppr))
            i += 1
            continue

        quote = re.match(r"^>\s?(.*)$", stripped)
        if quote:
            flush_para()
            doc.body.append(
                paragraph(quote.group(1), "", '<w:ind w:left="720"/><w:pBdr><w:left w:val="single" w:sz="12" w:space="8" w:color="BFBFBF"/></w:pBdr>')
            )
            i += 1
            continue

        para.append(line)
        i += 1

    flush_para()
    if not doc.body:
        doc.body.append(paragraph(""))
    doc.write(out)
    return doc.warnings


# --------------------------------------------------------------------------- read


def docx_to_markdown(src: Path) -> str:
    w = f"{{{NS_W}}}"
    with zipfile.ZipFile(src) as zf:
        root = ET.fromstring(zf.read("word/document.xml"))
    body = root.find(f"{w}body")
    if body is None:
        return ""
    out: list[str] = []

    def text_of(el: ET.Element) -> str:
        parts: list[str] = []
        for node in el.iter():
            if node.tag == f"{w}t" and node.text:
                parts.append(node.text)
            elif node.tag == f"{w}tab":
                parts.append("\t")
            elif node.tag == f"{w}br":
                parts.append("\n")
        return "".join(parts)

    for child in body:
        if child.tag == f"{w}p":
            text = text_of(child).strip()
            ppr = child.find(f"{w}pPr")
            style = ""
            is_list = False
            if ppr is not None:
                st = ppr.find(f"{w}pStyle")
                style = (st.get(f"{w}val") or "") if st is not None else ""
                is_list = ppr.find(f"{w}numPr") is not None
            if not text:
                out.append("")
                continue
            lower = style.lower()
            if lower == "title":
                out.append(f"# {text}")
            elif lower.startswith("heading"):
                level = int(re.sub(r"\D", "", lower) or "1")
                out.append("#" * min(level + 1, 6) + f" {text}")
            elif is_list or lower.startswith("list"):
                out.append(f"- {text}")
            else:
                out.append(text)
        elif child.tag == f"{w}tbl":
            rows = []
            for tr in child.iter(f"{w}tr"):
                rows.append([text_of(tc).strip().replace("\n", " ") for tc in tr.findall(f"{w}tc")])
            if rows:
                cols = max(len(r) for r in rows)
                out.append("| " + " | ".join(rows[0] + [""] * (cols - len(rows[0]))) + " |")
                out.append("|" + " --- |" * cols)
                for r in rows[1:]:
                    out.append("| " + " | ".join(r + [""] * (cols - len(r))) + " |")
                out.append("")
    # Collapse runs of blank lines.
    text = "\n".join(out)
    return re.sub(r"\n{3,}", "\n\n", text).strip() + "\n"


# --------------------------------------------------------------------------- main


def main() -> int:
    if len(sys.argv) != 4 or sys.argv[1] not in ("write", "read"):
        print("usage: run.py write <input.md> <output.docx> | run.py read <input.docx> <output.md>", file=sys.stderr)
        return 2
    mode, src, dst = sys.argv[1], Path(sys.argv[2]), Path(sys.argv[3])
    if not src.is_file():
        print(f"input not found: {src}", file=sys.stderr)
        return 2
    if mode == "write":
        if dst.suffix.lower() != ".docx":
            print("output must end with .docx", file=sys.stderr)
            return 2
        warnings = markdown_to_docx(src.read_text(encoding="utf-8", errors="replace"), src.parent, dst)
        for warning in warnings:
            print(f"warning: {warning}", file=sys.stderr)
    else:
        if not zipfile.is_zipfile(src):
            print("input is not a .docx file", file=sys.stderr)
            return 2
        dst.parent.mkdir(parents=True, exist_ok=True)
        dst.write_text(docx_to_markdown(src), encoding="utf-8")
    print(dst)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
