import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { messages } from '@/i18n'
import type { Note, NoteSummary, Project } from '@/services/tauri'

vi.mock('@/services/tauri', () => ({
  listProjects: vi.fn(),
  setActiveProject: vi.fn(),
  listNotes: vi.fn(),
  createNote: vi.fn(),
  readNote: vi.fn(),
  writeNote: vi.fn(),
  deleteNote: vi.fn(),
  revealPath: vi.fn().mockResolvedValue(undefined),
  asCommandError: (e: unknown) =>
    e && typeof e === 'object' && 'code' in e ? e : { code: 'unexpected', message: String(e) },
}))

import NotesView from '@/views/NotesView.vue'
import { useProjectsStore } from '@/stores/projects'
import { AUTOSAVE_DELAY_MS } from '@/composables/useNotes'
import * as api from '@/services/tauri'

function project(id: string, name: string): Project {
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
      chat: '',
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

const work = project('p1', 'Work')
const home = project('p2', 'Home')

function summary(name: string, title: string): NoteSummary {
  return { name, title, updatedAt: '2026-09-10T10:00:00Z', size: 12 }
}

function note(name: string, content: string): Note {
  return {
    name,
    title: content.startsWith('# ') ? content.slice(2).split('\n')[0] : name.replace(/\.md$/, ''),
    content,
    updatedAt: '2026-09-10T10:00:00Z',
    path: `/home/u/Synaplan/projects/work/notes/${name}`,
  }
}

/** Stands in for the real mic button: the test emits the take events itself. */
const DictationStub = {
  name: 'DictationButton',
  props: ['projectId', 'disabled'],
  emits: ['start', 'interim', 'done', 'error'],
  template: '<button type="button" data-testid="dictation-toggle"></button>',
}

async function factory(active: Project = work) {
  const pinia = createPinia()
  setActivePinia(pinia)
  const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages })
  vi.mocked(api.listProjects).mockResolvedValue({
    projects: [active, home],
    activeId: 'p1',
    personalId: 'p1',
  })
  await useProjectsStore().load()
  const wrapper = mount(NotesView, {
    attachTo: document.body,
    global: { plugins: [pinia, i18n], stubs: { DictationButton: DictationStub } },
  })
  await flushPromises()
  return wrapper
}

describe('NotesView', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.mocked(api.listNotes).mockReset()
    vi.mocked(api.listNotes).mockResolvedValue([summary('kitchen.md', 'Kitchen plan')])
    vi.mocked(api.readNote).mockReset()
    vi.mocked(api.readNote).mockResolvedValue(note('kitchen.md', '# Kitchen plan\n\nFridge'))
    vi.mocked(api.writeNote).mockReset()
    vi.mocked(api.writeNote).mockImplementation(async (_p, name, content) => ({
      ...summary(name, content.startsWith('# ') ? content.slice(2).split('\n')[0] : name),
    }))
    vi.mocked(api.createNote).mockReset()
    vi.mocked(api.deleteNote).mockReset()
    vi.mocked(api.deleteNote).mockResolvedValue(undefined)
  })

  afterEach(() => {
    vi.useRealTimers()
    document.body.innerHTML = ''
  })

  it('lists the notes of the active project and shows the local-only promise', async () => {
    const wrapper = await factory()

    expect(api.listNotes).toHaveBeenCalledWith('p1', '')
    expect(wrapper.get('[data-testid="note-list"]').text()).toContain('Kitchen plan')
    expect(wrapper.text()).toContain('Notes stay on this computer')
    expect(wrapper.find('[data-testid="note-editor"]').exists()).toBe(false)
  })

  it('opens a note by name and autosaves edits after a pause', async () => {
    const wrapper = await factory()

    await wrapper.get('[data-testid="note-kitchen.md"]').trigger('click')
    await flushPromises()
    expect(api.readNote).toHaveBeenCalledWith('p1', 'kitchen.md')
    const body = wrapper.get('[data-testid="note-body"]')
    expect((body.element as HTMLTextAreaElement).value).toBe('# Kitchen plan\n\nFridge')
    expect(wrapper.get('[data-testid="note-save-state"]').text()).toBe('Saved')

    await body.setValue('# Kitchen plan\n\nFridge and oven')
    expect(wrapper.get('[data-testid="note-save-state"]').text()).toBe('Unsaved changes')
    expect(api.writeNote).not.toHaveBeenCalled()

    vi.advanceTimersByTime(AUTOSAVE_DELAY_MS + 1)
    await flushPromises()
    expect(api.writeNote).toHaveBeenCalledWith(
      'p1',
      'kitchen.md',
      '# Kitchen plan\n\nFridge and oven',
    )
    expect(wrapper.get('[data-testid="note-save-state"]').text()).toBe('Saved')
  })

  it('creates a note in Rust and opens it', async () => {
    vi.mocked(api.createNote).mockResolvedValue(note('2026-09-10-1200.md', ''))
    vi.mocked(api.listNotes).mockResolvedValueOnce([summary('kitchen.md', 'Kitchen plan')])
    const wrapper = await factory()

    await wrapper.get('[data-testid="note-new"]').trigger('click')
    await flushPromises()

    expect(api.createNote).toHaveBeenCalledWith('p1')
    expect(wrapper.find('[data-testid="note-editor"]').exists()).toBe(true)
    expect(wrapper.get('[data-testid="note-editor"]').text()).toContain('2026-09-10-1200')
  })

  it('deletes only after confirmation', async () => {
    const wrapper = await factory()
    await wrapper.get('[data-testid="note-kitchen.md"]').trigger('click')
    await flushPromises()

    await wrapper.get('[data-testid="note-delete"]').trigger('click')
    await flushPromises()
    expect(api.deleteNote).not.toHaveBeenCalled()
    const confirm = document.body.querySelector('[data-testid="confirm-dialog-confirm"]')
    expect(confirm).not.toBeNull()
    expect(document.body.textContent).toContain('Kitchen plan')

    vi.mocked(api.listNotes).mockResolvedValue([])
    ;(confirm as HTMLButtonElement).click()
    await flushPromises()

    expect(api.deleteNote).toHaveBeenCalledWith('p1', 'kitchen.md')
    expect(wrapper.find('[data-testid="note-editor"]').exists()).toBe(false)
  })

  it('searches through Rust and reloads on project switch', async () => {
    const wrapper = await factory()

    vi.mocked(api.listNotes).mockResolvedValue([])
    await wrapper.get('[data-testid="note-search"]').setValue('fridge')
    await flushPromises()
    expect(api.listNotes).toHaveBeenLastCalledWith('p1', 'fridge')
    expect(wrapper.get('[data-testid="note-list"]').text()).toContain('No note matches')

    vi.mocked(api.setActiveProject).mockResolvedValue({
      projects: [work, home],
      activeId: 'p2',
      personalId: 'p1',
    })
    await useProjectsStore().select('p2')
    await flushPromises()
    expect(api.listNotes).toHaveBeenLastCalledWith('p2', '')
  })

  it('never hands the webview a path to build on — only names cross the seam', async () => {
    const wrapper = await factory()
    await wrapper.get('[data-testid="note-kitchen.md"]').trigger('click')
    await flushPromises()

    for (const call of vi.mocked(api.readNote).mock.calls) {
      expect(call[1]).not.toContain('/')
    }
    expect(wrapper.text()).not.toContain('/home/u/Synaplan/projects/work/notes/kitchen.md')
  })
  it('offers dictation only once the project has a Dictation model', async () => {
    const wrapper = await factory()
    await wrapper.get('[data-testid="note-kitchen.md"]').trigger('click')
    await flushPromises()

    expect(wrapper.find('[data-testid="dictation-toggle"]').exists()).toBe(false)
    expect(wrapper.get('[data-testid="dictation-need-model"]').text()).toBe('Set up dictation')
  })

  it('writes one take at the caret: interim readings replace only their own span', async () => {
    const withVoice: Project = { ...work, models: { ...work.models, voice: 'whisper-1' } }
    const wrapper = await factory(withVoice)
    await wrapper.get('[data-testid="note-kitchen.md"]').trigger('click')
    await flushPromises()

    const body = wrapper.get('[data-testid="note-body"]').element as HTMLTextAreaElement
    body.setSelectionRange(body.value.length, body.value.length)
    const mic = wrapper.getComponent({ name: 'DictationButton' })

    mic.vm.$emit('start')
    mic.vm.$emit('interim', 'buy a')
    await flushPromises()
    expect(body.value).toBe('# Kitchen plan\n\nFridge buy a')

    mic.vm.$emit('interim', 'buy a new')
    await flushPromises()
    expect(body.value).toBe('# Kitchen plan\n\nFridge buy a new')

    mic.vm.$emit('done', 'Buy a new oven.')
    await flushPromises()
    expect(body.value).toBe('# Kitchen plan\n\nFridge Buy a new oven.')
    expect(body.selectionStart).toBe(body.value.length)
    expect(wrapper.get('[data-testid="note-save-state"]').text()).toBe('Unsaved changes')
  })

  it('names a refused Dictation model instead of failing silently', async () => {
    const withVoice: Project = { ...work, models: { ...work.models, voice: 'whisper-1' } }
    const wrapper = await factory(withVoice)
    await wrapper.get('[data-testid="note-kitchen.md"]').trigger('click')
    await flushPromises()

    wrapper
      .getComponent({ name: 'DictationButton' })
      .vm.$emit('error', { code: 'voice_model_unknown', message: '' })
    await flushPromises()
    expect(wrapper.get('[data-testid="dictation-error"]').text()).toContain(
      'does not know this Dictation model',
    )
  })
})
