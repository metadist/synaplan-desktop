<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import MessageText from '@/components/MessageText.vue'
import type { ResearchCitation, ResearchSource } from '@/lib/researchRun'

const props = defineProps<{
  markdown: string
  sources: ResearchSource[]
  citations: ResearchCitation[]
  version: number
  versions: number[]
}>()

const emit = defineEmits<{
  close: []
  cite: [sourceId: string]
  ask: [text: string]
  deeper: []
  restore: [version: number]
  revise: [text: string]
}>()

const { t } = useI18n()
const askText = ref('')
const html = computed(() => props.markdown)

function sourceNumber(id: string): number {
  const index = props.sources.findIndex((source) => source.id === id)
  return index >= 0 ? index + 1 : 0
}
</script>

<template>
  <section class="report" data-testid="research-report">
    <header>
      <h2>{{ t('research.report') }}</h2>
      <label>
        {{ t('research.version', { n: version }) }}
        <select
          :value="version"
          @change="emit('restore', Number(($event.target as HTMLSelectElement).value))"
        >
          <option v-for="item in versions" :key="item" :value="item">{{ item }}</option>
        </select>
      </label>
      <button class="btn btn-ghost" type="button" @click="emit('close')">
        {{ t('common.close') }}
      </button>
    </header>
    <MessageText :content="html" />
    <p class="chips">
      <button
        v-for="cite in citations"
        :key="cite.n"
        class="btn btn-ghost"
        type="button"
        :aria-label="
          t('research.sourceLabel', { n: sourceNumber(cite.source), title: cite.source })
        "
        @click="emit('cite', cite.source)"
      >
        {{ cite.marker }}
      </button>
    </p>
    <h3>{{ t('research.sources') }} ({{ sources.length }})</h3>
    <ol>
      <li v-for="(source, index) in sources" :key="source.id">
        <button class="btn-link" type="button" @click="emit('cite', source.id)">
          [{{ index + 1 }}] {{ source.title }}
        </button>
        <span v-if="source.host"> · {{ source.host }}</span>
      </li>
    </ol>
    <form class="ask" @submit.prevent="askText.trim() && emit('ask', askText.trim())">
      <input
        v-model="askText"
        class="input"
        type="text"
        :placeholder="t('research.askPlaceholder')"
      />
      <button class="btn btn-primary" type="submit">{{ t('research.ask') }}</button>
      <button class="btn btn-ghost" type="button" @click="emit('revise', t('research.shorter'))">
        {{ t('research.shorter') }}
      </button>
      <button class="btn btn-ghost" type="button" @click="emit('deeper')">
        {{ t('research.deeper') }}
      </button>
    </form>
  </section>
</template>

<style scoped>
.report {
  margin: 0.8rem 1.2rem;
  padding: 0.9rem 1rem;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-card);
}
header,
.ask,
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
  align-items: center;
}
</style>
