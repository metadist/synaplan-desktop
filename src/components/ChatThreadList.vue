<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ChatSummary } from '@/services/tauri'

/**
 * The project's chat history. Newest first, grouped into Today / Yesterday /
 * Earlier so a glance answers "what did we talk about, and when"; the open
 * chat is highlighted.
 */
const props = defineProps<{
  threads: ChatSummary[]
  currentId: string | null
  busy: boolean
}>()

const emit = defineEmits<{
  open: [chatId: string]
  new: []
  remove: [chatId: string]
}>()

const { t, locale } = useI18n()

type Bucket = 'today' | 'yesterday' | 'earlier'

interface Group {
  bucket: Bucket
  threads: ChatSummary[]
}

function startOfDay(d: Date): number {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime()
}

function bucketOf(iso: string, now: Date): Bucket {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) {
    return 'earlier'
  }
  const today = startOfDay(now)
  const day = startOfDay(d)
  if (day >= today) {
    return 'today'
  }
  if (day >= today - 86_400_000) {
    return 'yesterday'
  }
  return 'earlier'
}

const groups = computed<Group[]>(() => {
  const now = new Date()
  const sorted = [...props.threads].sort((a, b) => b.updatedAt.localeCompare(a.updatedAt))
  const order: Bucket[] = ['today', 'yesterday', 'earlier']
  return order
    .map((bucket) => ({
      bucket,
      threads: sorted.filter((th) => bucketOf(th.updatedAt, now) === bucket),
    }))
    .filter((g) => g.threads.length > 0)
})

function when(th: ChatSummary, bucket: Bucket): string {
  const d = new Date(th.updatedAt)
  if (Number.isNaN(d.getTime())) {
    return ''
  }
  const fmt =
    bucket === 'earlier'
      ? new Intl.DateTimeFormat(locale.value, { dateStyle: 'medium' })
      : new Intl.DateTimeFormat(locale.value, { timeStyle: 'short' })
  return fmt.format(d)
}
</script>

<template>
  <aside class="threads" data-testid="chat-threads">
    <button
      class="btn btn-secondary new-thread"
      type="button"
      :disabled="busy"
      data-testid="chat-new-thread"
      @click="emit('new')"
    >
      + {{ t('chat.newChat') }}
    </button>

    <h2 class="history-title muted">
      {{ t('chat.history') }}
      <span v-if="threads.length" class="history-count">{{ threads.length }}</span>
    </h2>

    <p v-if="threads.length === 0" class="muted none">{{ t('chat.noThreads') }}</p>

    <template v-for="group in groups" :key="group.bucket">
      <h3 class="group-title muted">{{ t(`chat.when.${group.bucket}`) }}</h3>
      <ul class="list">
        <li
          v-for="th in group.threads"
          :key="th.id"
          class="item"
          :class="{ active: th.id === currentId }"
        >
          <button
            class="open"
            type="button"
            :disabled="busy"
            :data-testid="`chat-thread-${th.id}`"
            @click="emit('open', th.id)"
          >
            <span class="title">{{ th.title || t('chat.untitled') }}</span>
            <span class="meta muted">
              {{ when(th, group.bucket) }} ·
              {{ t('chat.messageCount', { count: th.messageCount }, th.messageCount) }}
            </span>
          </button>
          <button
            class="remove"
            type="button"
            :disabled="busy"
            :aria-label="t('chat.deleteThread')"
            :title="t('chat.deleteThread')"
            @click="emit('remove', th.id)"
          >
            ×
          </button>
        </li>
      </ul>
    </template>
  </aside>
</template>

<style scoped>
.threads {
  width: 220px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  padding: 0.8rem 0.7rem;
  border-right: 1px solid var(--border);
  background: var(--bg);
  overflow-y: auto;
}

.new-thread {
  width: 100%;
  font-size: 0.85rem;
}

.history-title {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  margin: 0.5rem 0.3rem 0;
  font-size: 0.72rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.history-count {
  font-size: 0.7rem;
  padding: 0 0.4rem;
  border-radius: 999px;
  background: var(--bg-elevated);
  color: var(--txt-secondary);
  font-weight: 600;
  letter-spacing: 0;
}

.none {
  margin: 0.2rem 0.3rem;
  font-size: 0.8rem;
}

.group-title {
  margin: 0.35rem 0.3rem 0;
  font-size: 0.7rem;
  font-weight: 600;
}

.list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.item {
  display: flex;
  align-items: stretch;
  border-radius: var(--radius-sm);
}

.item:hover {
  background: var(--bg-elevated);
}

.item.active {
  background: var(--accent-soft);
}

.open {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
  padding: 0.45rem 0.55rem;
  border: none;
  background: transparent;
  color: var(--txt);
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.item.active .open {
  color: var(--accent);
}

.title {
  font-size: 0.85rem;
  font-weight: 550;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.meta {
  font-size: 0.7rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.remove {
  border: none;
  background: transparent;
  color: var(--txt-secondary);
  font: inherit;
  font-size: 1rem;
  padding: 0 0.5rem;
  cursor: pointer;
  opacity: 0;
}

.item:hover .remove,
.remove:focus-visible {
  opacity: 1;
}

.remove:hover {
  color: var(--danger);
}
</style>
