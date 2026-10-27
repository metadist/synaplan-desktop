import { describe, expect, it } from 'vitest'
import { messages, supportedLanguages } from '@/i18n'

const banned = [
  'Claude',
  'Anthropic',
  'MCP',
  'shell',
  'Bash',
  'tool_use',
  'sk_',
  'lease',
  'Tauri',
  'protocol',
  'capability',
  'evidence card',
  'ledger',
  'sorter',
  'reader',
  'lead',
  'checker',
  'reflect',
  'token',
  'RAG',
  'embedding',
  'rerank',
  'prompt',
]

function values(node: unknown): string[] {
  if (typeof node === 'string') {
    return [node]
  }
  if (!node || typeof node !== 'object') {
    return []
  }
  return Object.values(node).flatMap(values)
}

describe('research copy', () => {
  it('uses the same placeholders in every language', () => {
    const packs = supportedLanguages.map((lang) => {
      const research = (messages as Record<string, { research?: unknown }>)[lang].research
      return values(research).join('\n')
    })
    const names = (text: string) =>
      [...text.matchAll(/\{([a-z]+)\}/g)].map((match) => match[1]).sort()
    const english = names(packs[0])
    for (const pack of packs.slice(1)) {
      expect(names(pack)).toEqual(english)
    }
  })

  it('keeps internal words out of the research sentences', () => {
    for (const lang of supportedLanguages) {
      const research = (messages as Record<string, { research?: unknown }>)[lang].research
      const text = values(research).join('\n')
      for (const word of banned) {
        const pattern =
          word.includes(' ') || word.includes('_')
            ? new RegExp(word.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'i')
            : new RegExp(`\\b${word}\\b`, 'i')
        expect(text, `${lang} contains ${word}`).not.toMatch(pattern)
      }
    }
  })
})
