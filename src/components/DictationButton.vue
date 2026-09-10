<script setup lang="ts">
import { computed, toRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useDictation } from '@/composables/useDictation'

/**
 * Microphone toggle for one dictation take. Interim text streams to the parent
 * while recording; the final text (one-shot preferred) arrives once on stop.
 * The parent decides where the text goes (caret, composer).
 */
const props = defineProps<{
  projectId: string
  disabled?: boolean
}>()

const emit = defineEmits<{
  start: []
  interim: [text: string]
  done: [text: string]
  error: [error: unknown]
}>()

const { t } = useI18n()

const dictation = useDictation(toRef(props, 'projectId'), {
  prompt: () => t('dictation.prompt'),
})

const busy = computed(
  () => dictation.state.value === 'starting' || dictation.state.value === 'finishing',
)
const recording = computed(() => dictation.state.value === 'recording')

watch(dictation.interim, (text) => {
  if (recording.value) {
    emit('interim', text)
  }
})

watch(dictation.error, (e) => {
  if (e !== null) {
    emit('error', e)
  }
})

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

defineExpose({ recording, busy })
</script>

<template>
  <button
    class="mic"
    :class="{ recording, busy }"
    type="button"
    :disabled="disabled || busy"
    :title="recording ? t('dictation.stop') : t('dictation.start')"
    :aria-label="recording ? t('dictation.stop') : t('dictation.start')"
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
    <span v-if="recording" class="label">{{ t('dictation.listening') }}</span>
  </button>
</template>

<style scoped>
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
</style>
