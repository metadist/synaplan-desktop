<script setup lang="ts">
import { computed, defineAsyncComponent, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import * as api from '@/services/tauri'
import type { KnowledgeFile, NoteSummary, Project } from '@/services/tauri'
import type { useNotes } from '@/composables/useNotes'
import type { useKnowledgeFiles } from '@/composables/useKnowledgeFiles'
import { useErrorText } from '@/composables/useErrorText'
import { useUiStore } from '@/stores/ui'
import ConfirmDialog from '@/components/ConfirmDialog.vue'

/**
 * Everything that belongs to the project, right next to the chat: notes and
 * files in one list, newest first, with one search box over both. "Notes" and
 * "Files" are filters on that list, not separate places. The lists and actions
 * are the same ones the Notes and Files pages use, so nothing here is a second
 * source of truth — the composables are handed in by the Chat view, which also
 * owns the counts on the pills.
 */
export type PanelTab = 'all' | 'notes' | 'files'

interface Row {
  kind: 'note' | 'file'
  key: string
  title: string
  at: string
  note?: NoteSummary
  file?: KnowledgeFile
}

const props = defineProps<{
  tab: PanelTab
  project: Project | null
  notes: ReturnType<typeof useNotes>
  knowledge: ReturnType<typeof useKnowledgeFiles>
}>()

const emit = defineEmits<{
  close: []
  tab: [tab: PanelTab]
  addFiles: []
}>()

// The Milkdown editor is the heaviest part of the app; it loads with the first note.
const NoteEditor = defineAsyncComponent(() => import('@/components/NoteEditor.vue'))

const { t, locale } = useI18n()
const ui = useUiStore()
const errorText = useErrorText()

const embedSet = computed(() => (props.project?.models.embed ?? '') !== '')
const showFiles = computed(() => props.tab !== 'notes')
const showNotes = computed(() => props.tab !== 'files')

const search = ref('')

/** Notes and files as one list, newest first, narrowed by the tab and the search box. */
const rows = computed<Row[]>(() => {
  const needle = search.value.trim().toLowerCase()
  const out: Row[] = []
  if (showNotes.value) {
    for (const note of props.notes.notes.value) {
      out.push({
        kind: 'note',
        key: `note:${note.name}`,
        title: note.title || t('notes.untitled'),
        at: note.updatedAt,
        note,
      })
    }
  }
  if (showFiles.value) {
    for (const file of props.knowledge.files.value) {
      out.push({
        kind: 'file',
        key: `file:${file.id}`,
        title: file.name,
        at: file.uploadedAt,
        file,
      })
    }
  }
  const matched = needle ? out.filter((row) => row.title.toLowerCase().includes(needle)) : out
  return matched.sort((a, b) => (a.at < b.at ? 1 : a.at > b.at ? -1 : 0))
})

const loading = computed(
  () =>
    (showNotes.value && props.notes.loading.value) ||
    (showFiles.value && props.knowledge.loading.value),
)

const emptyText = computed(() => {
  if (search.value.trim() !== '') {
    return t('panel.noMatches')
  }
  if (props.tab === 'notes') {
    return t('panel.noNotes')
  }
  if (props.tab === 'files') {
    return t('panel.noFiles')
  }
  return t('panel.nothingYet')
})

const confirmNote = ref<string | null>(null)
const confirmFile = ref<KnowledgeFile | null>(null)
const busy = ref(false)

const confirmNoteTitle = computed(() => {
  const name = confirmNote.value
  const note = props.notes.notes.value.find((n) => n.name === name)
  return note?.title || name || ''
})

function when(iso: string): string {
  const date = new Date(iso)
  if (!iso || Number.isNaN(date.getTime())) {
    return ''
  }
  return new Intl.DateTimeFormat(locale.value, { dateStyle: 'medium' }).format(date)
}

async function deleteNote(): Promise<void> {
  const name = confirmNote.value
  if (!name) {
    return
  }
  busy.value = true
  try {
    await props.notes.remove(name)
  } finally {
    busy.value = false
    confirmNote.value = null
  }
}

async function removeFile(): Promise<void> {
  const target = confirmFile.value
  if (!target) {
    return
  }
  busy.value = true
  try {
    await props.knowledge.remove(target.id)
  } finally {
    busy.value = false
    confirmFile.value = null
  }
}

function revealNote(): void {
  const target = props.notes.current.value?.path ?? props.project?.notesDir
  if (target) {
    void api.revealPath(target)
  }
}

function pendingErrorText(err: unknown): string {
  return api.asCommandError(err).code === 'file_outside_allowed'
    ? t('files.outsideAllowed')
    : errorText(err)
}
</script>

<template>
  <aside class="panel" data-testid="project-panel">
    <header class="panel-head">
      <div class="tabs" role="tablist">
        <button
          class="tab"
          :class="{ active: tab === 'all' }"
          type="button"
          role="tab"
          :aria-selected="tab === 'all'"
          data-testid="panel-tab-all"
          @click="emit('tab', 'all')"
        >
          {{ t('panel.all') }}
        </button>
        <button
          class="tab"
          :class="{ active: tab === 'notes' }"
          type="button"
          role="tab"
          :aria-selected="tab === 'notes'"
          data-testid="panel-tab-notes"
          @click="emit('tab', 'notes')"
        >
          {{ t('nav.notes') }}
          <span class="count">{{ notes.notes.value.length }}</span>
        </button>
        <button
          class="tab"
          :class="{ active: tab === 'files' }"
          type="button"
          role="tab"
          :aria-selected="tab === 'files'"
          data-testid="panel-tab-files"
          @click="emit('tab', 'files')"
        >
          {{ t('nav.files') }}
          <span class="count">{{ knowledge.files.value.length }}</span>
        </button>
      </div>
      <button
        class="close"
        type="button"
        :aria-label="t('common.close')"
        :title="t('common.close')"
        data-testid="panel-close"
        @click="emit('close')"
      >
        ×
      </button>
    </header>

    <!-- An open note takes the whole panel; everything else is one list. -->
    <div v-if="notes.current.value" class="note-open">
      <button
        class="btn-link back"
        type="button"
        data-testid="panel-note-back"
        @click="notes.close"
      >
        ← {{ t('panel.allNotes') }}
      </button>
      <NoteEditor
        :note="notes.current.value"
        :draft="notes.draft.value"
        :dirty="notes.dirty.value"
        :saving="notes.saving.value"
        @edit="notes.edit"
        @delete="confirmNote = notes.current.value?.name ?? null"
        @reveal="revealNote"
      />
    </div>

    <div v-else class="panel-body">
      <div class="quick">
        <button
          v-if="showNotes"
          class="btn btn-primary quick-btn"
          type="button"
          data-testid="panel-new-note"
          @click="notes.create"
        >
          + {{ t('notes.newNote') }}
        </button>
        <button
          v-if="showFiles"
          class="btn quick-btn"
          :class="showNotes ? 'btn-secondary' : 'btn-primary'"
          type="button"
          :disabled="!embedSet"
          data-testid="panel-add-files"
          @click="emit('addFiles')"
        >
          + {{ t('panel.addFiles') }}
        </button>
      </div>

      <input
        v-model="search"
        class="input search"
        type="search"
        :placeholder="t('panel.search')"
        :aria-label="t('panel.search')"
        data-testid="panel-search"
      />

      <p
        v-if="showFiles && !embedSet"
        class="banner banner-warn notice"
        data-testid="panel-embed-unset"
      >
        {{ t('files.embedUnset') }}
        <button class="btn-link" type="button" @click="ui.setView('models')">
          {{ t('files.openModels') }} →
        </button>
      </p>

      <ul
        v-if="showFiles && knowledge.pending.value.length"
        class="rows"
        data-testid="panel-pending"
      >
        <li
          v-for="row in knowledge.pending.value"
          :key="row.path"
          class="row static"
          :class="{ failed: row.error !== null }"
        >
          <span class="row-icon" aria-hidden="true">📎</span>
          <span class="row-main">
            <span class="row-title">{{ row.name }}</span>
            <span v-if="row.error === null" class="row-meta muted">
              <span class="spinner small"></span> {{ t('files.sending') }}
            </span>
            <span v-else class="row-meta error">
              {{ pendingErrorText(row.error) }}
              <button class="btn-link" type="button" @click="knowledge.dismiss(row.path)">
                {{ t('files.dismiss') }}
              </button>
            </span>
          </span>
        </li>
      </ul>

      <p v-if="rows.length === 0 && !loading" class="muted none" data-testid="panel-empty">
        {{ emptyText }}
      </p>
      <ul v-else class="rows">
        <li v-for="row in rows" :key="row.key">
          <button
            v-if="row.note"
            class="row"
            type="button"
            :data-testid="`panel-note-${row.note.name}`"
            @click="notes.open(row.note.name)"
          >
            <span class="row-icon" aria-hidden="true">📝</span>
            <span class="row-main">
              <span class="row-title">{{ row.title }}</span>
              <span class="row-meta muted">{{ when(row.at) }}</span>
            </span>
          </button>
          <div v-else-if="row.file" class="row static" :data-testid="`panel-file-${row.file.id}`">
            <span class="row-icon" aria-hidden="true">📎</span>
            <span class="row-main">
              <span class="row-title">{{ row.title }}</span>
              <span class="row-meta state" :class="row.file.state">
                <span
                  v-if="row.file.state === 'reading' || row.file.state === 'indexing'"
                  class="spinner small"
                ></span>
                {{ t(`files.states.${row.file.state}`) }}
                <span class="muted"> · {{ when(row.at) }}</span>
              </span>
            </span>
            <button
              class="remove"
              type="button"
              :aria-label="t('files.remove')"
              :title="t('files.remove')"
              :data-testid="`panel-file-${row.file.id}-remove`"
              @click="confirmFile = row.file"
            >
              ×
            </button>
          </div>
        </li>
      </ul>

      <p class="muted hint">
        {{ tab === 'files' ? t('panel.filesHint') : t('panel.notesHint') }}
      </p>
      <div class="more">
        <button v-if="showNotes" class="btn-link" type="button" @click="ui.setView('notes')">
          {{ t('panel.openNotes') }} →
        </button>
        <button v-if="showFiles" class="btn-link" type="button" @click="ui.setView('files')">
          {{ t('panel.openFiles') }} →
        </button>
      </div>
    </div>

    <p v-if="notes.error.value" class="banner banner-error notice" role="alert">
      {{ errorText(notes.error.value) }}
    </p>
    <p v-else-if="knowledge.error.value" class="banner banner-error notice" role="alert">
      {{ errorText(knowledge.error.value) }}
    </p>

    <ConfirmDialog
      v-if="confirmNote"
      :title="t('notes.deleteTitle', { title: confirmNoteTitle })"
      :body="t('notes.deleteBody')"
      :confirm-label="t('notes.delete')"
      danger
      :busy="busy"
      @cancel="confirmNote = null"
      @confirm="deleteNote"
    />
    <ConfirmDialog
      v-if="confirmFile"
      :title="t('files.removeTitle', { name: confirmFile.name })"
      :body="t('files.removeBody')"
      :confirm-label="t('files.remove')"
      danger
      :busy="busy"
      @cancel="confirmFile = null"
      @confirm="removeFile"
    />
  </aside>
</template>

<style scoped>
.panel {
  width: 320px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  min-height: 0;
  border-left: 1px solid var(--border);
  background: var(--bg);
}

.panel-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.55rem 0.6rem 0.55rem 0.8rem;
  border-bottom: 1px solid var(--border);
}

.tabs {
  display: inline-flex;
  gap: 0.25rem;
  padding: 0.2rem;
  border-radius: 999px;
  background: var(--bg-elevated);
}

.tab {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.3rem 0.8rem;
  border: none;
  border-radius: 999px;
  background: transparent;
  color: var(--txt-secondary);
  font: inherit;
  font-size: 0.84rem;
  font-weight: 600;
  cursor: pointer;
}

.tab.active {
  background: var(--bg-card);
  color: var(--txt);
  box-shadow: var(--shadow);
}

.count {
  font-size: 0.72rem;
  padding: 0 0.4rem;
  border-radius: 999px;
  background: var(--accent-soft);
  color: var(--accent);
}

.close {
  border: none;
  background: transparent;
  color: var(--txt-secondary);
  font: inherit;
  font-size: 1.2rem;
  line-height: 1;
  padding: 0.2rem 0.45rem;
  cursor: pointer;
  border-radius: var(--radius-sm);
}

.close:hover {
  background: var(--bg-elevated);
  color: var(--txt);
}

.panel-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0.8rem;
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}

.hint {
  margin: 0.2rem 0 0;
  font-size: 0.78rem;
}

.none {
  margin: 0.6rem 0.2rem;
  font-size: 0.85rem;
}

.notice {
  margin: 0;
  font-size: 0.82rem;
}

.rows {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}

.row {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.55rem 0.65rem;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-card);
  color: var(--txt);
  font: inherit;
  text-align: left;
  cursor: pointer;
  transition: border-color 0.12s ease;
}

.row:hover {
  border-color: var(--accent);
}

.row.static {
  cursor: default;
}

.row.static:hover {
  border-color: var(--border);
}

.row.failed {
  border-color: color-mix(in srgb, var(--danger) 40%, transparent);
}

.row-icon {
  font-size: 1rem;
  flex-shrink: 0;
}

.row-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 0.05rem;
}

.row-title {
  font-size: 0.86rem;
  font-weight: 550;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.row-meta {
  font-size: 0.72rem;
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
}

.row-meta.error {
  color: var(--danger);
  white-space: normal;
}

.state {
  color: var(--txt-secondary);
  font-weight: 600;
}

.state.ready {
  color: var(--ok);
}

.state.failed,
.state.stale {
  color: var(--danger);
}

.remove {
  border: none;
  background: transparent;
  color: var(--txt-secondary);
  font: inherit;
  font-size: 1rem;
  padding: 0 0.3rem;
  cursor: pointer;
  opacity: 0;
}

.row:hover .remove,
.remove:focus-visible {
  opacity: 1;
}

.remove:hover {
  color: var(--danger);
}

.more {
  display: flex;
  flex-wrap: wrap;
  gap: 0.3rem 1rem;
  font-size: 0.8rem;
  margin-top: 0.2rem;
}

.quick {
  display: flex;
  gap: 0.5rem;
}

.quick-btn {
  flex: 1;
  white-space: nowrap;
}

.search {
  width: 100%;
  font-size: 0.85rem;
}

.note-open {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.back {
  align-self: flex-start;
  margin: 0.55rem 0.8rem 0;
  font-size: 0.8rem;
}

.spinner.small {
  width: 11px;
  height: 11px;
}
</style>
