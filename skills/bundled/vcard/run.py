#!/usr/bin/env python3
"""vcard — write a .vcf contact card you can import into any address book.
Pure standard library.

Usage:
    python3 run.py <output.vcf> <full_name> [email] [phone] [org] [title]
"""
import sys


def esc(text):
    return text.replace("\\", "\\\\").replace(";", "\\;").replace(",", "\\,").replace("\n", "\\n")


def main():
    if len(sys.argv) < 3:
        print("usage: run.py <output.vcf> <full_name> [email] [phone] [org] [title]", file=sys.stderr)
        return 2
    dst = sys.argv[1]
    full = sys.argv[2]
    email = sys.argv[3] if len(sys.argv) > 3 else ""
    phone = sys.argv[4] if len(sys.argv) > 4 else ""
    org = sys.argv[5] if len(sys.argv) > 5 else ""
    title = sys.argv[6] if len(sys.argv) > 6 else ""

    parts = full.split()
    last = parts[-1] if len(parts) > 1 else ""
    first = " ".join(parts[:-1]) if len(parts) > 1 else full

    lines = ["BEGIN:VCARD", "VERSION:3.0", f"N:{esc(last)};{esc(first)};;;", f"FN:{esc(full)}"]
    if org:
        lines.append(f"ORG:{esc(org)}")
    if title:
        lines.append(f"TITLE:{esc(title)}")
    if email:
        lines.append(f"EMAIL;TYPE=INTERNET:{esc(email)}")
    if phone:
        lines.append(f"TEL;TYPE=CELL:{esc(phone)}")
    lines.append("END:VCARD")

    with open(dst, "w", encoding="utf-8", newline="") as f:
        f.write("\r\n".join(lines) + "\r\n")
    print(f"Wrote contact card: {dst}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
