import { computed, onUnmounted, ref, watch, type Ref } from 'vue'
import * as api from '@/services/tauri'
import type { KnowledgeFile } from '@/services/tauri'

/** How often the list is re-read while a file is still being processed. */
export const PROCESSING_POLL_MS = 5000

export interface PendingUpload {
  path: string
  name: string
  error: unknown
}

/**
 * The project's knowledge folder on the workspace: list, add (by path), remove.
 * Every upload goes through Rust with this project's index model attached; the
 * composable only tracks what the person sees.
 */
export function useKnowledgeFiles(projectId: Ref<string>) {
  const files = ref<KnowledgeFile[]>([])
  const pending = ref<PendingUpload[]>([])
  const loading = ref(false)
  const error = ref<unknown>(null)

  let pollTimer: ReturnType<typeof setInterval> | null = null

  const processing = computed(() =>
    files.value.some((f) => f.state === 'sent' || f.state === 'reading' || f.state === 'indexing'),
  )

  async function refresh(): Promise<void> {
    const id = projectId.value
    if (!id) {
      files.value = []
      return
    }
    loading.value = true
    try {
      const list = await api.listProjectFiles(id)
      if (projectId.value === id) {
        files.value = list
        error.value = null
      }
    } catch (e) {
      if (projectId.value === id) {
        error.value = e
      }
    } finally {
      loading.value = false
    }
  }

  function fileName(path: string): string {
    const parts = path.split(/[\\/]/)
    return parts[parts.length - 1] || path
  }

  /** Upload the given OS paths one by one; failures stay listed with their reason. */
  async function add(paths: string[]): Promise<void> {
    const id = projectId.value
    if (!id) {
      return
    }
    for (const path of paths) {
      const trimmed = path.trim()
      if (!trimmed) {
        continue
      }
      const row: PendingUpload = { path: trimmed, name: fileName(trimmed), error: null }
      pending.value = [...pending.value.filter((p) => p.path !== trimmed), row]
      try {
        const uploaded = await api.uploadProjectFile(id, trimmed)
        if (projectId.value === id) {
          files.value = [uploaded, ...files.value.filter((f) => f.id !== uploaded.id)]
          pending.value = pending.value.filter((p) => p.path !== trimmed)
        }
      } catch (e) {
        if (projectId.value === id) {
          pending.value = pending.value.map((p) => (p.path === trimmed ? { ...p, error: e } : p))
        }
      }
    }
  }

  function dismiss(path: string): void {
    pending.value = pending.value.filter((p) => p.path !== path)
  }

  async function remove(fileId: number): Promise<void> {
    const id = projectId.value
    try {
      await api.deleteProjectFile(id, fileId)
      if (projectId.value === id) {
        files.value = files.value.filter((f) => f.id !== fileId)
      }
    } catch (e) {
      error.value = e
    }
  }

  function stopPolling(): void {
    if (pollTimer !== null) {
      clearInterval(pollTimer)
      pollTimer = null
    }
  }

  watch(
    processing,
    (active) => {
      stopPolling()
      if (active) {
        pollTimer = setInterval(() => void refresh(), PROCESSING_POLL_MS)
      }
    },
    { immediate: true },
  )

  watch(projectId, () => {
    files.value = []
    pending.value = []
    error.value = null
    void refresh()
  })

  onUnmounted(stopPolling)

  return { files, pending, loading, error, processing, refresh, add, dismiss, remove }
}
