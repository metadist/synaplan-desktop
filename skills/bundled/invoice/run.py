#!/usr/bin/env python3
"""invoice — turn a JSON spec into a clean, print-ready HTML invoice with line
totals, subtotal, optional tax, and grand total. Pure standard library.

Usage:
    python3 run.py <spec.json> <output.html>

spec.json:
    {
      "number": "INV-1001",
      "date": "2026-09-01",
      "from": "Acme GmbH\\nBerlin",
      "to": "Client Ltd\\nLondon",
      "currency": "EUR",
      "tax_rate": 19,
      "notes": "Payment within 14 days.",
      "items": [
        {"description": "Consulting", "quantity": 3, "price": 150},
        {"description": "Hosting", "quantity": 1, "price": 40}
      ]
    }
"""
import html
import json
import sys
from datetime import datetime

CURRENCY = {"EUR": "€", "USD": "$", "GBP": "£", "CHF": "CHF ", "JPY": "¥"}

PAGE = """<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Invoice {number}</title>
<style>
:root {{ color-scheme: light dark; --bg:#f6f8fb; --card:#fff; --ink:#1a2230; --muted:#5b6577; --accent:#2f6bff; --border:#e4e9f2; }}
@media (prefers-color-scheme: dark) {{ :root {{ --bg:#0d1119; --card:#151b26; --ink:#e8edf5; --muted:#9aa6b8; --accent:#6d9bff; --border:#232c3b; }} }}
body {{ margin:0; background:var(--bg); color:var(--ink); font:15px/1.55 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,Helvetica,Arial,sans-serif; }}
.sheet {{ max-width:800px; margin:2rem auto; background:var(--card); border:1px solid var(--border); border-radius:14px; padding:2.6rem 3rem; box-shadow:0 12px 40px rgba(20,30,60,.07); }}
.head {{ display:flex; justify-content:space-between; align-items:flex-start; gap:2rem; margin-bottom:2rem; }}
.title {{ font-size:2rem; letter-spacing:-.02em; margin:0; }}
.meta {{ text-align:right; color:var(--muted); font-size:.9rem; }}
.parties {{ display:flex; justify-content:space-between; gap:2rem; margin-bottom:2rem; }}
.party h3 {{ margin:0 0 .3rem; font-size:.72rem; text-transform:uppercase; letter-spacing:.08em; color:var(--muted); }}
.party div {{ white-space:pre-line; }}
table {{ width:100%; border-collapse:collapse; margin-bottom:1.4rem; }}
th {{ text-align:left; font-size:.72rem; text-transform:uppercase; letter-spacing:.06em; color:var(--muted); padding:.5rem .4rem; border-bottom:2px solid var(--border); }}
td {{ padding:.6rem .4rem; border-bottom:1px solid var(--border); }}
.num {{ text-align:right; font-variant-numeric:tabular-nums; }}
.totals {{ margin-left:auto; width:280px; }}
.totals td {{ border:none; padding:.35rem .4rem; }}
.grand td {{ border-top:2px solid var(--border); font-weight:700; font-size:1.15rem; padding-top:.7rem; }}
.notes {{ margin-top:2rem; color:var(--muted); font-size:.9rem; white-space:pre-line; }}
.foot {{ margin-top:2.5rem; text-align:center; color:var(--muted); font-size:.75rem; }}
@media print {{ body {{ background:#fff; }} .sheet {{ box-shadow:none; border:none; margin:0; }} .foot {{ display:none; }} }}
</style></head><body><div class="sheet">
<div class="head">
  <div><h1 class="title">Invoice</h1><div style="color:var(--accent);font-weight:700">#{number}</div></div>
  <div class="meta">{date}</div>
</div>
<div class="parties">
  <div class="party"><h3>From</h3><div>{frm}</div></div>
  <div class="party"><h3>Bill to</h3><div>{to}</div></div>
</div>
<table>
<thead><tr><th>Description</th><th class="num">Qty</th><th class="num">Unit</th><th class="num">Amount</th></tr></thead>
<tbody>{rows}</tbody>
</table>
<table class="totals">{totals}</table>
{notes}
<div class="foot">Generated {stamp} · Synaplan Desktop</div>
</div></body></html>
"""


def num(v, default=0.0):
    try:
        return float(v)
    except (TypeError, ValueError):
        return default


def money(sym, v):
    return f"{sym}{v:,.2f}"


def main():
    if len(sys.argv) < 3:
        print("usage: run.py <spec.json> <output.html>", file=sys.stderr)
        return 2
    with open(sys.argv[1], encoding="utf-8") as f:
        spec = json.load(f)

    cur = spec.get("currency", "USD")
    sym = CURRENCY.get(str(cur).upper(), str(cur) + " " if len(str(cur)) > 1 else str(cur))

    def party(v):
        if isinstance(v, dict):
            v = "\n".join(str(x) for x in v.values() if x)
        return html.escape(str(v or ""))

    rows_html = []
    subtotal = 0.0
    for it in spec.get("items", []):
        desc = it.get("description") or it.get("desc") or ""
        qty = num(it.get("quantity", it.get("qty", 1)), 1)
        price = num(it.get("price", it.get("unit_price", it.get("rate", 0))))
        amount = qty * price
        subtotal += amount
        rows_html.append(
            f'<tr><td>{html.escape(str(desc))}</td>'
            f'<td class="num">{qty:g}</td>'
            f'<td class="num">{money(sym, price)}</td>'
            f'<td class="num">{money(sym, amount)}</td></tr>'
        )

    rate = num(spec.get("tax_rate", 0))
    if rate > 1:  # given as a percentage
        rate /= 100.0
    tax = subtotal * rate
    total = subtotal + tax

    totals = [f'<tr><td>Subtotal</td><td class="num">{money(sym, subtotal)}</td></tr>']
    if rate > 0:
        totals.append(f'<tr><td>Tax ({rate*100:g}%)</td><td class="num">{money(sym, tax)}</td></tr>')
    totals.append(f'<tr class="grand"><td>Total</td><td class="num">{money(sym, total)}</td></tr>')

    notes = spec.get("notes", "")
    notes_html = f'<div class="notes">{html.escape(str(notes))}</div>' if notes else ""

    page = PAGE.format(
        number=html.escape(str(spec.get("number", "—"))),
        date=html.escape(str(spec.get("date", datetime.now().strftime("%Y-%m-%d")))),
        frm=party(spec.get("from", "")),
        to=party(spec.get("to", "")),
        rows="".join(rows_html) or '<tr><td colspan="4">No items.</td></tr>',
        totals="".join(totals),
        notes=notes_html,
        stamp=datetime.now().strftime("%Y-%m-%d %H:%M"),
    )
    with open(sys.argv[2], "w", encoding="utf-8") as f:
        f.write(page)
    print(f"Wrote invoice {spec.get('number', '')}: {sys.argv[2]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
