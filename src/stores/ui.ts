import { defineStore } from 'pinia'
import { ref } from 'vue'

export type View = 'chat' | 'skills' | 'computer' | 'doctor'

/** Which primary section the app shell is showing. */
export const useUiStore = defineStore('ui', () => {
  const view = ref<View>('chat')

  function setView(next: View): void {
    view.value = next
  }

  return { view, setView }
})
