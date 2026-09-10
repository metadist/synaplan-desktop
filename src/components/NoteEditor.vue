<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Note } from '@/services/tauri'
import { useMarkdownEditor } from '@/composables/useMarkdownEditor'
import MarkdownToolbar from '@/components/MarkdownToolbar.vue'
import '@milkdown/kit/prose/view/style/prosemirror.css'

/**
 * WYSIWYG Markdown editor for one note. The file is the Markdown; there is
 * no other storage format. The caret stays addressable so dictation can
 * insert where the person is writing.
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
const host = ref<HTMLElement | null>(null)
const md = useMarkdownEditor(host, (markdown) => emit('edit', markdown))

onMounted(() => {
  void md.create(props.draft)
})

// Another note, or an outside change to this one, reloads the document. Our
// own edits echo back through `draft` and are recognised by the composable.
watch(
  () => [props.note.name, props.draft] as const,
  () => md.load(props.draft),
)

/** Insert text at the caret (replacing any selection); returns the new caret. */
async function insertAtCaret(text: string): Promise<number> {
  const caret = md.insertAtCaret(text)
  md.focus()
  return caret
}

/** The selected range (editor positions), for dictation to replace an interim span. */
function selection(): { start: number; end: number } {
  return md.selection()
}

/**
 * Replace `[start, end)` with `text` and put the caret after it. Dictation uses
 * this to swap the interim span of one take for the next reading, and finally
 * for the one-shot result — never the whole document.
 */
async function replaceRange(start: number, end: number, text: string): Promise<number> {
  return md.replaceRange(start, end, text)
}

defineExpose({
  insertAtCaret,
  selection,
  replaceRange,
  focus: () => md.focus(),
  markdown: () => md.markdown(),
})
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

    <MarkdownToolbar @action="md.run" @link="md.link" />

    <div
      ref="host"
      class="body note-md"
      :data-placeholder="t('notes.placeholder')"
      spellcheck="true"
      data-testid="note-body"
    ></div>
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
  overflow: auto;
  background: var(--bg);
}
</style>

<style>
/* Not scoped: the editor DOM is created by ProseMirror inside `.body`. */
.note-md .milkdown {
  min-height: 100%;
}
.note-md .ProseMirror {
  min-height: 100%;
  padding: 1.1rem 1.4rem 3rem;
  outline: none;
  color: var(--txt);
  font-size: 0.95rem;
  line-height: 1.65;
}
.note-md .ProseMirror:has(> p:only-child > br.ProseMirror-trailingBreak:only-child)::before {
  content: attr(data-placeholder);
  position: absolute;
  pointer-events: none;
  color: var(--txt-secondary);
  opacity: 0.7;
}
.note-md[data-placeholder] .ProseMirror {
  position: relative;
}
.note-md .ProseMirror > * + * {
  margin-top: 0.6rem;
}
.note-md .ProseMirror h1,
.note-md .ProseMirror h2,
.note-md .ProseMirror h3 {
  line-height: 1.25;
  margin-top: 1.1rem;
}
.note-md .ProseMirror h1 {
  font-size: 1.55rem;
}
.note-md .ProseMirror h2 {
  font-size: 1.3rem;
}
.note-md .ProseMirror h3 {
  font-size: 1.1rem;
}
.note-md .ProseMirror ul,
.note-md .ProseMirror ol {
  padding-left: 1.5rem;
}
.note-md .ProseMirror li + li {
  margin-top: 0.15rem;
}
.note-md .ProseMirror a {
  color: var(--accent);
  text-decoration: underline;
}
.note-md .ProseMirror code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.86em;
  padding: 0.1em 0.35em;
  border-radius: 4px;
  background: var(--accent-soft);
}
.note-md .ProseMirror pre {
  padding: 0.8rem 1rem;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  overflow: auto;
}
.note-md .ProseMirror pre code {
  padding: 0;
  background: transparent;
}
.note-md .ProseMirror blockquote {
  margin: 0;
  padding-left: 0.9rem;
  border-left: 3px solid var(--border-strong);
  color: var(--txt-secondary);
}
.note-md .ProseMirror hr {
  border: none;
  border-top: 1px solid var(--border);
}
.note-md .ProseMirror table {
  border-collapse: collapse;
}
.note-md .ProseMirror th,
.note-md .ProseMirror td {
  border: 1px solid var(--border);
  padding: 0.3rem 0.6rem;
}
.note-md .ProseMirror ::selection {
  background: var(--accent-soft);
}
</style>
