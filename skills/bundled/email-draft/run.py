#!/usr/bin/env python3
"""email-draft — compose an .eml file you can open in any mail client.

Usage:
    python3 run.py <output.eml> <to> <subject> <body>
    python3 run.py <output.eml> <to> <subject> --body-file <body.txt>

`body` may contain \\n (or %0A) for line breaks; a longer text is easier to
pass as a file. Standard library only. This only writes a local .eml draft; it
never sends anything.
"""
import sys
from datetime import datetime, timezone
from email.message import EmailMessage
from email.utils import format_datetime, make_msgid
from pathlib import Path


def unescape(body: str) -> str:
    """Models sometimes hand over newlines as \\n or URL-encoded %0A."""
    return body.replace("\\r\\n", "\n").replace("\\n", "\n").replace("%0D%0A", "\n").replace("%0A", "\n")


def main():
    if len(sys.argv) < 5:
        print("usage: run.py <output.eml> <to> <subject> <body> | --body-file <body.txt>", file=sys.stderr)
        return 2
    dst, to_addr, subject = sys.argv[1], sys.argv[2], sys.argv[3]
    if sys.argv[4] == "--body-file":
        if len(sys.argv) < 6 or not Path(sys.argv[5]).is_file():
            print("--body-file needs an existing text file", file=sys.stderr)
            return 2
        body = Path(sys.argv[5]).read_text(encoding="utf-8", errors="replace")
    else:
        body = sys.argv[4]
    body = unescape(body)

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
