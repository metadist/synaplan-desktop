import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { messages } from '@/i18n'
import type { Project, ProjectsState } from '@/services/tauri'

vi.mock('@/services/tauri', () => ({
  listProjects: vi.fn(),
  applyDefaultModels: vi.fn().mockRejectedValue({ code: 'network', message: 'offline' }),
  setActiveProject: vi.fn(),
  createProject: vi.fn(),
  updateProject: vi.fn(),
  deleteProject: vi.fn(),
  asCommandError: (e: unknown) =>
    e && typeof e === 'object' && 'code' in e ? e : { code: 'unexpected', message: String(e) },
}))

import ProjectSwitcher from '@/components/ProjectSwitcher.vue'
import { useProjectsStore } from '@/stores/projects'
import { useUiStore } from '@/stores/ui'
import * as api from '@/services/tauri'

function project(id: string, name: string, extra: Partial<Project> = {}): Project {
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
    ...extra,
  }
}

const personal = project('p1', 'Personal', {
  kind: 'personal',
  models: {
    chat: 'openai:gpt-4o-mini:chat',
    voice: '',
    speak: '',
    vision: '',
    image: '',
    video: '',
    embed: '',
    docs: '',
    chatLegacyProviderId: null,
  },
})
const work = project('p2', 'Work')

function state(active: string, projects: Project[] = [personal, work]): ProjectsState {
  return { projects, activeId: active, personalId: 'p1' }
}

async function factory(locale = 'en') {
  const pinia = createPinia()
  setActivePinia(pinia)
  const i18n = createI18n({ legacy: false, locale, fallbackLocale: 'en', messages })
  const store = useProjectsStore()
  vi.mocked(api.listProjects).mockResolvedValue(state('p1'))
  await store.load()
  const wrapper = mount(ProjectSwitcher, {
    global: { plugins: [pinia, i18n], stubs: { Teleport: true } },
  })
  return { wrapper, store, ui: useUiStore() }
}

describe('ProjectSwitcher', () => {
  beforeEach(() => {
    vi.mocked(api.setActiveProject).mockReset()
    vi.mocked(api.createProject).mockReset()
    vi.mocked(api.updateProject).mockReset()
    vi.mocked(api.deleteProject).mockReset()
  })

  it('shows the translated Personal name and lists projects on open', async () => {
    const { wrapper } = await factory('de')
    expect(wrapper.find('[data-testid="project-switcher"]').text()).toContain('Persönlich')
    await wrapper.find('[data-testid="project-switcher"]').trigger('click')
    const menu = wrapper.find('[data-testid="project-menu"]')
    expect(menu.text()).toContain('Persönlich')
    expect(menu.text()).toContain('Work')
  })

  it('switches the active project through the Rust side', async () => {
    const { wrapper, store, ui } = await factory()
    ui.setView('doctor')
    vi.mocked(api.setActiveProject).mockResolvedValue(state('p2'))
    await wrapper.find('[data-testid="project-switcher"]').trigger('click')
    const items = wrapper.findAll('[role="menuitemradio"]')
    await items[1].trigger('click')
    await flushPromises()
    expect(api.setActiveProject).toHaveBeenCalledWith('p2')
    expect(store.activeId).toBe('p2')
    expect(ui.view).toBe('chat')
  })

  it('creates a project, offering to copy the current models', async () => {
    const { wrapper, ui } = await factory()
    const kitchen = project('p3', 'Kitchen')
    vi.mocked(api.createProject).mockResolvedValue(kitchen)
    vi.mocked(api.setActiveProject).mockResolvedValue(state('p3', [personal, work, kitchen]))

    await wrapper.find('[data-testid="project-switcher"]').trigger('click')
    const newBtn = wrapper.findAll('[role="menuitem"]')[0]
    await newBtn.trigger('click')

    const copy = wrapper.find('[data-testid="project-copy-models"]')
    expect((copy.element as HTMLInputElement).checked).toBe(true)
    await wrapper.find('[data-testid="project-name"]').setValue('  Kitchen ')
    await wrapper.find('[data-testid="project-language"]').setValue('de')
    await wrapper.find('form').trigger('submit')
    await flushPromises()

    expect(api.createProject).toHaveBeenCalledWith('Kitchen', 'de', 'p1')
    expect(api.setActiveProject).toHaveBeenCalledWith('p3')
    expect(ui.view).toBe('chat')
    expect(wrapper.find('[data-testid="project-name"]').exists()).toBe(false)
  })

  it('rename only sends the name and language', async () => {
    const { wrapper } = await factory()
    vi.mocked(api.updateProject).mockResolvedValue({ ...personal, name: 'Me' })
    await wrapper.find('[data-testid="project-switcher"]').trigger('click')
    await wrapper.findAll('[role="menuitem"]')[1].trigger('click')
    await wrapper.find('[data-testid="project-name"]').setValue('Me')
    await wrapper.find('form').trigger('submit')
    await flushPromises()
    expect(api.updateProject).toHaveBeenCalledWith('p1', { name: 'Me', dictationLanguage: 'en' })
  })

  it('delete asks about the notes folder and never touches Synaplan files', async () => {
    const { wrapper, store } = await factory()
    vi.mocked(api.deleteProject).mockResolvedValue(state('p2', [work]))
    await wrapper.find('[data-testid="project-switcher"]').trigger('click')
    await wrapper.findAll('[role="menuitem"]')[2].trigger('click')
    expect(wrapper.text()).toContain(messages.en.projects.deleteSynaplanNote)
    await wrapper.find('[data-testid="project-delete-files"]').setValue(true)
    await wrapper.find('[data-testid="project-delete-confirm"]').trigger('click')
    await flushPromises()
    expect(api.deleteProject).toHaveBeenCalledWith('p1', true)
    expect(store.projects.map((p) => p.id)).toEqual(['p2'])
  })

  it('disables delete when only one project is left', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages })
    vi.mocked(api.listProjects).mockResolvedValue(state('p1', [personal]))
    await useProjectsStore().load()
    const wrapper = mount(ProjectSwitcher, {
      global: { plugins: [pinia, i18n], stubs: { Teleport: true } },
    })
    await wrapper.find('[data-testid="project-switcher"]').trigger('click')
    const del = wrapper.findAll('[role="menuitem"]')[2]
    expect(del.attributes('disabled')).toBeDefined()
  })

  it('shows a localized error when the Rust side rejects', async () => {
    const { wrapper } = await factory()
    vi.mocked(api.createProject).mockRejectedValue({
      code: 'project_invalid_name',
      message: 'x',
    })
    await wrapper.find('[data-testid="project-switcher"]').trigger('click')
    await wrapper.findAll('[role="menuitem"]')[0].trigger('click')
    await wrapper.find('[data-testid="project-name"]').setValue('X')
    await wrapper.find('form').trigger('submit')
    await flushPromises()
    expect(wrapper.find('[role="alert"]').text()).toBe(messages.en.errors.project_invalid_name)
  })
})
