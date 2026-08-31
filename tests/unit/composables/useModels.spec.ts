import { describe, expect, it } from 'vitest'
import { chatModelGroups, defaultChatModel, isChatModel } from '@/composables/useModels'
import type { ModelInfo } from '@/services/tauri'

describe('useModels', () => {
  it('keeps chat models and drops utility/stub models', () => {
    expect(isChatModel('gpt-4o-mini')).toBe(true)
    expect(isChatModel('claude-sonnet-5')).toBe(true)
    expect(isChatModel('grok-4.6')).toBe(true)
    expect(isChatModel('text-embedding-3-small')).toBe(false)
    expect(isChatModel('whisper-large-v3')).toBe(false)
    expect(isChatModel('gpt-image-1')).toBe(false)
    expect(isChatModel('veo-3.1-generate-preview')).toBe(false)
    expect(isChatModel('stub-chat-model')).toBe(false)
    expect(isChatModel('test-model')).toBe(false)
  })

  it('dedupes and groups by provider, sorted', () => {
    const models: ModelInfo[] = [
      { id: 'gpt-4o-mini', provider: 'openai' },
      { id: 'gpt-4o-mini', provider: 'openai' }, // duplicate
      { id: 'claude-sonnet-5', provider: 'anthropic' },
      { id: 'text-embedding-3-small', provider: 'openai' }, // filtered out
      { id: 'grok-4.6', provider: 'xai' },
    ]
    const groups = chatModelGroups(models)
    expect(groups.map((g) => g.provider)).toEqual(['anthropic', 'openai', 'xai'])
    const openai = groups.find((g) => g.provider === 'openai')!
    expect(openai.models).toHaveLength(1)
    expect(openai.models[0].id).toBe('gpt-4o-mini')
  })

  it('picks a sensible default model, preferring broadly-available ones', () => {
    const groups = chatModelGroups([
      { id: 'gemini-2.5-pro', provider: 'google' },
      { id: 'gpt-4o-mini', provider: 'openai' },
    ])
    expect(defaultChatModel(groups)).toBe('gpt-4o-mini')
    expect(defaultChatModel([])).toBe('')
  })
})
