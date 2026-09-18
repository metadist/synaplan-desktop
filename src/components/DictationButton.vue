<script setup lang="ts">
import { computed, toRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useDictation } from '@/composables/useDictation'
import { DICTATION_AUTO, useDictationLanguages } from '@/composables/useDictationLanguages'

/**
 * Microphone toggle for one dictation take, with a visible language control.
 * Interim text streams to the parent while recording and during the correction
 * pass; the final text arrives on stop. The parent decides where the text goes
 * (caret, composer) and persists the language choice on the project.
 *
 * The language is a native <select> so the OS positions and scrolls the list —
 * a hand-rolled drop-up clipped at the window edge. `Auto` detects and
 * transcribes as spoken; a fixed language expects that language (and can
 * translate other languages into it).
 */
const props = defineProps<{
  projectId: string
  /** The project's dictation language: `auto` (or empty) detects; a code fixes it. */
  language?: string
  disabled?: boolean
}>()

const emit = defineEmits<{
  start: []
  interim: [text: string]
  done: [text: string]
  error: [error: unknown]
  'update:language': [code: string]
}>()

const { t } = useI18n()
const languages = useDictationLanguages()

const dictation = useDictation(toRef(props, 'projectId'), {
  prompt: () => t('dictation.prompt'),
})

const busy = computed(
  () => dictation.state.value === 'starting' || dictation.state.value === 'finishing',
)
const recording = computed(() => dictation.state.value === 'recording')

/** '' from an older project counts as auto so the control is never blank. */
const activeLanguage = computed(() => props.language || DICTATION_AUTO)
const languageShort = computed(() => languages.shortLabel(activeLanguage.value))
const languageLabel = computed(() => languages.labelFor(activeLanguage.value))
const isAuto = computed(() => activeLanguage.value === DICTATION_AUTO)

function onLanguageChange(event: Event): void {
  const code = (event.target as HTMLSelectElement).value
  if (code !== activeLanguage.value) {
    emit('update:language', code)
  }
}

watch(dictation.interim, (text) => {
  if (recording.value || dictation.state.value === 'finishing') {
    emit('interim', text)
  }
})

watch(dictation.error, (e) => {
  if (e !== null) {
    emit('error', e)
  }
})

watch(
  () => props.projectId,
  () => {
    void dictation.cancel()
  },
)

async function toggle(): Promise<void> {
  if (busy.value || props.disabled) {
    return
  }
  if (recording.value) {
    const text = await dictation.stop()
    emit('done', text)
    return
  }
  if (await dictation.start()) {
    emit('start')
  }
}

defineExpose({ recording, busy, cancel: dictation.cancel })
</script>

<template>
  <div class="dictation">
    <label
      class="lang"
      :class="{ auto: isAuto }"
      :title="t('dictation.languageTitle', { language: languageLabel })"
    >
      <span class="globe" aria-hidden="true">🌐</span>
      <select
        class="lang-select"
        :value="activeLanguage"
        :disabled="disabled || busy || recording"
        :aria-label="t('dictation.languageTitle', { language: languageLabel })"
        data-testid="dictation-language"
        @change="onLanguageChange"
      >
        <option v-for="opt in languages.optionsWithAuto.value" :key="opt.code" :value="opt.code">
          {{ opt.label }}
        </option>
      </select>
    </label>

    <button
      class="mic"
      :class="{ recording, busy }"
      type="button"
      :disabled="disabled || busy"
      :title="
        recording
          ? t('dictation.stop')
          : busy
            ? t('dictation.correcting')
            : t('dictation.startIn', { language: languageLabel })
      "
      :aria-label="
        recording
          ? t('dictation.stop')
          : busy
            ? t('dictation.correcting')
            : t('dictation.startIn', { language: languageLabel })
      "
      :aria-pressed="recording"
      data-testid="dictation-toggle"
      @click="toggle"
    >
      <span v-if="busy" class="spinner small"></span>
      <span v-else-if="recording" class="stop-square"></span>
      <svg v-else class="mic-icon" viewBox="0 0 24 24" aria-hidden="true">
        <path
          fill="currentColor"
          d="M12 15a4 4 0 0 0 4-4V6a4 4 0 1 0-8 0v5a4 4 0 0 0 4 4Zm6-4a6 6 0 0 1-5 5.92V20h3v2H8v-2h3v-3.08A6 6 0 0 1 6 11h2a4 4 0 0 0 8 0h2Z"
        />
      </svg>
      <span v-if="recording" class="label"
        >{{ t('dictation.listening') }} · {{ languageShort }}</span
      >
      <span v-else-if="busy" class="label">{{ t('dictation.correcting') }}</span>
    </button>
  </div>
</template>

<style scoped>
.dictation {
  display: inline-flex;
  align-items: stretch;
  gap: 0.3rem;
}

.lang {
  display: inline-flex;
  align-items: center;
  gap: 0.2rem;
  padding: 0 0.5rem 0 0.55rem;
  border: 1px solid var(--border-strong);
  border-radius: 999px;
  background: var(--bg-card);
  cursor: pointer;
}

.lang:hover {
  border-color: var(--accent);
}

.lang.auto {
  border-color: color-mix(in srgb, var(--accent) 45%, var(--border-strong));
}

.globe {
  font-size: 0.9rem;
  line-height: 1;
}

.lang-select {
  border: none;
  background: transparent;
  color: var(--txt);
  font: inherit;
  font-size: 0.78rem;
  font-weight: 600;
  padding: 0.35rem 0.1rem;
  cursor: pointer;
  max-width: 150px;
}

.lang.auto .lang-select {
  color: var(--accent);
}

.lang-select:disabled {
  opacity: 0.6;
  cursor: default;
}

.lang-select:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
  border-radius: 4px;
}

.mic {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.35rem 0.6rem;
  border: 1px solid var(--border-strong);
  border-radius: 999px;
  background: var(--bg-card);
  color: var(--txt);
  font: inherit;
  font-size: 0.8rem;
  cursor: pointer;
}
.mic:hover:not(:disabled) {
  border-color: var(--accent);
}
.mic:disabled {
  opacity: 0.6;
  cursor: default;
}
.mic.recording {
  border-color: var(--danger);
  color: var(--danger);
}
.mic-icon {
  width: 16px;
  height: 16px;
}
.stop-square {
  width: 10px;
  height: 10px;
  border-radius: 2px;
  background: var(--danger);
  animation: pulse 1.2s ease-in-out infinite;
}
.spinner.small {
  width: 12px;
  height: 12px;
}
.label {
  font-weight: 600;
}
@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.35;
  }
}

@media (prefers-reduced-motion: reduce) {
  .stop-square {
    animation: none;
  }
}
</style>
