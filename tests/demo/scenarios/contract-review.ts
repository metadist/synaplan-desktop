import type { DemoModels, DemoStoryboard } from './types'

/**
 * UC-D2 (English) — Solicitor: contract review against the firm's clause
 * playbook, a trainee briefing deck, the client cover email and a call reminder.
 * See docs/demos/02-contract-review.md.
 */

const HOME = 'C:\\Users\\priya'
const PROJECT = `${HOME}\\Synaplan\\projects\\harbourline-msa-review`
const OUT = `${PROJECT}\\out`

const models: DemoModels = {
  chat: 'openai:gpt-6-astra:chat',
  voice: 'openai:gpt-4o-transcribe:sound2text',
  speak: '',
  vision: 'openai:gpt-6-astra:pic2text',
  image: '',
  video: '',
  embed: 'ollama:bge-m3:vectorize',
  docs: 'openai:gpt-6-astra:analyze',
  chatLegacyProviderId: null,
}

export const contractReview: DemoStoryboard = {
  title: 'Nair & Co Solicitors — a draft MSA is reviewed against the firm playbook.',
  scenario: {
    id: 'uc-d2-contract-review',
    language: 'en',
    apiBaseUrl: 'https://synaplan.nairco.example',
    deviceName: 'PRIYA-LAPTOP',
    keyBackend: 'windows-credential-manager',
    paths: {
      sep: '\\',
      projectsDir: `${HOME}\\Synaplan\\projects`,
      outboxDir: `${HOME}\\Synaplan\\out`,
      skillsDir: `${HOME}\\AppData\\Local\\Synaplan\\Desktop\\skills`,
      configDir: `${HOME}\\AppData\\Local\\Synaplan\\Desktop`,
    },
    personal: {
      id: 'personal',
      slug: 'personal',
      name: 'Personal',
      kind: 'personal',
      enabledSkills: [],
      webSearch: false,
      models,
      projectDir: `${HOME}\\Synaplan\\projects\\personal`,
      notesDir: `${HOME}\\Synaplan\\projects\\personal\\notes`,
      outDir: `${HOME}\\Synaplan\\projects\\personal\\out`,
    },
    project: {
      id: 'harbourline-msa-review',
      slug: 'harbourline-msa-review',
      name: 'Harbourline – MSA review',
      kind: 'project',
      enabledSkills: ['docx', 'xlsx', 'pptx', 'email-draft', 'calendar-event'],
      webSearch: false,
      models,
      projectDir: PROJECT,
      notesDir: `${PROJECT}\\notes`,
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
        name: 'call-notes-22-sep.md',
        title: 'Call with Harbourline GC, 22 Sep',
        content:
          '# Call with Harbourline GC, 22 Sep\n\n- They want signature before their board on 8 Oct.\n- Sensitive on liability cap — their insurer set £2m.\n- Open to carving out pre-existing IP.\n- Payment: "60 days is the norm for them", 90 in the draft was a template artefact.\n',
        updatedAt: '2026-09-22T17:40:00.000Z',
      },
    ],
    wordMs: 20,
    turns: [
      {
        thinkMs: 1200,
        events: [
          {
            kind: 'step',
            tool: 'list_files',
            start: 'Listing harbourline-msa-review',
            end: 'Listed harbourline-msa-review (5 entries)',
            ms: 600,
          },
          {
            kind: 'step',
            tool: 'read_file',
            start: 'Reading docx/SKILL.md',
            end: 'Read docx/SKILL.md',
            ms: 450,
          },
          {
            kind: 'step',
            tool: 'run_program',
            start: 'Running python.exe',
            end: 'Ran python.exe',
            ms: 1500,
          },
          {
            kind: 'step',
            tool: 'run_program',
            start: 'Running python.exe',
            end: 'Ran python.exe',
            ms: 1300,
          },
          {
            kind: 'step',
            tool: 'read_file',
            start: 'Reading notes\\call-notes-22-sep.md',
            end: 'Read notes\\call-notes-22-sep.md',
            ms: 400,
          },
          {
            kind: 'step',
            tool: 'write_file',
            start: 'Writing out\\review-memo.md',
            end: 'Saved review-memo.md',
            ms: 1600,
          },
          {
            kind: 'step',
            tool: 'run_program',
            start: 'Running python.exe',
            end: 'Ran python.exe',
            artifact: `${OUT}\\Harbourline-MSA-Review-Memo.docx`,
            size: 96_430,
            ms: 1800,
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
            start: 'Writing out\\risk-register.json',
            end: 'Saved risk-register.json',
            ms: 700,
          },
          {
            kind: 'step',
            tool: 'run_program',
            start: 'Running python.exe',
            end: 'Ran python.exe',
            artifact: `${OUT}\\Harbourline-Risk-Register.xlsx`,
            size: 21_004,
            ms: 1200,
          },
          {
            kind: 'text',
            text:
              'Done. I read the draft (v3, 41 clauses) and the 2026 playbook, and used your call note from 22 September for the client context. Two files are in **out**:\n\n' +
              '**Harbourline-MSA-Review-Memo.docx** — executive summary, then a clause-by-clause table: clause · their wording · playbook position · risk · proposed redline. 13 clauses need attention, 28 are aligned or immaterial.\n\n' +
              '**Harbourline-Risk-Register.xlsx** — the same 13 items sorted by severity, with owner and status columns for the negotiation.\n\n' +
              '**High (3)**\n' +
              '1. **Cl. 12.2 Liability** — uncapped for "any breach of confidentiality". Playbook: cap at 12 months\' fees, carve out only fraud and death/personal injury. Their insurer set £2m per your note; the redline proposes £2m or 150 % of annual fees, whichever is higher.\n' +
              '2. **Cl. 9.1 IP** — assigns all "Deliverables and related materials" to the client, which would sweep in our client\'s pre-existing tools. Redline: assignment of bespoke deliverables only, licence to background IP.\n' +
              '3. **Cl. 6.3 Payment** — 90 days from invoice. Playbook floor is 30, their GC said 60 is normal; redline proposes 45 with statutory interest.\n\n' +
              '**Medium (4)** — 15.1 termination for convenience (no notice period), 11.4 data processing (no sub-processor list), 18 governing law (New York; playbook: England & Wales), 7.2 change control (unilateral).\n\n' +
              '**Low (6)** — definitions, notices address, counterparts, order of precedence, survival list, e-signature clause.\n\n' +
              'One thing I could not check: schedule 2 (service levels) is referenced but not in the draft you have — flagged as **[missing]** in the memo.',
          },
        ],
      },
      {
        thinkMs: 900,
        events: [
          {
            kind: 'step',
            tool: 'read_file',
            start: 'Reading pptx/SKILL.md',
            end: 'Read pptx/SKILL.md',
            ms: 450,
          },
          {
            kind: 'step',
            tool: 'write_file',
            start: 'Writing out\\trainee-deck.md',
            end: 'Saved trainee-deck.md',
            ms: 1200,
          },
          {
            kind: 'step',
            tool: 'run_program',
            start: 'Running python.exe',
            end: 'Ran python.exe',
            artifact: `${OUT}\\Trainee-Briefing-Liability-IP-Payment.pptx`,
            size: 148_220,
            ms: 1700,
          },
          {
            kind: 'text',
            text:
              '**Trainee-Briefing-Liability-IP-Payment.pptx** is in **out** — five slides:\n\n' +
              '1. Title: *Three clauses that decide the deal — Harbourline MSA*\n' +
              '2. **Liability caps** — what "uncapped" costs, why insurers set the number, the 12-months-fees rule of thumb\n' +
              '3. **IP assignment vs licence** — background IP, bespoke deliverables, the sweep-in trap in "related materials"\n' +
              '4. **Payment terms** — cash-flow maths on 30/45/60/90 days for a £480k contract, statutory interest\n' +
              '5. **How to spot them in ten minutes** — the three search terms and the playbook page for each\n\n' +
              'Client names stay in; if this goes beyond the trainees, tell me and I will make an anonymised copy.',
          },
        ],
      },
      {
        thinkMs: 800,
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
            start: 'Running python.exe',
            end: 'Ran python.exe',
            artifact: `${OUT}\\Harbourline-cover-email.eml`,
            size: 4_310,
            ms: 1000,
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
            start: 'Running python.exe',
            end: 'Ran python.exe',
            artifact: `${OUT}\\Harbourline-negotiation-call.ics`,
            size: 980,
            ms: 900,
          },
          {
            kind: 'text',
            text:
              'Both are in **out**:\n\n' +
              '**Harbourline-cover-email.eml** — an unsent draft that opens in Outlook. To: the GC, subject "Harbourline MSA v3 — our review and proposed redlines", three short paragraphs (headline, the three high-risk points in one sentence each, the call on Thursday), memo and register listed as attachments.\n\n' +
              '**Harbourline-negotiation-call.ics** — Thursday, 1 October 2026, 14:00–15:00, "Harbourline MSA — negotiation call", with the three headline points in the description.\n\n' +
              'The draft and the playbook never left this computer; the skills read them here and only the text I needed went to your Synaplan workspace.',
          },
        ],
      },
    ],
  },
  acts: [
    {
      kind: 'caption',
      text: 'Priya Nair, solicitor. A 41-clause draft MSA arrived at 16:10. The client wants a position by tomorrow.',
      holdMs: 3200,
    },
    {
      kind: 'openNote',
      name: 'call-notes-22-sep.md',
      caption:
        'Her call note from Monday is already in the project. Notes are Markdown files on this computer.',
      holdMs: 3000,
    },
    { kind: 'closePanel' },
    {
      kind: 'caption',
      text: 'The draft and the firm playbook sit in the project folder. They are not uploaded anywhere — the skills read them here.',
      holdMs: 3200,
    },
    {
      kind: 'send',
      prompt:
        'Review the draft MSA against our clause playbook. Give me a Word memo with a clause-by-clause table (clause, their wording, our position, risk, proposed redline) and an Excel risk register sorted by severity. Use my call note for the client context.',
      caption:
        'One request. The desktop reads both Word files with the docx skill, writes the memo and the register as real Office files.',
    },
    {
      kind: 'caption',
      text: 'The answer names what it could not check — schedule 2 is missing from the draft — instead of guessing.',
      holdMs: 4200,
    },
    {
      kind: 'send',
      prompt:
        'Turn the three high-risk findings into a 5-slide training deck for our trainees — plain English, one clause per slide with the why.',
      caption:
        'Teaching from the same material: a trainee briefing deck, built by the pptx skill on this computer.',
    },
    {
      kind: 'send',
      prompt:
        'Draft the cover email to the client with the memo and register attached, and put the negotiation call in my calendar: Thursday 1 October, 14:00.',
      caption: 'Client email as an unsent Outlook draft, the call as a calendar file.',
    },
    {
      kind: 'reveal',
      artifactIndex: 0,
      caption:
        "Every result is a file in the project's out folder — one click reveals it in Explorer.",
    },
    { kind: 'wait', ms: 2500 },
  ],
}
