import { describe, expect, it } from 'vitest'
import { groupTaskCards, visibleTaskCards, TASK_CATALOG } from '@/composables/useTaskStudio'
import type { SkillFilter } from '@/composables/useTaskStudio'

function skill(name: string, extra: Partial<SkillFilter> = {}): SkillFilter {
  return { name, enabled: true, blocked: false, ...extra }
}

describe('useTaskStudio', () => {
  it('keeps only enabled, unblocked skills', () => {
    const cards = visibleTaskCards([
      skill('email-draft'),
      skill('vcard', { enabled: false }),
      skill('pptx', { blocked: true }),
      skill('slides'),
    ])
    expect(cards.map((c) => c.skill)).toEqual(['email-draft', 'slides'])
    expect(cards.some((c) => c.skill === 'pptx')).toBe(false)
  })

  it('omits the catalog when nothing is ready', () => {
    expect(visibleTaskCards([])).toEqual([])
    expect(visibleTaskCards([skill('hello-files')])).toEqual([])
  })

  it('does not treat the Later strip as a skill filter', () => {
    expect(TASK_CATALOG.every((c) => c.skill.length > 0)).toBe(true)
    expect(TASK_CATALOG.some((c) => c.id === 'later')).toBe(false)
  })

  it('groups visible cards and drops empty groups', () => {
    const sections = groupTaskCards(visibleTaskCards([skill('chart'), skill('email-draft')]))
    expect(sections.map((s) => s.group)).toEqual(['outlook', 'data'])
    expect(sections[0].cards).toHaveLength(1)
    expect(sections[1].cards[0].id).toBe('chart')
  })
})
