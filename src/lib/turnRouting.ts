/**
 * Whether this turn needs the file tools (list, read, write) without a skill.
 *
 * A named file routes the turn either way: one that exists is an input, one
 * that does not is a requested output. A mention of the project folder or the
 * out-box does too. Attachments and drops are not signals — they are uploaded
 * to the knowledge folder and never appear in this text.
 */

const FILE_EXTENSIONS = [
  'md',
  'txt',
  'json',
  'csv',
  'html',
  'htm',
  'doc',
  'docx',
  'xls',
  'xlsx',
  'ppt',
  'pptx',
  'pdf',
  'eml',
  'ics',
  'vcf',
] as const

/** Folder words in the five UI languages, plus the product name "out-box". */
const FOLDER_PHRASES = [
  'project folder',
  'out-box',
  'outbox',
  'projektordner',
  'carpeta del proyecto',
  'bandeja de salida',
  'dossier du projet',
  'boîte de sortie',
  'boite de sortie',
  'proje klasör',
  'çıktı kutusu',
]

const EXTENSION_PATTERN = new RegExp(
  `(?:^|[\\s"'“”«»(\\[])([^\\s"'()[\\]<>]+)\\.(${FILE_EXTENSIONS.join('|')})(?=$|[\\s"'.,;:!?)\\]])`,
  'gi',
)

export function namedFiles(text: string): string[] {
  const found: string[] = []
  for (const match of text.matchAll(EXTENSION_PATTERN)) {
    const stem = match[1] ?? ''
    if (stem.includes('://') || stem.toLowerCase().startsWith('www.')) {
      continue
    }
    const name = `${stem}.${match[2]}`
    if (!found.some((have) => have.toLowerCase() === name.toLowerCase())) {
      found.push(name)
    }
  }
  return found
}

export function refersToProjectFolder(text: string): boolean {
  const lower = text.toLocaleLowerCase()
  return FOLDER_PHRASES.some((phrase) => lower.includes(phrase.toLocaleLowerCase()))
}

export function turnNeedsFileTools(text: string): boolean {
  return namedFiles(text).length > 0 || refersToProjectFolder(text)
}
