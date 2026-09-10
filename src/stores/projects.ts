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

  async function load(): Promise<void> {
    loading.value = true
    try {
      apply(await api.listProjects())
    } finally {
      loading.value = false
    }
  }

  async function select(id: string): Promise<void> {
    if (id === activeId.value) {
      return
    }
    apply(await api.setActiveProject(id))
  }

  /** Create and switch to a project. `copyModelsFrom` copies that project's eight slots. */
  async function create(
    name: string,
    dictationLanguage: string,
    copyModelsFrom: string | null,
  ): Promise<api.Project> {
    const project = await api.createProject(name, dictationLanguage, copyModelsFrom)
    apply(await api.setActiveProject(project.id))
    return project
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
    reset,
  }
})
