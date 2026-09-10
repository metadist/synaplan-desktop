<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { KnowledgeFile } from '@/services/tauri'

/** The files in the knowledge folder with their plain-language state. */
defineProps<{
  files: KnowledgeFile[]
  loading: boolean
}>()

const emit = defineEmits<{
  remove: [file: KnowledgeFile]
  refresh: []
}>()

const { t, locale } = useI18n()

function size(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} B`
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(0)} KB`
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

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
  <div class="list" data-testid="files-list">
    <div class="list-head">
      <h2 class="list-title">{{ t('files.inFolder') }}</h2>
      <button
        class="btn btn-ghost small"
        type="button"
        :disabled="loading"
        data-testid="files-refresh"
        @click="emit('refresh')"
      >
        {{ t('files.refresh') }}
      </button>
    </div>

    <p v-if="!loading && files.length === 0" class="muted empty">{{ t('files.none') }}</p>

    <ul v-else class="rows">
      <li v-for="file in files" :key="file.id" class="row card" :data-testid="`file-${file.id}`">
        <div class="row-main">
          <span class="row-name">{{ file.name }}</span>
          <span class="row-meta muted">{{ size(file.size) }} · {{ when(file.uploadedAt) }}</span>
        </div>
        <div class="row-state">
          <span class="state" :class="file.state" :data-testid="`file-${file.id}-state`">
            <span
              v-if="file.state === 'reading' || file.state === 'indexing'"
              class="spinner small"
            ></span>
            {{ t(`files.states.${file.state}`) }}
          </span>
          <span v-if="file.state === 'failed' && file.detail" class="detail">{{
            file.detail
          }}</span>
        </div>
        <button
          class="btn btn-ghost small danger"
          type="button"
          :data-testid="`file-${file.id}-remove`"
          @click="emit('remove', file)"
        >
          {{ t('files.remove') }}
        </button>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.list {
  max-width: 720px;
}
.list-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 0.5rem;
}
.list-title {
  font-size: 0.9rem;
}
.small {
  padding: 0.3rem 0.65rem;
  font-size: 0.78rem;
}
.danger {
  color: var(--danger);
}
.empty {
  margin: 0;
  font-size: 0.85rem;
}
.rows {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}
.row {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.6rem 0.9rem;
}
.row-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}
.row-name {
  font-weight: 550;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.row-meta {
  font-size: 0.74rem;
}
.row-state {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 0.1rem;
  max-width: 260px;
}
.state {
  font-size: 0.78rem;
  font-weight: 600;
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  color: var(--txt-secondary);
}
.state.ready {
  color: var(--ok);
}
.state.failed,
.state.stale {
  color: var(--danger);
}
.detail {
  font-size: 0.74rem;
  color: var(--danger);
  text-align: right;
}
.spinner.small {
  width: 12px;
  height: 12px;
}
</style>
