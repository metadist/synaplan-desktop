/** One empty-chat tile. `id` matches `chat.studio.cards.{id}` when we have copy. */
export interface TaskCard {
  id: string
  skill: string
}

export const STUDIO_TILE_LIMIT = 3

/** First-run examples: Outlook-openable files the user can double-click. */
export const DEFAULT_STUDIO_SKILLS = ['email-draft', 'calendar-event', 'vcard'] as const

export const TASK_CATALOG: TaskCard[] = [
  { id: 'followupEmail', skill: 'email-draft' },
  { id: 'meetingInvite', skill: 'calendar-event' },
  { id: 'saveContact', skill: 'vcard' },
  { id: 'invoice', skill: 'invoice' },
  { id: 'slides', skill: 'slides' },
  { id: 'brief', skill: 'web-report' },
  { id: 'spreadsheet', skill: 'csv-insights' },
  { id: 'chart', skill: 'chart' },
  { id: 'table', skill: 'data-table' },
]

export interface SkillFilter {
  name: string
  description: string
  enabled: boolean
  blocked: boolean
}

export function catalogCardForSkill(skill: string): TaskCard | undefined {
  return TASK_CATALOG.find((card) => card.skill === skill)
}

export function cardForSkill(skill: SkillFilter): TaskCard {
  return catalogCardForSkill(skill.name) ?? { id: skill.name, skill: skill.name }
}

export function hasStudioCopy(card: TaskCard): boolean {
  return TASK_CATALOG.some((c) => c.id === card.id)
}

export function readySkills(skills: SkillFilter[]): SkillFilter[] {
  return skills.filter((s) => s.enabled && !s.blocked)
}

/**
 * At most three tiles: the user's saved picks first, then the default
 * examples, then any other ready skill.
 */
export function resolveStudioTiles(skills: SkillFilter[], preferred: string[]): TaskCard[] {
  const ready = readySkills(skills)
  const readyNames = new Set(ready.map((s) => s.name))
  const chosen: string[] = []
  for (const name of [...preferred, ...DEFAULT_STUDIO_SKILLS, ...ready.map((s) => s.name)]) {
    if (!readyNames.has(name) || chosen.includes(name)) {
      continue
    }
    chosen.push(name)
    if (chosen.length >= STUDIO_TILE_LIMIT) {
      break
    }
  }
  return chosen.map((name) => cardForSkill(ready.find((s) => s.name === name)!))
}

export function toggleStudioPick(current: string[], skill: string): string[] {
  if (current.includes(skill)) {
    return current.filter((name) => name !== skill)
  }
  if (current.length >= STUDIO_TILE_LIMIT) {
    return current
  }
  return [...current, skill]
}
