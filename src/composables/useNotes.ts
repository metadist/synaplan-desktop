import { computed, onUnmounted, ref, watch, type Ref } from 'vue'
import * as api from '@/services/tauri'
import type { Note, NoteSummary } from '@/services/tauri'

/** Edits are written this long after the last keystroke. */
export const AUTOSAVE_DELAY_MS = 800

/**
 * The note manager for one project: list + search, open, create, autosave,
 * delete. Notes are Markdown files on this computer; the only handle held here
 * is the file name — paths stay in Rust.
 */
export function useNotes(projectId: Ref<string>) {
  const notes = ref<NoteSummary[]>([])
  const query = ref('')
  const current = ref<Note | null>(null)
  const draft = ref('')
  const loading = ref(false)
  const saving = ref(false)
  const error = ref<unknown>(null)

  let saveTimer: ReturnType<typeof setTimeout> | null = null

  const dirty = computed(() => current.value !== null && draft.value !== current.value.content)

  async function refresh(): Promise<void> {
    const id = projectId.value
    if (!id) {
      notes.value = []
      return
    }
    loading.value = true
    try {
      const list = await api.listNotes(id, query.value)
      if (projectId.value === id) {
        notes.value = list
      }
    } catch (e) {
      error.value = e
    } finally {
      loading.value = false
    }
  }

  function cancelTimer(): void {
    if (saveTimer !== null) {
      clearTimeout(saveTimer)
      saveTimer = null
    }
  }

  /** Write the draft now if it differs from what is on disk. */
  async function flush(): Promise<void> {
    cancelTimer()
    const note = current.value
    if (!note || !dirty.value) {
      return
    }
    const id = projectId.value
    const content = draft.value
    saving.value = true
    error.value = null
    try {
      const summary = await api.writeNote(id, note.name, content)
      if (current.value?.name === note.name) {
        current.value = { ...note, content, title: summary.title, updatedAt: summary.updatedAt }
      }
      notes.value = notes.value.map((n) => (n.name === summary.name ? summary : n))
      if (!notes.value.some((n) => n.name === summary.name)) {
        await refresh()
      }
    } catch (e) {
      error.value = e
    } finally {
      saving.value = false
    }
  }

  /** Called on every edit: keep the draft and schedule a save. */
  function edit(content: string): void {
    draft.value = content
    cancelTimer()
    if (dirty.value) {
      saveTimer = setTimeout(() => void flush(), AUTOSAVE_DELAY_MS)
    }
  }

  async function open(name: string): Promise<void> {
    await flush()
    error.value = null
    try {
      const note = await api.readNote(projectId.value, name)
      current.value = note
      draft.value = note.content
    } catch (e) {
      error.value = e
    }
  }

  async function create(): Promise<void> {
    await flush()
    error.value = null
    try {
      const note = await api.createNote(projectId.value)
      current.value = note
      draft.value = note.content
      await refresh()
    } catch (e) {
      error.value = e
    }
  }

  async function remove(name: string): Promise<void> {
    cancelTimer()
    error.value = null
    try {
      await api.deleteNote(projectId.value, name)
      if (current.value?.name === name) {
        current.value = null
        draft.value = ''
      }
      await refresh()
    } catch (e) {
      error.value = e
    }
  }

  function close(): void {
    void flush()
    current.value = null
    draft.value = ''
  }

  watch(projectId, async (_next, prev) => {
    if (prev) {
      await flush()
    }
    current.value = null
    draft.value = ''
    query.value = ''
    await refresh()
  })

  watch(query, () => void refresh())

  onUnmounted(() => {
    void flush()
  })

  return {
    notes,
    query,
    current,
    draft,
    dirty,
    loading,
    saving,
    error,
    refresh,
    open,
    create,
    edit,
    flush,
    remove,
    close,
  }
}
