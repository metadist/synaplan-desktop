<script setup lang="ts">
import { nextTick, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { EditorAction } from '@/composables/useMarkdownEditor'

/**
 * The slim formatting bar over a note: headings, bold/italic, lists, link,
 * code. Nothing else on purpose — the file is Markdown and stays readable
 * as such.
 */
const emit = defineEmits<{
  action: [action: EditorAction]
  link: [href: string]
}>()

const { t } = useI18n()

const buttons: Array<{ action: EditorAction; label: string; key: string }> = [
  { action: 'heading1', label: 'H1', key: 'heading1' },
  { action: 'heading2', label: 'H2', key: 'heading2' },
  { action: 'heading3', label: 'H3', key: 'heading3' },
  { action: 'paragraph', label: '¶', key: 'paragraph' },
  { action: 'bold', label: 'B', key: 'bold' },
  { action: 'italic', label: 'I', key: 'italic' },
  { action: 'bulletList', label: '•', key: 'bulletList' },
  { action: 'orderedList', label: '1.', key: 'orderedList' },
  { action: 'inlineCode', label: '</>', key: 'inlineCode' },
  { action: 'codeBlock', label: '{ }', key: 'codeBlock' },
]

const linkOpen = ref(false)
const href = ref('')
const hrefInput = ref<HTMLInputElement | null>(null)

async function openLink(): Promise<void> {
  linkOpen.value = true
  href.value = ''
  await nextTick()
  hrefInput.value?.focus()
}

function applyLink(): void {
  const value = href.value.trim()
  linkOpen.value = false
  if (value !== '') {
    emit('link', value)
  }
}
</script>

<template>
  <div
    class="toolbar"
    role="toolbar"
    :aria-label="t('notes.toolbar.label')"
    data-testid="note-toolbar"
  >
    <button
      v-for="b in buttons"
      :key="b.key"
      class="tool"
      :class="b.action"
      type="button"
      :title="t(`notes.toolbar.${b.key}`)"
      :aria-label="t(`notes.toolbar.${b.key}`)"
      :data-testid="`tool-${b.key}`"
      @mousedown.prevent
      @click="emit('action', b.action)"
    >
      {{ b.label }}
    </button>
    <button
      class="tool"
      type="button"
      :title="t('notes.toolbar.link')"
      :aria-label="t('notes.toolbar.link')"
      data-testid="tool-link"
      @mousedown.prevent
      @click="openLink"
    >
      🔗
    </button>
    <form v-if="linkOpen" class="link-form" @submit.prevent="applyLink">
      <input
        ref="hrefInput"
        v-model="href"
        class="input link-input"
        type="url"
        :placeholder="t('notes.toolbar.linkPlaceholder')"
        data-testid="tool-link-href"
        @keydown.esc.prevent="linkOpen = false"
      />
      <button class="btn btn-primary small" type="submit">
        {{ t('notes.toolbar.linkApply') }}
      </button>
      <button class="btn btn-ghost small" type="button" @click="linkOpen = false">
        {{ t('notes.toolbar.linkCancel') }}
      </button>
    </form>
  </div>
</template>

<style scoped>
.toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.15rem;
  padding: 0.35rem 1rem;
  border-bottom: 1px solid var(--border);
  background: var(--bg-card);
}
.tool {
  min-width: 2rem;
  height: 1.9rem;
  padding: 0 0.5rem;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: var(--txt-secondary);
  font: inherit;
  font-size: 0.8rem;
  cursor: pointer;
}
.tool:hover {
  color: var(--txt);
  border-color: var(--border);
  background: var(--accent-soft);
}
.tool.bold {
  font-weight: 700;
}
.tool.italic {
  font-style: italic;
}
.tool.inlineCode,
.tool.codeBlock {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.link-form {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  margin-left: 0.5rem;
}
.link-input {
  width: 16rem;
  padding: 0.25rem 0.5rem;
  font-size: 0.8rem;
}
.small {
  padding: 0.25rem 0.6rem;
  font-size: 0.78rem;
}
</style>
