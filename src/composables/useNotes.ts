import { computed, onUnmounted, ref, watch, type Ref } from 'vue'
import * as api from '@/services/tauri'
import type { Note, NoteSummary } from '@/services/tauri'

/** Edits are written this long after the last keystroke. */
export const AUTOSAVE_DELAY_MS = 800

/** A first line longer than this is not turned into the heading. */
const MAX_TITLE_CHARS = 80

/**
 * Turn any text — an answer, the composer draft — into a note. The first line
 * becomes the `#` heading the Rust side reads the title from, unless the text
 * already starts with a heading or the line is too long to be one. An optional
 * footer records where the text came from (the chat it was kept from).
 */
export function composeNote(text: string, footer = ''): string {
  const body = text.trim()
  const lines = body.split('\n')
  const first = (lines[0] ?? '').trim()
  let content = body
  if (/^#\s+\S/.test(first)) {
    content = body
  } else {
    const plain = first
      .replace(/^#+\s*/, '')
      .replace(/^([-*+>]|\d+[.)])\s+/, '')
      .replace(/[*_`~]/g, '')
      .trim()
    if (plain !== '' && plain.length <= MAX_TITLE_CHARS) {
      const rest = lines.slice(1).join('\n').trim()
      content = rest === '' ? `# ${plain}` : `# ${plain}\n\n${rest}`
    }
  }
  return footer === '' ? content : `${content}\n\n---\n\n${footer}`
}

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
  let flushChain: Promise<boolean> = Promise.resolve(true)

  const dirty = computed(() => current.value !== null && draft.value !== current.value.content)

  async function refresh(): Promise<void> {
    const id = projectId.value
    const q = query.value
    if (!id) {
      notes.value = []
      return
    }
    loading.value = true
    try {
      const list = await api.listNotes(id, q)
      if (projectId.value === id && query.value === q) {
        notes.value = list
      }
    } catch (e) {
      if (projectId.value === id && query.value === q) {
        error.value = e
      }
    } finally {
      if (projectId.value === id) {
        loading.value = false
      }
    }
  }

  function cancelTimer(): void {
    if (saveTimer !== null) {
      clearTimeout(saveTimer)
      saveTimer = null
    }
  }

  async function writeDraft(id: string, note: Note, content: string): Promise<boolean> {
    saving.value = true
    error.value = null
    try {
      const summary = await api.writeNote(id, note.name, content)
      if (projectId.value === id && current.value?.name === note.name) {
        current.value = { ...note, content, title: summary.title, updatedAt: summary.updatedAt }
      }
      if (projectId.value === id) {
        notes.value = notes.value.map((n) => (n.name === summary.name ? summary : n))
        if (!notes.value.some((n) => n.name === summary.name)) {
          await refresh()
        }
      }
      return true
    } catch (e) {
      error.value = e
      return false
    } finally {
      saving.value = false
    }
  }

  /** Write the draft now if it differs from what is on disk. False on failure. */
  function flush(forProject: string = projectId.value): Promise<boolean> {
    cancelTimer()
    const note = current.value
    const content = draft.value
    if (!note || content === note.content || !forProject) {
      return Promise.resolve(true)
    }
    const run = flushChain.then(
      () => writeDraft(forProject, note, content),
      () => writeDraft(forProject, note, content),
    )
    flushChain = run.then(
      () => true,
      () => true,
    )
    return run
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
    if (!(await flush())) {
      return
    }
    const id = projectId.value
    error.value = null
    try {
      const note = await api.readNote(id, name)
      if (projectId.value !== id) {
        return
      }
      current.value = note
      draft.value = note.content
    } catch (e) {
      if (projectId.value === id) {
        error.value = e
      }
    }
  }

  async function create(): Promise<void> {
    if (!(await flush())) {
      return
    }
    const id = projectId.value
    error.value = null
    try {
      const note = await api.createNote(id)
      if (projectId.value !== id) {
        return
      }
      current.value = note
      draft.value = note.content
      await refresh()
    } catch (e) {
      if (projectId.value === id) {
        error.value = e
      }
    }
  }

  /**
   * Save text as a new note without opening it: the one-click "keep" from a
   * chat answer or the composer. Returns the saved note, or null on failure.
   */
  async function keep(text: string, footer = ''): Promise<NoteSummary | null> {
    const id = projectId.value
    if (!id || text.trim() === '') {
      return null
    }
    error.value = null
    try {
      const note = await api.createNote(id)
      const summary = await api.writeNote(id, note.name, composeNote(text, footer))
      if (projectId.value === id) {
        await refresh()
      }
      return summary
    } catch (e) {
      if (projectId.value === id) {
        error.value = e
      }
      return null
    }
  }

  async function remove(name: string): Promise<void> {
    cancelTimer()
    const id = projectId.value
    error.value = null
    try {
      await api.deleteNote(id, name)
      if (projectId.value !== id) {
        return
      }
      if (current.value?.name === name) {
        current.value = null
        draft.value = ''
      }
      await refresh()
    } catch (e) {
      if (projectId.value === id) {
        error.value = e
      }
    }
  }

  function close(): void {
    void flush()
    current.value = null
    draft.value = ''
  }

  watch(projectId, async (_next, prev) => {
    if (prev) {
      await flush(prev)
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
    keep,
    remove,
    close,
  }
}
