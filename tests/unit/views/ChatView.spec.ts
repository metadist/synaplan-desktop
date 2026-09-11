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
  dropCb: null as ((e: FileDropEvent) => void) | null,
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
  onFileDrop: vi.fn(async (cb: (e: FileDropEvent) => void) => {
    h.dropCb = cb
    return () => {}
  }),
  getUiPrefs: vi
    .fn()
    .mockResolvedValue({ language: null, sidebarCollapsed: false, historyCollapsed: false }),
  setUiPrefs: vi.fn(async (prefs: unknown) => prefs),
  listNotes: vi.fn().mockResolvedValue([]),
  createNote: vi.fn(),
  readNote: vi.fn(),
  writeNote: vi.fn(),
  deleteNote: vi.fn().mockResolvedValue(undefined),
  listProjectFiles: vi.fn().mockResolvedValue([]),
  uploadProjectFile: vi.fn(),
  deleteProjectFile: vi.fn().mockResolvedValue(undefined),
  pickFiles: vi.fn().mockResolvedValue([]),
  listProjects: vi.fn(),
  applyDefaultModels: vi.fn().mockRejectedValue({ code: 'network', message: 'offline' }),
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
  classifyGeneration: vi.fn().mockResolvedValue(null),
  generateAndAttach: vi.fn(),
  saveTextArtifact: vi.fn(),
  attachLocalArtifact: vi.fn(),
  localFileUrl: vi.fn((path: string) => path),
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

// The real editor pulls in Milkdown; the chat tests only care that a note opens.
vi.mock('@/components/NoteEditor.vue', () => ({
  __esModule: true,
  default: {
    name: 'NoteEditor',
    props: ['note', 'draft', 'dirty', 'saving'],
    emits: ['edit', 'delete', 'reveal'],
    template: '<div data-testid="note-editor">{{ note.title }}</div>',
  },
}))

import ChatView from '@/views/ChatView.vue'
import * as api from '@/services/tauri'
import type { FileDropEvent, Project, Skill } from '@/services/tauri'
import { useProjectsStore } from '@/stores/projects'
import { useUiStore } from '@/stores/ui'

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
    vi.mocked(api.classifyGeneration).mockReset()
    vi.mocked(api.classifyGeneration).mockResolvedValue(null)
    vi.mocked(api.generateAndAttach).mockReset()
    vi.mocked(api.saveTextArtifact).mockReset()
    vi.mocked(api.saveChat).mockClear()
    vi.mocked(api.newChat).mockClear()
    vi.mocked(api.listChats).mockResolvedValue([])
    vi.mocked(api.listSkills).mockResolvedValue([])
    vi.mocked(api.getStudioTiles).mockResolvedValue([])
    vi.mocked(api.setStudioTiles).mockClear()
    vi.mocked(api.listNotes).mockReset()
    vi.mocked(api.listNotes).mockResolvedValue([])
    vi.mocked(api.listProjectFiles).mockReset()
    vi.mocked(api.listProjectFiles).mockResolvedValue([])
    vi.mocked(api.pickFiles).mockReset()
    vi.mocked(api.pickFiles).mockResolvedValue([])
    vi.mocked(api.uploadProjectFile).mockReset()
    vi.mocked(api.createNote).mockReset()
    h.dropCb = null
  })

  it('opens on the project with a welcome card and pills that count chats, notes and files', async () => {
    vi.mocked(api.listChats).mockResolvedValue([
      {
        id: 'c1',
        projectId: 'p1',
        title: 'Plan the trip',
        createdAt: '2026-09-10T00:00:00Z',
        updatedAt: '2026-09-10T00:00:00Z',
        messageCount: 4,
        assistantId: null,
      },
    ])
    vi.mocked(api.listNotes).mockResolvedValue([
      { name: 'ideas.md', title: 'Ideas', updatedAt: '2026-09-10T00:00:00Z', size: 12 },
      { name: 'todo.md', title: 'To do', updatedAt: '2026-09-09T00:00:00Z', size: 12 },
    ])
    vi.mocked(api.listProjectFiles).mockResolvedValue([
      { id: 1, name: 'a.pdf', size: 10, state: 'ready', detail: null, uploadedAt: '' },
      { id: 2, name: 'b.pdf', size: 10, state: 'indexing', detail: null, uploadedAt: '' },
      { id: 3, name: 'c.pdf', size: 10, state: 'ready', detail: null, uploadedAt: '' },
    ])
    const wrapper = await factory()
    await flushPromises()

    expect(wrapper.get('h1').text()).toBe('Work')
    expect(wrapper.get('[data-testid="chat-welcome"]').text()).toContain(
      'This is your Work project',
    )
    expect(wrapper.get('[data-testid="pill-chats"]').text()).toContain('1 chat')
    expect(wrapper.get('[data-testid="pill-notes"]').text()).toContain('2 notes')
    expect(wrapper.get('[data-testid="pill-files"]').text()).toContain('3 files')
    // History groups by day and says how long each chat is.
    expect(wrapper.get('[data-testid="chat-threads"]').text()).toContain('Plan the trip')
    expect(wrapper.get('[data-testid="chat-threads"]').text()).toContain('4 messages')

    // The side panel is closed until a pill asks for it.
    expect(wrapper.find('[data-testid="project-panel"]').exists()).toBe(false)
    await wrapper.get('[data-testid="pill-notes"]').trigger('click')
    expect(wrapper.get('[data-testid="project-panel"]').text()).toContain('Ideas')
    expect(wrapper.get('[data-testid="project-panel"]').text()).toContain('To do')
    await wrapper.get('[data-testid="panel-tab-files"]').trigger('click')
    expect(wrapper.get('[data-testid="project-panel"]').text()).toContain('a.pdf')
    expect(wrapper.get('[data-testid="panel-file-2"]').text()).toContain('Indexing')
    await wrapper.get('[data-testid="panel-close"]').trigger('click')
    expect(wrapper.find('[data-testid="project-panel"]').exists()).toBe(false)
  })

  it('adds files to the project straight from the composer', async () => {
    const indexed = {
      ...withModel,
      models: { ...withModel.models, embed: 'ollama:bge-m3:vectorize' },
    }
    vi.mocked(api.pickFiles).mockResolvedValue(['/home/u/Documents/report.pdf'])
    vi.mocked(api.uploadProjectFile).mockResolvedValue({
      id: 11,
      name: 'report.pdf',
      size: 2048,
      state: 'sent',
      detail: null,
      uploadedAt: '2026-09-10T10:00:00Z',
    })
    const wrapper = await factory(indexed, [indexed])
    await flushPromises()

    await wrapper.get('[data-testid="composer-add-files"]').trigger('click')
    await flushPromises()

    expect(api.uploadProjectFile).toHaveBeenCalledWith('p1', '/home/u/Documents/report.pdf')
    expect(wrapper.get('[data-testid="pill-files"]').text()).toContain('1 file')
    expect(wrapper.get('[data-testid="panel-file-11"]').text()).toContain('report.pdf')
    expect(wrapper.text()).not.toMatch(/DESKTOP:|group_key|VECTORIZE/)
  })

  it('takes a drop anywhere over the chat, but only while Chat is on screen', async () => {
    const indexed = {
      ...withModel,
      models: { ...withModel.models, embed: 'ollama:bge-m3:vectorize' },
    }
    vi.mocked(api.uploadProjectFile).mockResolvedValue({
      id: 12,
      name: 'notes.docx',
      size: 2048,
      state: 'sent',
      detail: null,
      uploadedAt: '2026-09-10T10:00:00Z',
    })
    const wrapper = await factory(indexed, [indexed])
    await flushPromises()

    h.dropCb?.({ type: 'enter', paths: ['/home/u/Documents/notes.docx'] })
    await flushPromises()
    expect(wrapper.get('[data-testid="chat-drop-overlay"]').text()).toContain('Work')
    h.dropCb?.({ type: 'drop', paths: ['/home/u/Documents/notes.docx'] })
    await flushPromises()
    expect(api.uploadProjectFile).toHaveBeenCalledWith('p1', '/home/u/Documents/notes.docx')
    expect(wrapper.find('[data-testid="chat-drop-overlay"]').exists()).toBe(false)

    useUiStore().setView('files')
    h.dropCb?.({ type: 'drop', paths: ['/home/u/Documents/other.docx'] })
    await flushPromises()
    expect(api.uploadProjectFile).toHaveBeenCalledTimes(1)
  })

  it('adds files even when the project has no Embed pick', async () => {
    vi.mocked(api.pickFiles).mockResolvedValue(['/home/u/Documents/notes.docx'])
    vi.mocked(api.uploadProjectFile).mockResolvedValue({
      id: 13,
      name: 'notes.docx',
      size: 2048,
      state: 'sent',
      detail: null,
      uploadedAt: '2026-09-10T10:00:00Z',
    })
    const wrapper = await factory()
    await flushPromises()

    await wrapper.get('[data-testid="composer-add-files"]').trigger('click')
    await flushPromises()

    expect(api.pickFiles).toHaveBeenCalled()
    expect(api.uploadProjectFile).toHaveBeenCalledWith('p1', '/home/u/Documents/notes.docx')
    expect(wrapper.find('[data-testid="panel-embed-unset"]').exists()).toBe(false)
  })

  it('writes a new note from the composer and opens it next to the chat', async () => {
    vi.mocked(api.createNote).mockResolvedValue({
      name: '2026-09-10-note.md',
      title: '',
      updatedAt: '2026-09-10T00:00:00Z',
      content: '',
      path: '/home/u/Synaplan/projects/work/notes/2026-09-10-note.md',
    })
    vi.mocked(api.listNotes)
      .mockResolvedValueOnce([])
      .mockResolvedValue([
        { name: '2026-09-10-note.md', title: '', updatedAt: '2026-09-10T00:00:00Z', size: 0 },
      ])
    const wrapper = await factory()
    await flushPromises()
    expect(wrapper.get('[data-testid="pill-notes"]').text()).toContain('0 notes')

    await wrapper.get('[data-testid="composer-new-note"]').trigger('click')
    await flushPromises()

    expect(api.createNote).toHaveBeenCalledWith('p1')
    expect(
      wrapper.get('[data-testid="project-panel"]').find('[data-testid="note-editor"]').exists(),
    ).toBe(true)
    expect(wrapper.get('[data-testid="pill-notes"]').text()).toContain('1 note')

    await wrapper.get('[data-testid="panel-note-back"]').trigger('click')
    await flushPromises()
    expect(wrapper.find('[data-testid="note-editor"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="panel-new-note"]').exists()).toBe(true)
  })

  it('keeps an answer as a note with one click, and the note remembers the chat', async () => {
    vi.mocked(api.createNote).mockResolvedValue({
      name: '2026-09-10-1200.md',
      title: '',
      updatedAt: '2026-09-10T12:00:00Z',
      content: '',
      path: '/home/u/Synaplan/projects/work/notes/2026-09-10-1200.md',
    })
    vi.mocked(api.writeNote).mockReset().mockResolvedValue({
      name: '2026-09-10-1200.md',
      title: 'Volcanoes',
      updatedAt: '2026-09-10T12:00:00Z',
      size: 60,
    })
    vi.mocked(api.listNotes)
      .mockResolvedValueOnce([])
      .mockResolvedValue([
        {
          name: '2026-09-10-1200.md',
          title: 'Volcanoes',
          updatedAt: '2026-09-10T12:00:00Z',
          size: 60,
        },
      ])
    vi.mocked(api.readNote).mockResolvedValue({
      name: '2026-09-10-1200.md',
      title: 'Volcanoes',
      updatedAt: '2026-09-10T12:00:00Z',
      content: '# Volcanoes',
      path: '/home/u/Synaplan/projects/work/notes/2026-09-10-1200.md',
    })
    const wrapper = await factory()
    await flushPromises()

    await wrapper.find('textarea').setValue('What is a volcano?')
    await wrapper.find('button.btn-primary').trigger('click')
    await flushPromises()
    h.tokenCb?.('Volcanoes\nA volcano is a mountain that erupts.')
    h.doneCb?.()
    await flushPromises()

    await wrapper.get('[data-testid="msg-keep-1"]').trigger('click')
    await flushPromises()

    const [projectId, name, content] = vi.mocked(api.writeNote).mock.calls[0]
    expect(projectId).toBe('p1')
    expect(name).toBe('2026-09-10-1200.md')
    expect(content).toContain('# Volcanoes\n\nA volcano is a mountain that erupts.')
    expect(content).toContain('Kept from the chat')
    expect(wrapper.get('[data-testid="pill-notes"]').text()).toContain('1 note')
    expect(wrapper.find('[data-testid="msg-keep-1"]').exists()).toBe(false)

    // The kept answer links to its note.
    await wrapper.get('[data-testid="msg-kept-1"]').trigger('click')
    await flushPromises()
    expect(api.readNote).toHaveBeenCalledWith('p1', '2026-09-10-1200.md')
    expect(wrapper.get('[data-testid="project-panel"]').text()).toContain('Volcanoes')
  })

  it('the composer note button keeps typed text as a note instead of sending it', async () => {
    vi.mocked(api.createNote).mockResolvedValue({
      name: '2026-09-10-1201.md',
      title: '',
      updatedAt: '2026-09-10T12:01:00Z',
      content: '',
      path: '/home/u/Synaplan/projects/work/notes/2026-09-10-1201.md',
    })
    vi.mocked(api.writeNote).mockReset().mockResolvedValue({
      name: '2026-09-10-1201.md',
      title: 'Bring the permission slip',
      updatedAt: '2026-09-10T12:01:00Z',
      size: 30,
    })
    const wrapper = await factory()
    await flushPromises()

    const textarea = wrapper.find('textarea')
    await textarea.setValue('Bring the permission slip\nMonday, before class')
    const noteButton = wrapper.get('[data-testid="composer-new-note"]')
    expect(noteButton.attributes('title')).toContain('Keep this text')
    await noteButton.trigger('click')
    await flushPromises()

    expect(api.writeNote).toHaveBeenCalledWith(
      'p1',
      '2026-09-10-1201.md',
      '# Bring the permission slip\n\nMonday, before class',
    )
    expect(api.sendChat).not.toHaveBeenCalled()
    expect((textarea.element as HTMLTextAreaElement).value).toBe('')
    // Nothing was opened: the user keeps chatting.
    expect(wrapper.find('[data-testid="project-panel"]').exists()).toBe(false)
    expect(noteButton.attributes('title')).toContain('Write a new note')
  })

  it('the panel is one list: notes and files together, newest first, one search box', async () => {
    vi.mocked(api.listNotes).mockResolvedValue([
      { name: 'ideas.md', title: 'Trip ideas', updatedAt: '2026-09-09T00:00:00Z', size: 12 },
      { name: 'packing.md', title: 'Packing list', updatedAt: '2026-09-11T00:00:00Z', size: 12 },
    ])
    vi.mocked(api.listProjectFiles).mockResolvedValue([
      {
        id: 1,
        name: 'hotel-booking.pdf',
        size: 10,
        state: 'ready',
        detail: null,
        uploadedAt: '2026-09-10T00:00:00Z',
      },
    ])
    const wrapper = await factory()
    await flushPromises()

    await wrapper.get('[data-testid="pill-notes"]').trigger('click')
    await wrapper.get('[data-testid="panel-tab-all"]').trigger('click')
    const titles = wrapper.findAll('.row-title').map((el) => el.text())
    expect(titles).toEqual(['Packing list', 'hotel-booking.pdf', 'Trip ideas'])

    await wrapper.get('[data-testid="panel-search"]').setValue('hotel')
    expect(wrapper.findAll('.row-title').map((el) => el.text())).toEqual(['hotel-booking.pdf'])

    await wrapper.get('[data-testid="panel-search"]').setValue('zzz')
    expect(wrapper.get('[data-testid="panel-empty"]').text()).toContain('Nothing matches')

    await wrapper.get('[data-testid="panel-search"]').setValue('')
    await wrapper.get('[data-testid="panel-tab-notes"]').trigger('click')
    expect(wrapper.findAll('.row-title').map((el) => el.text())).toEqual([
      'Packing list',
      'Trip ideas',
    ])
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

  it('does not fall back to chat when classify fails', async () => {
    vi.mocked(api.classifyGeneration).mockRejectedValue({
      code: 'classify_missing',
      message: 'classify_generation not found',
    })
    const wrapper = await factory()
    await flushPromises()

    await wrapper.find('textarea').setValue('ein echtes bild einer katze')
    await wrapper.find('button.btn-primary').trigger('click')
    await flushPromises()

    expect(api.sendChat).not.toHaveBeenCalled()
    expect(api.generateAndAttach).not.toHaveBeenCalled()
    expect(wrapper.text()).toMatch(/classify_generation not found|Something went wrong/i)
  })

  it('creates an image in the project instead of chatting when asked for a picture', async () => {
    vi.mocked(api.classifyGeneration).mockResolvedValue('image')
    vi.mocked(api.generateAndAttach).mockResolvedValue({
      path: '/tmp/out/cat.png',
      name: 'cat.png',
      kind: 'image',
      fileId: 12,
    })
    const wrapper = await factory()
    await flushPromises()

    await wrapper.find('textarea').setValue('ein echtes bild einer katze')
    await wrapper.find('button.btn-primary').trigger('click')
    await flushPromises()

    expect(api.generateAndAttach).toHaveBeenCalledWith('p1', 'image', 'ein echtes bild einer katze')
    expect(api.sendChat).not.toHaveBeenCalled()
    expect(wrapper.find('.artifact-image').exists()).toBe(true)
    expect(wrapper.text()).toContain('Saved this image in the project.')
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

  it('folds the history to a date rail whose stamps unfold a preview card to the right', async () => {
    vi.useFakeTimers()
    try {
      vi.mocked(api.listChats).mockResolvedValue([
        {
          id: 'c1',
          projectId: 'p1',
          title: 'Volcano essay',
          createdAt: '2026-09-10T00:00:00Z',
          updatedAt: '2026-09-10T09:30:00Z',
          messageCount: 2,
          assistantId: null,
        },
      ])
      vi.mocked(api.loadChat)
        .mockReset()
        .mockResolvedValue({
          id: 'c1',
          projectId: 'p1',
          title: 'Volcano essay',
          createdAt: '2026-09-10T00:00:00Z',
          updatedAt: '2026-09-10T09:30:00Z',
          assistantId: null,
          messages: [
            { role: 'user', content: 'How do volcanoes form?', model: '', createdAt: '' },
            {
              role: 'assistant',
              content: 'Magma rises through the crust…',
              model: 'x',
              createdAt: '',
            },
          ],
        })
      const wrapper = await factory()
      await flushPromises()

      // Unfolded: titles are visible.
      const rail = wrapper.get('[data-testid="chat-threads"]')
      expect(rail.text()).toContain('Volcano essay')

      // The chats pill folds it to a rail: "+" and one stamp per chat, no title.
      await wrapper.get('[data-testid="pill-chats"]').trigger('click')
      await flushPromises()
      expect(api.setUiPrefs).toHaveBeenCalledWith(
        expect.objectContaining({ historyCollapsed: true }),
      )
      expect(rail.classes()).toContain('collapsed')
      expect(rail.get('[data-testid="chat-new-thread"]').text()).toBe('+')
      const stamp = rail.get('[data-testid="chat-thread-c1"]')
      expect(stamp.text()).not.toContain('Volcano essay')
      expect(stamp.attributes('title')).toBe('Volcano essay')

      // Hover: the card shows the title and the first lines, without opening the chat.
      await stamp.trigger('mouseenter')
      vi.advanceTimersByTime(300)
      await flushPromises()
      const peek = document.body.querySelector('[data-testid="chat-peek"]')
      expect(peek?.textContent).toContain('Volcano essay')
      expect(peek?.textContent).toContain('Magma rises through the crust')
      expect(api.loadChat).toHaveBeenCalledWith('p1', 'c1')
      expect(wrapper.find('[data-testid="chat-welcome"]').exists()).toBe(true)

      await stamp.trigger('mouseleave')
      vi.advanceTimersByTime(300)
      await flushPromises()
      expect(document.body.querySelector('[data-testid="chat-peek"]')).toBeNull()

      // The rail's own button unfolds it again.
      await rail.get('[data-testid="chat-history-toggle"]').trigger('click')
      await flushPromises()
      expect(rail.classes()).not.toContain('collapsed')
      expect(rail.text()).toContain('Volcano essay')
    } finally {
      vi.useRealTimers()
    }
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

  it('keeps a composer draft when switching projects', async () => {
    const wrapper = await factory()
    await flushPromises()
    await wrapper.find('textarea').setValue('typed in work')
    vi.mocked(api.setActiveProject).mockResolvedValue({
      projects: [withModel, withoutModel],
      activeId: 'p2',
      personalId: 'p1',
    })
    await useProjectsStore().select('p2')
    await flushPromises()
    expect((wrapper.find('textarea').element as HTMLTextAreaElement).value).toBe('')

    vi.mocked(api.setActiveProject).mockResolvedValue({
      projects: [withModel, withoutModel],
      activeId: 'p1',
      personalId: 'p1',
    })
    await useProjectsStore().select('p1')
    await flushPromises()
    expect((wrapper.find('textarea').element as HTMLTextAreaElement).value).toBe('typed in work')
  })

  it('has no mic until the project has a Dictation model', async () => {
    const wrapper = await factory()
    await flushPromises()
    expect(wrapper.find('[data-testid="dictation-toggle"]').exists()).toBe(false)
  })
})
