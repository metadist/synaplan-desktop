import { describe, expect, it } from 'vitest'
import { worldMismatches } from '@/composables/useAssistantWorld'
import type { Assistant, ProjectModels } from '@/services/tauri'

const models: ProjectModels = {
  chat: 'ollama:llama3.2:chat',
  voice: '',
  speak: '',
  vision: '',
  image: '',
  video: '',
  embed: 'ollama:bge-m3:vectorize',
  docs: '',
  chatLegacyProviderId: null,
}

function assistant(recipe: Assistant['models']): Assistant {
  return { id: 1, name: 'A', description: null, icon: null, status: null, models: recipe }
}

describe('worldMismatches', () => {
  it('is silent when the recipe names exactly the project models', () => {
    const a = assistant({
      chat: 'ollama:llama3.2:chat',
      vision: null,
      vectorize: 'ollama:bge-m3:vectorize',
    })
    expect(worldMismatches(a, models)).toEqual([])
  })

  it('flags a recipe key that differs from the project binding', () => {
    const a = assistant({
      chat: 'anthropic:claude-sonnet-4:chat',
      vision: null,
      vectorize: 'ollama:bge-m3:vectorize',
    })
    expect(worldMismatches(a, models)).toEqual([
      { slot: 'chat', recipe: 'anthropic:claude-sonnet-4:chat' },
    ])
  })

  it('flags a workspace-default key when the project made its own choice', () => {
    const a = assistant({ chat: null, vision: null, vectorize: null })
    expect(worldMismatches(a, models)).toEqual([
      { slot: 'chat', recipe: null },
      { slot: 'embed', recipe: null },
    ])
  })

  it('says nothing when the workspace did not send recipe models', () => {
    expect(worldMismatches(assistant(null), models)).toEqual([])
  })
})
