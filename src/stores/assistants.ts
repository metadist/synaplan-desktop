import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import * as api from '@/services/tauri'

export type AssistantsState = 'idle' | 'loading' | 'ready' | 'disabled' | 'error'

/**
 * The Assistants the paired key may run, shared by the Agents board and the
 * chat header. Loaded once per session on demand; `disabled` is the honest
 * "turned off on this workspace" state, distinct from an empty list.
 */
export const useAssistantsStore = defineStore('assistants', () => {
  const list = ref<api.Assistant[]>([])
  const state = ref<AssistantsState>('idle')
  const error = ref<unknown>(null)

  const byId = computed(() => new Map(list.value.map((a) => [a.id, a])))

  async function load(force = false): Promise<void> {
    if (!force && (state.value === 'loading' || state.value === 'ready')) {
      return
    }
    state.value = 'loading'
    error.value = null
    try {
      list.value = await api.listAssistants()
      state.value = 'ready'
    } catch (e) {
      list.value = []
      if (api.asCommandError(e).code === 'assistants_disabled') {
        state.value = 'disabled'
      } else {
        error.value = e
        state.value = 'error'
      }
    }
  }

  function name(id: number | null): string {
    if (id === null) {
      return ''
    }
    return byId.value.get(id)?.name ?? ''
  }

  function reset(): void {
    list.value = []
    state.value = 'idle'
    error.value = null
  }

  return { list, state, error, byId, load, name, reset }
})
