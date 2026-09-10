import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

/** ISO 639-1 codes offered in the dictation-language picker. */
export const DICTATION_LANGUAGES = [
  'en',
  'de',
  'es',
  'fr',
  'tr',
  'it',
  'pt',
  'nl',
  'pl',
  'sv',
  'da',
  'nb',
  'fi',
  'cs',
  'el',
  'ru',
  'uk',
  'ar',
  'ja',
  'zh',
] as const

export interface LanguageOption {
  code: string
  label: string
}

/**
 * Localized language names via `Intl.DisplayNames` (no strings to translate);
 * falls back to the code when the runtime lacks the data.
 */
export function useDictationLanguages() {
  const { locale } = useI18n()

  const options = computed<LanguageOption[]>(() => {
    let names: Intl.DisplayNames | null = null
    try {
      names = new Intl.DisplayNames([locale.value, 'en'], { type: 'language' })
    } catch {
      names = null
    }
    return DICTATION_LANGUAGES.map((code): LanguageOption => {
      let label: string = code
      try {
        label = names?.of(code) ?? code
      } catch {
        label = code
      }
      return { code, label: label.charAt(0).toUpperCase() + label.slice(1) }
    })
  })

  /** The UI language when it is offered, otherwise English. */
  const defaultCode = computed(() =>
    (DICTATION_LANGUAGES as readonly string[]).includes(locale.value) ? locale.value : 'en',
  )

  function labelFor(code: string): string {
    return options.value.find((o) => o.code === code)?.label ?? code
  }

  return { options, defaultCode, labelFor }
}
