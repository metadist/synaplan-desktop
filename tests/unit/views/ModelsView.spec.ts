import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { messages } from '@/i18n'
import type { CatalogEntry, ModelCatalog, Project, ProjectModels } from '@/services/tauri'

vi.mock('@/services/tauri', () => ({
  MODEL_SLOTS: ['chat', 'voice', 'speak', 'vision', 'image', 'video', 'embed', 'docs'],
  listProjects: vi.fn(),
  applyDefaultModels: vi.fn().mockRejectedValue({ code: 'network', message: 'offline' }),
  updateProject: vi.fn(),
  getModelCatalog: vi.fn(),
  asCommandError: (e: unknown) =>
    e && typeof e === 'object' && 'code' in e ? e : { code: 'unexpected', message: String(e) },
}))

import ModelsView from '@/views/ModelsView.vue'
import { useProjectsStore } from '@/stores/projects'
import * as api from '@/services/tauri'

const EMPTY_MODELS: ProjectModels = {
  chat: '',
  voice: '',
  speak: '',
  vision: '',
  image: '',
  video: '',
  embed: '',
  docs: '',
  chatLegacyProviderId: null,
}

function project(id: string, models: Partial<ProjectModels> = {}): Project {
  return {
    id,
    slug: id,
    name: 'Work',
    kind: 'project',
    createdAt: '2026-09-10T00:00:00Z',
    updatedAt: '2026-09-10T00:00:00Z',
    dictationLanguage: 'en',
    defaultAssistantId: null,
    assistantIds: [],
    enabledSkills: [],
    models: { ...EMPTY_MODELS, ...models },
    knowledgeFolder: `DESKTOP:${id}`,
    projectDir: `/home/u/Synaplan/projects/${id}`,
    notesDir: `/home/u/Synaplan/projects/${id}/notes`,
    outDir: `/home/u/Synaplan/projects/${id}/out`,
  }
}

function entry(id: string, extra: Partial<CatalogEntry> = {}): CatalogEntry {
  const [service, providerId] = id.split(':')
  return {
    id,
    providerId,
    service,
    name: '',
    available: true,
    unavailableReason: null,
    ...extra,
  }
}

function catalog(partial: Partial<ModelCatalog['slots']> = {}, missing = false): ModelCatalog {
  const slots: ModelCatalog['slots'] = {
    chat: [],
    voice: [],
    speak: [],
    vision: [],
    image: [],
    video: [],
    embed: [],
    docs: [],
    ...partial,
  }
  const source = missing ? 'none' : 'catalog'
  return {
    slots,
    sources: {
      chat: source,
      voice: source,
      speak: source,
      vision: source,
      image: source,
      video: source,
      embed: source,
      docs: source,
    },
    catalogMissing: missing,
  }
}

const fullCatalog = catalog({
  chat: [
    entry('openai:gpt-4o-mini:chat'),
    entry('ollama:llama3.2:chat'),
    entry('anthropic:x-large:chat', {
      available: false,
      unavailableReason: 'Provider key missing',
    }),
  ],
  embed: [entry('ollama:bge-m3:vectorize')],
  docs: [entry('openai:gpt-4o:docs')],
})

async function factory(active: Project) {
  const pinia = createPinia()
  setActivePinia(pinia)
  const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages })
  vi.mocked(api.listProjects).mockResolvedValue({
    projects: [active],
    activeId: active.id,
    personalId: active.id,
  })
  await useProjectsStore().load()
  const wrapper = mount(ModelsView, { global: { plugins: [pinia, i18n] } })
  await flushPromises()
  return wrapper
}

describe('ModelsView', () => {
  beforeEach(() => {
    vi.mocked(api.listProjects).mockClear()
    vi.mocked(api.updateProject).mockReset()
    vi.mocked(api.getModelCatalog).mockReset()
    vi.mocked(api.getModelCatalog).mockResolvedValue({ catalog: fullCatalog, rebound: false })
  })

  it('renders all eight slots with the provider id, never the catalog key', async () => {
    const wrapper = await factory(project('p1', { chat: 'openai:gpt-4o-mini:chat' }))

    for (const slot of ['chat', 'voice', 'speak', 'vision', 'image', 'video', 'embed', 'docs']) {
      expect(wrapper.find(`[data-testid="slot-${slot}"]`).exists()).toBe(true)
    }
    const select = wrapper.get('[data-testid="slot-chat-select"]').element as HTMLSelectElement
    expect(select.value).toBe('openai:gpt-4o-mini:chat')
    const chatCard = wrapper.get('[data-testid="slot-chat"]')
    expect(chatCard.text()).toContain('gpt-4o-mini')
    expect(chatCard.text()).not.toContain('openai:gpt-4o-mini:chat')
    expect(chatCard.text()).not.toMatch(/DEFAULTMODEL|VECTORIZE|SOUND2TEXT/)
    expect(wrapper.text()).toContain('Data is processed only by the models you picked')
  })

  it('the "Stay in this world" chip explains itself when clicked', async () => {
    const wrapper = await factory(
      project('p1', {
        chat: 'openai:gpt-4o-mini:chat',
        voice: 'groq:whisper:voice',
        embed: 'ollama:bge-m3:vectorize',
        docs: 'openai:gpt-4o:docs',
      }),
    )

    const chip = wrapper.get('[data-testid="world-chip"]')
    expect(chip.text()).toContain('Stay in this world')
    expect(wrapper.find('[data-testid="world-card"]').exists()).toBe(false)

    await chip.trigger('click')
    expect(wrapper.get('[data-testid="world-card"]').text()).toContain(
      'stays between your computer and these AI models',
    )
    await chip.trigger('click')
    expect(wrapper.find('[data-testid="world-card"]').exists()).toBe(false)
  })

  it('persists a pick as the catalog key on the project', async () => {
    const p = project('p1')
    vi.mocked(api.updateProject).mockResolvedValue({
      ...p,
      models: { ...EMPTY_MODELS, docs: 'openai:gpt-4o:docs' },
    })
    const wrapper = await factory(p)

    await wrapper.get('[data-testid="slot-docs-select"]').setValue('openai:gpt-4o:docs')
    await flushPromises()

    expect(api.updateProject).toHaveBeenCalledWith('p1', {
      models: { ...EMPTY_MODELS, docs: 'openai:gpt-4o:docs' },
    })
  })

  it('locks Index files to the workspace search model', async () => {
    const wrapper = await factory(project('p1', { embed: 'ollama:bge-m3:vectorize' }))
    const select = wrapper.get('[data-testid="slot-embed-select"]').element as HTMLSelectElement
    expect(select.disabled).toBe(true)
    expect(select.value).toBe('ollama:bge-m3:vectorize')
    expect(wrapper.get('[data-testid="slot-embed"]').text()).toMatch(/workspace uses to search/i)
  })

  it('shows an unavailable model as disabled with its reason and never substitutes', async () => {
    const wrapper = await factory(project('p1', { chat: 'anthropic:x-large:chat' }))

    const option = wrapper
      .findAll('[data-testid="slot-chat-select"] option')
      .find((o) => (o.element as HTMLOptionElement).value === 'anthropic:x-large:chat')
    expect(option).toBeDefined()
    expect((option!.element as HTMLOptionElement).disabled).toBe(true)
    expect(wrapper.get('[data-testid="slot-chat-status"]').text()).toContain('Provider key missing')
    expect(api.updateProject).not.toHaveBeenCalled()
  })

  it('keeps a pick the workspace no longer offers visible instead of dropping it', async () => {
    const wrapper = await factory(project('p1', { chat: 'groq:gone-model:chat' }))

    const select = wrapper.get('[data-testid="slot-chat-select"]').element as HTMLSelectElement
    expect(select.value).toBe('groq:gone-model:chat')
    expect(wrapper.get('[data-testid="slot-chat-status"]').text()).toContain('gone-model')
    expect(api.updateProject).not.toHaveBeenCalled()
  })

  it('says so when the workspace has no catalog yet and keeps the picks', async () => {
    vi.mocked(api.getModelCatalog).mockResolvedValue({
      catalog: catalog({}, true),
      rebound: false,
    })
    const wrapper = await factory(project('p1', { chat: 'openai:gpt-4o-mini:chat' }))

    expect(wrapper.find('[data-testid="models-missing"]').exists()).toBe(true)
    const select = wrapper.get('[data-testid="slot-chat-select"]').element as HTMLSelectElement
    expect(select.value).toBe('openai:gpt-4o-mini:chat')
    expect(wrapper.get('[data-testid="slot-chat"]').text()).toContain('gpt-4o-mini')
  })

  it('remembers a flat-list chat pick as a legacy provider id', async () => {
    const flat = catalog({ chat: [entry('openai:gpt-4o-mini', { id: 'gpt-4o-mini' })] }, true)
    flat.sources.chat = 'flat_models'
    vi.mocked(api.getModelCatalog).mockResolvedValue({ catalog: flat, rebound: false })
    const p = project('p1')
    vi.mocked(api.updateProject).mockResolvedValue(p)
    const wrapper = await factory(p)

    await wrapper.get('[data-testid="slot-chat-select"]').setValue('gpt-4o-mini')
    await flushPromises()

    expect(api.updateProject).toHaveBeenCalledWith('p1', {
      models: { ...EMPTY_MODELS, chat: 'gpt-4o-mini', chatLegacyProviderId: 'gpt-4o-mini' },
    })
  })

  it('reloads the project when the catalog upgraded a legacy pick', async () => {
    vi.mocked(api.getModelCatalog).mockResolvedValue({ catalog: fullCatalog, rebound: true })
    await factory(project('p1', { chat: 'gpt-4o-mini', chatLegacyProviderId: 'gpt-4o-mini' }))

    expect(api.listProjects).toHaveBeenCalledTimes(2)
  })

  it('shows a retry banner when the catalog cannot be loaded', async () => {
    vi.mocked(api.getModelCatalog).mockRejectedValue({
      code: 'network',
      message: 'offline',
    })
    const wrapper = await factory(project('p1', { chat: 'openai:gpt-4o-mini:chat' }))

    expect(wrapper.find('[data-testid="models-error"]').exists()).toBe(true)
    const select = wrapper.get('[data-testid="slot-chat-select"]').element as HTMLSelectElement
    expect(select.value).toBe('openai:gpt-4o-mini:chat')

    vi.mocked(api.getModelCatalog).mockResolvedValue({ catalog: fullCatalog, rebound: false })
    await wrapper.get('[data-testid="models-error"] button').trigger('click')
    await flushPromises()
    expect(wrapper.find('[data-testid="models-error"]').exists()).toBe(false)
  })
})
