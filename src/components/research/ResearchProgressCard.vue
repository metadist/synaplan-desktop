<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ResearchDetail, ResearchPoint } from '@/lib/researchRun'

defineProps<{
  depthLabel: string
  points: ResearchPoint[]
  found: number
  read: number
  closed: number
  now: string
  queuedSince: number | null
  details: ResearchDetail[]
}>()

const emit = defineEmits<{ stop: []; writeNow: [] }>()
const { t } = useI18n()
const open = ref(false)

function marker(status: string): string {
  if (status === 'answered' || status === 'done') {
    return t('research.pointDone')
  }
  if (status === 'reading' || status === 'open') {
    return t('research.pointNow')
  }
  return t('research.pointWait')
}
</script>

<template>
  <section class="card" data-testid="research-progress" aria-live="polite">
    <header>
      <h2>{{ t('research.progress', { depth: depthLabel }) }}</h2>
      <button class="btn btn-ghost" type="button" data-testid="research-stop" @click="emit('stop')">
        {{ t('research.stop') }}
      </button>
    </header>
    <ul>
      <li v-for="point in points" :key="point.id">{{ marker(point.status) }} {{ point.title }}</li>
    </ul>
    <p>{{ t('research.counts', { found, read, closed }) }}</p>
    <p v-if="queuedSince !== null && queuedSince >= 20">{{ t('research.waitingWorker') }}</p>
    <p v-else-if="now">{{ t('research.now', { title: now }) }}</p>
    <p>{{ t('research.closeWindow') }}</p>
    <div class="actions">
      <button
        class="btn btn-ghost"
        type="button"
        data-testid="research-write-now"
        @click="emit('writeNow')"
      >
        {{ t('research.writeNow') }}
      </button>
      <button class="btn-link" type="button" :aria-expanded="open" @click="open = !open">
        {{ t('research.details') }}
      </button>
    </div>
    <ul v-if="open" data-testid="research-details">
      <li v-for="(row, index) in details" :key="index">
        <template v-if="row.kind === 'search'">{{
          t('research.detailSearch', { query: row.query, scope: row.scope, results: row.results })
        }}</template>
        <template v-else>{{
          t('research.detailClosed', { title: row.title, reason: row.reason })
        }}</template>
      </li>
    </ul>
  </section>
</template>

<style scoped>
.card {
  margin: 0.8rem 1.2rem;
  padding: 0.9rem 1rem;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-card);
}
header,
.actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: 0.4rem;
  align-items: center;
}
</style>
