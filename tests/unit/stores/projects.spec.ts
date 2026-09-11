import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import type { Project, ProjectModels, ProjectsState } from '@/services/tauri'

vi.mock('@/services/tauri', () => ({
  listProjects: vi.fn(),
  applyDefaultModels: vi.fn(),
  setActiveProject: vi.fn(),
  createProject: vi.fn(),
  updateProject: vi.fn(),
  deleteProject: vi.fn(),
}))

import { useProjectsStore } from '@/stores/projects'
import * as api from '@/services/tauri'

const empty: ProjectModels = {
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

const ready: ProjectModels = {
  ...empty,
  chat: 'openai:gpt-4o-mini:chat',
  voice: 'groq:whisper:voice',
  embed: 'openai:text-embedding-3-small:embed',
  docs: 'openai:gpt-4o:docs',
}

function project(id: string, name: string, models: ProjectModels): Project {
  return {
    id,
    slug: name.toLowerCase(),
    name,
    kind: id === 'p1' ? 'personal' : 'project',
    createdAt: '2026-09-10T00:00:00Z',
    updatedAt: '2026-09-10T00:00:00Z',
    dictationLanguage: 'en',
    defaultAssistantId: null,
    assistantIds: [],
    enabledSkills: [],
    models,
    knowledgeFolder: `DESKTOP:${id}`,
    projectDir: `/home/u/Synaplan/projects/${name.toLowerCase()}`,
    notesDir: `/home/u/Synaplan/projects/${name.toLowerCase()}/notes`,
    outDir: `/home/u/Synaplan/projects/${name.toLowerCase()}/out`,
  }
}

const fresh = project('p2', 'Homework', empty)
const configured = project('p1', 'Personal', ready)

function state(active: string, projects: Project[]): ProjectsState {
  return { projects, activeId: active, personalId: 'p1' }
}

describe('projects store — recommended defaults', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    setActivePinia(createPinia())
  })

  it('asks for the workspace defaults when the active project has empty slots', async () => {
    vi.mocked(api.listProjects).mockResolvedValue(state('p2', [configured, fresh]))
    vi.mocked(api.applyDefaultModels).mockResolvedValue({ ...fresh, models: ready })

    const store = useProjectsStore()
    await store.load()
    await flushPromises()

    expect(api.applyDefaultModels).toHaveBeenCalledWith('p2')
    expect(store.active?.models.chat).toBe('openai:gpt-4o-mini:chat')
  })

  it('leaves a configured project alone — a user pick is never overwritten', async () => {
    vi.mocked(api.listProjects).mockResolvedValue(state('p1', [configured, fresh]))

    const store = useProjectsStore()
    await store.load()
    await flushPromises()

    expect(api.applyDefaultModels).not.toHaveBeenCalled()
  })

  it('tries once per project per session and stays usable offline', async () => {
    vi.mocked(api.listProjects).mockResolvedValue(state('p1', [configured, fresh]))
    vi.mocked(api.setActiveProject).mockResolvedValue(state('p2', [configured, fresh]))
    vi.mocked(api.applyDefaultModels).mockRejectedValue({ code: 'network', message: 'offline' })

    const store = useProjectsStore()
    await store.load()
    await store.select('p2')
    await flushPromises()
    expect(api.applyDefaultModels).toHaveBeenCalledTimes(1)
    expect(store.active?.id).toBe('p2')
    expect(store.active?.models.chat).toBe('')

    // Switching back and forth does not hammer the workspace.
    vi.mocked(api.setActiveProject).mockResolvedValue(state('p1', [configured, fresh]))
    await store.select('p1')
    vi.mocked(api.setActiveProject).mockResolvedValue(state('p2', [configured, fresh]))
    await store.select('p2')
    await flushPromises()
    expect(api.applyDefaultModels).toHaveBeenCalledTimes(1)

    // A new session (reset) may try again.
    store.reset()
    await store.load()
    vi.mocked(api.setActiveProject).mockResolvedValue(state('p2', [configured, fresh]))
    await store.select('p2')
    await flushPromises()
    expect(api.applyDefaultModels).toHaveBeenCalledTimes(2)
  })

  it('returns the project with its defaults already applied on create', async () => {
    vi.mocked(api.listProjects).mockResolvedValue(state('p1', [configured]))
    vi.mocked(api.createProject).mockResolvedValue(fresh)
    vi.mocked(api.setActiveProject).mockResolvedValue(state('p2', [configured, fresh]))
    vi.mocked(api.applyDefaultModels).mockResolvedValue({ ...fresh, models: ready })

    const store = useProjectsStore()
    await store.load()
    const created = await store.create('Homework', 'en', null)

    expect(api.applyDefaultModels).toHaveBeenCalledWith('p2')
    expect(created.models.embed).toBe('openai:text-embedding-3-small:embed')
    expect(store.active?.models.embed).toBe('openai:text-embedding-3-small:embed')
  })
})
