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
  setActiveProject: vi.fn(),
  getUiPrefs: vi.fn(),
  setUiPrefs: vi.fn(async (prefs: unknown) => prefs),
  openUrl: vi.fn(),
  asCommandError: (e: unknown) =>
    e && typeof e === 'object' && 'code' in e ? e : { code: 'unexpected', message: String(e) },
}))

import AppSidebar from '@/components/AppSidebar.vue'
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
  dictationLanguage: 'en',
  defaultAssistantId: null,
  assistantIds: [],
  enabledSkills: [],
  models: {
    chat: 'openai:gpt-4o-mini:chat',
    voice: 'groq:whisper:voice',
    speak: '',
    vision: '',
    image: '',
    video: '',
    embed: 'ollama:bge-m3:vectorize',
    docs: 'openai:gpt-4o:docs',
    chatLegacyProviderId: null,
  },
  knowledgeFolder: 'DESKTOP:p1',
  projectDir: '/home/u/Synaplan/projects/homework',
  notesDir: '/home/u/Synaplan/projects/homework/notes',
  outDir: '/home/u/Synaplan/projects/homework/out',
}

async function factory(
  prefs = { language: null, sidebarCollapsed: false, historyCollapsed: false },
) {
  const pinia = createPinia()
  setActivePinia(pinia)
  const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages })
  vi.mocked(api.getUiPrefs).mockResolvedValue(prefs)
  vi.mocked(api.getStatus).mockResolvedValue({
    paired: true,
    apiBaseUrl: 'https://web.synaplan.com',
    deviceId: 4,
    keyBackend: 'keyring',
    keyIsPlaintext: false,
  })
  vi.mocked(api.listProjects).mockResolvedValue({
    projects: [homework],
    activeId: 'p1',
    personalId: 'p1',
  })
  await useConfigStore().load()
  await useProjectsStore().load()
  await useUiStore().loadPrefs()
  const wrapper = mount(AppSidebar, { global: { plugins: [pinia, i18n] } })
  await flushPromises()
  return wrapper
}

describe('AppSidebar', () => {
  beforeEach(() => {
    vi.mocked(api.setUiPrefs).mockClear()
  })

  it('shows the bird, the project and all pages with labels when unfolded', async () => {
    const wrapper = await factory()
    const img = wrapper.get('.brand-mark img')
    expect(img.attributes('src')).toContain('single_bird')
    expect(wrapper.get('.brand-mark source').attributes('media')).toContain('dark')
    expect(wrapper.text()).toContain('Synaplan Desktop')
    expect(wrapper.get('[data-testid="project-switcher"]').text()).toContain('Homework')
    expect(wrapper.get('[data-testid="nav-notes"]').text()).toBe('Notes')
    expect(wrapper.get('[data-testid="nav-settings"]').text()).toBe('Settings')
    expect(wrapper.text()).toContain('web.synaplan.com')
    expect(wrapper.classes()).not.toContain('collapsed')
  })

  it('folds to an icon rail and remembers it on this computer', async () => {
    const wrapper = await factory()
    await wrapper.get('[data-testid="sidebar-toggle"]').trigger('click')
    await flushPromises()

    expect(wrapper.classes()).toContain('collapsed')
    expect(api.setUiPrefs).toHaveBeenCalledWith({
      language: null,
      sidebarCollapsed: true,
      historyCollapsed: false,
    })
    // Icons only — the label moves into the tooltip; the project becomes its initial.
    expect(wrapper.get('[data-testid="nav-notes"]').text()).toBe('')
    expect(wrapper.get('[data-testid="nav-notes"]').attributes('title')).toBe('Notes')
    expect(wrapper.get('[data-testid="project-switcher"]').text()).toBe('H')
    expect(wrapper.find('.brand-mark img').exists()).toBe(true)
    expect(wrapper.text()).not.toContain('web.synaplan.com')

    // The folded project badge still opens the menu (to the right, on the body).
    await wrapper.get('[data-testid="project-switcher"]').trigger('click')
    await flushPromises()
    const menu = document.body.querySelector('[data-testid="project-menu"]')
    expect(menu?.textContent).toContain('Homework')
    expect(menu?.classList.contains('flyout')).toBe(true)
  })

  it('starts folded when that is how it was left', async () => {
    const wrapper = await factory({
      language: null,
      sidebarCollapsed: true,
      historyCollapsed: true,
    })
    expect(wrapper.classes()).toContain('collapsed')
    expect(wrapper.get('[data-testid="sidebar-toggle"]').attributes('aria-expanded')).toBe('false')
  })

  it('the connection line opens Settings', async () => {
    const wrapper = await factory()
    await wrapper.get('.conn').trigger('click')
    expect(useUiStore().view).toBe('settings')
  })
})
