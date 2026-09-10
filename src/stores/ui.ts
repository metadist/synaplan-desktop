import { defineStore } from 'pinia'
import { ref } from 'vue'

/**
 * Primary rail: the five project views. Machine-level screens (`computer`,
 * `doctor`, `skills`) are reached from the sidebar footer, never from the rail.
 */
export type View =
  'chat' | 'notes' | 'files' | 'agents' | 'models' | 'computer' | 'doctor' | 'skills'

export const PROJECT_VIEWS: readonly View[] = ['chat', 'notes', 'files', 'agents', 'models']
export const MACHINE_VIEWS: readonly View[] = ['computer', 'doctor', 'skills']

/** Which section the app shell is showing. */
export const useUiStore = defineStore('ui', () => {
  const view = ref<View>('chat')

  function setView(next: View): void {
    view.value = next
  }

  return { view, setView }
})
