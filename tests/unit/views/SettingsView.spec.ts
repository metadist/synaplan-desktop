import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { messages } from '@/i18n'
import type { Project } from '@/services/tauri'

vi.mock('@/services/tauri', () => ({
  getStatus: vi.fn(),
  signOut: vi.fn().mockResolvedValue(undefined),
  listProjects: vi.fn(),
  applyDefaultModels: vi.fn().mockRejectedValue({ code: 'network', message: 'offline' }),
  updateProject: vi.fn(),
  deleteProject: vi.fn(),
  getUiPrefs: vi
    .fn()
    .mockResolvedValue({ language: null, sidebarCollapsed: false, historyCollapsed: false }),
  setUiPrefs: vi.fn(async (prefs: unknown) => prefs),
  getStorageInfo: vi.fn(),
  revealPath: vi.fn().mockResolvedValue(undefined),
  openUrl: vi.fn(),
  asCommandError: (e: unknown) =>
    e && typeof e === 'object' && 'code' in e ? e : { code: 'unexpected', message: String(e) },
}))

import SettingsView from '@/views/SettingsView.vue'
import { useConfigStore } from '@/stores/config'
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
  projectDir: 'C:\\Users\\u\\Synaplan\\projects\\homework',
  notesDir: 'C:\\Users\\u\\Synaplan\\projects\\homework\\notes',
  outDir: 'C:\\Users\\u\\Synaplan\\projects\\homework\\out',
}

async function factory() {
  const pinia = createPinia()
  setActivePinia(pinia)
  const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages })
  vi.mocked(api.getStatus).mockResolvedValue({
    paired: true,
    apiBaseUrl: 'https://web.synaplan.com',
    deviceId: 4,
    keyBackend: 'keyring',
    keyIsPlaintext: false,
  })
  vi.mocked(api.getStorageInfo).mockResolvedValue({
    projectsDir: 'C:\\Users\\u\\Synaplan\\projects',
    outboxDir: 'C:\\Users\\u\\Synaplan\\out',
    skillsDir: 'C:\\Users\\u\\AppData\\Roaming\\Synaplan\\Desktop\\skills',
    configDir: 'C:\\Users\\u\\AppData\\Roaming\\Synaplan\\Desktop',
  })
  vi.mocked(api.listProjects).mockResolvedValue({
    projects: [homework, { ...homework, id: 'p2', name: 'Other' }],
    activeId: 'p1',
    personalId: 'p1',
  })
  await useConfigStore().load()
  await useProjectsStore().load()
  const wrapper = mount(SettingsView, { global: { plugins: [pinia, i18n] } })
  await flushPromises()
  return wrapper
}

describe('SettingsView', () => {
  beforeEach(() => {
    vi.mocked(api.setUiPrefs).mockClear()
    vi.mocked(api.revealPath).mockClear()
    vi.mocked(api.updateProject).mockReset()
    document.body.innerHTML = ''
  })

  it('shows the account, the language, where things are kept and the current project', async () => {
    const wrapper = await factory()

    const account = wrapper.get('[data-testid="settings-account"]')
    expect(account.text()).toContain('web.synaplan.com')
    expect(account.text()).toContain('#4')
    expect(account.text()).toContain('secure store')
    expect(account.text()).not.toMatch(/sk_|keyring/)

    // Storage paths are shown as the OS spells them.
    const storage = wrapper.get('[data-testid="settings-storage"]')
    expect(storage.text()).toContain('C:\\Users\\u\\Synaplan\\projects')
    expect(storage.text()).not.toContain('~/')

    const project = wrapper.get('[data-testid="settings-project"]')
    expect(project.text()).toContain('Project: Homework')
    expect(project.text()).toContain('German')
    expect(project.text()).toContain('C:\\Users\\u\\Synaplan\\projects\\homework\\notes')
    expect(project.text()).not.toContain('DESKTOP:')
    expect(project.text()).toContain('“Homework” knowledge folder')
  })

  it('changes the interface language and keeps it on this computer', async () => {
    const wrapper = await factory()
    await wrapper.get('[data-testid="settings-language-select"]').setValue('de')
    await flushPromises()

    expect(api.setUiPrefs).toHaveBeenCalledWith({
      language: 'de',
      sidebarCollapsed: false,
      historyCollapsed: false,
    })
    expect(useUiStore().language).toBe('de')

    await wrapper.get('[data-testid="settings-language-select"]').setValue('')
    await flushPromises()
    expect(useUiStore().language).toBeNull()
  })

  it('reveals folders instead of building on paths', async () => {
    const wrapper = await factory()
    const buttons = wrapper.get('[data-testid="settings-storage"]').findAll('button.btn-link')
    await buttons[0].trigger('click')
    expect(api.revealPath).toHaveBeenCalledWith('C:\\Users\\u\\Synaplan\\projects')
  })

  it('renames the project from here', async () => {
    vi.mocked(api.updateProject).mockResolvedValue({ ...homework, name: 'Science homework' })
    const wrapper = await factory()

    await wrapper.get('[data-testid="settings-rename"]').trigger('click')
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
    expect(wrapper.get('[data-testid="settings-project"]').text()).toContain('Science homework')
  })

  it('disconnecting asks first', async () => {
    const wrapper = await factory()
    await wrapper.get('[data-testid="settings-signout"]').trigger('click')
    await flushPromises()
    expect(api.signOut).not.toHaveBeenCalled()
    expect(document.body.textContent).toContain('Disconnect this computer?')
  })
})
