#!/usr/bin/env python3
"""email-draft — compose an .eml file you can open in any mail client.

Usage:
    python3 run.py <output.eml> <to> <subject> <body>

`body` may contain \\n for line breaks. Standard library only. This only writes
a local .eml draft; it never sends anything.
"""
import sys
from datetime import datetime, timezone
from email.message import EmailMessage
from email.utils import format_datetime, make_msgid


def main():
    if len(sys.argv) < 5:
        print("usage: run.py <output.eml> <to> <subject> <body>", file=sys.stderr)
        return 2
    dst, to_addr, subject, body = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
    body = body.replace("\\n", "\n")

    msg = EmailMessage()
    msg["To"] = to_addr
    msg["Subject"] = subject
    msg["Date"] = format_datetime(datetime.now(timezone.utc))
    msg["Message-ID"] = make_msgid(domain="synaplan.local")
    msg["X-Unsent"] = "1"  # Outlook/Apple Mail open this as an editable draft
    msg.set_content(body)

    with open(dst, "wb") as f:
        f.write(bytes(msg))
    print(f"Wrote draft: {dst}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
