import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

/** Sentinel language: detect and transcribe as spoken (no forced translation). */
export const DICTATION_AUTO = 'auto'

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
  const { t, locale } = useI18n()

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

  /** Auto-detect first, then the fixed languages. */
  const optionsWithAuto = computed<LanguageOption[]>(() => [
    { code: DICTATION_AUTO, label: t('dictation.languageAuto') },
    ...options.value,
  ])

  /** New projects auto-detect, so nothing gets translated by surprise. */
  const defaultCode = computed(() => DICTATION_AUTO)

  /** Full label for a code, including a friendly name for `auto` and empty. */
  function labelFor(code: string): string {
    if (!code || code === DICTATION_AUTO) {
      return t('dictation.languageAuto')
    }
    return options.value.find((o) => o.code === code)?.label ?? code
  }

  /** Compact label for the mic chip: `Auto` or the uppercased code. */
  function shortLabel(code: string): string {
    if (!code || code === DICTATION_AUTO) {
      return t('dictation.languageAutoShort')
    }
    return code.toUpperCase()
  }

  return { options, optionsWithAuto, defaultCode, labelFor, shortLabel }
}
