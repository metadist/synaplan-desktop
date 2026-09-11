<script setup lang="ts">
import { computed, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import * as api from '@/services/tauri'
import type { ChatSummary, ChatThread } from '@/services/tauri'

/**
 * The project's chat history. Newest first, grouped into Today / Yesterday /
 * Earlier so a glance answers "what did we talk about, and when"; the open
 * chat is highlighted.
 *
 * Folded, it is a 60px rail: "+" for a new chat, then one date/time stamp per
 * chat. Hovering a stamp unfolds a card to the right with the title and the
 * first lines of the conversation — the history stays readable without giving
 * the column its width back.
 */
const props = defineProps<{
  threads: ChatSummary[]
  currentId: string | null
  busy: boolean
  collapsed: boolean
}>()

const emit = defineEmits<{
  open: [chatId: string]
  new: []
  remove: [chatId: string]
  toggle: []
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

const sorted = computed(() =>
  [...props.threads].sort((a, b) => b.updatedAt.localeCompare(a.updatedAt)),
)

const groups = computed<Group[]>(() => {
  const now = new Date()
  const order: Bucket[] = ['today', 'yesterday', 'earlier']
  return order
    .map((bucket) => ({
      bucket,
      threads: sorted.value.filter((th) => bucketOf(th.updatedAt, now) === bucket),
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

/** The rail stamp: a short day on top, the time below. */
function stamp(th: ChatSummary): { day: string; time: string } {
  const d = new Date(th.updatedAt)
  if (Number.isNaN(d.getTime())) {
    return { day: '', time: '' }
  }
  const bucket = bucketOf(th.updatedAt, new Date())
  const day =
    bucket === 'today'
      ? t('chat.when.today')
      : new Intl.DateTimeFormat(locale.value, { day: 'numeric', month: 'short' }).format(d)
  return { day, time: new Intl.DateTimeFormat(locale.value, { timeStyle: 'short' }).format(d) }
}

function titleOf(th: ChatSummary): string {
  return th.title || t('chat.untitled')
}

// ---- hover card ------------------------------------------------------------

/** How long the pointer rests on a stamp before the card unfolds. */
const PEEK_DELAY_MS = 180
/** How much of each message the card shows. */
const PREVIEW_CHARS = 160
const PREVIEW_MESSAGES = 4

const peek = ref<ChatSummary | null>(null)
const peekAt = ref({ top: 0, left: 0 })
const peekThread = ref<ChatThread | null>(null)
const previews = new Map<string, ChatThread>()
let showTimer: ReturnType<typeof setTimeout> | undefined
let hideTimer: ReturnType<typeof setTimeout> | undefined

const peekLines = computed(() => {
  const thread = peekThread.value
  if (!thread || thread.id !== peek.value?.id) {
    return []
  }
  return thread.messages.slice(0, PREVIEW_MESSAGES).map((m) => ({
    role: m.role,
    text:
      m.content.length > PREVIEW_CHARS
        ? `${m.content.slice(0, PREVIEW_CHARS).trimEnd()}…`
        : m.content,
  }))
})

async function loadPreview(th: ChatSummary): Promise<void> {
  const key = `${th.id}:${th.updatedAt}`
  const cached = previews.get(key)
  if (cached) {
    peekThread.value = cached
    return
  }
  try {
    const thread = await api.loadChat(th.projectId, th.id)
    previews.set(key, thread)
    if (peek.value?.id === th.id) {
      peekThread.value = thread
    }
  } catch {
    // The card still shows title and time.
  }
}

function onStampEnter(th: ChatSummary, event: MouseEvent): void {
  if (!props.collapsed) {
    return
  }
  clearTimeout(hideTimer)
  clearTimeout(showTimer)
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
  showTimer = setTimeout(() => {
    peekAt.value = { top: rect.top, left: rect.right + 10 }
    peek.value = th
    peekThread.value = null
    void loadPreview(th)
  }, PEEK_DELAY_MS)
}

function onStampLeave(): void {
  clearTimeout(showTimer)
  hideTimer = setTimeout(() => {
    peek.value = null
  }, 120)
}

function keepPeek(): void {
  clearTimeout(hideTimer)
}

function openPeek(): void {
  const th = peek.value
  peek.value = null
  if (th) {
    emit('open', th.id)
  }
}

onUnmounted(() => {
  clearTimeout(showTimer)
  clearTimeout(hideTimer)
})
</script>

<template>
  <aside class="threads" :class="{ collapsed }" data-testid="chat-threads">
    <div class="rail-head">
      <button
        class="fold"
        type="button"
        :title="collapsed ? t('chat.historyExpand') : t('chat.historyCollapse')"
        :aria-label="collapsed ? t('chat.historyExpand') : t('chat.historyCollapse')"
        :aria-expanded="!collapsed"
        data-testid="chat-history-toggle"
        @click="emit('toggle')"
      >
        {{ collapsed ? '»' : '«' }}
      </button>
      <h2 v-if="!collapsed" class="history-title muted">
        {{ t('chat.history') }}
        <span v-if="threads.length" class="history-count">{{ threads.length }}</span>
      </h2>
    </div>

    <button
      class="btn new-thread"
      :class="collapsed ? 'btn-primary round' : 'btn-secondary'"
      type="button"
      :disabled="busy"
      :title="t('chat.newChat')"
      :aria-label="t('chat.newChat')"
      data-testid="chat-new-thread"
      @click="emit('new')"
    >
      <template v-if="collapsed">+</template>
      <template v-else>+ {{ t('chat.newChat') }}</template>
    </button>

    <!-- Folded: one stamp per chat, newest first. -->
    <ul v-if="collapsed" class="stamps">
      <li v-for="th in sorted" :key="th.id">
        <button
          class="stamp"
          :class="{ active: th.id === currentId }"
          type="button"
          :disabled="busy"
          :title="titleOf(th)"
          :aria-label="titleOf(th)"
          :data-testid="`chat-thread-${th.id}`"
          @click="emit('open', th.id)"
          @mouseenter="onStampEnter(th, $event)"
          @mouseleave="onStampLeave"
          @focus="onStampEnter(th, $event as unknown as MouseEvent)"
          @blur="onStampLeave"
        >
          <span class="stamp-day">{{ stamp(th).day }}</span>
          <span class="stamp-time">{{ stamp(th).time }}</span>
        </button>
      </li>
    </ul>

    <!-- Unfolded: grouped by day, with titles. -->
    <template v-else>
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
              <span class="title">{{ titleOf(th) }}</span>
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
    </template>

    <!-- The hover card unfolds to the right of the rail; fixed so the rail's scroll box cannot clip it. -->
    <Teleport to="body">
      <div
        v-if="peek"
        class="peek card"
        :style="{ top: `${peekAt.top}px`, left: `${peekAt.left}px` }"
        role="tooltip"
        data-testid="chat-peek"
        @mouseenter="keepPeek"
        @mouseleave="onStampLeave"
        @click="openPeek"
      >
        <div class="peek-title">{{ titleOf(peek) }}</div>
        <div class="peek-meta muted">
          {{ when(peek, bucketOf(peek.updatedAt, new Date())) }} ·
          {{ t('chat.messageCount', { count: peek.messageCount }, peek.messageCount) }}
        </div>
        <ul v-if="peekLines.length" class="peek-lines">
          <li v-for="(line, i) in peekLines" :key="i" class="peek-line" :class="line.role">
            <span class="peek-role muted">{{
              line.role === 'user' ? t('chat.you') : t('chat.assistant')
            }}</span>
            <span class="peek-text">{{ line.text }}</span>
          </li>
        </ul>
        <p v-else class="muted peek-loading"><span class="spinner small"></span></p>
        <div class="peek-foot muted">{{ t('chat.peekOpen') }}</div>
      </div>
    </Teleport>
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
  overflow-x: hidden;
  transition: width 0.16s ease;
}

.threads.collapsed {
  width: 60px;
  padding: 0.7rem 0.45rem;
  align-items: center;
  gap: 0.4rem;
}

.rail-head {
  display: flex;
  align-items: center;
  gap: 0.3rem;
}

.collapsed .rail-head {
  justify-content: center;
}

.fold {
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--txt-secondary);
  font: inherit;
  font-size: 0.95rem;
  line-height: 1;
  cursor: pointer;
}

.fold:hover {
  background: var(--bg-elevated);
  color: var(--txt);
}

.new-thread {
  width: 100%;
  font-size: 0.85rem;
}

.new-thread.round {
  width: 40px;
  height: 40px;
  padding: 0;
  border-radius: 12px;
  font-size: 1.3rem;
  line-height: 1;
}

.history-title {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  margin: 0;
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

.list,
.stamps {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.stamps {
  width: 100%;
  align-items: center;
  gap: 0.3rem;
  margin-top: 0.3rem;
}

.stamp {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 48px;
  padding: 0.35rem 0;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: var(--txt-secondary);
  font: inherit;
  line-height: 1.2;
  cursor: pointer;
}

.stamp:hover {
  background: var(--bg-elevated);
  color: var(--txt);
}

.stamp.active {
  background: var(--accent-soft);
  border-color: var(--accent);
  color: var(--accent);
}

.stamp-day {
  font-size: 0.62rem;
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.stamp-time {
  font-size: 0.78rem;
  font-weight: 650;
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

.peek {
  position: fixed;
  z-index: 40;
  width: 320px;
  max-height: 60vh;
  overflow: hidden;
  padding: 0.7rem 0.8rem;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  box-shadow: var(--shadow-lg);
  cursor: pointer;
}

.peek-title {
  font-weight: 650;
  font-size: 0.9rem;
}

.peek-meta {
  font-size: 0.72rem;
}

.peek-lines {
  list-style: none;
  margin: 0.2rem 0 0;
  padding: 0.4rem 0 0;
  border-top: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.peek-line {
  display: flex;
  flex-direction: column;
  gap: 0.05rem;
  font-size: 0.8rem;
  line-height: 1.35;
}

.peek-role {
  font-size: 0.66rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.peek-line.assistant .peek-role {
  color: var(--accent);
}

.peek-text {
  white-space: pre-line;
  overflow-wrap: anywhere;
}

.peek-loading {
  margin: 0.3rem 0 0;
}

.peek-foot {
  margin-top: 0.2rem;
  font-size: 0.68rem;
}

.spinner.small {
  width: 11px;
  height: 11px;
}
</style>
