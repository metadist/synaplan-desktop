<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, toRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useDictation } from '@/composables/useDictation'
import { DICTATION_AUTO, useDictationLanguages } from '@/composables/useDictationLanguages'

/**
 * Microphone toggle for one dictation take, with a visible language control.
 * Interim text streams to the parent while recording and during the correction
 * pass; the final text arrives on stop. The parent decides where the text goes
 * (caret, composer) and persists the language choice on the project.
 *
 * The language list is drawn by the app, not the operating system. A native
 * select on Windows opens a light menu and copies this page's light text
 * into it, so the names disappear. `Auto` detects and transcribes as spoken;
 * a fixed language expects that language (and can translate other languages
 * into it).
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
const menuOpen = ref(false)
const langRoot = ref<HTMLElement | null>(null)

function toggleMenu(): void {
  if (props.disabled || busy.value || recording.value) {
    return
  }
  menuOpen.value = !menuOpen.value
}

function pickLanguage(code: string): void {
  menuOpen.value = false
  if (code !== activeLanguage.value) {
    emit('update:language', code)
  }
}

function onDocumentPointer(event: MouseEvent): void {
  const root = langRoot.value
  if (!menuOpen.value || !root) {
    return
  }
  if (event.target instanceof Node && root.contains(event.target)) {
    return
  }
  menuOpen.value = false
}

function onDocumentKey(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    menuOpen.value = false
  }
}

onMounted(() => {
  document.addEventListener('pointerdown', onDocumentPointer)
  document.addEventListener('keydown', onDocumentKey)
})

onUnmounted(() => {
  document.removeEventListener('pointerdown', onDocumentPointer)
  document.removeEventListener('keydown', onDocumentKey)
})

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
    <div ref="langRoot" class="lang" :class="{ auto: isAuto, open: menuOpen }">
      <button
        class="lang-trigger"
        type="button"
        :disabled="disabled || busy || recording"
        :title="t('dictation.languageTitle', { language: languageLabel })"
        :aria-label="t('dictation.languageTitle', { language: languageLabel })"
        aria-haspopup="listbox"
        :aria-expanded="menuOpen"
        data-testid="dictation-language"
        @click="toggleMenu"
      >
        <span class="globe" aria-hidden="true">🌐</span>
        <span class="lang-name">{{ languageLabel }}</span>
      </button>
      <ul
        v-if="menuOpen"
        class="lang-menu"
        role="listbox"
        :aria-label="t('dictation.languageTitle', { language: languageLabel })"
        data-testid="dictation-language-menu"
      >
        <li v-for="opt in languages.optionsWithAuto.value" :key="opt.code">
          <button
            type="button"
            role="option"
            :aria-selected="opt.code === activeLanguage"
            :data-lang="opt.code"
            @click="pickLanguage(opt.code)"
          >
            {{ opt.label }}
          </button>
        </li>
      </ul>
    </div>

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
  position: relative;
  display: inline-flex;
  align-items: stretch;
}

.lang-trigger {
  display: inline-flex;
  align-items: center;
  gap: 0.2rem;
  padding: 0 0.65rem 0 0.55rem;
  border: 1px solid var(--border-strong);
  border-radius: 999px;
  background: var(--bg-card);
  color: var(--txt);
  font: inherit;
  cursor: pointer;
}

.lang-trigger:hover:not(:disabled) {
  border-color: var(--accent);
}

.lang.auto .lang-trigger {
  border-color: color-mix(in srgb, var(--accent) 45%, var(--border-strong));
  color: var(--accent);
}

.lang-trigger:disabled {
  opacity: 0.6;
  cursor: default;
}

.lang-trigger:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

.globe {
  font-size: 0.9rem;
  line-height: 1;
}

.lang-name {
  font-size: 0.78rem;
  font-weight: 600;
  max-width: 150px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.lang-menu {
  position: absolute;
  right: 0;
  bottom: calc(100% + 6px);
  z-index: 40;
  width: max-content;
  min-width: 11rem;
  max-height: 16rem;
  margin: 0;
  padding: 0.3rem;
  overflow-y: auto;
  list-style: none;
  background: var(--bg-card);
  color: var(--txt);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  box-shadow: var(--shadow-lg);
}

.lang-menu button {
  display: block;
  width: 100%;
  padding: 0.4rem 0.6rem;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--txt);
  font: inherit;
  font-size: 0.82rem;
  text-align: start;
  cursor: pointer;
}

.lang-menu button:hover {
  background: var(--bg-elevated);
}

.lang-menu button[aria-selected='true'] {
  background: var(--accent-soft);
  color: var(--txt);
  font-weight: 650;
}

.lang-menu button:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
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
