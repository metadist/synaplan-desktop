#!/usr/bin/env python3
"""slides — build a self-contained HTML presentation from a JSON spec. Opens in
any browser, no dependencies, arrow-key navigation.

Usage:
    python3 run.py <spec.json> <output.html>

spec.json:
    {
      "title": "Q3 Review",
      "subtitle": "Optional",
      "slides": [
        {"title": "Agenda", "bullets": ["One", "Two", "Three"]},
        {"title": "Big idea", "text": "A single centered statement."}
      ]
    }

Standard library only.
"""
import html
import json
import sys
from datetime import datetime

PAGE = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
:root {{ --bg:#0b1020; --fg:#eef2fb; --muted:#9fb0d0; --accent:#5b8cff; --accent2:#a55bff; }}
* {{ box-sizing:border-box; margin:0; padding:0; }}
html,body {{ height:100%; }}
body {{ background:radial-gradient(1200px 700px at 20% 10%, #16203a, var(--bg)); color:var(--fg);
  font:400 clamp(18px,2.4vh,24px)/1.5 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,Helvetica,Arial,sans-serif;
  overflow:hidden; }}
.deck {{ height:100%; }}
.slide {{ position:absolute; inset:0; display:none; flex-direction:column; justify-content:center;
  padding:8vh 12vw; opacity:0; transition:opacity .35s ease; }}
.slide.active {{ display:flex; opacity:1; }}
.kicker {{ color:var(--accent); font-size:.7em; letter-spacing:.22em; text-transform:uppercase; font-weight:700; margin-bottom:1.2rem; }}
h1 {{ font-size:2.4em; line-height:1.1; letter-spacing:-.02em; background:linear-gradient(90deg,var(--accent),var(--accent2));
  -webkit-background-clip:text; background-clip:text; color:transparent; }}
.subtitle {{ color:var(--muted); font-size:1.2em; margin-top:1rem; }}
.slide h2 {{ font-size:1.9em; margin-bottom:1.4rem; letter-spacing:-.01em; }}
ul {{ list-style:none; display:flex; flex-direction:column; gap:1rem; }}
li {{ position:relative; padding-left:1.8rem; font-size:1.15em; }}
li::before {{ content:""; position:absolute; left:0; top:.55em; width:.7rem; height:.7rem; border-radius:3px;
  background:linear-gradient(135deg,var(--accent),var(--accent2)); }}
.big {{ font-size:1.7em; line-height:1.35; max-width:22ch; }}
.bar {{ position:fixed; left:0; bottom:0; height:4px; background:linear-gradient(90deg,var(--accent),var(--accent2)); transition:width .3s ease; }}
.hud {{ position:fixed; right:1.5vw; bottom:1.6vh; color:var(--muted); font-size:.7em; letter-spacing:.05em; }}
.hint {{ position:fixed; left:1.5vw; bottom:1.6vh; color:var(--muted); font-size:.66em; opacity:.7; }}
</style>
</head>
<body>
<div class="deck" id="deck">
{slides}
</div>
<div class="bar" id="bar"></div>
<div class="hud" id="hud"></div>
<div class="hint">← → or space to navigate · {stamp}</div>
<script>
const slides=[...document.querySelectorAll('.slide')];let i=0;
function show(n){{i=Math.max(0,Math.min(slides.length-1,n));
  slides.forEach((s,k)=>s.classList.toggle('active',k===i));
  document.getElementById('bar').style.width=((i+1)/slides.length*100)+'%';
  document.getElementById('hud').textContent=(i+1)+' / '+slides.length;}}
document.addEventListener('keydown',e=>{{
  if(['ArrowRight',' ','PageDown','Enter'].includes(e.key)){{e.preventDefault();show(i+1);}}
  else if(['ArrowLeft','PageUp'].includes(e.key)){{show(i-1);}}
  else if(e.key==='Home'){{show(0);}} else if(e.key==='End'){{show(slides.length-1);}}
}});
document.addEventListener('click',()=>show(i+1));
show(0);
</script>
</body>
</html>
"""


def esc(s):
    return html.escape(str(s))


def title_slide(title, subtitle):
    sub = f'<div class="subtitle">{esc(subtitle)}</div>' if subtitle else ""
    return f'<section class="slide"><div class="kicker">Presentation</div><h1>{esc(title)}</h1>{sub}</section>'


def content_slide(s):
    parts = ['<section class="slide">']
    if s.get("title"):
        parts.append(f"<h2>{esc(s['title'])}</h2>")

    # Accept several common shapes the model may produce.
    bullets = s.get("bullets") or s.get("points") or s.get("items") or []
    body = s.get("text") or s.get("content") or s.get("body")
    if isinstance(body, list):  # "content": ["a", "b"] -> bullets
        bullets = bullets or body
        body = None
    if isinstance(bullets, str):
        bullets = [bullets]

    if body:
        parts.append(f'<div class="big">{esc(body)}</div>')
    if bullets:
        items = "".join(f"<li>{esc(b)}</li>" for b in bullets)
        parts.append(f"<ul>{items}</ul>")
    parts.append("</section>")
    return "".join(parts)


def main():
    if len(sys.argv) < 3:
        print("usage: run.py <spec.json> <output.html>", file=sys.stderr)
        return 2
    with open(sys.argv[1], encoding="utf-8") as f:
        spec = json.load(f)

    title = spec.get("title", "Presentation")
    slides_html = [title_slide(title, spec.get("subtitle"))]
    for s in spec.get("slides", []):
        slides_html.append(content_slide(s))

    page = PAGE.format(
        title=esc(title),
        slides="\n".join(slides_html),
        stamp=datetime.now().strftime("%Y-%m-%d"),
    )
    with open(sys.argv[2], "w", encoding="utf-8") as f:
        f.write(page)
    print(f"Wrote deck ({len(slides_html)} slides): {sys.argv[2]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
