import { useI18n } from 'vue-i18n'
import { asCommandError } from '@/services/tauri'

const KNOWN_CODES = new Set([
  'invalid_url',
  'invalid_code',
  'invalid_key',
  'feature_disabled',
  'rate_limited',
  'unauthorized',
  'network',
  'gateway_disabled',
  'model_unavailable',
  'not_paired',
  'secret_store',
  'secret_store_unavailable',
  'config',
  'unexpected',
  'autostart',
  'project_io',
  'project_corrupt',
  'project_not_found',
  'project_invalid_name',
  'project_last',
  'project_invalid_id',
  'chat_model_unset',
  'embed_model_unset',
  'file_outside_allowed',
  'file_denied',
  'file_unreadable',
  'file_too_large',
  'assistant_not_bound',
  'assistants_disabled',
  'voice_model_unset',
  'voice_model_unknown',
  'dictation_session_gone',
  'microphone_denied',
])

/**
 * Turn any thrown command error (or stream error payload) into a localized,
 * user-facing string. Known error codes map to a translated message; unknown
 * ones fall back to the server-provided message, then a generic string.
 */
export function useErrorText() {
  const { t, te } = useI18n()

  return (err: unknown): string => {
    const e = asCommandError(err)
    const key = `errors.${e.code}`
    if (KNOWN_CODES.has(e.code) && te(key)) {
      return t(key)
    }
    return e.message || t('errors.generic')
  }
}
