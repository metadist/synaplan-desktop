import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { messages } from '@/i18n'

// Capture the event callbacks the view registers so a test can drive them.
const h = vi.hoisted(() => ({
  tokenCb: null as ((t: string) => void) | null,
  doneCb: null as (() => void) | null,
  errorCb: null as ((e: { code: string; message: string }) => void) | null,
}))

vi.mock('@/services/tauri', () => ({
  onChatToken: vi.fn(async (cb: (t: string) => void) => {
    h.tokenCb = cb
    return () => {}
  }),
  onChatDone: vi.fn(async (cb: () => void) => {
    h.doneCb = cb
    return () => {}
  }),
  onChatError: vi.fn(async (cb: (e: { code: string; message: string }) => void) => {
    h.errorCb = cb
    return () => {}
  }),
  onAgentText: vi.fn(async () => () => {}),
  onAgentTool: vi.fn(async () => () => {}),
  onAgentDone: vi.fn(async () => () => {}),
  onAgentError: vi.fn(async () => () => {}),
  listProjects: vi.fn(),
  setActiveProject: vi.fn(),
  listChats: vi.fn().mockResolvedValue([]),
  newChat: vi.fn(async (projectId: string) => ({
    id: 'c-new',
    projectId,
    title: '',
    createdAt: '2026-09-10T00:00:00Z',
    updatedAt: '2026-09-10T00:00:00Z',
    assistantId: null,
    messages: [],
  })),
  loadChat: vi.fn(),
  saveChat: vi.fn().mockResolvedValue(undefined),
  deleteChat: vi.fn().mockResolvedValue(undefined),
  getStudioTiles: vi.fn().mockResolvedValue([]),
  setStudioTiles: vi.fn(async (tiles: string[]) => tiles),
  sendChat: vi.fn().mockResolvedValue(undefined),
  sendAgentChat: vi.fn().mockResolvedValue(undefined),
  listAssistants: vi.fn().mockResolvedValue([]),
  cancelChat: vi.fn().mockResolvedValue(undefined),
  listSkills: vi.fn().mockResolvedValue([]),
  getExecutionConsent: vi.fn().mockResolvedValue(false),
  setExecutionConsent: vi.fn().mockResolvedValue(undefined),
  revealPath: vi.fn().mockResolvedValue(undefined),
  getStatus: vi.fn().mockResolvedValue({
    paired: false,
    apiBaseUrl: null,
    deviceId: null,
    keyBackend: 'memory',
    keyIsPlaintext: false,
  }),
  asCommandError: (e: unknown) =>
    e && typeof e === 'object' && 'code' in e ? e : { code: 'unexpected', message: String(e) },
}))

import ChatView from '@/views/ChatView.vue'
import * as api from '@/services/tauri'
import type { Project, Skill } from '@/services/tauri'
import { useProjectsStore } from '@/stores/projects'

function project(id: string, name: string, chat: string): Project {
  return {
    id,
    slug: name.toLowerCase(),
    name,
    kind: 'project',
    createdAt: '2026-09-10T00:00:00Z',
    updatedAt: '2026-09-10T00:00:00Z',
    dictationLanguage: 'en',
    defaultAssistantId: null,
    assistantIds: [],
    enabledSkills: [],
    models: {
      chat,
      voice: '',
      speak: '',
      vision: '',
      image: '',
      video: '',
      embed: '',
      docs: '',
      chatLegacyProviderId: null,
    },
    knowledgeFolder: `DESKTOP:${id}`,
    projectDir: `/home/u/Synaplan/projects/${name.toLowerCase()}`,
    notesDir: `/home/u/Synaplan/projects/${name.toLowerCase()}/notes`,
    outDir: `/home/u/Synaplan/projects/${name.toLowerCase()}/out`,
  }
}

const withModel = project('p1', 'Work', 'openai:gpt-4o-mini:chat')
// The project overlay: only these installed skills may run in this project.
withModel.enabledSkills = [
  'email-draft',
  'calendar-event',
  'vcard',
  'slides',
  'pptx',
  'invoice',
  'chart',
]
const withoutModel = project('p2', 'Bare', '')

function skill(name: string, extra: Partial<Skill> = {}): Skill {
  return {
    name,
    description: name,
    dir: `/tmp/${name}`,
    bundled: true,
    enabled: true,
    source: 'bundled',
    license: 'Apache-2.0',
    compatibilityWarning: false,
    allowUnattended: false,
    blocked: false,
    blockedReason: null,
    version: null,
    url: null,
    sha: null,
    needsPython: true,
    needsNode: false,
    needsLibreoffice: false,
    pythonImports: [],
    ...extra,
  }
}

async function factory(
  active: Project = withModel,
  projects: Project[] = [withModel, withoutModel],
) {
  const pinia = createPinia()
  setActivePinia(pinia)
  const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages })
  vi.mocked(api.listProjects).mockResolvedValue({
    projects,
    activeId: active.id,
    personalId: projects[0].id,
  })
  const store = useProjectsStore()
  await store.load()
  return mount(ChatView, {
    global: { plugins: [pinia, i18n], stubs: { DictationButton: DictationStub } },
  })
}

/** Stands in for the real mic button: the test emits the take events itself. */
const DictationStub = {
  name: 'DictationButton',
  props: ['projectId', 'disabled'],
  emits: ['start', 'interim', 'done', 'error'],
  template: '<button type="button" data-testid="dictation-toggle"></button>',
}

describe('ChatView', () => {
  beforeEach(() => {
    vi.mocked(api.sendChat).mockClear()
    vi.mocked(api.sendAgentChat).mockClear()
    vi.mocked(api.saveChat).mockClear()
    vi.mocked(api.newChat).mockClear()
    vi.mocked(api.listChats).mockResolvedValue([])
    vi.mocked(api.listSkills).mockResolvedValue([])
    vi.mocked(api.getStudioTiles).mockResolvedValue([])
    vi.mocked(api.setStudioTiles).mockClear()
  })

  it('renders streamed tokens into an assistant message', async () => {
    const wrapper = await factory()
    await flushPromises()

    await wrapper.find('textarea').setValue('Ping')
    await wrapper.find('button.btn-primary').trigger('click')
    await flushPromises()

    h.tokenCb?.('PO')
    h.tokenCb?.('NG')
    await flushPromises()

    expect(wrapper.text()).toContain('PONG')
  })

  it('sends inside the active project and never picks a model itself', async () => {
    const wrapper = await factory()
    await flushPromises()

    expect(wrapper.get('[data-testid="chat-model-chip"]').text()).toContain('gpt-4o-mini')
    expect(wrapper.get('[data-testid="chat-model-chip"]').text()).not.toContain('openai:')

    await wrapper.find('textarea').setValue('Ping')
    await wrapper.find('button.btn-primary').trigger('click')
    await flushPromises()

    expect(api.sendChat).toHaveBeenCalledWith('p1', [{ role: 'user', content: 'Ping' }], null)
  })

  it('shows the bound Assistant and pins the thread pick on the wire', async () => {
    vi.mocked(api.listAssistants).mockResolvedValue([
      {
        id: 7,
        name: 'Writer',
        description: null,
        icon: null,
        status: 'published',
        models: null,
      },
      {
        id: 9,
        name: 'Reviewer',
        description: null,
        icon: null,
        status: 'published',
        models: null,
      },
    ])
    const bound = { ...withModel, assistantIds: [7, 9], defaultAssistantId: 7 }
    const wrapper = await factory(bound, [bound])
    await flushPromises()

    const select = wrapper.get('[data-testid="chat-assistant-select"]')
    expect(select.text()).toContain('Project default (Writer)')
    expect(select.text()).toContain('Reviewer')

    await select.setValue('9')
    await wrapper.find('textarea').setValue('Ping')
    await wrapper.find('button.btn-primary').trigger('click')
    await flushPromises()

    // The pin travels as an id for the Rust side; the model is still the project's.
    expect(api.sendChat).toHaveBeenCalledWith('p1', [{ role: 'user', content: 'Ping' }], 9)
    expect(vi.mocked(api.saveChat).mock.calls[0][0].assistantId).toBe(9)
    expect(wrapper.get('[data-testid="chat-model-chip"]').text()).toContain('gpt-4o-mini')
  })

  it('only counts skills the project enabled as active', async () => {
    vi.mocked(api.listSkills).mockResolvedValue([skill('slides'), skill('csv-insights')])
    const narrowed = { ...withModel, enabledSkills: ['slides'] }
    const wrapper = await factory(narrowed, [narrowed])
    await flushPromises()
    expect(wrapper.get('.skills-pill').text()).toContain('1 skill')

    const none = { ...withModel, enabledSkills: [] }
    const bare = await factory(none, [none])
    await flushPromises()
    expect(bare.find('.skills-pill').exists()).toBe(false)
    await bare.find('textarea').setValue('Ping')
    await bare.find('button.btn-primary').trigger('click')
    await flushPromises()
    // Without an overlay the turn is a plain chat, never an agent turn.
    expect(api.sendAgentChat).not.toHaveBeenCalled()
    expect(api.sendChat).toHaveBeenCalled()
  })

  it('shows a single bound Assistant as a chip that leads to Agents', async () => {
    vi.mocked(api.listAssistants).mockResolvedValue([
      { id: 7, name: 'Writer', description: null, icon: null, status: null, models: null },
    ])
    const bound = { ...withModel, assistantIds: [7], defaultAssistantId: 7 }
    const wrapper = await factory(bound, [bound])
    await flushPromises()

    expect(wrapper.find('[data-testid="chat-assistant-select"]').exists()).toBe(false)
    expect(wrapper.get('[data-testid="chat-assistant-chip"]').text()).toContain('Writer')
  })

  it('blocks sending when the project has no Chat model', async () => {
    const wrapper = await factory(withoutModel)
    await flushPromises()

    expect(wrapper.find('[data-testid="chat-no-model"]').exists()).toBe(true)
    expect((wrapper.get('textarea').element as HTMLTextAreaElement).disabled).toBe(true)
    await wrapper.find('textarea').setValue('Ping')
    await wrapper.find('button.btn-primary').trigger('click')
    await flushPromises()
    expect(api.sendChat).not.toHaveBeenCalled()
  })

  it('persists the thread on send and again when the answer completes', async () => {
    vi.mocked(api.listChats).mockResolvedValue([
      {
        id: 'c-new',
        projectId: 'p1',
        title: 'Ping',
        createdAt: '2026-09-10T00:00:00Z',
        updatedAt: '2026-09-10T00:00:01Z',
        messageCount: 2,
        assistantId: null,
      },
    ])
    const wrapper = await factory()
    await flushPromises()

    await wrapper.find('textarea').setValue('Ping')
    await wrapper.find('button.btn-primary').trigger('click')
    await flushPromises()
    expect(api.newChat).toHaveBeenCalledWith('p1')
    expect(api.saveChat).toHaveBeenCalledTimes(1)

    h.tokenCb?.('PONG')
    h.doneCb?.()
    await flushPromises()

    expect(api.saveChat).toHaveBeenCalledTimes(2)
    const saved = vi.mocked(api.saveChat).mock.calls[1][0]
    expect(saved.projectId).toBe('p1')
    expect(saved.messages.map((m) => m.role)).toEqual(['user', 'assistant'])
    expect(saved.messages[1].model).toBe('openai:gpt-4o-mini:chat')
    expect(saved.messages[0].model).toBe('')
    expect(JSON.stringify(saved)).not.toContain('sk_')
    expect(wrapper.get('[data-testid="chat-threads"]').text()).toContain('Ping')
  })

  it('opens a saved thread and reloads the list on project switch', async () => {
    vi.mocked(api.listChats).mockResolvedValueOnce([
      {
        id: 'c1',
        projectId: 'p1',
        title: 'Old',
        createdAt: '2026-09-10T00:00:00Z',
        updatedAt: '2026-09-10T00:00:00Z',
        messageCount: 2,
        assistantId: null,
      },
    ])
    vi.mocked(api.loadChat).mockResolvedValue({
      id: 'c1',
      projectId: 'p1',
      title: 'Old',
      createdAt: '2026-09-10T00:00:00Z',
      updatedAt: '2026-09-10T00:00:00Z',
      assistantId: null,
      messages: [
        { role: 'user', content: 'Earlier question', model: '', createdAt: '' },
        { role: 'assistant', content: 'Earlier answer', model: 'x', createdAt: '' },
      ],
    })
    const wrapper = await factory()
    await flushPromises()

    await wrapper.get('[data-testid="chat-thread-c1"]').trigger('click')
    await flushPromises()
    expect(api.loadChat).toHaveBeenCalledWith('p1', 'c1')
    expect(wrapper.text()).toContain('Earlier answer')

    vi.mocked(api.listChats).mockResolvedValueOnce([])
    vi.mocked(api.setActiveProject).mockResolvedValue({
      projects: [withModel, withoutModel],
      activeId: 'p2',
      personalId: 'p1',
    })
    await useProjectsStore().select('p2')
    await flushPromises()
    expect(api.listChats).toHaveBeenLastCalledWith('p2')
    expect(wrapper.text()).not.toContain('Earlier answer')
  })

  it('shows the disconnected copy on an unauthorized stream error', async () => {
    const wrapper = await factory()
    await flushPromises()

    h.errorCb?.({ code: 'unauthorized', message: 'gone' })
    await flushPromises()

    expect(wrapper.find('.banner-error').text()).toBe(messages.en.errors.unauthorized)
  })

  it('shows three default example tiles for ready skills', async () => {
    vi.mocked(api.listSkills).mockResolvedValueOnce([
      skill('email-draft'),
      skill('calendar-event'),
      skill('vcard'),
      skill('slides'),
      skill('pptx', { blocked: true }),
    ])
    const wrapper = await factory()
    await flushPromises()

    expect(wrapper.get('[data-testid="task-studio"]').text()).toContain('What can I do here')
    expect(wrapper.text()).toContain('Follow up in Outlook')
    expect(wrapper.text()).toContain('Put it on the calendar')
    expect(wrapper.text()).toContain('Save a contact')
    expect(wrapper.text()).not.toContain('Pitch it in five slides')
    expect(wrapper.text()).not.toContain('While you are away')
    expect(wrapper.findAll('.studio-card')).toHaveLength(3)
  })

  it('fills the composer from a card without sending', async () => {
    vi.mocked(api.listSkills).mockResolvedValueOnce([skill('email-draft')])
    const wrapper = await factory()
    await flushPromises()

    await wrapper.get('[data-task="followupEmail"]').trigger('click')

    const textarea = wrapper.get('textarea').element as HTMLTextAreaElement
    expect(textarea.value).toContain('.eml')
    expect(api.sendAgentChat).not.toHaveBeenCalled()
    expect(api.sendChat).not.toHaveBeenCalled()
  })

  it('shows the user-saved example tiles', async () => {
    vi.mocked(api.getStudioTiles).mockResolvedValueOnce(['slides', 'invoice', 'chart'])
    vi.mocked(api.listSkills).mockResolvedValueOnce([
      skill('email-draft'),
      skill('slides'),
      skill('invoice'),
      skill('chart'),
    ])
    const wrapper = await factory()
    await flushPromises()

    expect(wrapper.text()).toContain('Pitch it in five slides')
    expect(wrapper.text()).toContain('Send a clean invoice')
    expect(wrapper.text()).toContain('Chart these numbers')
    expect(wrapper.text()).not.toContain('Follow up in Outlook')
    expect(wrapper.findAll('.studio-card')).toHaveLength(3)
  })

  it('lets the user change which example tiles are shown', async () => {
    vi.mocked(api.listSkills).mockResolvedValueOnce([
      skill('email-draft'),
      skill('calendar-event'),
      skill('vcard'),
      skill('slides'),
    ])
    const wrapper = await factory()
    await flushPromises()

    await wrapper.get('[data-testid="btn-choose-tiles"]').trigger('click')
    const slides = wrapper.get('[data-skill="slides"] input')
    expect((slides.element as HTMLInputElement).disabled).toBe(true)

    await wrapper.get('[data-skill="vcard"] input').setValue(false)
    await flushPromises()
    expect((slides.element as HTMLInputElement).disabled).toBe(false)

    await slides.setValue(true)
    await wrapper.get('[data-testid="btn-save-tiles"]').trigger('click')
    await flushPromises()

    expect(api.setStudioTiles).toHaveBeenCalledWith(['email-draft', 'calendar-event', 'slides'])
    expect(wrapper.text()).toContain('Pitch it in five slides')
    expect(wrapper.text()).not.toContain('Save a contact')
  })
  it('dictates into the composer only — nothing is sent until the user does', async () => {
    const withVoice = { ...withModel, models: { ...withModel.models, voice: 'whisper-1' } }
    const wrapper = await factory(withVoice, [withVoice, withoutModel])
    await flushPromises()
    const textarea = wrapper.find('textarea')
    await textarea.setValue('Draft:')
    const mic = wrapper.getComponent({ name: 'DictationButton' })

    mic.vm.$emit('start')
    mic.vm.$emit('interim', 'call the')
    await flushPromises()
    expect((textarea.element as HTMLTextAreaElement).value).toBe('Draft: call the')
    expect((wrapper.find('button.btn-primary').element as HTMLButtonElement).disabled).toBe(true)

    mic.vm.$emit('done', 'Call the landlord tomorrow.')
    await flushPromises()
    expect((textarea.element as HTMLTextAreaElement).value).toBe(
      'Draft: Call the landlord tomorrow.',
    )
    expect((wrapper.find('button.btn-primary').element as HTMLButtonElement).disabled).toBe(false)
    expect(api.sendChat).not.toHaveBeenCalled()
    expect(api.sendAgentChat).not.toHaveBeenCalled()
  })

  it('has no mic until the project has a Dictation model', async () => {
    const wrapper = await factory()
    await flushPromises()
    expect(wrapper.find('[data-testid="dictation-toggle"]').exists()).toBe(false)
  })
})
