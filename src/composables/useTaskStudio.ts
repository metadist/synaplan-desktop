export type TaskGroup = 'outlook' | 'documents' | 'data'

/** One first-run card. `id` matches `chat.studio.cards.{id}` i18n keys. */
export interface TaskCard {
  id: string
  skill: string
  group: TaskGroup
}

export const TASK_GROUPS: TaskGroup[] = ['outlook', 'documents', 'data']

export const TASK_CATALOG: TaskCard[] = [
  { id: 'followupEmail', skill: 'email-draft', group: 'outlook' },
  { id: 'meetingInvite', skill: 'calendar-event', group: 'outlook' },
  { id: 'saveContact', skill: 'vcard', group: 'outlook' },
  { id: 'invoice', skill: 'invoice', group: 'documents' },
  { id: 'slides', skill: 'slides', group: 'documents' },
  { id: 'brief', skill: 'web-report', group: 'documents' },
  { id: 'spreadsheet', skill: 'csv-insights', group: 'data' },
  { id: 'chart', skill: 'chart', group: 'data' },
  { id: 'table', skill: 'data-table', group: 'data' },
]

export interface SkillFilter {
  name: string
  enabled: boolean
  blocked: boolean
}

/** Cards whose skill is enabled and not blocked. The Later strip is not a card. */
export function visibleTaskCards(skills: SkillFilter[]): TaskCard[] {
  const ready = new Set(skills.filter((s) => s.enabled && !s.blocked).map((s) => s.name))
  return TASK_CATALOG.filter((card) => ready.has(card.skill))
}

export interface TaskGroupSection {
  group: TaskGroup
  cards: TaskCard[]
}

export function groupTaskCards(cards: TaskCard[]): TaskGroupSection[] {
  return TASK_GROUPS.map((group) => ({
    group,
    cards: cards.filter((card) => card.group === group),
  })).filter((section) => section.cards.length > 0)
}
