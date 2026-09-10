import { ref, watch, type Ref } from 'vue'
import * as api from '@/services/tauri'

/**
 * The per-project chat threads behind the Chat view. Threads are files the
 * Rust side owns; this composable only mirrors the list and the open thread.
 * Switching project drops the open thread and reloads the list.
 */
export function useChatThreads(projectId: Ref<string>) {
  const threads = ref<api.ChatSummary[]>([])
  const current = ref<api.ChatThread | null>(null)
  const loading = ref(false)

  async function refresh(): Promise<void> {
    const id = projectId.value
    if (!id) {
      threads.value = []
      return
    }
    loading.value = true
    try {
      const list = await api.listChats(id)
      if (projectId.value === id) {
        threads.value = list
      }
    } finally {
      if (projectId.value === id) {
        loading.value = false
      }
    }
  }

  async function open(chatId: string): Promise<api.ChatThread> {
    const id = projectId.value
    const thread = await api.loadChat(id, chatId)
    if (projectId.value === id && thread.projectId === id && thread.id === chatId) {
      current.value = thread
    }
    return thread
  }

  /** Start composing a new thread; nothing is written until the first message. */
  function startNew(): void {
    current.value = null
  }

  /**
   * Save `messages` into the open thread, minting the thread on first use.
   * `assistantId` is the Assistant pinned on this thread (`null` = project default).
   */
  async function persist(
    messages: api.StoredChatMessage[],
    assistantId: number | null = null,
  ): Promise<void> {
    const id = projectId.value
    if (!id) {
      return
    }
    if (!current.value || current.value.projectId !== id) {
      const minted = await api.newChat(id)
      if (projectId.value !== id) {
        return
      }
      current.value = minted
    }
    current.value.messages = messages
    current.value.assistantId = assistantId
    const thread = current.value
    await api.saveChat(thread)
    if (projectId.value !== id || current.value?.id !== thread.id) {
      return
    }
    await refresh()
    if (projectId.value !== id) {
      return
    }
    const saved = threads.value.find((t) => t.id === current.value?.id)
    if (saved && current.value) {
      current.value.title = saved.title
      current.value.updatedAt = saved.updatedAt
    }
  }

  async function remove(chatId: string): Promise<void> {
    const id = projectId.value
    await api.deleteChat(id, chatId)
    if (projectId.value !== id) {
      return
    }
    if (current.value?.id === chatId) {
      current.value = null
    }
    await refresh()
  }

  watch(projectId, () => {
    current.value = null
    void refresh().catch(() => {
      threads.value = []
    })
  })

  return { threads, current, loading, refresh, open, startNew, persist, remove }
}
