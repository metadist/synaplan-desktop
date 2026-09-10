<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { NoteSummary } from '@/services/tauri'

/** Search + list + new for a project's notes. Titles come from the first heading. */
defineProps<{
  notes: NoteSummary[]
  currentName: string | null
  query: string
  loading: boolean
}>()

const emit = defineEmits<{
  open: [name: string]
  new: []
  search: [query: string]
}>()

const { t, locale } = useI18n()

function when(iso: string): string {
  if (!iso) {
    return ''
  }
  const date = new Date(iso)
  if (Number.isNaN(date.getTime())) {
    return ''
  }
  return new Intl.DateTimeFormat(locale.value, { dateStyle: 'medium', timeStyle: 'short' }).format(
    date,
  )
}
</script>

<template>
  <aside class="note-list" data-testid="note-list">
    <div class="list-head">
      <input
        class="input search"
        type="search"
        :value="query"
        :placeholder="t('notes.search')"
        :aria-label="t('notes.search')"
        data-testid="note-search"
        @input="emit('search', ($event.target as HTMLInputElement).value)"
      />
      <button
        class="btn btn-primary new"
        type="button"
        data-testid="note-new"
        :title="t('notes.newNote')"
        :aria-label="t('notes.newNote')"
        @click="emit('new')"
      >
        +
      </button>
    </div>

    <ul class="items">
      <li v-for="note in notes" :key="note.name">
        <button
          class="item"
          :class="{ active: note.name === currentName }"
          type="button"
          :data-testid="`note-${note.name}`"
          @click="emit('open', note.name)"
        >
          <span class="item-title">{{ note.title || t('notes.untitled') }}</span>
          <span class="item-meta muted">{{ when(note.updatedAt) }}</span>
        </button>
      </li>
    </ul>

    <p v-if="!loading && notes.length === 0" class="muted empty">
      {{ query ? t('notes.noMatches') : t('notes.noNotes') }}
    </p>
  </aside>
</template>

<style scoped>
.note-list {
  width: 240px;
  flex-shrink: 0;
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  min-height: 0;
  background: var(--bg-elevated);
}
.list-head {
  display: flex;
  gap: 0.4rem;
  padding: 0.7rem 0.7rem 0.5rem;
}
.search {
  padding: 0.4rem 0.6rem;
  font-size: 0.85rem;
  min-width: 0;
}
.new {
  padding: 0.35rem 0.7rem;
  font-size: 1rem;
  line-height: 1;
  flex-shrink: 0;
}
.items {
  list-style: none;
  margin: 0;
  padding: 0 0.5rem 0.5rem;
  overflow-y: auto;
  flex: 1;
  min-height: 0;
}
.item {
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.1rem;
  padding: 0.5rem 0.6rem;
  border: 1px solid transparent;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--txt);
  font: inherit;
  text-align: left;
  cursor: pointer;
}
.item:hover {
  background: var(--bg-card);
}
.item.active {
  background: var(--bg-card);
  border-color: var(--border-strong);
}
.item-title {
  font-size: 0.86rem;
  font-weight: 550;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 100%;
}
.item-meta {
  font-size: 0.72rem;
}
.empty {
  margin: 0;
  padding: 0.4rem 1rem 1rem;
  font-size: 0.82rem;
}
</style>
