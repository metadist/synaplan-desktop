import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { messages } from '@/i18n'
import type { Project } from '@/services/tauri'

vi.mock('@/services/tauri', () => ({
  getStatus: vi.fn(),
  listProjects: vi.fn(),
  applyDefaultModels: vi.fn().mockRejectedValue({ code: 'network', message: 'offline' }),
  updateProject: vi.fn(),
  deleteProject: vi.fn().mockResolvedValue(undefined),
  setActiveProject: vi.fn(),
  revealPath: vi.fn().mockResolvedValue(undefined),
  asCommandError: (e: unknown) =>
    e && typeof e === 'object' && 'code' in e ? e : { code: 'unexpected', message: String(e) },
}))

import ProjectConfigView from '@/views/ProjectConfigView.vue'
import { useProjectsStore } from '@/stores/projects'
import { useUiStore } from '@/stores/ui'
import * as api from '@/services/tauri'

const homework: Project = {
  id: 'p1',
  slug: 'homework',
  name: 'Homework',
  kind: 'project',
  createdAt: '2026-09-10T00:00:00Z',
  updatedAt: '2026-09-10T00:00:00Z',
  dictationLanguage: 'de',
  defaultAssistantId: null,
  assistantIds: [],
  enabledSkills: [],
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
  knowledgeFolder: 'DESKTOP:p1',
  webSearch: false,
  projectDir: 'C:\\Users\\u\\Synaplan\\projects\\homework',
  notesDir: 'C:\\Users\\u\\Synaplan\\projects\\homework\\notes',
  outDir: 'C:\\Users\\u\\Synaplan\\projects\\homework\\out',
}

async function factory() {
  const pinia = createPinia()
  setActivePinia(pinia)
  const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages })
  vi.mocked(api.listProjects).mockResolvedValue({
    projects: [homework, { ...homework, id: 'p2', name: 'Other' }],
    activeId: 'p1',
    personalId: 'p1',
  })
  await useProjectsStore().load()
  const wrapper = mount(ProjectConfigView, { global: { plugins: [pinia, i18n] } })
  await flushPromises()
  return wrapper
}

describe('ProjectConfigView', () => {
  beforeEach(() => {
    vi.mocked(api.updateProject).mockReset()
    vi.mocked(api.revealPath).mockClear()
    document.body.innerHTML = ''
  })

  it('shows the project name, dictation language and folders — never the raw group key', async () => {
    const wrapper = await factory()

    expect(wrapper.get('[data-testid="project-config-view"]').text()).toContain('Homework')
    const basics = wrapper.get('[data-testid="project-config-basics"]')
    expect(basics.text()).toContain('German')

    const folders = wrapper.get('[data-testid="project-config-folders"]')
    expect(folders.text()).toContain('C:\\Users\\u\\Synaplan\\projects\\homework\\notes')
    expect(folders.text()).not.toContain('DESKTOP:')
    expect(folders.text()).toContain('“Homework” knowledge folder')
  })

  it('opens Models and Agents from here', async () => {
    const wrapper = await factory()
    await wrapper.get('[data-testid="project-config-open-models"]').trigger('click')
    expect(useUiStore().view).toBe('models')
    await wrapper.get('[data-testid="project-config-open-agents"]').trigger('click')
    expect(useUiStore().view).toBe('agents')
  })

  it('renames the project from here', async () => {
    vi.mocked(api.updateProject).mockResolvedValue({ ...homework, name: 'Science homework' })
    const wrapper = await factory()

    await wrapper.get('[data-testid="project-config-rename"]').trigger('click')
    await flushPromises()
    const dialog = document.body.querySelector('[role="dialog"]')
    expect(dialog).not.toBeNull()
    const input = dialog!.querySelector('input') as HTMLInputElement
    input.value = 'Science homework'
    input.dispatchEvent(new Event('input'))
    dialog!.querySelector('form')!.dispatchEvent(new Event('submit'))
    await flushPromises()

    expect(api.updateProject).toHaveBeenCalledWith('p1', {
      name: 'Science homework',
      dictationLanguage: 'de',
    })
    expect(wrapper.get('[data-testid="project-config-view"]').text()).toContain('Science homework')
  })

  it('asks before deleting the project', async () => {
    const wrapper = await factory()
    await wrapper.get('[data-testid="project-config-delete"]').trigger('click')
    await flushPromises()
    expect(api.deleteProject).not.toHaveBeenCalled()
    expect(document.body.querySelector('[role="dialog"]')).not.toBeNull()
    expect(document.body.textContent).toContain('Homework')
  })

  it('reveals a project folder instead of building on the path', async () => {
    const wrapper = await factory()
    const buttons = wrapper.get('[data-testid="project-config-folders"]').findAll('button.btn-link')
    await buttons[0].trigger('click')
    expect(api.revealPath).toHaveBeenCalledWith('C:\\Users\\u\\Synaplan\\projects\\homework')
  })
})
