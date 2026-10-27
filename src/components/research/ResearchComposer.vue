<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { researchMinutes as minutes, researchStartBlock } from '@/lib/researchRun'

const props = defineProps<{
  question: string
  depth: string
  web: boolean
  files: boolean
  webAvailable: boolean
  filesReady: number
  depths: string[]
  thoroughBusy: boolean
  standardLeft: number | null
  thoroughLeft: number | null
  busy: boolean
}>()

const emit = defineEmits<{
  'update:question': [value: string]
  'update:depth': [value: string]
  'update:web': [value: boolean]
  'update:files': [value: boolean]
  start: []
}>()

const { t } = useI18n()

const block = computed(() =>
  researchStartBlock({
    question: props.question,
    web: props.web,
    files: props.files,
    webAvailable: props.webAvailable,
    filesReady: props.filesReady,
    depth: props.depth,
    thoroughBusy: props.thoroughBusy,
    standardLeft: props.standardLeft,
    thoroughLeft: props.thoroughLeft,
  }),
)
</script>

<template>
  <div class="research-composer" data-testid="research-composer">
    <textarea
      class="input"
      rows="2"
      :value="question"
      :placeholder="t('research.placeholder')"
      data-testid="research-question"
      @input="emit('update:question', ($event.target as HTMLTextAreaElement).value)"
      @keydown.enter.exact.prevent="block || busy ? undefined : emit('start')"
    ></textarea>
    <p class="depth-label">{{ t('research.depthQuestion') }}</p>
    <div class="depths" role="radiogroup" :aria-label="t('research.depthQuestion')">
      <label v-for="item in depths" :key="item" class="depth">
        <input
          type="radio"
          name="research-depth"
          :value="item"
          :checked="depth === item"
          @change="emit('update:depth', item)"
        />
        {{ t(`research.depth.${item}`, { minutes: minutes(item) }) }}
      </label>
    </div>
    <p class="sources">{{ t('research.sources') }}</p>
    <label class="source">
      <input
        type="checkbox"
        :checked="web"
        :disabled="!webAvailable"
        @change="emit('update:web', ($event.target as HTMLInputElement).checked)"
      />
      {{ webAvailable ? t('research.web') : t('research.webUnavailable') }}
    </label>
    <label class="source">
      <input
        type="checkbox"
        :checked="files"
        :disabled="filesReady < 1"
        @change="emit('update:files', ($event.target as HTMLInputElement).checked)"
      />
      {{
        filesReady > 0 ? t('research.files', { count: filesReady }) : t('research.filesUnavailable')
      }}
    </label>
    <p v-if="block" class="hint" data-testid="research-start-hint">{{ t(block) }}</p>
    <button
      class="btn btn-primary"
      type="button"
      :disabled="!!block || busy"
      data-testid="research-start"
      @click="emit('start')"
    >
      {{ t('research.start') }}
    </button>
  </div>
</template>

<style scoped>
.research-composer {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  padding: 0.8rem 1.2rem 0;
}
.depths,
.source {
  display: flex;
  flex-wrap: wrap;
  gap: 0.8rem;
}
.hint {
  margin: 0;
  color: var(--muted);
}
</style>
