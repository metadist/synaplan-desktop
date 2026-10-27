<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ResearchCitation, ResearchSource } from '@/lib/researchRun'

defineProps<{
  source: ResearchSource | null
  quotes: ResearchCitation[]
  number: number
}>()

const emit = defineEmits<{ close: []; openPage: []; openFile: []; copyQuote: [text: string] }>()
const { t } = useI18n()

function onKey(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    emit('close')
  }
}

onMounted(() => window.addEventListener('keydown', onKey))
onUnmounted(() => window.removeEventListener('keydown', onKey))
</script>

<template>
  <div
    v-if="source"
    class="drawer"
    role="dialog"
    aria-modal="true"
    data-testid="research-source"
    @click.self="emit('close')"
  >
    <div class="panel">
      <h2>{{ t('research.sourceLabel', { n: number, title: source.title }) }}</h2>
      <p>{{ source.host }}</p>
      <p v-for="quote in quotes" :key="quote.n">{{ quote.sentence }}</p>
      <div class="actions">
        <button v-if="source.url" class="btn btn-primary" type="button" @click="emit('openPage')">
          {{ t('research.openPage') }}
        </button>
        <button
          v-if="source.fileId"
          class="btn btn-primary"
          type="button"
          @click="emit('openFile')"
        >
          {{ t('research.openFile') }}
        </button>
        <button
          class="btn btn-ghost"
          type="button"
          @click="emit('copyQuote', quotes[0]?.sentence || source.title)"
        >
          {{ t('research.copyQuote') }}
        </button>
        <button class="btn btn-ghost" type="button" @click="emit('close')">
          {{ t('common.close') }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.drawer {
  position: fixed;
  inset: 0;
  background: color-mix(in srgb, var(--bg) 30%, transparent);
  display: flex;
  justify-content: flex-end;
}
.panel {
  width: min(28rem, 100%);
  background: var(--bg-card);
  border-left: 1px solid var(--border);
  padding: 1rem;
}
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}
</style>
