import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as api from '@/services/tauri'
import { detectLocale, i18n, supportedLanguages, type SupportedLanguage } from '@/i18n'

/**
 * Primary rail: the five project views. Machine-level screens (`computer`,
 * `doctor`, `skills`, `settings`) are reached from the sidebar footer, never
 * from the rail.
 */
export type View =
  'chat' | 'notes' | 'files' | 'agents' | 'models' | 'computer' | 'doctor' | 'skills' | 'settings'

export const PROJECT_VIEWS: readonly View[] = ['chat', 'notes', 'files', 'agents', 'models']
export const MACHINE_VIEWS: readonly View[] = ['computer', 'doctor', 'skills', 'settings']

/**
 * Which section the app shell is showing, plus how the person likes the
 * window: interface language and which columns are folded to a rail. The
 * preferences are persisted by the Rust side and survive sign-out.
 */
export const useUiStore = defineStore('ui', () => {
  const view = ref<View>('chat')
  const sidebarCollapsed = ref(false)
  const historyCollapsed = ref(false)
  /** The picked interface language; null follows the system. */
  const language = ref<SupportedLanguage | null>(null)

  function setView(next: View): void {
    view.value = next
  }

  function applyLanguage(): void {
    i18n.global.locale.value = language.value ?? detectLocale()
  }

  function apply(prefs: api.UiPrefs): void {
    sidebarCollapsed.value = prefs.sidebarCollapsed
    historyCollapsed.value = prefs.historyCollapsed
    language.value = (supportedLanguages as readonly string[]).includes(prefs.language ?? '')
      ? (prefs.language as SupportedLanguage)
      : null
    applyLanguage()
  }

  async function loadPrefs(): Promise<void> {
    try {
      apply(await api.getUiPrefs())
    } catch {
      // Defaults are fine; the window still works.
    }
  }

  async function save(): Promise<void> {
    try {
      await api.setUiPrefs({
        language: language.value,
        sidebarCollapsed: sidebarCollapsed.value,
        historyCollapsed: historyCollapsed.value,
      })
    } catch {
      // A preference that did not stick is not worth an error banner.
    }
  }

  function toggleSidebar(): void {
    sidebarCollapsed.value = !sidebarCollapsed.value
    void save()
  }

  function setHistoryCollapsed(collapsed: boolean): void {
    if (historyCollapsed.value === collapsed) {
      return
    }
    historyCollapsed.value = collapsed
    void save()
  }

  async function setLanguage(next: SupportedLanguage | null): Promise<void> {
    language.value = next
    applyLanguage()
    await save()
  }

  return {
    view,
    sidebarCollapsed,
    historyCollapsed,
    language,
    setView,
    loadPrefs,
    toggleSidebar,
    setHistoryCollapsed,
    setLanguage,
  }
})
