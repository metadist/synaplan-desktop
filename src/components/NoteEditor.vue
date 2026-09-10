<script setup lang="ts">
import { nextTick, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Note } from '@/services/tauri'

/**
 * Plain Markdown editor for one note. The file is the Markdown; there is no
 * other storage format. The caret stays addressable so dictation can insert
 * where the person is writing.
 */
const props = defineProps<{
  note: Note
  draft: string
  dirty: boolean
  saving: boolean
}>()

const emit = defineEmits<{
  edit: [content: string]
  delete: []
  reveal: []
}>()

const { t } = useI18n()
const textarea = ref<HTMLTextAreaElement | null>(null)

function onInput(event: Event): void {
  emit('edit', (event.target as HTMLTextAreaElement).value)
}

/** Insert text at the caret (replacing any selection) and keep the caret after it. */
async function insertAtCaret(text: string): Promise<void> {
  const el = textarea.value
  if (!el) {
    emit('edit', props.draft + text)
    return
  }
  const start = el.selectionStart ?? props.draft.length
  const end = el.selectionEnd ?? start
  const next = props.draft.slice(0, start) + text + props.draft.slice(end)
  emit('edit', next)
  await nextTick()
  const caret = start + text.length
  el.setSelectionRange(caret, caret)
  el.focus()
}

/** The selected range, for dictation to replace an interim span. */
function selection(): { start: number; end: number } {
  const el = textarea.value
  const start = el?.selectionStart ?? props.draft.length
  return { start, end: el?.selectionEnd ?? start }
}

defineExpose({ insertAtCaret, selection, focus: () => textarea.value?.focus() })
</script>

<template>
  <div class="editor" data-testid="note-editor">
    <header class="editor-head">
      <div class="head-main">
        <h2 class="note-title">{{ note.title || t('notes.untitled') }}</h2>
        <span class="save-state muted" data-testid="note-save-state">
          {{ saving ? t('notes.saving') : dirty ? t('notes.unsaved') : t('notes.saved') }}
        </span>
      </div>
      <div class="head-actions">
        <slot name="actions"></slot>
        <button class="btn btn-ghost small" type="button" @click="emit('reveal')">
          {{ t('notes.reveal') }}
        </button>
        <button
          class="btn btn-ghost small danger"
          type="button"
          data-testid="note-delete"
          @click="emit('delete')"
        >
          {{ t('notes.delete') }}
        </button>
      </div>
    </header>

    <textarea
      ref="textarea"
      class="input body"
      :value="draft"
      :placeholder="t('notes.placeholder')"
      spellcheck="true"
      data-testid="note-body"
      @input="onInput"
    ></textarea>
  </div>
</template>

<style scoped>
.editor {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.editor-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.7rem 1.2rem;
  border-bottom: 1px solid var(--border);
}
.head-main {
  display: flex;
  align-items: baseline;
  gap: 0.7rem;
  min-width: 0;
}
.note-title {
  font-size: 1rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.save-state {
  font-size: 0.76rem;
  flex-shrink: 0;
}
.head-actions {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  flex-shrink: 0;
}
.small {
  padding: 0.35rem 0.7rem;
  font-size: 0.8rem;
}
.danger {
  color: var(--danger);
}
.body {
  flex: 1;
  min-height: 0;
  resize: none;
  border: none;
  border-radius: 0;
  padding: 1.1rem 1.4rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.9rem;
  line-height: 1.6;
  background: var(--bg);
}
.body:focus {
  outline: none;
}
</style>
