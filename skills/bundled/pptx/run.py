#!/usr/bin/env python3
"""pptx — write a real PowerPoint deck (.pptx) from a Markdown outline, or read
a .pptx back to Markdown. Pure standard library (zipfile + XML): no python-pptx,
no PowerPoint.

Usage:
    python3 run.py write <outline.md> <output.pptx>
    python3 run.py read  <input.pptx> <output.md>

Outline for write:
    # Deck title                 (first H1; the next paragraph is the subtitle)
    ## Slide title
    - bullet                     (up to ~8 per slide; "  - " makes a sub-bullet)
    ![](figure.png)              picture on the right half (path relative to the .md)
    ```chart ... ```             native chart on the right half (same block as docx:
                                 type/title/categories/series lines)
    | a | b |                    Markdown table -> native PowerPoint table
    Plain lines under a slide heading become short body text.
"""
from __future__ import annotations

import re
import struct
import sys
import zipfile
from pathlib import Path
from xml.etree import ElementTree as ET
from xml.sax.saxutils import escape

# 16:9, EMU
SLIDE_W, SLIDE_H = 12192000, 6858000
MARGIN = 609600  # 0.667 in
PALETTE = ["5B8CFF", "A55BFF", "22C55E", "FF8A5B", "F2C14E", "4ECDC4"]
ACCENT = "2F5496"
INK = "1A2230"
MUTED = "5B6577"

NS_A = "http://schemas.openxmlformats.org/drawingml/2006/main"
NS_P = "http://schemas.openxmlformats.org/presentationml/2006/main"
NS_R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
NS_PKG = "http://schemas.openxmlformats.org/package/2006/relationships"

XMLNS = f'xmlns:a="{NS_A}" xmlns:r="{NS_R}" xmlns:p="{NS_P}"'

CONTENT_TYPES = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Default Extension="png" ContentType="image/png"/>
<Default Extension="jpeg" ContentType="image/jpeg"/>
<Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
<Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
<Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
<Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
{overrides}</Types>
"""

ROOT_RELS = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="{NS_PKG}">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>
"""

THEME = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="{NS_A}" name="Synaplan"><a:themeElements>
<a:clrScheme name="Synaplan"><a:dk1><a:srgbClr val="{INK}"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="{ACCENT}"/></a:dk2><a:lt2><a:srgbClr val="F2F4F8"/></a:lt2>
<a:accent1><a:srgbClr val="{PALETTE[0]}"/></a:accent1><a:accent2><a:srgbClr val="{PALETTE[1]}"/></a:accent2><a:accent3><a:srgbClr val="{PALETTE[2]}"/></a:accent3><a:accent4><a:srgbClr val="{PALETTE[3]}"/></a:accent4><a:accent5><a:srgbClr val="{PALETTE[4]}"/></a:accent5><a:accent6><a:srgbClr val="{PALETTE[5]}"/></a:accent6>
<a:hlink><a:srgbClr val="0563C1"/></a:hlink><a:folHlink><a:srgbClr val="954F72"/></a:folHlink></a:clrScheme>
<a:fontScheme name="Synaplan"><a:majorFont><a:latin typeface="Calibri Light"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/><a:ea typeface=""/><a:cs typeface=""/></a:minorFont></a:fontScheme>
<a:fmtScheme name="Synaplan">
<a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:fillStyleLst>
<a:lnStyleLst><a:ln w="6350"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="12700"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="19050"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln></a:lnStyleLst>
<a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst>
<a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:bgFillStyleLst>
</a:fmtScheme></a:themeElements><a:objectDefaults/><a:extraClrSchemeLst/></a:theme>
"""

SLIDE_MASTER = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster {XMLNS}><p:cSld><p:bg><p:bgPr><a:solidFill><a:schemeClr val="lt1"/></a:solidFill><a:effectLst/></p:bgPr></p:bg><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr></p:spTree></p:cSld>
<p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
<p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/></p:sldLayoutIdLst>
<p:txStyles><p:titleStyle><a:lvl1pPr><a:defRPr sz="3600"/></a:lvl1pPr></p:titleStyle><p:bodyStyle><a:lvl1pPr><a:defRPr sz="2000"/></a:lvl1pPr></p:bodyStyle><p:otherStyle><a:lvl1pPr><a:defRPr sz="1800"/></a:lvl1pPr></p:otherStyle></p:txStyles>
</p:sldMaster>
"""

SLIDE_MASTER_RELS = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="{NS_PKG}">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/>
</Relationships>
"""

SLIDE_LAYOUT = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout {XMLNS} type="blank" preserve="1"><p:cSld name="Blank"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>
"""

SLIDE_LAYOUT_RELS = f"""<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="{NS_PKG}">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>
</Relationships>
"""


# --------------------------------------------------------------------------- helpers


def image_size(data: bytes) -> tuple[int, int] | None:
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
            i += 2 + struct.unpack(">H", data[i + 2 : i + 4])[0]
    return None


def strip_inline(text: str) -> tuple[str, bool]:
    """Drop Markdown emphasis markers; report whether the whole run was bold."""
    bold = text.startswith("**") and text.endswith("**") and len(text) > 4
    return re.sub(r"(\*\*|__|`)", "", text).replace("*", ""), bold


def rpr(size: int, color: str = INK, bold: bool = False) -> str:
    b = ' b="1"' if bold else ""
    return f'<a:rPr lang="en-US" sz="{size}"{b} dirty="0"><a:solidFill><a:srgbClr val="{color}"/></a:solidFill></a:rPr>'


def textbox(shape_id: int, name: str, x: int, y: int, w: int, h: int, paragraphs: list[str], anchor: str = "t") -> str:
    return (
        f'<p:sp><p:nvSpPr><p:cNvPr id="{shape_id}" name="{escape(name)}"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>'
        f'<p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{w}" cy="{h}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/></p:spPr>'
        f'<p:txBody><a:bodyPr wrap="square" anchor="{anchor}" lIns="0" rIns="0"><a:normAutofit/></a:bodyPr><a:lstStyle/>{"".join(paragraphs)}</p:txBody></p:sp>'
    )


def para(text: str, size: int, color: str = INK, bold: bool = False, align: str = "l", bullet: int | None = None, space_after: int = 600) -> str:
    ppr_attrs = f' algn="{align}"'
    bu = ""
    if bullet is not None:
        indent = 342900 * (bullet + 1)
        ppr_attrs += f' marL="{indent}" indent="-285750"'
        bu = f'<a:buClr><a:srgbClr val="{ACCENT}"/></a:buClr><a:buSzPct val="100000"/><a:buFont typeface="Arial"/><a:buChar char="{"•" if bullet == 0 else "–"}"/>'
    else:
        bu = "<a:buNone/>"
    return (
        f"<a:p><a:pPr{ppr_attrs}><a:spcAft><a:spcPts val=\"{space_after}\"/></a:spcAft>{bu}</a:pPr>"
        f'<a:r>{rpr(size, color, bold)}<a:t>{escape(text)}</a:t></a:r></a:p>'
    )


def picture(shape_id: int, rid: str, x: int, y: int, box_w: int, box_h: int, px_w: int, px_h: int) -> str:
    # Fit the image into the box, keep aspect ratio, centre it.
    ratio = min(box_w / px_w, box_h / px_h) if px_w and px_h else 1
    w, h = int(px_w * ratio), int(px_h * ratio)
    ox, oy = x + (box_w - w) // 2, y + (box_h - h) // 2
    return (
        f'<p:pic><p:nvPicPr><p:cNvPr id="{shape_id}" name="Picture {shape_id}"/><p:cNvPicPr><a:picLocks noChangeAspect="1"/></p:cNvPicPr><p:nvPr/></p:nvPicPr>'
        f'<p:blipFill><a:blip r:embed="{rid}"/><a:stretch><a:fillRect/></a:stretch></p:blipFill>'
        f'<p:spPr><a:xfrm><a:off x="{ox}" y="{oy}"/><a:ext cx="{w}" cy="{h}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr></p:pic>'
    )


def table_frame(shape_id: int, rows: list[list[str]], x: int, y: int, w: int, max_h: int) -> tuple[str, int]:
    """A native table; returns the XML and the height it uses."""
    cols = max(len(r) for r in rows)
    row_h = min(370840, max_h // max(len(rows), 1))
    col_w = w // cols
    size = 1200 if cols <= 4 else 1000 if cols <= 6 else 900
    grid = "".join(f'<a:gridCol w="{col_w}"/>' for _ in range(cols))
    border = "".join(
        f'<a:{side} w="6350"><a:solidFill><a:srgbClr val="BFBFBF"/></a:solidFill></a:{side}>'
        for side in ("lnL", "lnR", "lnT", "lnB")
    )
    trs = []
    for ri, row in enumerate(rows):
        cells = list(row) + [""] * (cols - len(row))
        tcs = []
        for cell in cells:
            text, _ = strip_inline(cell)
            if ri == 0:
                run_pr = f'<a:rPr lang="en-US" sz="{size}" b="1"><a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill></a:rPr>'
                fill = f'<a:solidFill><a:srgbClr val="{ACCENT}"/></a:solidFill>'
            else:
                run_pr = f'<a:rPr lang="en-US" sz="{size}"><a:solidFill><a:srgbClr val="{INK}"/></a:solidFill></a:rPr>'
                fill = f'<a:solidFill><a:srgbClr val="{"F2F4F8" if ri % 2 == 0 else "FFFFFF"}"/></a:solidFill>'
            tcs.append(
                f'<a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r>{run_pr}<a:t>{escape(text)}</a:t></a:r></a:p></a:txBody>'
                f'<a:tcPr marL="72000" marR="72000" marT="36000" marB="36000">{border}{fill}</a:tcPr></a:tc>'
            )
        trs.append(f'<a:tr h="{row_h}">{"".join(tcs)}</a:tr>')
    xml = (
        f'<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="{shape_id}" name="Table {shape_id}"/><p:cNvGraphicFramePr><a:graphicFrameLocks noGrp="1"/></p:cNvGraphicFramePr><p:nvPr/></p:nvGraphicFramePr>'
        f'<p:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{col_w * cols}" cy="{row_h * len(rows)}"/></p:xfrm>'
        f'<a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">'
        f'<a:tbl><a:tblPr firstRow="1" bandRow="1"/><a:tblGrid>{grid}</a:tblGrid>{"".join(trs)}</a:tbl>'
        f"</a:graphicData></a:graphic></p:graphicFrame>"
    )
    return xml, row_h * len(rows)


def chart_frame(shape_id: int, rid: str, x: int, y: int, w: int, h: int) -> str:
    return (
        f'<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="{shape_id}" name="Chart {shape_id}"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>'
        f'<p:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{w}" cy="{h}"/></p:xfrm>'
        f'<a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">'
        f'<c:chart xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" r:id="{rid}"/></a:graphicData></a:graphic></p:graphicFrame>'
    )


# --------------------------------------------------------------------------- charts (same block syntax as the docx skill)


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
                try:
                    values.append(float(n.strip().replace("%", "")))
                except ValueError:
                    values.append(0.0)
            spec["series"].append((name.strip() or f"Series {len(spec['series']) + 1}", values))
    return spec


def chart_xml(spec: dict) -> str:
    cats = spec["categories"] or [str(i + 1) for i in range(len(spec["series"][0][1]) if spec["series"] else 0)]
    kind = spec["type"]

    def cat_lit() -> str:
        pts = "".join(f'<c:pt idx="{i}"><c:v>{escape(c)}</c:v></c:pt>' for i, c in enumerate(cats))
        return f'<c:cat><c:strLit><c:ptCount val="{len(cats)}"/>{pts}</c:strLit></c:cat>'

    def val_lit(values: list[float]) -> str:
        pts = "".join(f'<c:pt idx="{i}"><c:v>{v:g}</c:v></c:pt>' for i, v in enumerate(values[: len(cats)]))
        return f'<c:val><c:numLit><c:formatCode>General</c:formatCode><c:ptCount val="{len(cats)}"/>{pts}</c:numLit></c:val>'

    sers = []
    for i, (name, values) in enumerate(spec["series"]):
        color = PALETTE[i % len(PALETTE)]
        tx = f"<c:tx><c:v>{escape(name)}</c:v></c:tx>"
        if kind == "bar":
            sers.append(
                f'<c:ser><c:idx val="{i}"/><c:order val="{i}"/>{tx}<c:spPr><a:solidFill><a:srgbClr val="{color}"/></a:solidFill></c:spPr>'
                f'<c:invertIfNegative val="0"/>{cat_lit()}{val_lit(values)}</c:ser>'
            )
        elif kind == "line":
            sers.append(
                f'<c:ser><c:idx val="{i}"/><c:order val="{i}"/>{tx}<c:spPr><a:ln w="28575" cap="rnd"><a:solidFill><a:srgbClr val="{color}"/></a:solidFill><a:round/></a:ln></c:spPr>'
                f'<c:marker><c:symbol val="circle"/><c:size val="6"/><c:spPr><a:solidFill><a:srgbClr val="{color}"/></a:solidFill></c:spPr></c:marker>'
                f'{cat_lit()}{val_lit(values)}<c:smooth val="0"/></c:ser>'
            )
        else:
            dpts = "".join(
                f'<c:dPt><c:idx val="{j}"/><c:bubble3D val="0"/><c:spPr><a:solidFill><a:srgbClr val="{PALETTE[j % len(PALETTE)]}"/></a:solidFill></c:spPr></c:dPt>'
                for j in range(len(cats))
            )
            dlbls = '<c:dLbls><c:showLegendKey val="0"/><c:showVal val="0"/><c:showCatName val="0"/><c:showSerName val="0"/><c:showPercent val="1"/><c:showBubbleSize val="0"/><c:showLeaderLines val="1"/></c:dLbls>'
            sers.append(f'<c:ser><c:idx val="{i}"/><c:order val="{i}"/>{tx}{dpts}{dlbls}{cat_lit()}{val_lit(values)}</c:ser>')
            break

    if kind == "bar":
        plot = '<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/><c:varyColors val="0"/>' + "".join(sers) + '<c:gapWidth val="150"/><c:axId val="10"/><c:axId val="20"/></c:barChart>'
    elif kind == "line":
        plot = '<c:lineChart><c:grouping val="standard"/><c:varyColors val="0"/>' + "".join(sers) + '<c:marker val="1"/><c:axId val="10"/><c:axId val="20"/></c:lineChart>'
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
    if spec["title"]:
        title = (
            '<c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr sz="1400" b="1"/></a:pPr>'
            f'<a:r><a:rPr lang="en-US" sz="1400" b="1"/><a:t>{escape(spec["title"])}</a:t></a:r></a:p></c:rich></c:tx><c:overlay val="0"/></c:title><c:autoTitleDeleted val="0"/>'
        )
    else:
        title = '<c:autoTitleDeleted val="1"/>'
    return (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
        f'<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="{NS_A}" xmlns:r="{NS_R}">'
        '<c:roundedCorners val="0"/><c:chart>' + title + "<c:plotArea><c:layout/>" + plot + axes
        + '</c:plotArea><c:legend><c:legendPos val="b"/><c:overlay val="0"/></c:legend><c:plotVisOnly val="1"/><c:dispBlanksAs val="gap"/></c:chart></c:chartSpace>'
    )


# --------------------------------------------------------------------------- outline → slides


class Slide:
    def __init__(self, title: str) -> None:
        self.title = title
        self.bullets: list[tuple[int, str]] = []
        self.image: str | None = None
        self.chart: dict | None = None
        self.table: list[list[str]] = []
        self.body: list[str] = []  # plain paragraphs


def parse_outline(md: str) -> tuple[str, str, list[Slide]]:
    deck_title, subtitle = "", ""
    slides: list[Slide] = []
    lines = md.splitlines()
    i = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()
        if stripped.startswith("```"):
            lang = stripped[3:].strip().lower()
            block: list[str] = []
            i += 1
            while i < len(lines) and not lines[i].strip().startswith("```"):
                block.append(lines[i])
                i += 1
            i += 1
            if lang == "chart" and slides:
                slides[-1].chart = parse_chart_block(block)
            continue
        heading = re.match(r"^(#{1,6})\s+(.*)$", stripped)
        if heading:
            level, text = len(heading.group(1)), heading.group(2).strip()
            if level == 1 and not deck_title:
                deck_title = text
            elif deck_title and not slides and re.fullmatch(r"(?i)title\s*slide|titelfolie", text):
                # The deck title already makes the title slide; a slide literally
                # named "Title slide" would only duplicate it.
                pass
            else:
                slides.append(Slide(text))
            i += 1
            continue
        if stripped.startswith("|") and slides:
            cells = [c.strip() for c in stripped.strip("|").split("|")]
            if not all(re.fullmatch(r":?-{2,}:?", c) for c in cells if c):
                slides[-1].table.append(cells)
            i += 1
            continue
        if not stripped:
            i += 1
            continue
        if not slides:
            if deck_title and not subtitle:
                subtitle = stripped
            i += 1
            continue
        slide = slides[-1]
        image = re.match(r"^!\[[^\]]*\]\(([^)]+)\)$", stripped)
        if image:
            slide.image = image.group(1).strip()
        else:
            bullet = re.match(r"^(\s*)[-*+]\s+(.*)$", line) or re.match(r"^(\s*)\d+[.)]\s+(.*)$", line)
            if bullet:
                level = 1 if len(bullet.group(1).replace("\t", "    ")) >= 2 else 0
                slide.bullets.append((level, bullet.group(2).strip()))
            else:
                slide.body.append(stripped)
        i += 1
    if not deck_title and slides:
        deck_title = slides[0].title
    return deck_title, subtitle, slides


class DeckBuilder:
    def __init__(self, base_dir: Path) -> None:
        self.base_dir = base_dir
        self.slides_xml: list[str] = []
        self.slide_rels: list[list[str]] = []
        self.media: list[tuple[str, bytes]] = []
        self.charts: list[str] = []
        self.warnings: list[str] = []

    def title_slide(self, title: str, subtitle: str) -> None:
        shapes = [
            f'<p:sp><p:nvSpPr><p:cNvPr id="2" name="Accent"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{SLIDE_W}" cy="{int(SLIDE_H * 0.62)}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="{ACCENT}"/></a:solidFill><a:ln><a:noFill/></a:ln></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:endParaRPr lang="en-US"/></a:p></p:txBody></p:sp>',
            textbox(3, "Title", MARGIN, int(SLIDE_H * 0.18), SLIDE_W - 2 * MARGIN, int(SLIDE_H * 0.36), [para(title, 4400, "FFFFFF", True, space_after=0)], anchor="b"),
            textbox(4, "Subtitle", MARGIN, int(SLIDE_H * 0.66), SLIDE_W - 2 * MARGIN, int(SLIDE_H * 0.2), [para(subtitle or " ", 2200, MUTED, space_after=0)]),
        ]
        self._add_slide(shapes, [])

    def content_slide(self, slide: Slide) -> None:
        rels: list[str] = ['<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>']
        shapes: list[str] = []
        title_h = int(SLIDE_H * 0.17)
        shapes.append(textbox(2, "Title", MARGIN, int(SLIDE_H * 0.06), SLIDE_W - 2 * MARGIN, title_h, [para(slide.title, 3200, ACCENT, True, space_after=0)], anchor="b"))
        shapes.append(
            f'<p:cxnSp><p:nvCxnSpPr><p:cNvPr id="3" name="Rule"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr><p:spPr><a:xfrm><a:off x="{MARGIN}" y="{int(SLIDE_H * 0.06) + title_h + 60000}"/><a:ext cx="{SLIDE_W - 2 * MARGIN}" cy="0"/></a:xfrm><a:prstGeom prst="line"><a:avLst/></a:prstGeom><a:ln w="19050"><a:solidFill><a:srgbClr val="{PALETTE[0]}"/></a:solidFill></a:ln></p:spPr></p:cxnSp>'
        )
        body_y = int(SLIDE_H * 0.06) + title_h + 260000
        body_h = SLIDE_H - body_y - MARGIN
        has_visual = bool(slide.image or slide.chart)
        body_w = (SLIDE_W - 2 * MARGIN) // 2 - 200000 if has_visual else SLIDE_W - 2 * MARGIN

        paragraphs: list[str] = []
        n_items = len(slide.bullets) + len(slide.body)
        size = 2000 if n_items <= 6 else 1600 if n_items <= 9 else 1400
        for text in slide.body:
            clean, bold = strip_inline(text)
            paragraphs.append(para(clean, size, INK, bold, space_after=800))
        for level, text in slide.bullets:
            clean, bold = strip_inline(text)
            paragraphs.append(para(clean, size if level == 0 else size - 200, INK, bold, bullet=level, space_after=700))
        text_h = body_h
        shape_id = 5
        if slide.table:
            # Bullets keep the top; the table takes the rest of the body area.
            text_h = int(body_h * 0.3) if paragraphs else 0
            table_xml, _ = table_frame(shape_id, slide.table, MARGIN, body_y + text_h, body_w, body_h - text_h)
            shapes.append(table_xml)
            shape_id += 1
        if not paragraphs:
            paragraphs.append(para(" ", size, space_after=0))
        if text_h > 0:
            shapes.append(textbox(4, "Body", MARGIN, body_y, body_w, text_h, paragraphs))

        if has_visual:
            vx = MARGIN + body_w + 400000
            vw = SLIDE_W - MARGIN - vx
            if slide.chart and slide.chart["series"]:
                self.charts.append(chart_xml(slide.chart))
                rid = f"rId{len(rels) + 1}"
                rels.append(f'<Relationship Id="{rid}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart{len(self.charts)}.xml"/>')
                shapes.append(chart_frame(shape_id, rid, vx, body_y, vw, body_h))
                shape_id += 1
            elif slide.image:
                path = Path(slide.image)
                if not path.is_absolute():
                    path = self.base_dir / path
                data = path.read_bytes() if path.is_file() else b""
                size_px = image_size(data) if data else None
                if size_px:
                    ext = "png" if data[:4] == b"\x89PNG" else "jpeg"
                    name = f"image{len(self.media) + 1}.{ext}"
                    self.media.append((name, data))
                    rid = f"rId{len(rels) + 1}"
                    rels.append(f'<Relationship Id="{rid}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/{name}"/>')
                    shapes.append(picture(shape_id, rid, vx, body_y, vw, body_h, size_px[0], size_px[1]))
                    shape_id += 1
                else:
                    self.warnings.append(f"image not found or not PNG/JPEG, skipped: {slide.image}")
        self._add_slide(shapes, rels[1:])

    def _add_slide(self, shapes: list[str], extra_rels: list[str]) -> None:
        tree = (
            '<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>'
            '<p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>'
        )
        xml = (
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            f"<p:sld {XMLNS}><p:cSld><p:spTree>{tree}{''.join(shapes)}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>"
        )
        self.slides_xml.append(xml)
        rels = ['<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>']
        rels.extend(extra_rels)
        self.slide_rels.append(rels)

    def write(self, out: Path) -> None:
        n = len(self.slides_xml)
        overrides = "".join(
            f'<Override PartName="/ppt/slides/slide{i + 1}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>\n'
            for i in range(n)
        ) + "".join(
            f'<Override PartName="/ppt/charts/chart{i + 1}.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>\n'
            for i in range(len(self.charts))
        )
        sld_ids = "".join(f'<p:sldId id="{256 + i}" r:id="rId{i + 2}"/>' for i in range(n))
        presentation = (
            '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
            f'<p:presentation {XMLNS} saveSubsetFonts="1">'
            '<p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst>'
            f"<p:sldIdLst>{sld_ids}</p:sldIdLst>"
            f'<p:sldSz cx="{SLIDE_W}" cy="{SLIDE_H}"/><p:notesSz cx="6858000" cy="9144000"/>'
            '<p:defaultTextStyle><a:defPPr><a:defRPr lang="en-US"/></a:defPPr></p:defaultTextStyle>'
            "</p:presentation>"
        )
        pres_rels = ['<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>']
        pres_rels += [
            f'<Relationship Id="rId{i + 2}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{i + 1}.xml"/>'
            for i in range(n)
        ]
        pres_rels.append(f'<Relationship Id="rId{n + 2}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>')

        def rels_doc(items: list[str]) -> str:
            return f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="{NS_PKG}">{"".join(items)}</Relationships>'

        out.parent.mkdir(parents=True, exist_ok=True)
        with zipfile.ZipFile(out, "w", compression=zipfile.ZIP_DEFLATED) as zf:
            zf.writestr("[Content_Types].xml", CONTENT_TYPES.format(overrides=overrides))
            zf.writestr("_rels/.rels", ROOT_RELS)
            zf.writestr("ppt/presentation.xml", presentation)
            zf.writestr("ppt/_rels/presentation.xml.rels", rels_doc(pres_rels))
            zf.writestr("ppt/slideMasters/slideMaster1.xml", SLIDE_MASTER)
            zf.writestr("ppt/slideMasters/_rels/slideMaster1.xml.rels", SLIDE_MASTER_RELS)
            zf.writestr("ppt/slideLayouts/slideLayout1.xml", SLIDE_LAYOUT)
            zf.writestr("ppt/slideLayouts/_rels/slideLayout1.xml.rels", SLIDE_LAYOUT_RELS)
            zf.writestr("ppt/theme/theme1.xml", THEME)
            for i, xml in enumerate(self.slides_xml):
                zf.writestr(f"ppt/slides/slide{i + 1}.xml", xml)
                zf.writestr(f"ppt/slides/_rels/slide{i + 1}.xml.rels", rels_doc(self.slide_rels[i]))
            for name, data in self.media:
                zf.writestr(f"ppt/media/{name}", data)
            for i, xml in enumerate(self.charts):
                zf.writestr(f"ppt/charts/chart{i + 1}.xml", xml)


def outline_to_pptx(md: str, base_dir: Path, out: Path) -> list[str]:
    deck_title, subtitle, slides = parse_outline(md)
    if not deck_title and not slides:
        raise ValueError("the outline has no '# Deck title' and no '## Slide' headings")
    deck = DeckBuilder(base_dir)
    deck.title_slide(deck_title, subtitle)
    for slide in slides:
        deck.content_slide(slide)
    deck.write(out)
    return deck.warnings


# --------------------------------------------------------------------------- read


def pptx_to_markdown(src: Path) -> str:
    a = f"{{{NS_A}}}"
    with zipfile.ZipFile(src) as zf:
        names = set(zf.namelist())
        pres = ET.fromstring(zf.read("ppt/presentation.xml"))
        rels = ET.fromstring(zf.read("ppt/_rels/presentation.xml.rels"))
        targets = {r.get("Id"): r.get("Target") for r in rels}
        order = []
        for sld in pres.iter(f"{{{NS_P}}}sldId"):
            target = targets.get(sld.get(f"{{{NS_R}}}id")) or ""
            part = target.lstrip("/") if target.startswith("/") else f"ppt/{target}"
            if part in names:
                order.append(part)
        out: list[str] = []
        for n, part in enumerate(order, start=1):
            root = ET.fromstring(zf.read(part))
            shapes_text: list[list[str]] = []
            for sp in root.iter(f"{{{NS_P}}}sp"):
                paras = []
                for p in sp.iter(f"{a}p"):
                    text = "".join(t.text or "" for t in p.iter(f"{a}t")).strip()
                    if text:
                        paras.append(text)
                if paras:
                    shapes_text.append(paras)
            if not shapes_text:
                out.append(f"## Slide {n}\n")
                continue
            title = shapes_text[0][0]
            out.append(f"## {title}" if n > 1 else f"# {title}")
            rest = shapes_text[0][1:] + [t for shape in shapes_text[1:] for t in shape]
            for text in rest:
                out.append(f"- {text}" if n > 1 else text)
            out.append("")
    return "\n".join(out).strip() + "\n"


# --------------------------------------------------------------------------- main


def main() -> int:
    if len(sys.argv) != 4 or sys.argv[1] not in ("write", "read"):
        print("usage: run.py write <outline.md> <output.pptx> | run.py read <input.pptx> <output.md>", file=sys.stderr)
        return 2
    mode, src, dst = sys.argv[1], Path(sys.argv[2]), Path(sys.argv[3])
    if not src.is_file():
        print(f"input not found: {src}", file=sys.stderr)
        return 2
    try:
        if mode == "write":
            if dst.suffix.lower() != ".pptx":
                print("output must end with .pptx", file=sys.stderr)
                return 2
            for warning in outline_to_pptx(src.read_text(encoding="utf-8", errors="replace"), src.parent, dst):
                print(f"warning: {warning}", file=sys.stderr)
        else:
            if not zipfile.is_zipfile(src):
                print("input is not a .pptx file", file=sys.stderr)
                return 2
            dst.parent.mkdir(parents=True, exist_ok=True)
            dst.write_text(pptx_to_markdown(src), encoding="utf-8")
    except (ValueError, KeyError) as exc:
        print(f"could not process {src.name}: {exc}", file=sys.stderr)
        return 1
    print(dst)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
