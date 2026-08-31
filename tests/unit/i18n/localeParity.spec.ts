import { describe, expect, it } from 'vitest'
import { messages, supportedLanguages } from '@/i18n'

function flatten(obj: Record<string, unknown>, prefix = ''): string[] {
  const keys: string[] = []
  for (const [key, value] of Object.entries(obj)) {
    const full = prefix ? `${prefix}.${key}` : key
    if (value && typeof value === 'object' && !Array.isArray(value)) {
      keys.push(...flatten(value as Record<string, unknown>, full))
    } else {
      keys.push(full)
    }
  }
  return keys.sort()
}

const all = messages as Record<string, Record<string, unknown>>
const enKeys = flatten(all.en)

describe('i18n locale parity', () => {
  for (const lang of supportedLanguages) {
    it(`${lang} has exactly the same keys as en`, () => {
      expect(flatten(all[lang])).toEqual(enKeys)
    })
  }

  it('keeps the {url} placeholder in every locale', () => {
    for (const lang of supportedLanguages) {
      const status = all[lang].status as Record<string, string>
      expect(status.connectedTo).toContain('{url}')
    }
  })
})
