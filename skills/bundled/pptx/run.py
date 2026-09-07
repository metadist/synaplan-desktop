#!/usr/bin/env python3
"""Create a minimal .pptx deck. Requires the python-pptx package."""

from __future__ import annotations

import sys
from pathlib import Path


def main() -> int:
    if len(sys.argv) < 4:
        print(
            "usage: run.py OUT.pptx TITLE SLIDE1_TITLE SLIDE1_BODY [SLIDE2_TITLE SLIDE2_BODY ...]",
            file=sys.stderr,
        )
        return 2
    try:
        from pptx import Presentation
        from pptx.util import Inches, Pt
    except ImportError:
        print(
            "python-pptx is not installed. Install it yourself (pip install python-pptx). "
            "Synaplan Desktop does not run pip.",
            file=sys.stderr,
        )
        return 3

    out = Path(sys.argv[1])
    title = sys.argv[2]
    rest = sys.argv[3:]
    pairs: list[tuple[str, str]] = []
    i = 0
    while i < len(rest):
        slide_title = rest[i]
        body = rest[i + 1] if i + 1 < len(rest) else ""
        pairs.append((slide_title, body))
        i += 2

    pres = Presentation()
    pres.slide_width = Inches(13.333)
    pres.slide_height = Inches(7.5)

    title_layout = pres.slide_layouts[0]
    slide = pres.slides.add_slide(title_layout)
    slide.shapes.title.text = title
    if slide.placeholders and len(slide.placeholders) > 1:
        slide.placeholders[1].text = "Created with Synaplan Desktop"

    body_layout = pres.slide_layouts[1]
    for slide_title, body in pairs:
        s = pres.slides.add_slide(body_layout)
        s.shapes.title.text = slide_title
        if s.placeholders and len(s.placeholders) > 1:
            tf = s.placeholders[1].text_frame
            tf.clear()
            p = tf.paragraphs[0]
            p.text = body
            p.font.size = Pt(20)

    out.parent.mkdir(parents=True, exist_ok=True)
    pres.save(str(out))
    print(out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
