# UC-D3 — Quote from a note dictated in the van

**Film:** [uc-d3-trade-quote.mp4](videos/uc-d3-trade-quote.mp4)
**Language of the app:** English. **Person:** Dan Hughes, electrician,
Hughes Electrical, Reading.

Script: `tests/demo/scenarios/trade-quote.ts`.

## The job

Yesterday, in the van, Dan dictated the site visit into the project: Mrs
Okafor, 14 Birch Grove, a full kitchen rewire after the old units come out
on the 2nd, a new consumer unit, eight sockets, dedicated circuits for the
hob and the oven, four metres of under-cabinet LEDs, two days of his labour
and one for the apprentice. She wants it finished before the fitters return
on the 7th. The 2026 price list is a CSV he already keeps.

The slow part is not the electrical design. It is turning that note into
line items at this year’s prices, then the email, the contact, and a day
in the diary.

## What the desktop does

Project **Okafor kitchen rewire**. Skills: `csv-insights`, `xlsx`,
`invoice`, `email-draft`, `vcard`, `calendar-event`. The dictated note is
in the project. The price list is a file beside it.

1. He opens the note so the recording shows the source: a Markdown file,
   not a form.
2. One request: an itemised Excel quote from the note and the price list,
   materials and labour, 20% VAT, plus a print-ready quote.
3. The run profiles the CSV, then writes:
   - `Okafor-kitchen-quote.xlsx` — a Quote sheet with a total row and a
     Materials sheet that names the price-list rows
   - `Okafor-kitchen-quote.html` — quote Q-2026-0147, valid 30 days
4. The reply totals **£1,942.00 net, £2,330.40 with VAT**, and leaves two
   things as options instead of guessing: bonding (“check on the day”,
   £85 only if needed) and a hob circuit that becomes 40 A and 10 mm² if
   the hob is over 7.2 kW (£38 more).
5. A second request writes the email to adaeze.okafor@example.com, her
   contact card, and the install on Monday 5 October, 08:00–17:00 — two
   days before the fitters come back.

| File | What he does with it |
| ---- | -------------------- |
| `Okafor-kitchen-quote.xlsx` | Checks the lines, edits a price if the list was wrong |
| `Okafor-kitchen-quote.html` | Prints or attaches the customer copy |
| `Okafor-quote-email.eml` | Opens in Mail as an unsent draft |
| `Adaeze-Okafor.vcf` | Adds her to Contacts |
| `Okafor-kitchen-install.ics` | Adds the day to Calendar |

## Why this is faster

The price list is the source of the money. He does not retype fourteen
lines, and a later change to the CSV is a new run rather than a hunt
through an old spreadsheet. The email, the contact and the diary entry
reuse the same name, address and date as the quote.

## What this film does not claim

The quote is not sent, and it is not a certificate. Bonding and the hob
rating stay options until he has looked. The desktop does not order the
board or book the wholesaler.
