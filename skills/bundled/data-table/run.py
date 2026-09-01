#!/usr/bin/env python3
"""data-table — turn a CSV into a searchable, sortable, self-contained HTML table
(light+dark, no external assets). Pure standard library.

Usage:
    python3 run.py <input.csv> <output.html> [title]
"""
import csv
import html
import json
import sys
from datetime import datetime

PAGE = """<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
:root {{ color-scheme: light dark; --bg:#f6f8fb; --card:#fff; --ink:#1a2230; --muted:#5b6577; --accent:#2f6bff; --border:#e4e9f2; --row:#f9fbfe; }}
@media (prefers-color-scheme: dark) {{ :root {{ --bg:#0d1119; --card:#151b26; --ink:#e8edf5; --muted:#9aa6b8; --accent:#6d9bff; --border:#232c3b; --row:#1b2330; }} }}
body {{ margin:0; background:var(--bg); color:var(--ink); font:15px/1.5 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,Helvetica,Arial,sans-serif; }}
.wrap {{ max-width:1000px; margin:0 auto; padding:2.2rem 1.5rem; }}
h1 {{ font-size:1.4rem; margin:0 0 1rem; }}
.bar {{ display:flex; justify-content:space-between; align-items:center; gap:1rem; margin-bottom:.9rem; }}
input {{ padding:.55rem .8rem; border:1px solid var(--border); border-radius:10px; background:var(--card); color:var(--ink); font:inherit; min-width:240px; }}
.count {{ color:var(--muted); font-size:.85rem; }}
table {{ width:100%; border-collapse:collapse; background:var(--card); border:1px solid var(--border); border-radius:12px; overflow:hidden; }}
th,td {{ text-align:left; padding:.6rem .8rem; border-bottom:1px solid var(--border); }}
th {{ cursor:pointer; user-select:none; font-size:.82rem; text-transform:uppercase; letter-spacing:.04em; color:var(--muted); position:sticky; top:0; background:var(--card); }}
th:hover {{ color:var(--accent); }}
tbody tr:nth-child(even) {{ background:var(--row); }}
.foot {{ text-align:center; color:var(--muted); font-size:.78rem; margin-top:1.2rem; }}
</style></head><body><div class="wrap">
<h1>{title}</h1>
<div class="bar"><input id="q" placeholder="Search…" oninput="filter()"><span class="count" id="count"></span></div>
<table><thead><tr>{head}</tr></thead><tbody id="tb"></tbody></table>
<div class="foot">Generated {stamp} · Synaplan Desktop</div>
<script>
const rows={rows}, head={headjson};
let dir=1, sortCol=-1;
const tb=document.getElementById('tb'), q=document.getElementById('q'), count=document.getElementById('count');
function render(data){{
  tb.innerHTML=data.map(r=>'<tr>'+r.map(c=>'<td>'+escapeHtml(c)+'</td>').join('')+'</tr>').join('');
  count.textContent=data.length+' / '+rows.length+' rows';
}}
function escapeHtml(s){{return String(s).replace(/[&<>"]/g,c=>({{'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}}[c]));}}
function current(){{const t=q.value.toLowerCase();let d=t?rows.filter(r=>r.some(c=>String(c).toLowerCase().includes(t))):rows.slice();
  if(sortCol>=0){{d.sort((a,b)=>{{const x=a[sortCol],y=b[sortCol];const nx=parseFloat(x),ny=parseFloat(y);
    if(!isNaN(nx)&&!isNaN(ny))return (nx-ny)*dir; return String(x).localeCompare(String(y))*dir;}});}}
  return d;}}
function filter(){{render(current());}}
document.querySelectorAll('th').forEach((th,i)=>th.onclick=()=>{{dir=(sortCol===i)?-dir:1;sortCol=i;filter();}});
render(rows);
</script>
</div></body></html>
"""


def main():
    if len(sys.argv) < 3:
        print("usage: run.py <input.csv> <output.html> [title]", file=sys.stderr)
        return 2
    src, dst = sys.argv[1], sys.argv[2]
    title = sys.argv[3] if len(sys.argv) > 3 else "Data table"
    with open(src, newline="", encoding="utf-8", errors="replace") as f:
        rows = list(csv.reader(f))
    if not rows:
        print("The CSV file is empty.", file=sys.stderr)
        return 1
    header = rows[0]
    data = [r for r in rows[1:] if r]
    head_html = "".join(f"<th>{html.escape(h)}</th>" for h in header)
    page = PAGE.format(
        title=html.escape(title),
        head=head_html,
        headjson=json.dumps(header),
        rows=json.dumps(data),
        stamp=datetime.now().strftime("%Y-%m-%d %H:%M"),
    )
    with open(dst, "w", encoding="utf-8") as f:
        f.write(page)
    print(f"Wrote table ({len(data)} rows): {dst}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
