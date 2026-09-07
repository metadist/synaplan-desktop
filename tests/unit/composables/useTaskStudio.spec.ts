import { describe, expect, it } from 'vitest'
import {
  DEFAULT_STUDIO_SKILLS,
  STUDIO_TILE_LIMIT,
  resolveStudioTiles,
  toggleStudioPick,
} from '@/composables/useTaskStudio'
import type { SkillFilter } from '@/composables/useTaskStudio'

function skill(name: string, extra: Partial<SkillFilter> = {}): SkillFilter {
  return { name, description: name, enabled: true, blocked: false, ...extra }
}

describe('useTaskStudio', () => {
  it('shows the three default examples when they are ready', () => {
    const cards = resolveStudioTiles(
      [skill('email-draft'), skill('calendar-event'), skill('vcard'), skill('slides')],
      [],
    )
    expect(cards.map((c) => c.skill)).toEqual([...DEFAULT_STUDIO_SKILLS])
    expect(cards).toHaveLength(STUDIO_TILE_LIMIT)
  })

  it('prefers the user picks, then fills from defaults', () => {
    const cards = resolveStudioTiles(
      [skill('email-draft'), skill('slides'), skill('invoice'), skill('vcard')],
      ['slides', 'invoice'],
    )
    expect(cards.map((c) => c.skill)).toEqual(['slides', 'invoice', 'email-draft'])
  })

  it('skips disabled and blocked skills', () => {
    const cards = resolveStudioTiles(
      [
        skill('email-draft', { enabled: false }),
        skill('calendar-event', { blocked: true }),
        skill('vcard'),
        skill('slides'),
      ],
      [],
    )
    expect(cards.map((c) => c.skill)).toEqual(['vcard', 'slides'])
  })

  it('caps a toggle at three tiles', () => {
    expect(toggleStudioPick(['a', 'b', 'c'], 'd')).toEqual(['a', 'b', 'c'])
    expect(toggleStudioPick(['a', 'b'], 'c')).toEqual(['a', 'b', 'c'])
    expect(toggleStudioPick(['a', 'b', 'c'], 'b')).toEqual(['a', 'c'])
  })

  it('falls back to the skill name for a community skill', () => {
    const cards = resolveStudioTiles([skill('acme-report')], [])
    expect(cards).toEqual([{ id: 'acme-report', skill: 'acme-report' }])
  })
})
