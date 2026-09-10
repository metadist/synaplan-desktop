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
    if (!projectId.value) {
      threads.value = []
      return
    }
    loading.value = true
    try {
      threads.value = await api.listChats(projectId.value)
    } finally {
      loading.value = false
    }
  }

  async function open(chatId: string): Promise<api.ChatThread> {
    const thread = await api.loadChat(projectId.value, chatId)
    current.value = thread
    return thread
  }

  /** Start composing a new thread; nothing is written until the first message. */
  function startNew(): void {
    current.value = null
  }

  /** Save `messages` into the open thread, minting the thread on first use. */
  async function persist(messages: api.StoredChatMessage[]): Promise<void> {
    if (!projectId.value) {
      return
    }
    if (!current.value || current.value.projectId !== projectId.value) {
      current.value = await api.newChat(projectId.value)
    }
    current.value.messages = messages
    await api.saveChat(current.value)
    await refresh()
    const saved = threads.value.find((t) => t.id === current.value?.id)
    if (saved && current.value) {
      current.value.title = saved.title
      current.value.updatedAt = saved.updatedAt
    }
  }

  async function remove(chatId: string): Promise<void> {
    await api.deleteChat(projectId.value, chatId)
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
