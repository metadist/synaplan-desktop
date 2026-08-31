<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { UnlistenFn } from '@tauri-apps/api/event'
import * as api from '@/services/tauri'
import { useConfigStore } from '@/stores/config'
import { useErrorText } from '@/composables/useErrorText'

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
const models = ref<string[]>([])
const selectedModel = ref('') // '' means the account default
const listEl = ref<HTMLElement | null>(null)

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
    if (e.code === 'unauthorized') void config.refresh()
  })
  try {
    models.value = await api.listModels()
  } catch {
    // The account default is used when models cannot be listed.
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
  if (!text || sending.value) {
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

function onKeydown(e: KeyboardEvent): void {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    void send()
  }
}
</script>

<template>
  <div class="chat">
    <div class="chat-toolbar">
      <label class="model">
        <span class="muted model-label">{{ t('chat.modelLabel') }}</span>
        <select v-model="selectedModel" class="input model-select">
          <option value="">{{ t('chat.modelDefault') }}</option>
          <option v-for="m in models" :key="m" :value="m">{{ m }}</option>
        </select>
      </label>
    </div>

    <div ref="listEl" class="messages">
      <p v-if="messages.length === 0" class="muted empty">{{ t('chat.empty') }}</p>

      <div v-for="(m, i) in messages" :key="i" class="msg" :class="m.role">
        <div class="msg-role muted">
          {{ m.role === 'user' ? t('chat.you') : t('chat.assistant') }}
        </div>
        <div class="msg-body">{{ m.content }}</div>
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
        @keydown="onKeydown"
      ></textarea>
      <button
        class="btn btn-primary"
        :disabled="sending || input.trim().length === 0"
        @click="send"
      >
        {{ t('chat.send') }}
      </button>
    </div>
  </div>
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
  justify-content: flex-end;
  padding: 0.5rem 1rem;
  border-bottom: 1px solid var(--border);
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
  padding: 0.35rem 0.5rem;
  font-size: 0.82rem;
}

.messages {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.9rem;
}

.empty {
  margin: auto;
  text-align: center;
  max-width: 320px;
}

.msg {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  max-width: 80%;
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
  padding: 0.6rem 0.8rem;
  border-radius: var(--radius);
  background: var(--bg-card);
  border: 1px solid var(--border);
}

.msg.user .msg-body {
  background: var(--accent);
  color: var(--accent-contrast);
  border-color: transparent;
}

.working {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  color: var(--txt-secondary);
}

.chat-error {
  margin: 0 1rem;
}

.composer {
  display: flex;
  gap: 0.6rem;
  align-items: flex-end;
  padding: 0.8rem 1rem;
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
