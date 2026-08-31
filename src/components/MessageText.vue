<script setup lang="ts">
import { computed } from 'vue'
import MarkdownIt from 'markdown-it'
import DOMPurify from 'dompurify'

const props = defineProps<{ content: string }>()

const md = new MarkdownIt({ linkify: true, breaks: true })

// Assistant output is untrusted; always sanitize the rendered HTML.
const html = computed(() => DOMPurify.sanitize(md.render(props.content)))
</script>

<template>
  <!-- eslint-disable-next-line vue/no-v-html -->
  <div class="markdown" v-html="html"></div>
</template>

<style scoped>
.markdown :deep(p) {
  margin: 0 0 0.6em;
}
.markdown :deep(p:last-child) {
  margin-bottom: 0;
}
.markdown :deep(pre) {
  background: var(--bg-elevated);
  padding: 0.6em 0.8em;
  border-radius: 8px;
  overflow-x: auto;
  margin: 0.4em 0;
}
.markdown :deep(code) {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.86em;
}
.markdown :deep(:not(pre) > code) {
  background: var(--bg-elevated);
  padding: 0.1em 0.35em;
  border-radius: 4px;
}
.markdown :deep(ul),
.markdown :deep(ol) {
  margin: 0.3em 0 0.6em;
  padding-left: 1.3em;
}
.markdown :deep(a) {
  color: var(--accent);
}
.markdown :deep(h1),
.markdown :deep(h2),
.markdown :deep(h3) {
  font-size: 1.05em;
  margin: 0.6em 0 0.3em;
}
.markdown :deep(table) {
  border-collapse: collapse;
  margin: 0.4em 0;
}
.markdown :deep(th),
.markdown :deep(td) {
  border: 1px solid var(--border);
  padding: 0.3em 0.5em;
}
.markdown :deep(blockquote) {
  border-left: 3px solid var(--border-strong);
  margin: 0.4em 0;
  padding-left: 0.8em;
  color: var(--txt-secondary);
}
</style>
