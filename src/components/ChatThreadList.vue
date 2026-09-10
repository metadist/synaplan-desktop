<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { ChatSummary } from '@/services/tauri'

/** The project's saved chats. Newest first; the open one is highlighted. */
defineProps<{
  threads: ChatSummary[]
  currentId: string | null
  busy: boolean
}>()

const emit = defineEmits<{
  open: [chatId: string]
  new: []
  remove: [chatId: string]
}>()

const { t } = useI18n()

function when(iso: string): string {
  const d = new Date(iso)
  return Number.isNaN(d.getTime()) ? '' : d.toLocaleDateString()
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

    <p v-if="threads.length === 0" class="muted none">{{ t('chat.noThreads') }}</p>

    <ul v-else class="list">
      <li v-for="th in threads" :key="th.id" class="item" :class="{ active: th.id === currentId }">
        <button
          class="open"
          type="button"
          :disabled="busy"
          :data-testid="`chat-thread-${th.id}`"
          @click="emit('open', th.id)"
        >
          <span class="title">{{ th.title || t('chat.untitled') }}</span>
          <span class="meta muted">{{ when(th.updatedAt) }}</span>
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
  </aside>
</template>

<style scoped>
.threads {
  width: 208px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  padding: 0.8rem 0.7rem;
  border-right: 1px solid var(--border);
  background: var(--bg);
  overflow-y: auto;
}

.new-thread {
  width: 100%;
  font-size: 0.85rem;
}

.none {
  margin: 0.4rem 0.3rem;
  font-size: 0.8rem;
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
