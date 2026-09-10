import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import * as api from '@/services/tauri'

/**
 * The local project list and the active project. Everything is persisted by
 * the Rust side; this store only mirrors the last answer so the switcher and
 * the views share one source of truth.
 */
export const useProjectsStore = defineStore('projects', () => {
  const projects = ref<api.Project[]>([])
  const activeId = ref('')
  const personalId = ref('')
  const loaded = ref(false)
  const loading = ref(false)
  let selectGen = 0
  /** Projects whose defaults were already requested this session (no re-fetch on every switch). */
  const defaultsTried = new Set<string>()

  const active = computed<api.Project | null>(
    () => projects.value.find((p) => p.id === activeId.value) ?? projects.value[0] ?? null,
  )

  function apply(state: api.ProjectsState): void {
    projects.value = state.projects
    activeId.value = state.activeId
    personalId.value = state.personalId
    loaded.value = true
  }

  function replace(project: api.Project): void {
    const idx = projects.value.findIndex((p) => p.id === project.id)
    if (idx === -1) {
      projects.value = [...projects.value, project]
    } else {
      projects.value = projects.value.map((p) => (p.id === project.id ? project : p))
    }
  }

  /** True when at least one of the slots the app actually uses is still empty. */
  function missingUsedSlot(project: api.Project): boolean {
    const m = project.models
    return m.chat === '' || m.voice === '' || m.embed === '' || m.docs === ''
  }

  /**
   * Make sure the project starts with the workspace's recommended models. Runs
   * at most once per project and session, only when a used slot is empty, and
   * never blocks the UI: when the workspace cannot be reached the project stays
   * as it is and the views offer the Models panel instead.
   */
  async function ensureDefaults(id: string = activeId.value): Promise<void> {
    const project = projects.value.find((p) => p.id === id)
    if (!project || defaultsTried.has(id) || !missingUsedSlot(project)) {
      return
    }
    defaultsTried.add(id)
    try {
      replace(await api.applyDefaultModels(id))
    } catch {
      // Offered, not forced: the chat and files panels explain what is missing.
    }
  }

  async function load(): Promise<void> {
    loading.value = true
    try {
      apply(await api.listProjects())
    } finally {
      loading.value = false
    }
    void ensureDefaults()
  }

  async function select(id: string): Promise<void> {
    if (id === activeId.value) {
      return
    }
    const gen = ++selectGen
    const state = await api.setActiveProject(id)
    if (gen === selectGen) {
      apply(state)
      void ensureDefaults(id)
    }
  }

  /** Create and switch to a project. `copyModelsFrom` copies that project's eight slots. */
  async function create(
    name: string,
    dictationLanguage: string,
    copyModelsFrom: string | null,
  ): Promise<api.Project> {
    const project = await api.createProject(name, dictationLanguage, copyModelsFrom)
    apply(await api.setActiveProject(project.id))
    await ensureDefaults(project.id)
    return projects.value.find((p) => p.id === project.id) ?? project
  }

  async function update(id: string, patch: api.ProjectPatch): Promise<api.Project> {
    const project = await api.updateProject(id, patch)
    replace(project)
    return project
  }

  async function remove(id: string, removeFiles: boolean): Promise<void> {
    apply(await api.deleteProject(id, removeFiles))
  }

  function reset(): void {
    projects.value = []
    activeId.value = ''
    personalId.value = ''
    loaded.value = false
    defaultsTried.clear()
  }

  return {
    projects,
    activeId,
    personalId,
    loaded,
    loading,
    active,
    load,
    select,
    create,
    update,
    remove,
    ensureDefaults,
    reset,
  }
})
