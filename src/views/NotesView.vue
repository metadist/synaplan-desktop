<script setup lang="ts">
import { computed, defineAsyncComponent, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useProjectsStore } from '@/stores/projects'
import { useUiStore } from '@/stores/ui'
import { useProjectName } from '@/composables/useProjectName'
import { useNotes } from '@/composables/useNotes'
import { useErrorText } from '@/composables/useErrorText'
import * as api from '@/services/tauri'
import NoteList from '@/components/NoteList.vue'
import type NoteEditorType from '@/components/NoteEditor.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import DictationButton from '@/components/DictationButton.vue'

/**
 * Notes: Markdown files in this project's notes folder on this computer.
 * Nothing here talks to the workspace — sharing a note is an explicit step in
 * Files.
 */
// The Milkdown editor is the heaviest part of the app; it loads with the first note.
const NoteEditor = defineAsyncComponent(() => import('@/components/NoteEditor.vue'))

const { t } = useI18n()
const projects = useProjectsStore()
const ui = useUiStore()
const projectName = useProjectName()
const errorText = useErrorText()

const project = computed(() => projects.active)
const projectId = computed(() => project.value?.id ?? '')
const notes = useNotes(projectId)
void notes.refresh()

const confirmDelete = ref<string | null>(null)
const deleting = ref(false)

const deleteTitle = computed(() => {
  const name = confirmDelete.value
  const note = notes.notes.value.find((n) => n.name === name)
  return note?.title || name || ''
})

async function performDelete(): Promise<void> {
  const name = confirmDelete.value
  if (!name) {
    return
  }
  deleting.value = true
  try {
    await notes.remove(name)
  } finally {
    deleting.value = false
    confirmDelete.value = null
  }
}

function reveal(): void {
  const target = notes.current.value?.path ?? project.value?.notesDir
  if (target) {
    void api.revealPath(target)
  }
}

// ---- dictation at the caret ------------------------------------------------
// One take owns one span in the note: it starts at the caret and grows with
// every interim reading; the final text replaces exactly that span.
const editor = ref<InstanceType<typeof NoteEditorType> | null>(null)
const dictationError = ref<unknown>(null)
const hasVoiceModel = computed(() => (project.value?.models.voice ?? '') !== '')
let take: { start: number; end: number } | null = null

function onDictationStart(): void {
  dictationError.value = null
  const sel = editor.value?.selection() ?? { start: 0, end: 0 }
  take = { start: sel.start, end: sel.end }
}

async function writeTake(text: string): Promise<void> {
  if (!take || !editor.value) {
    return
  }
  // The editor adds the glue space itself when the take runs into a word.
  take.end = await editor.value.replaceRange(take.start, take.end, text)
}

function onDictationInterim(text: string): void {
  void writeTake(text)
}

async function onDictationDone(text: string): Promise<void> {
  await writeTake(text)
  take = null
  editor.value?.focus()
}

function onDictationError(e: unknown): void {
  dictationError.value = e
}
</script>

<template>
  <section class="view">
    <header class="view-header">
      <div>
        <h1>{{ t('notes.title') }}</h1>
        <p class="muted subtitle">{{ projectName(project) }}</p>
      </div>
      <p class="muted local-note">{{ t('notes.localOnly') }}</p>
    </header>

    <div class="view-body">
      <NoteList
        :notes="notes.notes.value"
        :current-name="notes.current.value?.name ?? null"
        :query="notes.query.value"
        :loading="notes.loading.value"
        @open="notes.open"
        @new="notes.create"
        @search="notes.query.value = $event"
      />

      <NoteEditor
        v-if="notes.current.value"
        ref="editor"
        :note="notes.current.value"
        :draft="notes.draft.value"
        :dirty="notes.dirty.value"
        :saving="notes.saving.value"
        @edit="notes.edit"
        @delete="confirmDelete = notes.current.value?.name ?? null"
        @reveal="reveal"
      >
        <template #actions>
          <DictationButton
            v-if="hasVoiceModel"
            :project-id="projectId"
            @start="onDictationStart"
            @interim="onDictationInterim"
            @done="onDictationDone"
            @error="onDictationError"
          />
          <button
            v-else
            class="btn btn-ghost small"
            type="button"
            :title="t('dictation.needModel')"
            data-testid="dictation-need-model"
            @click="ui.setView('models')"
          >
            {{ t('dictation.pickModel') }}
          </button>
        </template>
      </NoteEditor>

      <div v-else-if="project" class="empty-wrap">
        <div class="empty card">
          <h2 class="empty-title">{{ t('notes.emptyTitle') }}</h2>
          <p class="muted">{{ t('notes.emptyBody') }}</p>
          <code class="path">{{ project.notesDir }}</code>
          <div class="actions">
            <button class="btn btn-primary" type="button" @click="notes.create">
              {{ t('notes.newNote') }}
            </button>
            <button class="btn btn-ghost" type="button" @click="reveal">
              {{ t('notes.revealFolder') }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <p v-if="notes.error.value" class="banner banner-error error" role="alert">
      {{ errorText(notes.error.value) }}
    </p>
    <p
      v-else-if="dictationError"
      class="banner banner-error error"
      role="alert"
      data-testid="dictation-error"
    >
      {{ errorText(dictationError) }}
    </p>

    <ConfirmDialog
      v-if="confirmDelete"
      :title="t('notes.deleteTitle', { title: deleteTitle })"
      :body="t('notes.deleteBody')"
      :confirm-label="t('notes.delete')"
      danger
      :busy="deleting"
      @cancel="confirmDelete = null"
      @confirm="performDelete"
    />
  </section>
</template>

<style scoped>
.view {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  position: relative;
}
.view-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.9rem 1.2rem;
  border-bottom: 1px solid var(--border);
}
.subtitle {
  margin: 0.1rem 0 0;
  font-size: 0.82rem;
}
.local-note {
  margin: 0;
  font-size: 0.78rem;
  text-align: right;
  max-width: 320px;
}
.view-body {
  flex: 1;
  min-height: 0;
  display: flex;
}
.empty-wrap {
  flex: 1;
  padding: 1.1rem 1.2rem;
  overflow-y: auto;
}
.empty {
  max-width: 560px;
  padding: 1.4rem 1.5rem;
}
.empty-title {
  margin: 0 0 0.35rem;
  font-size: 1.05rem;
}
.path {
  display: block;
  margin: 0.8rem 0;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.76rem;
  color: var(--txt-secondary);
  overflow-wrap: anywhere;
}
.actions {
  display: flex;
  gap: 0.5rem;
}
.small {
  padding: 0.35rem 0.7rem;
  font-size: 0.8rem;
}
.error {
  position: absolute;
  left: 1rem;
  right: 1rem;
  bottom: 1rem;
  margin: 0;
}
</style>
