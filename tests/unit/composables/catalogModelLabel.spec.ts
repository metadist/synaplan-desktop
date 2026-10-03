import { describe, expect, it } from 'vitest'
import { catalogModelLabel } from '@/composables/useModelCatalog'
import type { CatalogEntry } from '@/services/tauri'

function entry(id: string, providerId: string, name: string): CatalogEntry {
  return {
    id,
    providerId,
    service: 'chat',
    name,
    available: true,
    unavailableReason: null,
  }
}

describe('catalogModelLabel', () => {
  const entries = [
    entry('anthropic:claude-sonnet-5:chat', 'claude-sonnet-5', 'Sonnet'),
    entry('openai:gpt-oss-120b:chat', 'gpt-oss-120b', 'GPT OSS'),
    entry('other:gpt-oss-120b:fast', 'gpt-oss-120b', 'GPT OSS fast'),
  ]

  it('uses the catalog name for an exact id', () => {
    expect(catalogModelLabel(entries, 'anthropic:claude-sonnet-5:chat')).toBe('Sonnet')
  })

  it('uses the name when a bare provider id matches once', () => {
    expect(catalogModelLabel(entries, 'claude-sonnet-5')).toBe('Sonnet')
  })

  it('keeps the provider id when several entries share it', () => {
    expect(catalogModelLabel(entries, 'gpt-oss-120b')).toBe('gpt-oss-120b')
  })
})
