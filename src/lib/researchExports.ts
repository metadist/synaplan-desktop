/** Fixed instructions that turn a saved report into another file through a skill. */

export type ResearchExport = 'docx' | 'pptx' | 'xlsx' | 'email-draft' | 'web-report'

const SKILL: Record<ResearchExport, string> = {
  docx: 'docx',
  pptx: 'pptx',
  xlsx: 'xlsx',
  'email-draft': 'email-draft',
  'web-report': 'web-report',
}

const ASK: Record<ResearchExport, string> = {
  docx: 'Make a Word document from this report.',
  pptx: 'Make slides from this report.',
  xlsx: 'Make a table from this report.',
  'email-draft': 'Draft an email from this report.',
  'web-report': 'Make a web page from this report.',
}

export function exportSkill(kind: ResearchExport): string {
  return SKILL[kind]
}

export function exportInstruction(
  kind: ResearchExport,
  reportPath: string,
  version: number,
): string {
  return `${ASK[kind]} Use the report file at ${reportPath}. This was made from version ${version} of the report. Do not invent sources that are not in that file.`
}

export function exportBlock(
  skill: { enabled: boolean; blocked: boolean; blockedReason: string | null } | undefined,
): string | null {
  if (!skill || !skill.enabled || skill.blocked) {
    return skill?.blockedReason || 'research.exportBlocked'
  }
  return null
}
