import { ref, watch, type Ref } from 'vue'
import * as api from '@/services/tauri'
import type { CatalogEntry, ModelCatalog, ModelSlot } from '@/services/tauri'
import { useProjectsStore } from '@/stores/projects'

/**
 * The workspace model catalog for the Models panel, reloaded per project.
 * Never invents entries: when the workspace has no catalog yet the result says
 * so (`catalogMissing`) and the persisted picks stay untouched.
 */
export function useModelCatalog(projectId: Ref<string>) {
  const projects = useProjectsStore()
  const catalog = ref<ModelCatalog | null>(null)
  const loading = ref(false)
  const error = ref<unknown>(null)

  async function load(): Promise<void> {
    const id = projectId.value
    if (!id) {
      catalog.value = null
      return
    }
    loading.value = true
    error.value = null
    try {
      const result = await api.getModelCatalog(id)
      if (projectId.value !== id) {
        return
      }
      catalog.value = result.catalog
      if (result.rebound) {
        await projects.load()
      }
    } catch (e) {
      if (projectId.value === id) {
        error.value = e
      }
    } finally {
      if (projectId.value === id) {
        loading.value = false
      }
    }
  }

  function entries(slot: ModelSlot): CatalogEntry[] {
    return catalog.value?.slots[slot] ?? []
  }

  watch(projectId, () => void load(), { immediate: true })

  return { catalog, loading, error, load, entries }
}

/** The provider id a person recognises; never the engineers' `service:providerId:tag`. */
export function displayModelId(binding: string): string {
  const parts = binding.split(':')
  return parts.length >= 3 ? parts.slice(1, -1).join(':') : binding
}

/** `true` when the binding is a full catalog key rather than a bare provider id. */
export function isCatalogKey(binding: string): boolean {
  const parts = binding.split(':')
  return parts.length >= 3 && parts.every((p) => p.length > 0)
}
