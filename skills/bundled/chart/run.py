#!/usr/bin/env python3
"""chart — turn a CSV into a clean SVG chart embedded in a standalone HTML page
(bar or line, multi-series, light+dark). Pure standard library, no dependencies.

Usage:
    python3 run.py <input.csv> <output.html> [--type bar|line] [--title TITLE]

The first column is used as the category labels (x-axis). Every other column that
is numeric becomes a series.
"""
import csv
import html
import sys
from datetime import datetime

PALETTE = ["#5b8cff", "#a55bff", "#22c55e", "#ff8a5b", "#f2c14e", "#4ecdc4"]

PAGE = """<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
:root {{ color-scheme: light dark; --bg:#f6f8fb; --card:#fff; --ink:#1a2230; --muted:#5b6577; --border:#e4e9f2; --grid:#e9eef6; }}
@media (prefers-color-scheme: dark) {{ :root {{ --bg:#0d1119; --card:#151b26; --ink:#e8edf5; --muted:#9aa6b8; --border:#232c3b; --grid:#212a39; }} }}
body {{ margin:0; background:var(--bg); color:var(--ink); font:15px/1.5 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,Helvetica,Arial,sans-serif; }}
.wrap {{ max-width:860px; margin:0 auto; padding:2.5rem 1.5rem; }}
.card {{ background:var(--card); border:1px solid var(--border); border-radius:16px; padding:1.8rem 2rem; box-shadow:0 12px 40px rgba(20,30,60,.07); }}
h1 {{ font-size:1.4rem; margin:0 0 1.2rem; }}
svg {{ width:100%; height:auto; }}
.legend {{ display:flex; flex-wrap:wrap; gap:1rem; margin-top:1rem; font-size:.85rem; color:var(--muted); }}
.legend span {{ display:inline-flex; align-items:center; gap:.4rem; }}
.sw {{ width:12px; height:12px; border-radius:3px; display:inline-block; }}
.foot {{ text-align:center; color:var(--muted); font-size:.78rem; margin-top:1.5rem; }}
.axis {{ stroke:var(--muted); stroke-width:1; }}
.grid {{ stroke:var(--grid); stroke-width:1; }}
.lbl {{ fill:var(--muted); font-size:11px; }}
</style></head><body><div class="wrap"><div class="card">
<h1>{title}</h1>
{svg}
<div class="legend">{legend}</div>
</div><div class="foot">Generated {stamp} · Synaplan Desktop</div></div></body></html>
"""

W, H = 820, 420
PAD_L, PAD_R, PAD_T, PAD_B = 56, 20, 20, 60


def is_number(v):
    try:
        float(v)
        return True
    except (TypeError, ValueError):
        return False


def esc(s):
    return html.escape(str(s))


def nice_max(v):
    if v <= 0:
        return 1
    import math

    exp = math.floor(math.log10(v))
    base = 10**exp
    for m in (1, 2, 2.5, 5, 10):
        if v <= m * base:
            return m * base
    return 10 * base


def build_svg(labels, series, chart_type):
    plot_w = W - PAD_L - PAD_R
    plot_h = H - PAD_T - PAD_B
    all_vals = [v for _, vals in series for v in vals]
    vmax = nice_max(max(all_vals) if all_vals else 1)
    parts = [f'<svg viewBox="0 0 {W} {H}" role="img">']

    # y grid + labels
    ticks = 5
    for i in range(ticks + 1):
        y = PAD_T + plot_h - (plot_h * i / ticks)
        val = vmax * i / ticks
        parts.append(f'<line class="grid" x1="{PAD_L}" y1="{y:.1f}" x2="{W-PAD_R}" y2="{y:.1f}"/>')
        parts.append(f'<text class="lbl" x="{PAD_L-8}" y="{y+3:.1f}" text-anchor="end">{val:g}</text>')

    n = len(labels)
    slot = plot_w / max(n, 1)

    if chart_type == "line":
        for si, (_, vals) in enumerate(series):
            color = PALETTE[si % len(PALETTE)]
            pts = []
            for i, v in enumerate(vals):
                x = PAD_L + slot * (i + 0.5)
                y = PAD_T + plot_h - (plot_h * v / vmax)
                pts.append(f"{x:.1f},{y:.1f}")
            parts.append(f'<polyline fill="none" stroke="{color}" stroke-width="2.5" points="{" ".join(pts)}"/>')
            for p in pts:
                x, y = p.split(",")
                parts.append(f'<circle cx="{x}" cy="{y}" r="3" fill="{color}"/>')
    else:  # bar (grouped)
        ns = len(series)
        group_w = slot * 0.7
        bar_w = group_w / max(ns, 1)
        for i in range(n):
            for si, (_, vals) in enumerate(series):
                v = vals[i]
                color = PALETTE[si % len(PALETTE)]
                x = PAD_L + slot * i + (slot - group_w) / 2 + si * bar_w
                bh = plot_h * v / vmax
                y = PAD_T + plot_h - bh
                parts.append(f'<rect x="{x:.1f}" y="{y:.1f}" width="{bar_w-2:.1f}" height="{bh:.1f}" rx="3" fill="{color}"/>')

    # x axis + labels
    parts.append(f'<line class="axis" x1="{PAD_L}" y1="{PAD_T+plot_h}" x2="{W-PAD_R}" y2="{PAD_T+plot_h}"/>')
    for i, lab in enumerate(labels):
        x = PAD_L + slot * (i + 0.5)
        parts.append(f'<text class="lbl" x="{x:.1f}" y="{PAD_T+plot_h+18}" text-anchor="middle">{esc(lab)[:14]}</text>')

    parts.append("</svg>")
    return "\n".join(parts)


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    opts = {}
    it = iter(sys.argv[1:])
    for a in it:
        if a.startswith("--"):
            opts[a[2:]] = next(it, "")
    if len(args) < 2:
        print("usage: run.py <input.csv> <output.html> [--type bar|line] [--title T]", file=sys.stderr)
        return 2
    src, dst = args[0], args[1]
    chart_type = opts.get("type", "bar").lower()

    with open(src, newline="", encoding="utf-8", errors="replace") as f:
        rows = list(csv.reader(f))
    if len(rows) < 2:
        print("Need a header row and at least one data row.", file=sys.stderr)
        return 1

    header = rows[0]
    data = [r for r in rows[1:] if r]
    labels = [r[0] if r else "" for r in data]

    series = []
    for ci in range(1, len(header)):
        col = [r[ci] if ci < len(r) else "" for r in data]
        if all(is_number(v) for v in col if v != ""):
            vals = [float(v) if v != "" else 0.0 for v in col]
            series.append((header[ci], vals))
    if not series:
        print("No numeric columns to chart.", file=sys.stderr)
        return 1

    title = opts.get("title", "Chart")
    svg = build_svg(labels, series, chart_type)
    legend = "".join(
        f'<span><i class="sw" style="background:{PALETTE[i % len(PALETTE)]}"></i>{esc(name)}</span>'
        for i, (name, _) in enumerate(series)
    )
    page = PAGE.format(title=esc(title), svg=svg, legend=legend, stamp=datetime.now().strftime("%Y-%m-%d %H:%M"))
    with open(dst, "w", encoding="utf-8") as f:
        f.write(page)
    print(f"Wrote {chart_type} chart ({len(series)} series): {dst}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
