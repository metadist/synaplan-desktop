import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import * as api from '@/services/tauri'

/**
 * Holds the pairing status loaded from the Rust side. The API base URL and
 * device id live here for display; the API key never crosses into the webview —
 * it stays in the OS secret store.
 */
export const useConfigStore = defineStore('config', () => {
  const status = ref<api.Status | null>(null)
  const loading = ref(true)

  const paired = computed(() => status.value?.paired ?? false)
  const apiBaseUrl = computed(() => status.value?.apiBaseUrl ?? null)
  const keyIsPlaintext = computed(() => status.value?.keyIsPlaintext ?? false)

  async function load(): Promise<void> {
    loading.value = true
    try {
      status.value = await api.getStatus()
    } finally {
      loading.value = false
    }
  }

  async function refresh(): Promise<void> {
    status.value = await api.getStatus()
  }

  function setStatus(next: api.Status): void {
    status.value = next
  }

  async function signOut(): Promise<void> {
    await api.signOut()
    await refresh()
  }

  return { status, loading, paired, apiBaseUrl, keyIsPlaintext, load, refresh, setStatus, signOut }
})
