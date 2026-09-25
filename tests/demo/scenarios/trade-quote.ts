import type { DemoModels, DemoStoryboard } from './types'

/**
 * UC-D3 (English) — Electrical contractor: dictated site note + price list →
 * itemised quote, print-ready quote, customer email, contact card, install date.
 * See docs/demos/03-trade-quote.md.
 */

const HOME = '/Users/dan'
const PROJECT = `${HOME}/Synaplan/projects/okafor-kitchen-rewire`
const OUT = `${PROJECT}/out`

const models: DemoModels = {
  chat: 'groq:openai/gpt-oss-120b:chat',
  voice: 'openai:gpt-4o-transcribe:sound2text',
  speak: '',
  vision: 'groq:meta-llama/llama-4-scout:pic2text',
  image: '',
  video: '',
  embed: 'ollama:bge-m3:vectorize',
  docs: 'groq:openai/gpt-oss-120b:analyze',
  chatLegacyProviderId: null,
}

export const tradeQuote: DemoStoryboard = {
  title:
    'Hughes Electrical — from a site note dictated in the van to a quote the customer can sign.',
  scenario: {
    id: 'uc-d3-trade-quote',
    language: 'en',
    apiBaseUrl: 'https://web.synaplan.com',
    deviceName: 'Dan’s MacBook Air',
    keyBackend: 'macos-keychain',
    paths: {
      sep: '/',
      projectsDir: `${HOME}/Synaplan/projects`,
      outboxDir: `${HOME}/Synaplan/out`,
      skillsDir: `${HOME}/Library/Application Support/com.synaplan.desktop/skills`,
      configDir: `${HOME}/Library/Application Support/com.synaplan.desktop`,
    },
    personal: {
      id: 'personal',
      slug: 'personal',
      name: 'Personal',
      kind: 'personal',
      enabledSkills: [],
      webSearch: false,
      models,
      projectDir: `${HOME}/Synaplan/projects/personal`,
      notesDir: `${HOME}/Synaplan/projects/personal/notes`,
      outDir: `${HOME}/Synaplan/projects/personal/out`,
    },
    project: {
      id: 'okafor-kitchen-rewire',
      slug: 'okafor-kitchen-rewire',
      name: 'Okafor kitchen rewire',
      kind: 'project',
      enabledSkills: ['csv-insights', 'xlsx', 'invoice', 'email-draft', 'vcard', 'calendar-event'],
      webSearch: false,
      models,
      projectDir: PROJECT,
      notesDir: `${PROJECT}/notes`,
      outDir: OUT,
    },
    skills: [
      'docx',
      'xlsx',
      'pptx',
      'email-draft',
      'calendar-event',
      'vcard',
      'chart',
      'csv-insights',
      'web-report',
      'invoice',
      'data-table',
      'json-csv',
      'slides',
      'hello-files',
    ],
    executionConsent: true,
    notes: [
      {
        name: 'site-visit-okafor-24-sep.md',
        title: 'Site visit – Okafor kitchen, 24 Sep',
        content:
          '# Site visit – Okafor kitchen, 24 Sep\n\n' +
          'Mrs Adaeze Okafor, 14 Birch Grove, Reading RG6 1PL, 07700 900321, adaeze.okafor@example.com.\n\n' +
          'Full kitchen rewire, first fix after the old units come out on the 2nd. Eight double sockets on the ring, two of them USB. Dedicated 32 amp radial for the induction hob, 20 amp for the oven, fused spur for the extractor and one for the dishwasher. Under-cabinet LED strip, about four metres, with a driver in the wall unit. Replace the consumer unit — old rewireable fuses, no RCD — new 10-way dual RCD board with SPD. Bonding to the gas and water looks fine, check on the day. Two days labour for me plus one day for the apprentice. She wants it done before the fitters come back on the 7th.\n',
        updatedAt: '2026-09-24T16:52:00.000Z',
      },
    ],
    files: [{ name: 'pricelist-2026.csv', size: 18_204, path: `${PROJECT}/pricelist-2026.csv` }],
    wordMs: 20,
    turns: [
      {
        thinkMs: 1000,
        events: [
          {
            kind: 'step',
            tool: 'list_files',
            start: 'Listing okafor-kitchen-rewire',
            end: 'Listed okafor-kitchen-rewire (4 entries)',
            ms: 500,
          },
          {
            kind: 'step',
            tool: 'read_file',
            start: 'Reading notes/site-visit-okafor-24-sep.md',
            end: 'Read notes/site-visit-okafor-24-sep.md',
            ms: 450,
          },
          {
            kind: 'step',
            tool: 'read_file',
            start: 'Reading csv-insights/SKILL.md',
            end: 'Read csv-insights/SKILL.md',
            ms: 400,
          },
          {
            kind: 'step',
            tool: 'run_program',
            start: 'Running python3',
            end: 'Ran python3',
            artifact: `${OUT}/pricelist-2026-insights.md`,
            size: 3_900,
            ms: 1300,
          },
          {
            kind: 'step',
            tool: 'read_file',
            start: 'Reading xlsx/SKILL.md',
            end: 'Read xlsx/SKILL.md',
            ms: 400,
          },
          {
            kind: 'step',
            tool: 'write_file',
            start: 'Writing out/quote.json',
            end: 'Saved quote.json',
            ms: 1200,
          },
          {
            kind: 'step',
            tool: 'run_program',
            start: 'Running python3',
            end: 'Ran python3',
            artifact: `${OUT}/Okafor-kitchen-quote.xlsx`,
            size: 19_880,
            ms: 1300,
          },
          {
            kind: 'step',
            tool: 'read_file',
            start: 'Reading invoice/SKILL.md',
            end: 'Read invoice/SKILL.md',
            ms: 400,
          },
          {
            kind: 'step',
            tool: 'run_program',
            start: 'Running python3',
            end: 'Ran python3',
            artifact: `${OUT}/Okafor-kitchen-quote.html`,
            size: 14_112,
            ms: 1100,
          },
          {
            kind: 'text',
            text:
              'Quote is ready — two files in **out**, both built from your site note and the 2026 price list (every price is a lookup, nothing estimated):\n\n' +
              '**Okafor-kitchen-quote.xlsx** — sheet *Quote* with 14 lines and a SUM total row, sheet *Materials* with the price-list references.\n\n' +
              '**Okafor-kitchen-quote.html** — the print-ready version with your logo block, quote number Q-2026-0147, valid 30 days.\n\n' +
              '| Section | Net |\n|---|---|\n| Consumer unit, 10-way dual RCD + SPD, supply and fit | £412.00 |\n| Ring final: 8 × double sockets (2 USB), cable, back boxes | £486.00 |\n| Hob radial 32 A, oven 20 A, 2 × fused spurs | £268.00 |\n| Under-cabinet LED 4 m + driver | £96.00 |\n| Labour: 2 days electrician, 1 day apprentice | £560.00 |\n| Test, certificate (EICR-style), notification | £120.00 |\n| **Net** | **£1,942.00** |\n| VAT 20 % | £388.40 |\n| **Total** | **£2,330.40** |\n\n' +
              'Two things from the note I left as options on the quote rather than guessing: **bonding** (you said "check on the day" — priced as a separate line, £85, only if needed) and the **hob circuit** — the note says 32 A; if the induction hob is over 7.2 kW it will need 40 A and 10 mm² cable, which adds £38.',
          },
        ],
      },
      {
        thinkMs: 900,
        events: [
          {
            kind: 'step',
            tool: 'read_file',
            start: 'Reading email-draft/SKILL.md',
            end: 'Read email-draft/SKILL.md',
            ms: 400,
          },
          {
            kind: 'step',
            tool: 'run_program',
            start: 'Running python3',
            end: 'Ran python3',
            artifact: `${OUT}/Okafor-quote-email.eml`,
            size: 3_650,
            ms: 1000,
          },
          {
            kind: 'step',
            tool: 'read_file',
            start: 'Reading vcard/SKILL.md',
            end: 'Read vcard/SKILL.md',
            ms: 400,
          },
          {
            kind: 'step',
            tool: 'run_program',
            start: 'Running python3',
            end: 'Ran python3',
            artifact: `${OUT}/Adaeze-Okafor.vcf`,
            size: 412,
            ms: 800,
          },
          {
            kind: 'step',
            tool: 'read_file',
            start: 'Reading calendar-event/SKILL.md',
            end: 'Read calendar-event/SKILL.md',
            ms: 400,
          },
          {
            kind: 'step',
            tool: 'run_program',
            start: 'Running python3',
            end: 'Ran python3',
            artifact: `${OUT}/Okafor-kitchen-install.ics`,
            size: 1_040,
            ms: 900,
          },
          {
            kind: 'text',
            text:
              'Three more files in **out**:\n\n' +
              '**Okafor-quote-email.eml** — unsent draft to adaeze.okafor@example.com, subject "Your kitchen rewire quote — Q-2026-0147", plain wording, the total, the two options explained in one line each, and the install date proposed. The HTML quote is listed as the attachment.\n\n' +
              '**Adaeze-Okafor.vcf** — name, phone, email, 14 Birch Grove. Double-click to add her to Contacts.\n\n' +
              '**Okafor-kitchen-install.ics** — Monday, 5 October 2026, 08:00–17:00, "Okafor kitchen rewire — first fix", location 14 Birch Grove, Reading. That is two days before the fitters return on the 7th, as she asked.',
          },
        ],
      },
    ],
  },
  acts: [
    {
      kind: 'caption',
      text: 'Dan Hughes, electrician. Yesterday he dictated the site visit into the Synaplan Desktop on his phone-sized window in the van.',
      holdMs: 3400,
    },
    {
      kind: 'openNote',
      name: 'site-visit-okafor-24-sep.md',
      caption:
        'The dictated note is a Markdown file in the project. The 2026 price list is a CSV in the same folder.',
      holdMs: 3800,
    },
    { kind: 'closePanel' },
    {
      kind: 'send',
      prompt:
        'Use my site note and the 2026 price list to build the quote for Mrs Okafor: an itemised Excel with materials and labour, 20% VAT, and a print-ready quote I can send.',
      caption:
        'Everything is priced from the CSV by the csv-insights and xlsx skills — the model looks numbers up, it does not invent them.',
    },
    {
      kind: 'caption',
      text: 'Unclear items from the note become options on the quote, not guesses — bonding "check on the day", hob rating.',
      holdMs: 4200,
    },
    {
      kind: 'send',
      prompt:
        'Email the quote to Mrs Okafor, save her as a contact, and book the install for Monday 5 October, 8:00 to 17:00.',
      caption: 'Email draft, contact card, calendar entry — three skills from one sentence.',
    },
    {
      kind: 'reveal',
      artifactIndex: 0,
      caption:
        'Every file opens in the app he already uses: Mail, Contacts, Calendar, Numbers or Excel.',
    },
    { kind: 'wait', ms: 2500 },
  ],
}
