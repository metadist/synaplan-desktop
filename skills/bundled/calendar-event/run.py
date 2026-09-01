#!/usr/bin/env python3
"""calendar-event — write an .ics calendar invite you can open in any calendar
app (Outlook, Apple Calendar, Google Calendar import). Pure standard library.

Usage:
    python3 run.py <output.ics> <title> <start> <end> [location] [description]

Dates: "YYYY-MM-DD HH:MM" (local) or "YYYY-MM-DD" for an all-day event.
"""
import sys
import uuid
from datetime import datetime, timedelta, timezone


def parse_dt(s):
    s = s.strip()
    for fmt in ("%Y-%m-%d %H:%M", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M:%S"):
        try:
            return datetime.strptime(s, fmt), False
        except ValueError:
            pass
    try:
        return datetime.strptime(s, "%Y-%m-%d"), True
    except ValueError:
        raise SystemExit(f"error: could not parse date/time: {s!r}")


def fold(line):
    # RFC 5545 line folding at 75 octets.
    out = []
    while len(line.encode("utf-8")) > 74:
        cut = 74
        while len(line[:cut].encode("utf-8")) > 74:
            cut -= 1
        out.append(line[:cut])
        line = " " + line[cut:]
    out.append(line)
    return "\r\n".join(out)


def esc(text):
    return (
        text.replace("\\", "\\\\")
        .replace(";", "\\;")
        .replace(",", "\\,")
        .replace("\n", "\\n")
    )


def main():
    if len(sys.argv) < 5:
        print("usage: run.py <output.ics> <title> <start> <end> [location] [description]", file=sys.stderr)
        return 2
    dst, title, start_s, end_s = sys.argv[1:5]
    location = sys.argv[5] if len(sys.argv) > 5 else ""
    description = sys.argv[6] if len(sys.argv) > 6 else ""

    start, all_day = parse_dt(start_s)
    end, _ = parse_dt(end_s)

    if all_day:
        dtstart = f"DTSTART;VALUE=DATE:{start:%Y%m%d}"
        dtend = f"DTEND;VALUE=DATE:{(end + timedelta(days=1)):%Y%m%d}"
    else:
        dtstart = f"DTSTART:{start:%Y%m%dT%H%M%S}"
        dtend = f"DTEND:{end:%Y%m%dT%H%M%S}"

    lines = [
        "BEGIN:VCALENDAR",
        "VERSION:2.0",
        "PRODID:-//Synaplan Desktop//calendar-event//EN",
        "CALSCALE:GREGORIAN",
        "METHOD:PUBLISH",
        "BEGIN:VEVENT",
        f"UID:{uuid.uuid4()}@synaplan.local",
        f"DTSTAMP:{datetime.now(timezone.utc):%Y%m%dT%H%M%SZ}",
        dtstart,
        dtend,
        fold(f"SUMMARY:{esc(title)}"),
    ]
    if location:
        lines.append(fold(f"LOCATION:{esc(location)}"))
    if description:
        lines.append(fold(f"DESCRIPTION:{esc(description)}"))
    lines += ["END:VEVENT", "END:VCALENDAR"]

    with open(dst, "w", encoding="utf-8", newline="") as f:
        f.write("\r\n".join(lines) + "\r\n")
    print(f"Wrote calendar event: {dst}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
