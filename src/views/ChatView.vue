<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { UnlistenFn } from '@tauri-apps/api/event'
import * as api from '@/services/tauri'
import { useConfigStore } from '@/stores/config'
import { useErrorText } from '@/composables/useErrorText'
import { chatModelGroups, defaultChatModel } from '@/composables/useModels'
import MessageText from '@/components/MessageText.vue'

interface UiMessage {
  role: 'user' | 'assistant'
  content: string
}

const { t } = useI18n()
const config = useConfigStore()
const errorText = useErrorText()

const messages = ref<UiMessage[]>([])
const input = ref('')
const sending = ref(false)
const error = ref('')
const models = ref<api.ModelInfo[]>([])
const selectedModel = ref('')
const listEl = ref<HTMLElement | null>(null)
const copiedIndex = ref<number | null>(null)

const modelGroups = computed(() => chatModelGroups(models.value))
const hasModels = computed(() => modelGroups.value.length > 0)
const showWorking = computed(
  () => sending.value && messages.value[messages.value.length - 1]?.role === 'user',
)

let unlistenToken: UnlistenFn | null = null
let unlistenDone: UnlistenFn | null = null
let unlistenError: UnlistenFn | null = null

onMounted(async () => {
  unlistenToken = await api.onChatToken((token) => appendToken(token))
  unlistenDone = await api.onChatDone(() => {
    sending.value = false
  })
  unlistenError = await api.onChatError((e) => {
    sending.value = false
    error.value = errorText(e)
    if (e.code === 'unauthorized') {
      void config.refresh()
    }
  })
  try {
    models.value = await api.listModels()
    if (!selectedModel.value) {
      selectedModel.value = defaultChatModel(modelGroups.value)
    }
  } catch {
    // No models: the composer shows a disabled hint.
  }
})

onUnmounted(() => {
  unlistenToken?.()
  unlistenDone?.()
  unlistenError?.()
})

function appendToken(token: string): void {
  const last = messages.value[messages.value.length - 1]
  if (last && last.role === 'assistant') {
    last.content += token
  } else {
    messages.value.push({ role: 'assistant', content: token })
  }
  void scrollToBottom()
}

async function scrollToBottom(): Promise<void> {
  await nextTick()
  if (listEl.value) {
    listEl.value.scrollTop = listEl.value.scrollHeight
  }
}

async function send(): Promise<void> {
  const text = input.value.trim()
  if (!text || sending.value || !hasModels.value) {
    return
  }
  error.value = ''
  messages.value.push({ role: 'user', content: text })
  input.value = ''
  sending.value = true
  void scrollToBottom()

  const wire: api.ChatMessage[] = messages.value.map((m) => ({ role: m.role, content: m.content }))
  try {
    await api.sendChat(wire, selectedModel.value || null)
  } catch (e) {
    sending.value = false
    if (!error.value) {
      error.value = errorText(e)
    }
    if (api.asCommandError(e).code === 'unauthorized') {
      void config.refresh()
    }
  }
}

async function stop(): Promise<void> {
  try {
    await api.cancelChat()
  } catch {
    // Ignore: the stream ends on its own shortly after.
  }
}

async function copyMessage(index: number, content: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(content)
    copiedIndex.value = index
    setTimeout(() => {
      if (copiedIndex.value === index) {
        copiedIndex.value = null
      }
    }, 1500)
  } catch {
    // Clipboard unavailable; ignore.
  }
}

function onKeydown(e: KeyboardEvent): void {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    void send()
  }
}
</script>

<template>
  <section class="chat">
    <header class="chat-toolbar">
      <h1 class="title">{{ t('nav.chat') }}</h1>
      <label class="model">
        <span class="muted model-label">{{ t('chat.modelLabel') }}</span>
        <select v-if="hasModels" v-model="selectedModel" class="input model-select">
          <optgroup v-for="g in modelGroups" :key="g.provider" :label="g.provider">
            <option v-for="m in g.models" :key="m.id" :value="m.id">{{ m.id }}</option>
          </optgroup>
        </select>
        <span v-else class="muted no-models">{{ t('chat.noModels') }}</span>
      </label>
    </header>

    <div ref="listEl" class="messages">
      <p v-if="messages.length === 0" class="muted empty">{{ t('chat.empty') }}</p>

      <div v-for="(m, i) in messages" :key="i" class="msg" :class="m.role">
        <div class="msg-role muted">
          {{ m.role === 'user' ? t('chat.you') : t('chat.assistant') }}
        </div>
        <div class="msg-body">
          <template v-if="m.role === 'assistant'">
            <MessageText :content="m.content" />
            <button class="copy" type="button" @click="copyMessage(i, m.content)">
              {{ copiedIndex === i ? t('chat.copied') : t('chat.copy') }}
            </button>
          </template>
          <template v-else>{{ m.content }}</template>
        </div>
      </div>

      <div v-if="showWorking" class="msg assistant">
        <div class="msg-role muted">{{ t('chat.assistant') }}</div>
        <div class="msg-body working"><span class="spinner"></span>{{ t('chat.working') }}</div>
      </div>
    </div>

    <p v-if="error" class="banner banner-error chat-error" role="alert">{{ error }}</p>

    <div class="composer">
      <textarea
        v-model="input"
        class="input composer-input"
        rows="1"
        :placeholder="t('chat.placeholder')"
        :disabled="!hasModels"
        @keydown="onKeydown"
      ></textarea>
      <button v-if="sending" class="btn btn-ghost" type="button" @click="stop">
        {{ t('chat.stop') }}
      </button>
      <button
        v-else
        class="btn btn-primary"
        type="button"
        :disabled="!hasModels || input.trim().length === 0"
        @click="send"
      >
        {{ t('chat.send') }}
      </button>
    </div>
  </section>
</template>

<style scoped>
.chat {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.chat-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.7rem 1.2rem;
  border-bottom: 1px solid var(--border);
}

.title {
  font-size: 1rem;
}

.model {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
}

.model-label {
  font-size: 0.78rem;
}

.model-select {
  width: auto;
  max-width: 260px;
  padding: 0.35rem 0.5rem;
  font-size: 0.82rem;
}

.no-models {
  font-size: 0.8rem;
}

.messages {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 1.1rem 1.2rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.empty {
  margin: auto;
  text-align: center;
  max-width: 340px;
}

.msg {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  max-width: 82%;
}

.msg.user {
  align-self: flex-end;
  align-items: flex-end;
}

.msg-role {
  font-size: 0.72rem;
}

.msg-body {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  padding: 0.65rem 0.85rem;
  border-radius: var(--radius);
  background: var(--bg-card);
  border: 1px solid var(--border);
}

.msg.assistant .msg-body {
  white-space: normal;
}

.msg.user .msg-body {
  background: var(--accent);
  color: var(--accent-contrast);
  border-color: transparent;
}

.copy {
  margin-top: 0.4rem;
  background: none;
  border: none;
  padding: 0;
  font: inherit;
  font-size: 0.72rem;
  color: var(--txt-secondary);
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.12s ease;
}

.msg.assistant:hover .copy {
  opacity: 1;
}

.copy:hover {
  color: var(--accent);
}

.working {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  color: var(--txt-secondary);
}

.chat-error {
  margin: 0 1.2rem;
}

.composer {
  display: flex;
  gap: 0.6rem;
  align-items: flex-end;
  padding: 0.8rem 1.2rem;
  border-top: 1px solid var(--border);
  background: var(--bg-card);
}

.composer-input {
  flex: 1;
  resize: none;
  max-height: 140px;
  min-height: 40px;
}
</style>
