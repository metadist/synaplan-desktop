<script setup lang="ts">
import { computed, nextTick, onUpdated, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import MarkdownIt from 'markdown-it'
import DOMPurify from 'dompurify'
import { escapeHtml, isRenderableImageSrc } from '@/composables/useChatMarkdown'

const props = defineProps<{ content: string }>()

const { t } = useI18n()
const root = ref<HTMLElement | null>(null)

const md = new MarkdownIt({ linkify: true, breaks: true })

md.renderer.rules.image = (tokens, idx) => {
  const token = tokens[idx]
  const src = token.attrGet('src') ?? ''
  const alt = token.content || token.attrGet('alt') || ''
  if (!isRenderableImageSrc(src)) {
    return fallbackHtml(alt)
  }
  return `<img src="${escapeHtml(src)}" alt="${escapeHtml(alt)}" class="chat-image" />`
}

function fallbackHtml(alt: string): string {
  const text = alt.trim() === '' ? t('chat.imageFallback') : alt
  return `<span class="chat-image-fallback">${escapeHtml(text)}</span>`
}

const html = computed(() =>
  DOMPurify.sanitize(md.render(props.content), {
    ADD_ATTR: ['class'],
  }),
)

function bindImageErrors(): void {
  const el = root.value
  if (!el) {
    return
  }
  el.querySelectorAll<HTMLImageElement>('img.chat-image').forEach((img) => {
    if (img.dataset.bound === '1') {
      return
    }
    img.dataset.bound = '1'
    img.addEventListener('error', () => {
      const span = document.createElement('span')
      span.className = 'chat-image-fallback'
      span.textContent = img.alt.trim() || t('chat.imageFallback')
      img.replaceWith(span)
    })
  })
}

watch(
  html,
  () => {
    void nextTick(bindImageErrors)
  },
  { flush: 'post' },
)
onUpdated(bindImageErrors)
</script>

<template>
  <!-- eslint-disable-next-line vue/no-v-html -->
  <div ref="root" class="markdown" data-testid="message-text" v-html="html"></div>
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
.markdown :deep(img.chat-image) {
  display: block;
  max-width: min(100%, 480px);
  height: auto;
  margin: 0.5em 0;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-elevated);
}
.markdown :deep(span.chat-image-fallback) {
  display: inline-flex;
  align-items: center;
  gap: 0.35em;
  margin: 0.25em 0;
  padding: 0.2em 0.55em;
  border-radius: 6px;
  background: var(--bg-elevated);
  color: var(--txt-secondary);
  font-size: 0.86em;
}
</style>
