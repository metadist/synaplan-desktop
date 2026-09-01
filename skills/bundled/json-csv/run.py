#!/usr/bin/env python3
"""json-csv — convert between JSON and CSV. Direction is chosen by the file
extensions. Pure standard library.

Usage:
    python3 run.py <input.json> <output.csv>   # JSON array of objects -> CSV
    python3 run.py <input.csv>  <output.json>  # CSV -> JSON array of objects
"""
import csv
import json
import sys


def json_to_csv(src, dst):
    with open(src, encoding="utf-8") as f:
        data = json.load(f)
    if isinstance(data, dict):
        data = [data]
    if not isinstance(data, list) or not data:
        print("JSON must be a non-empty array of objects.", file=sys.stderr)
        return 1
    # Union of keys, preserving first-seen order.
    keys = []
    for row in data:
        if not isinstance(row, dict):
            print("Every JSON array item must be an object.", file=sys.stderr)
            return 1
        for k in row:
            if k not in keys:
                keys.append(k)
    with open(dst, "w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=keys, extrasaction="ignore")
        w.writeheader()
        for row in data:
            w.writerow({k: stringify(row.get(k, "")) for k in keys})
    print(f"Wrote CSV ({len(data)} rows): {dst}")
    return 0


def stringify(v):
    if isinstance(v, (dict, list)):
        return json.dumps(v, ensure_ascii=False)
    return v


def csv_to_json(src, dst):
    with open(src, newline="", encoding="utf-8", errors="replace") as f:
        rows = list(csv.DictReader(f))
    with open(dst, "w", encoding="utf-8") as f:
        json.dump(rows, f, ensure_ascii=False, indent=2)
    print(f"Wrote JSON ({len(rows)} rows): {dst}")
    return 0


def main():
    if len(sys.argv) < 3:
        print("usage: run.py <input> <output>  (.json<->.csv by extension)", file=sys.stderr)
        return 2
    src, dst = sys.argv[1], sys.argv[2]
    sl, dl = src.lower(), dst.lower()
    if sl.endswith(".json") and dl.endswith(".csv"):
        return json_to_csv(src, dst)
    if sl.endswith(".csv") and dl.endswith(".json"):
        return csv_to_json(src, dst)
    print("Extensions must be one of: .json -> .csv, or .csv -> .json", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
