import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { messages } from '@/i18n'
import type { Assistant, KnowledgeFile, OutFile, Project, Skill } from '@/services/tauri'

vi.mock('@/services/tauri', () => ({
  listProjects: vi.fn(),
  updateProject: vi.fn(),
  listAssistants: vi.fn(),
  listSkills: vi.fn(),
  listProjectFiles: vi.fn(),
  listOutFiles: vi.fn(),
  revealPath: vi.fn().mockResolvedValue(undefined),
  asCommandError: (e: unknown) =>
    e && typeof e === 'object' && 'code' in e ? e : { code: 'unexpected', message: String(e) },
}))

import AgentsView from '@/views/AgentsView.vue'
import * as api from '@/services/tauri'
import { useProjectsStore } from '@/stores/projects'

function project(extra: Partial<Project> = {}): Project {
  return {
    id: 'p1',
    slug: 'work',
    name: 'Work',
    kind: 'project',
    createdAt: '2026-09-10T00:00:00Z',
    updatedAt: '2026-09-10T00:00:00Z',
    dictationLanguage: 'en',
    defaultAssistantId: null,
    assistantIds: [],
    enabledSkills: [],
    models: {
      chat: 'ollama:llama3.2:chat',
      voice: '',
      speak: '',
      vision: '',
      image: '',
      video: '',
      embed: 'ollama:bge-m3:vectorize',
      docs: '',
      chatLegacyProviderId: null,
    },
    knowledgeFolder: 'DESKTOP:p1',
    projectDir: '/home/u/Synaplan/projects/work',
    notesDir: '/home/u/Synaplan/projects/work/notes',
    outDir: '/home/u/Synaplan/projects/work/out',
    ...extra,
  }
}

function assistant(id: number, name: string, models: Assistant['models'] = null): Assistant {
  return { id, name, description: `${name} does things`, icon: null, status: 'published', models }
}

function skill(name: string, extra: Partial<Skill> = {}): Skill {
  return {
    name,
    description: `${name} skill`,
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

function knowledgeFile(id: number, name: string, state: KnowledgeFile['state']): KnowledgeFile {
  return { id, name, size: 10, state, detail: null, uploadedAt: `2026-09-1${id}T10:00:00Z` }
}

function outFile(name: string): OutFile {
  return {
    name,
    size: 5,
    modifiedAt: '2026-09-10T12:00:00Z',
    path: `/home/u/Synaplan/projects/work/out/${name}`,
  }
}

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
  const wrapper = mount(AgentsView, { global: { plugins: [pinia, i18n] } })
  await flushPromises()
  return wrapper
}

describe('AgentsView — Assistants', () => {
  beforeEach(() => {
    vi.mocked(api.listAssistants).mockReset()
    vi.mocked(api.listSkills).mockReset()
    vi.mocked(api.listSkills).mockResolvedValue([])
    vi.mocked(api.listProjectFiles).mockReset()
    vi.mocked(api.listProjectFiles).mockResolvedValue([])
    vi.mocked(api.listOutFiles).mockReset()
    vi.mocked(api.listOutFiles).mockResolvedValue([])
    vi.mocked(api.revealPath).mockClear()
    vi.mocked(api.updateProject).mockReset()
    vi.mocked(api.updateProject).mockImplementation(async (id, patch) =>
      project({ id, ...patch } as Partial<Project>),
    )
  })

  it('names the turned-off state instead of showing an empty list', async () => {
    vi.mocked(api.listAssistants).mockRejectedValue({ code: 'assistants_disabled', message: '' })
    const wrapper = await factory(project())

    expect(wrapper.get('[data-testid="assistants-disabled"]').text()).toBe(
      'Assistants are turned off on this workspace.',
    )
    expect(wrapper.find('[data-testid="assistant-list"]').exists()).toBe(false)
  })

  it('binds an Assistant, makes the first one default, and persists both', async () => {
    vi.mocked(api.listAssistants).mockResolvedValue([
      assistant(7, 'Writer'),
      assistant(9, 'Reviewer'),
    ])
    const wrapper = await factory(project())

    await wrapper.get('[data-testid="assistant-7-bind"]').setValue(true)
    await flushPromises()
    expect(api.updateProject).toHaveBeenCalledWith('p1', {
      assistantIds: [7],
      defaultAssistantId: 7,
    })
    expect(wrapper.find('[data-testid="assistant-7-default"]').exists()).toBe(true)

    await wrapper.get('[data-testid="assistant-9-bind"]').setValue(true)
    await flushPromises()
    expect(api.updateProject).toHaveBeenLastCalledWith('p1', {
      assistantIds: [7, 9],
      defaultAssistantId: 7,
    })

    await wrapper.get('[data-testid="assistant-9-make-default"]').trigger('click')
    await flushPromises()
    expect(api.updateProject).toHaveBeenLastCalledWith('p1', {
      assistantIds: [7, 9],
      defaultAssistantId: 9,
    })
  })

  it('unbinding the default clears it', async () => {
    vi.mocked(api.listAssistants).mockResolvedValue([assistant(7, 'Writer')])
    const wrapper = await factory(project({ assistantIds: [7], defaultAssistantId: 7 }))

    await wrapper.get('[data-testid="assistant-7-bind"]').setValue(false)
    await flushPromises()
    expect(api.updateProject).toHaveBeenLastCalledWith('p1', {
      assistantIds: [],
      defaultAssistantId: null,
    })
  })

  it('warns when a bound recipe was written for other models and keeps the project models', async () => {
    vi.mocked(api.listAssistants).mockResolvedValue([
      assistant(7, 'Writer', {
        chat: 'anthropic:claude-sonnet-4:chat',
        vision: null,
        vectorize: 'ollama:bge-m3:vectorize',
      }),
      assistant(9, 'Reviewer', {
        chat: 'anthropic:claude-sonnet-4:chat',
        vision: null,
        vectorize: null,
      }),
    ])
    const wrapper = await factory(project({ assistantIds: [7], defaultAssistantId: 7 }))

    const warning = wrapper.get('[data-testid="assistant-7-world"]')
    expect(warning.text()).toContain('set up to use other models')
    expect(warning.text()).toContain('Chat: the Assistant expects claude-sonnet-4')
    expect(warning.text()).not.toContain('anthropic:claude-sonnet-4:chat')
    // Unbound recipes do not nag.
    expect(wrapper.find('[data-testid="assistant-9-world"]').exists()).toBe(false)
    // Nothing was written to the project's models.
    expect(api.updateProject).not.toHaveBeenCalled()
  })
})

describe('AgentsView — skills overlay and In/Out', () => {
  beforeEach(() => {
    vi.mocked(api.listAssistants).mockReset()
    vi.mocked(api.listAssistants).mockResolvedValue([])
    vi.mocked(api.listSkills).mockReset()
    vi.mocked(api.listSkills).mockResolvedValue([])
    vi.mocked(api.listProjectFiles).mockReset()
    vi.mocked(api.listProjectFiles).mockResolvedValue([])
    vi.mocked(api.listOutFiles).mockReset()
    vi.mocked(api.listOutFiles).mockResolvedValue([])
    vi.mocked(api.revealPath).mockClear()
    vi.mocked(api.updateProject).mockReset()
    vi.mocked(api.updateProject).mockImplementation(async (id, patch) =>
      project({ id, ...patch } as Partial<Project>),
    )
  })

  it('lets a project narrow the installed skills but never widen them', async () => {
    vi.mocked(api.listSkills).mockResolvedValue([
      skill('slides'),
      skill('csv-insights'),
      skill('off-here', { enabled: false }),
      skill('no-python', { blocked: true, blockedReason: 'Python is missing' }),
    ])
    const wrapper = await factory(project({ enabledSkills: ['slides'] }))

    expect(
      (wrapper.get('[data-testid="skill-slides-toggle"]').element as HTMLInputElement).checked,
    ).toBe(true)
    expect(
      (wrapper.get('[data-testid="skill-off-here-toggle"]').element as HTMLInputElement).disabled,
    ).toBe(true)
    expect(wrapper.get('[data-testid="skill-off-here-status"]').text()).toContain(
      'Off on this computer',
    )
    expect(
      (wrapper.get('[data-testid="skill-no-python-toggle"]').element as HTMLInputElement).disabled,
    ).toBe(true)
    expect(wrapper.get('[data-testid="skill-no-python-status"]').text()).toContain(
      'Python is missing',
    )

    await wrapper.get('[data-testid="skill-csv-insights-toggle"]').setValue(true)
    await flushPromises()
    expect(api.updateProject).toHaveBeenLastCalledWith('p1', {
      enabledSkills: ['slides', 'csv-insights'],
    })

    await wrapper.get('[data-testid="skill-slides-toggle"]').setValue(false)
    await flushPromises()
    expect(api.updateProject).toHaveBeenLastCalledWith('p1', { enabledSkills: ['csv-insights'] })
  })

  it('summarises In and Out in plain language and reveals the out folder', async () => {
    vi.mocked(api.listProjectFiles).mockResolvedValue([
      knowledgeFile(1, 'a.pdf', 'ready'),
      knowledgeFile(2, 'b.pdf', 'ready'),
      knowledgeFile(3, 'c.docx', 'indexing'),
      knowledgeFile(4, 'd.txt', 'failed'),
    ])
    vi.mocked(api.listOutFiles).mockResolvedValue([outFile('report.pptx'), outFile('data.csv')])
    const wrapper = await factory(project())

    expect(wrapper.get('[data-testid="inout-ready"]').text()).toBe('2 ready for chat')
    expect(wrapper.get('[data-testid="inout-working"]').text()).toContain('1 still being indexed')
    expect(wrapper.get('[data-testid="inout-failed"]').text()).toBe('1 could not be indexed')
    expect(wrapper.get('[data-testid="inout-latest"]').text()).toContain('b.pdf')
    expect(wrapper.text()).not.toMatch(/DESKTOP:|group_key|sk_/)

    expect(wrapper.get('[data-testid="out-report.pptx"]').text()).toBe('report.pptx')
    await wrapper.get('[data-testid="inout-reveal-out"]').trigger('click')
    expect(api.revealPath).toHaveBeenCalledWith('/home/u/Synaplan/projects/work/out')
    await wrapper.get('[data-testid="out-data.csv"]').trigger('click')
    expect(api.revealPath).toHaveBeenLastCalledWith('/home/u/Synaplan/projects/work/out/data.csv')
  })

  it('shows empty states when nothing went in or out yet', async () => {
    vi.mocked(api.listSkills).mockResolvedValue([])
    const wrapper = await factory(project())

    expect(wrapper.find('[data-testid="inout-in-empty"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="inout-out-empty"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="project-skills-empty"]').exists()).toBe(true)
  })
})
