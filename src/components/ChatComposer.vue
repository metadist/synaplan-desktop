<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import DictationButton from '@/components/DictationButton.vue'

const props = defineProps<{
  modelValue: string
  picking: boolean
  webSearchOn: boolean
  webSearchBusy: boolean
  projectId: string
  sending: boolean
  hasChatModel: boolean
  agentMode: boolean
  hasVoiceModel: boolean
  dictationLanguage: string
  armed: boolean
  studioOpen: boolean
  researchAvailable?: boolean
  researchOn?: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
  send: []
  stop: []
  addFiles: []
  note: []
  toggleWeb: []
  toggleResearch: []
  'update:language': [code: string]
  dictationError: [error: unknown]
}>()

const { t } = useI18n()
const composerInput = ref<HTMLTextAreaElement | null>(null)
const dictationButton = ref<{ cancel: () => Promise<void> } | null>(null)
const dictating = ref(false)
let takeBase = ''

const draft = computed({
  get: () => props.modelValue,
  set: (value: string) => emit('update:modelValue', value),
})

const canKeepDraft = computed(() => props.modelValue.trim() !== '')

function focus(): void {
  composerInput.value?.focus()
}

async function cancelTake(): Promise<void> {
  if (dictationButton.value) {
    await dictationButton.value.cancel()
  }
  dictating.value = false
}

function onKeydown(event: KeyboardEvent): void {
  // Enter that confirms an IME candidate is not a send.
  if (event.key !== 'Enter' || event.shiftKey || event.isComposing) {
    return
  }
  event.preventDefault()
  // A take still writes into the box when it ends; sending now would bring the text back.
  if (!dictating.value) {
    emit('send')
  }
}

function onDictationStart(): void {
  dictating.value = true
  takeBase = props.modelValue
}

function writeTake(text: string): void {
  const glue = takeBase !== '' && !/\s$/.test(takeBase) ? ' ' : ''
  emit('update:modelValue', text ? takeBase + glue + text : takeBase)
}

function onDictationDone(text: string): void {
  writeTake(text)
  dictating.value = false
  void nextTick(() => composerInput.value?.focus())
}

function onDictationError(error: unknown): void {
  dictating.value = false
  emit('dictationError', error)
}

defineExpose({ focus, cancelTake })
</script>

<template>
  <div class="composer" :class="{ 'studio-open': studioOpen, armed }">
    <div class="composer-tools">
      <button
        class="tool-btn"
        type="button"
        :disabled="picking"
        :title="t('chat.attachFiles')"
        :aria-label="t('chat.attachFiles')"
        data-testid="composer-add-files"
        @click="emit('addFiles')"
      >
        📎
      </button>
      <button
        class="tool-btn"
        :class="{ keep: canKeepDraft }"
        type="button"
        :title="canKeepDraft ? t('chat.keepDraft') : t('chat.attachNote')"
        :aria-label="canKeepDraft ? t('chat.keepDraft') : t('chat.attachNote')"
        data-testid="composer-new-note"
        @click="emit('note')"
      >
        📝<span v-if="canKeepDraft" class="tool-label">{{ t('chat.keepShort') }}</span>
      </button>
      <button
        class="tool-btn web"
        :class="{ on: webSearchOn }"
        type="button"
        :disabled="webSearchBusy || !projectId"
        :aria-pressed="webSearchOn"
        :title="webSearchOn ? t('chat.webSearchOnHint') : t('chat.webSearchOffHint')"
        :aria-label="webSearchOn ? t('chat.webSearchOnHint') : t('chat.webSearchOffHint')"
        data-testid="composer-web-search"
        @click="emit('toggleWeb')"
      >
        🌐<span class="tool-label">{{ t('chat.webSearchShort') }}</span>
      </button>
      <button
        v-if="researchAvailable"
        class="tool-btn"
        :class="{ on: researchOn }"
        type="button"
        :aria-pressed="!!researchOn"
        data-testid="composer-research"
        @click="emit('toggleResearch')"
      >
        <span class="tool-label">{{ t('research.chip') }}</span>
      </button>
    </div>
    <textarea
      ref="composerInput"
      v-model="draft"
      class="input composer-input"
      rows="1"
      :placeholder="agentMode ? t('chat.placeholderSkills') : t('chat.placeholder')"
      :disabled="!hasChatModel"
      @keydown="onKeydown"
    ></textarea>
    <DictationButton
      v-if="hasVoiceModel"
      ref="dictationButton"
      :project-id="projectId"
      :language="dictationLanguage"
      :disabled="sending || !hasChatModel"
      @start="onDictationStart"
      @interim="writeTake"
      @done="onDictationDone"
      @error="onDictationError"
      @update:language="emit('update:language', $event)"
    />
    <button v-if="sending" class="btn btn-ghost" type="button" @click="emit('stop')">
      {{ t('chat.stop') }}
    </button>
    <button
      v-else
      class="btn btn-primary"
      type="button"
      :disabled="!hasChatModel || dictating || modelValue.trim().length === 0"
      @click="emit('send')"
    >
      {{ t('chat.send') }}
    </button>
  </div>
</template>

<style scoped>
.composer {
  display: flex;
  flex-wrap: wrap;
  gap: 0.6rem;
  align-items: flex-end;
  padding: 0.8rem 1.2rem;
  border-top: 1px solid var(--border);
  background: var(--bg-card);
}

.composer.studio-open {
  background: var(--accent-soft);
  border-top: 2px solid var(--accent);
  padding: 1rem 1.2rem 1.05rem;
}

.composer-tools {
  display: inline-flex;
  flex-wrap: wrap;
  gap: 0.2rem;
  align-self: flex-end;
  flex-shrink: 1;
  max-width: 100%;
}

.tool-btn {
  width: 40px;
  height: 40px;
  display: grid;
  place-items: center;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg);
  font-size: 1.05rem;
  cursor: pointer;
  transition: border-color 0.12s ease;
}

.tool-btn:hover {
  border-color: var(--accent);
}

.tool-btn:disabled {
  opacity: 0.55;
  cursor: default;
}

.tool-btn.keep {
  width: auto;
  padding: 0 0.7rem 0 0.55rem;
  grid-auto-flow: column;
  gap: 0.35rem;
  border-color: var(--accent);
  background: var(--accent-soft);
}

.tool-label {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--accent);
}

.tool-btn.web {
  width: auto;
  padding: 0 0.7rem 0 0.55rem;
  grid-auto-flow: column;
  gap: 0.35rem;
}

.tool-btn.web .tool-label {
  color: var(--txt-secondary);
}

.tool-btn.web.on {
  border-color: var(--accent);
  background: var(--accent-soft);
}

.tool-btn.web.on .tool-label {
  color: var(--accent);
}

.composer-input {
  flex: 1 1 12rem;
  min-width: 8rem;
  resize: none;
  max-height: 140px;
  min-height: 40px;
}

.composer > .btn {
  flex-shrink: 0;
  white-space: nowrap;
}

.composer.studio-open .composer-input {
  min-height: 72px;
  border-color: var(--accent);
  border-width: 2px;
  background: var(--bg-card);
}

.composer.armed .composer-input {
  animation: composer-armed 0.9s ease;
}

@keyframes composer-armed {
  0% {
    box-shadow: 0 0 0 0 color-mix(in srgb, var(--accent) 45%, transparent);
  }
  40% {
    box-shadow: 0 0 0 6px color-mix(in srgb, var(--accent) 22%, transparent);
  }
  100% {
    box-shadow: 0 0 0 0 transparent;
  }
}
</style>
