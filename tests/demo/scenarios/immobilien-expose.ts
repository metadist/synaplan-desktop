import type { DemoModels, DemoStoryboard } from './types'

/**
 * UC-D1 (German) — Immobilienmaklerin: Exposé, Eckdaten, Besichtigungseinladung.
 * See docs/demos/01-immobilien-expose.md for the research brief this plays out.
 */

const HOME = '/Users/katrin'
const PROJECT = `${HOME}/Synaplan/projects/objekt-lindenstrasse-12`
const OUT = `${PROJECT}/out`

const models: DemoModels = {
  chat: 'anthropic:claude-sonnet-5:chat',
  voice: 'openai:gpt-4o-transcribe:sound2text',
  speak: '',
  vision: 'anthropic:claude-sonnet-5:pic2text',
  image: '',
  video: '',
  embed: 'ollama:bge-m3:vectorize',
  docs: 'openai:gpt-6-astra:analyze',
  chatLegacyProviderId: null,
}

export const immobilienExpose: DemoStoryboard = {
  title: 'Weber Immobilien, Hamburg — ein Objektordner wird zum Exposé.',
  scenario: {
    id: 'uc-d1-immobilien-expose',
    language: 'de',
    apiBaseUrl: 'https://web.synaplan.com',
    deviceName: 'MacBook von Katrin',
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
      name: 'Persönlich',
      kind: 'personal',
      enabledSkills: [],
      webSearch: false,
      models,
      projectDir: `${HOME}/Synaplan/projects/personal`,
      notesDir: `${HOME}/Synaplan/projects/personal/notes`,
      outDir: `${HOME}/Synaplan/projects/personal/out`,
    },
    project: {
      id: 'objekt-lindenstrasse-12',
      slug: 'objekt-lindenstrasse-12',
      name: 'Objekt Lindenstraße 12',
      kind: 'project',
      enabledSkills: ['docx', 'xlsx', 'email-draft', 'calendar-event', 'chart'],
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
    executionConsent: false,
    pickFiles: [
      { path: `${PROJECT}/unterlagen/Grundriss-EG-OG.pdf`, size: 1_842_211 },
      { path: `${PROJECT}/unterlagen/Energieausweis-2024.pdf`, size: 612_400 },
      { path: `${PROJECT}/unterlagen/Wohnflaechenberechnung.xlsx`, size: 24_118 },
      { path: `${PROJECT}/unterlagen/Objektbeschreibung-Eigentuemer.docx`, size: 88_902 },
    ],
    uploadMs: 650,
    wordMs: 22,
    turns: [
      {
        thinkMs: 1100,
        events: [
          {
            kind: 'step',
            tool: 'list_files',
            start: 'Listing unterlagen',
            end: 'Listed unterlagen (4 entries)',
            ms: 600,
          },
          {
            kind: 'step',
            tool: 'read_file',
            start: 'Reading xlsx/SKILL.md',
            end: 'Read xlsx/SKILL.md',
            ms: 500,
          },
          {
            kind: 'step',
            tool: 'run_program',
            start: 'Running python3',
            end: 'Ran python3',
            ms: 1300,
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
            start: 'Running python3',
            end: 'Ran python3',
            ms: 1400,
          },
          {
            kind: 'step',
            tool: 'write_file',
            start: 'Writing out/expose.md',
            end: 'Saved expose.md',
            artifact: `${OUT}/expose.md`,
            size: 6_210,
            ms: 900,
          },
          {
            kind: 'step',
            tool: 'run_program',
            start: 'Running python3',
            end: 'Ran python3',
            artifact: `${OUT}/Expose-Lindenstrasse-12.docx`,
            size: 214_330,
            ms: 1900,
          },
          {
            kind: 'step',
            tool: 'write_file',
            start: 'Writing out/eckdaten.json',
            end: 'Saved eckdaten.json',
            ms: 600,
          },
          {
            kind: 'step',
            tool: 'run_program',
            start: 'Running python3',
            end: 'Ran python3',
            artifact: `${OUT}/Eckdaten-Lindenstrasse-12.xlsx`,
            size: 18_442,
            ms: 1300,
          },
          {
            kind: 'text',
            text:
              'Fertig. Ich habe die vier Unterlagen gelesen und daraus zwei Dateien im Ordner **out** erstellt:\n\n' +
              '**Expose-Lindenstrasse-12.docx** — Lage (Hamburg-Rahlstedt, ruhige Wohnstraße, S-Bahn 9 Min.), Objektdaten, Ausstattung aus der Eigentümerbeschreibung, Energieabschnitt, Preis, Kontaktblock.\n\n' +
              '**Eckdaten-Lindenstrasse-12.xlsx** — ein Blatt mit allen Kennzahlen und einem Blatt „Flächen“ aus der Wohnflächenberechnung.\n\n' +
              '| Kennzahl | Wert |\n|---|---|\n| Wohnfläche | 142,3 m² (DIN 277) |\n| Grundstück | 480 m² |\n| Zimmer | 5 |\n| Baujahr | 1998, Dach 2021 saniert |\n| Energie | Klasse C, 89 kWh/(m²·a), Gas-Brennwert 2019 |\n| Kaufpreis | 689.000 € |\n| Preis je m² | 4.842 € |\n\n' +
              'Zwei Stellen habe ich im Exposé mit **[prüfen]** markiert: Die Eigentümerbeschreibung nennt „Fußbodenheizung im gesamten EG“, die Wohnflächenberechnung weist den Hauswirtschaftsraum ohne Heizkörper aus. Und der Energieausweis läuft am 14.03.2027 ab — bis zur Vermarktung reicht er.',
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
            ms: 450,
          },
          {
            kind: 'step',
            tool: 'write_file',
            start: 'Writing out/einladung.md',
            end: 'Saved einladung.md',
            ms: 700,
          },
          {
            kind: 'step',
            tool: 'run_program',
            start: 'Running python3',
            end: 'Ran python3',
            artifact: `${OUT}/Einladung-Besichtigung-Lindenstrasse-12.eml`,
            size: 5_120,
            ms: 1100,
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
            artifact: `${OUT}/Besichtigung-Lindenstrasse-12.ics`,
            size: 1_024,
            ms: 900,
          },
          {
            kind: 'text',
            text:
              'Beides liegt im Ordner **out**:\n\n' +
              '**Einladung-Besichtigung-Lindenstrasse-12.eml** — ein ungesendeter Entwurf. Ein Doppelklick öffnet ihn in Outlook oder Apple Mail; Empfänger sind noch leer, damit du die Interessentenliste als BCC einträgst. Betreff: „Besichtigung Lindenstraße 12 — Samstag, 3. Oktober, 11:00 Uhr“. Das Exposé ist als Anhang eingetragen.\n\n' +
              '**Besichtigung-Lindenstrasse-12.ics** — Samstag, 3. Oktober 2026, 11:00–12:30 Uhr, Ort: Lindenstraße 12, 22143 Hamburg. Ein Doppelklick trägt den Termin in deinen Kalender ein.\n\n' +
              'Direkt in deinen Outlook-Kalender schreiben kann ich von diesem Computer aus nicht — dafür ist die Kalenderdatei da.',
          },
        ],
      },
    ],
  },
  acts: [
    {
      kind: 'caption',
      text: 'Katrin Weber, Maklerin. Neues Objekt, vier Unterlagen vom Eigentümer, Exposé bis heute Abend.',
      holdMs: 3200,
    },
    {
      kind: 'addFiles',
      caption:
        'Sie legt die Unterlagen in die Wissensmappe des Projekts — Grundriss, Energieausweis, Flächenberechnung, Eigentümerbeschreibung.',
      waitReadyMs: 7000,
    },
    { kind: 'closePanel' },
    {
      kind: 'send',
      prompt:
        'Erstelle aus den Unterlagen ein Exposé als Word-Dokument (Lage, Objektdaten, Ausstattung, Energie, Preis) und eine Excel-Tabelle mit den Eckdaten. Kaufpreis: 689.000 €.',
      caption:
        'Eine Anfrage in ihren Worten. Beim ersten Skill-Lauf fragt der Desktop einmal, ob Skills auf diesem Computer laufen dürfen.',
      consent: true,
    },
    {
      kind: 'caption',
      text: 'Jeder Schritt ist sichtbar: Dateien lesen, Skill-Anleitung lesen, Python-Skript ausführen. Die Ergebnisse landen als echte Office-Dateien im Projektordner.',
      holdMs: 4500,
    },
    {
      kind: 'send',
      prompt:
        'Schreibe eine E-Mail an die Interessentenliste mit Einladung zur Besichtigung am Samstag, 3. Oktober 2026, 11:00 Uhr, und erstelle den Termin für meinen Kalender.',
      caption: 'Zweiter Auftrag: Einladungsmail und Kalendertermin — beides als Dateien, die Outlook öffnet.',
    },
    {
      kind: 'reveal',
      artifactIndex: 0,
      caption:
        '„Im Ordner anzeigen“ führt in zehn Sekunden zur Datei. Der Assistent sagt ehrlich, was er nicht kann: den Termin direkt in Outlook eintragen.',
    },
    { kind: 'wait', ms: 2500 },
  ],
}
