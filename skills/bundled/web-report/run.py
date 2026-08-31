#!/usr/bin/env python3
"""web-report — turn a Markdown/plain-text file into a polished, self-contained
HTML page (system fonts, light+dark, no external assets).

Usage:
    python3 run.py <input.md> <output.html> [title]

Standard library only. Supports #/##/### headings, **bold**, *italic*, `code`,
- bullet lists, 1. ordered lists, > quotes, --- rules, and paragraphs.
"""
import html
import re
import sys
from datetime import datetime

TEMPLATE = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
:root {{ color-scheme: light dark; --bg:#f6f8fb; --card:#fff; --ink:#1a2230; --muted:#5b6577; --accent:#2f6bff; --border:#e4e9f2; }}
@media (prefers-color-scheme: dark) {{ :root {{ --bg:#0d1119; --card:#151b26; --ink:#e8edf5; --muted:#9aa6b8; --accent:#6d9bff; --border:#232c3b; }} }}
* {{ box-sizing:border-box; }}
body {{ margin:0; background:var(--bg); color:var(--ink); font:16px/1.65 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,Helvetica,Arial,sans-serif; }}
.wrap {{ max-width:760px; margin:0 auto; padding:3rem 1.5rem 5rem; }}
.card {{ background:var(--card); border:1px solid var(--border); border-radius:16px; padding:2.4rem 2.6rem; box-shadow:0 12px 40px rgba(20,30,60,.07); }}
h1 {{ font-size:2rem; margin:.2em 0 .5em; letter-spacing:-.01em; }}
h2 {{ font-size:1.4rem; margin:1.6em 0 .5em; }}
h3 {{ font-size:1.12rem; margin:1.4em 0 .4em; }}
p {{ margin:.7em 0; }}
a {{ color:var(--accent); }}
code {{ background:color-mix(in srgb,var(--accent) 12%,transparent); padding:.12em .4em; border-radius:6px; font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace; font-size:.92em; }}
blockquote {{ margin:1em 0; padding:.4em 1.1em; border-left:3px solid var(--accent); color:var(--muted); }}
ul,ol {{ padding-left:1.4em; }}
li {{ margin:.25em 0; }}
hr {{ border:none; border-top:1px solid var(--border); margin:2em 0; }}
.foot {{ margin-top:2.5rem; color:var(--muted); font-size:.82rem; text-align:center; }}
.badge {{ display:inline-block; font-size:.72rem; letter-spacing:.08em; text-transform:uppercase; color:var(--accent); font-weight:700; }}
</style>
</head>
<body>
<div class="wrap">
<div class="card">
<span class="badge">Synaplan</span>
{body}
</div>
<div class="foot">Generated {stamp} · Synaplan Desktop</div>
</div>
</body>
</html>
"""


def inline(text):
    text = html.escape(text)
    text = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", text)
    text = re.sub(r"(?<!\*)\*(?!\*)(.+?)(?<!\*)\*(?!\*)", r"<em>\1</em>", text)
    text = re.sub(r"`(.+?)`", r"<code>\1</code>", text)
    text = re.sub(r"\[(.+?)\]\((https?://[^\s)]+)\)", r'<a href="\2">\1</a>', text)
    return text


def render(md):
    out = []
    lines = md.splitlines()
    i = 0
    list_type = None

    def close_list():
        nonlocal list_type
        if list_type:
            out.append(f"</{list_type}>")
            list_type = None

    while i < len(lines):
        line = lines[i].rstrip()
        if not line.strip():
            close_list()
            i += 1
            continue
        if re.match(r"^---+\s*$", line):
            close_list()
            out.append("<hr>")
        elif line.startswith("### "):
            close_list()
            out.append(f"<h3>{inline(line[4:])}</h3>")
        elif line.startswith("## "):
            close_list()
            out.append(f"<h2>{inline(line[3:])}</h2>")
        elif line.startswith("# "):
            close_list()
            out.append(f"<h1>{inline(line[2:])}</h1>")
        elif line.startswith("> "):
            close_list()
            out.append(f"<blockquote>{inline(line[2:])}</blockquote>")
        elif re.match(r"^\s*[-*]\s+", line):
            if list_type != "ul":
                close_list()
                out.append("<ul>")
                list_type = "ul"
            out.append(f"<li>{inline(re.sub(r'^\\s*[-*]\\s+', '', line))}</li>")
        elif re.match(r"^\s*\d+\.\s+", line):
            if list_type != "ol":
                close_list()
                out.append("<ol>")
                list_type = "ol"
            out.append(f"<li>{inline(re.sub(r'^\\s*\\d+\\.\\s+', '', line))}</li>")
        else:
            close_list()
            out.append(f"<p>{inline(line)}</p>")
        i += 1
    close_list()
    return "\n".join(out)


def main():
    if len(sys.argv) < 3:
        print("usage: run.py <input.md> <output.html> [title]", file=sys.stderr)
        return 2
    src, dst = sys.argv[1], sys.argv[2]
    with open(src, encoding="utf-8", errors="replace") as f:
        md = f.read()
    title = sys.argv[3] if len(sys.argv) > 3 else "Report"
    page = TEMPLATE.format(
        title=html.escape(title),
        body=render(md),
        stamp=datetime.now().strftime("%Y-%m-%d %H:%M"),
    )
    with open(dst, "w", encoding="utf-8") as f:
        f.write(page)
    print(f"Wrote page: {dst}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
