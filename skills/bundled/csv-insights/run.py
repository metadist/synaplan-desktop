#!/usr/bin/env python3
"""csv-insights — profile a CSV file and write a Markdown report.

Usage:
    python3 run.py <input.csv> <output.md> [title]

Standard library only. Safe: reads one CSV, writes one Markdown file.
"""
import csv
import os
import statistics
import sys
from datetime import datetime


def is_number(value):
    try:
        float(value)
        return True
    except (TypeError, ValueError):
        return False


def profile_column(name, values):
    non_empty = [v for v in values if v is not None and v != ""]
    filled = len(non_empty)
    missing = len(values) - filled
    unique = len(set(non_empty))
    numeric = [float(v) for v in non_empty if is_number(v)]
    lines = [f"### {name}", ""]
    lines.append(f"- Filled: {filled} / {len(values)} ({missing} missing)")
    lines.append(f"- Unique values: {unique}")
    if numeric and len(numeric) == filled and filled > 0:
        lines.append(f"- Min: {min(numeric):g}")
        lines.append(f"- Max: {max(numeric):g}")
        lines.append(f"- Mean: {statistics.mean(numeric):g}")
        if len(numeric) > 1:
            lines.append(f"- Std dev: {statistics.pstdev(numeric):g}")
    else:
        counts = {}
        for v in non_empty:
            counts[v] = counts.get(v, 0) + 1
        top = sorted(counts.items(), key=lambda kv: (-kv[1], kv[0]))[:5]
        if top:
            lines.append("- Most common: " + ", ".join(f"`{k}` ({c})" for k, c in top))
    lines.append("")
    return lines


def main():
    if len(sys.argv) < 3:
        print("usage: run.py <input.csv> <output.md> [title]", file=sys.stderr)
        return 2
    src, dst = sys.argv[1], sys.argv[2]
    title = sys.argv[3] if len(sys.argv) > 3 else os.path.basename(src)

    with open(src, newline="", encoding="utf-8", errors="replace") as f:
        reader = csv.reader(f)
        rows = list(reader)
    if not rows:
        print("The CSV file is empty.", file=sys.stderr)
        return 1

    header = rows[0]
    data = rows[1:]
    columns = {h: [] for h in header}
    for row in data:
        for i, h in enumerate(header):
            columns[h].append(row[i] if i < len(row) else "")

    out = [f"# CSV insights — {title}", ""]
    out.append(f"_Generated {datetime.now().strftime('%Y-%m-%d %H:%M')}_")
    out.append("")
    out.append(f"- **Rows:** {len(data)}")
    out.append(f"- **Columns:** {len(header)} ({', '.join(header)})")
    out.append("")
    out.append("## Column details")
    out.append("")
    for h in header:
        out.extend(profile_column(h, columns[h]))

    with open(dst, "w", encoding="utf-8") as f:
        f.write("\n".join(out))
    print(f"Wrote report: {dst}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
