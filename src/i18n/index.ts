import { createI18n } from 'vue-i18n'
import en from './en.json'
import de from './de.json'
import es from './es.json'
import fr from './fr.json'
import tr from './tr.json'

export const supportedLanguages = ['en', 'de', 'es', 'fr', 'tr'] as const
export type SupportedLanguage = (typeof supportedLanguages)[number]

export const messages = { en, de, es, fr, tr }

function detectLocale(): SupportedLanguage {
  const nav = typeof navigator !== 'undefined' ? navigator.language : 'en'
  const short = (nav || 'en').slice(0, 2).toLowerCase()
  return (supportedLanguages as readonly string[]).includes(short)
    ? (short as SupportedLanguage)
    : 'en'
}

export const i18n = createI18n({
  legacy: false,
  locale: detectLocale(),
  fallbackLocale: 'en',
  messages,
})
