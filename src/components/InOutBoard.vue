<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { KnowledgeFile, OutFile } from '@/services/tauri'

/**
 * The In / Out activity board: what went into the project's knowledge folder
 * and what skills produced in its `out/` folder. Plain language, no ids.
 */
const props = defineProps<{
  files: KnowledgeFile[]
  filesLoading: boolean
  outFiles: OutFile[]
  outLoading: boolean
}>()

const emit = defineEmits<{
  openFiles: []
  reveal: [path: string]
  revealOut: []
}>()

const { t, locale } = useI18n()

const ready = computed(() => props.files.filter((f) => f.state === 'ready'))
const working = computed(() =>
  props.files.filter((f) => f.state === 'sent' || f.state === 'reading' || f.state === 'indexing'),
)
const failed = computed(() =>
  props.files.filter((f) => f.state === 'failed' || f.state === 'stale'),
)
const latestReady = computed(
  () => [...ready.value].sort((a, b) => b.uploadedAt.localeCompare(a.uploadedAt))[0] ?? null,
)

const OUT_LIMIT = 8
const recentOut = computed(() => props.outFiles.slice(0, OUT_LIMIT))

function when(iso: string): string {
  const date = new Date(iso)
  if (!iso || Number.isNaN(date.getTime())) {
    return ''
  }
  return new Intl.DateTimeFormat(locale.value, { dateStyle: 'medium', timeStyle: 'short' }).format(
    date,
  )
}
</script>

<template>
  <div class="board">
    <section class="col card" data-testid="inout-in">
      <header class="col-head">
        <h3 class="col-title">{{ t('inout.inTitle') }}</h3>
        <button class="btn-link" type="button" @click="emit('openFiles')">
          {{ t('inout.openFiles') }} →
        </button>
      </header>
      <p class="muted col-hint">{{ t('inout.inHint') }}</p>

      <p v-if="filesLoading && files.length === 0" class="muted">{{ t('common.loading') }}</p>
      <p v-else-if="files.length === 0" class="muted" data-testid="inout-in-empty">
        {{ t('inout.inEmpty') }}
      </p>
      <ul v-else class="facts">
        <li data-testid="inout-ready">{{ t('inout.ready', { count: ready.length }) }}</li>
        <li v-if="working.length > 0" data-testid="inout-working">
          <span class="spinner small"></span> {{ t('inout.working', { count: working.length }) }}
        </li>
        <li v-if="failed.length > 0" class="bad" data-testid="inout-failed">
          {{ t('inout.failed', { count: failed.length }) }}
        </li>
        <li v-if="latestReady" class="muted" data-testid="inout-latest">
          {{ t('inout.latest', { name: latestReady.name, when: when(latestReady.uploadedAt) }) }}
        </li>
      </ul>
    </section>

    <section class="col card" data-testid="inout-out">
      <header class="col-head">
        <h3 class="col-title">{{ t('inout.outTitle') }}</h3>
        <button
          class="btn-link"
          type="button"
          data-testid="inout-reveal-out"
          @click="emit('revealOut')"
        >
          {{ t('inout.showFolder') }} →
        </button>
      </header>
      <p class="muted col-hint">{{ t('inout.outHint') }}</p>

      <p v-if="outLoading && outFiles.length === 0" class="muted">{{ t('common.loading') }}</p>
      <p v-else-if="outFiles.length === 0" class="muted" data-testid="inout-out-empty">
        {{ t('inout.outEmpty') }}
      </p>
      <ul v-else class="out-list">
        <li v-for="f in recentOut" :key="f.path" class="out-row">
          <button
            class="out-name"
            type="button"
            :title="t('inout.showFolder')"
            :data-testid="`out-${f.name}`"
            @click="emit('reveal', f.path)"
          >
            {{ f.name }}
          </button>
          <span class="muted out-when">{{ when(f.modifiedAt) }}</span>
        </li>
        <li v-if="outFiles.length > recentOut.length" class="muted more">
          {{ t('inout.more', { count: outFiles.length - recentOut.length }) }}
        </li>
      </ul>
    </section>
  </div>
</template>

<style scoped>
.board {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 0.8rem;
}
.col {
  padding: 0.85rem 1rem;
}
.col-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}
.col-title {
  margin: 0;
  font-size: 0.95rem;
}
.col-hint {
  margin: 0.15rem 0 0.6rem;
  font-size: 0.78rem;
}
.facts {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  font-size: 0.86rem;
}
.facts .bad {
  color: var(--danger);
}
.spinner.small {
  width: 11px;
  height: 11px;
  display: inline-block;
  vertical-align: middle;
}
.out-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}
.out-row {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 0.6rem;
  font-size: 0.86rem;
}
.out-name {
  border: none;
  background: none;
  padding: 0;
  font: inherit;
  color: var(--accent);
  cursor: pointer;
  text-align: left;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}
.out-name:hover {
  text-decoration: underline;
}
.out-when {
  font-size: 0.74rem;
  flex-shrink: 0;
}
.more {
  font-size: 0.78rem;
}
</style>
