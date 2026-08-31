<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { UnlistenFn } from '@tauri-apps/api/event'
import * as api from '@/services/tauri'
import { useConfigStore } from '@/stores/config'
import { useErrorText } from '@/composables/useErrorText'
import { chatModelGroups, defaultChatModel } from '@/composables/useModels'
import MessageText from '@/components/MessageText.vue'

interface RunStep {
  summary: string
  status: 'running' | 'ok' | 'error'
}

interface UiMessage {
  role: 'user' | 'assistant'
  content: string
  steps?: RunStep[]
  artifacts?: string[]
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

const enabledSkillCount = ref(0)
const executionConsent = ref(false)
const showConsent = ref(false)
const pendingText = ref('')

const modelGroups = computed(() => chatModelGroups(models.value))
const hasModels = computed(() => modelGroups.value.length > 0)
const agentMode = computed(() => enabledSkillCount.value > 0)
const showWorking = computed(
  () => sending.value && messages.value[messages.value.length - 1]?.role === 'user',
)

const unlisteners: UnlistenFn[] = []

onMounted(async () => {
  unlisteners.push(
    await api.onChatToken((token) => appendAssistantText(token)),
    await api.onChatDone(() => {
      sending.value = false
    }),
    await api.onChatError((e) => onStreamError(e)),
    await api.onAgentText((text) => appendAssistantText(text)),
    await api.onAgentTool((ev) => applyToolEvent(ev)),
    await api.onAgentDone(() => {
      sending.value = false
    }),
    await api.onAgentError((e) => onStreamError(e)),
  )

  try {
    models.value = await api.listModels()
    if (!selectedModel.value) {
      selectedModel.value = defaultChatModel(modelGroups.value)
    }
  } catch {
    // No models: the composer shows a disabled hint.
  }
  await refreshSkillState()
})

onUnmounted(() => {
  unlisteners.forEach((u) => u())
})

async function refreshSkillState(): Promise<void> {
  try {
    const skills = await api.listSkills()
    enabledSkillCount.value = skills.filter((s) => s.enabled).length
    executionConsent.value = await api.getExecutionConsent()
  } catch {
    enabledSkillCount.value = 0
  }
}

function onStreamError(e: api.StreamError): void {
  sending.value = false
  error.value = errorText(e)
  if (e.code === 'unauthorized') {
    void config.refresh()
  }
}

function currentAssistant(): UiMessage {
  const last = messages.value[messages.value.length - 1]
  if (last && last.role === 'assistant') {
    return last
  }
  const created: UiMessage = { role: 'assistant', content: '' }
  messages.value.push(created)
  return created
}

function appendAssistantText(text: string): void {
  const msg = currentAssistant()
  msg.content += text
  void scrollToBottom()
}

function applyToolEvent(ev: api.AgentToolEvent): void {
  const msg = currentAssistant()
  if (!msg.steps) {
    msg.steps = []
  }
  if (ev.phase === 'start') {
    msg.steps.push({ summary: ev.summary, status: 'running' })
  } else {
    const running = [...msg.steps].reverse().find((s) => s.status === 'running')
    if (running) {
      running.summary = ev.summary
      running.status = ev.ok ? 'ok' : 'error'
    } else {
      msg.steps.push({ summary: ev.summary, status: ev.ok ? 'ok' : 'error' })
    }
    if (ev.artifact) {
      if (!msg.artifacts) {
        msg.artifacts = []
      }
      if (!msg.artifacts.includes(ev.artifact)) {
        msg.artifacts.push(ev.artifact)
      }
    }
  }
  void scrollToBottom()
}

async function scrollToBottom(): Promise<void> {
  await nextTick()
  if (listEl.value) {
    listEl.value.scrollTop = listEl.value.scrollHeight
  }
}

function send(): void {
  const text = input.value.trim()
  if (!text || sending.value || !hasModels.value) {
    return
  }
  // First skill turn on this install asks for execution consent once.
  if (agentMode.value && !executionConsent.value) {
    pendingText.value = text
    showConsent.value = true
    return
  }
  void dispatchSend(text, agentMode.value, executionConsent.value)
}

async function confirmConsent(allow: boolean): Promise<void> {
  showConsent.value = false
  if (allow) {
    try {
      await api.setExecutionConsent()
      executionConsent.value = true
    } catch {
      // If persisting fails we still proceed for this turn without exec.
    }
  }
  const text = pendingText.value
  pendingText.value = ''
  await dispatchSend(text, true, allow)
}

async function dispatchSend(text: string, useAgent: boolean, allowExec: boolean): Promise<void> {
  error.value = ''
  messages.value.push({ role: 'user', content: text })
  input.value = ''
  sending.value = true
  void scrollToBottom()

  const wire: api.ChatMessage[] = messages.value.map((m) => ({ role: m.role, content: m.content }))
  try {
    if (useAgent) {
      await api.sendAgentChat(wire, selectedModel.value || null, allowExec)
    } else {
      await api.sendChat(wire, selectedModel.value || null)
    }
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
    // Ignore: the turn ends on its own shortly after.
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

function fileName(path: string): string {
  return path.split(/[/\\]/).pop() || path
}

async function reveal(path: string): Promise<void> {
  try {
    await api.revealPath(path)
  } catch {
    // Ignore.
  }
}

function onKeydown(e: KeyboardEvent): void {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    send()
  }
}
</script>

<template>
  <section class="chat">
    <header class="chat-toolbar">
      <h1 class="title">{{ t('nav.chat') }}</h1>
      <div class="toolbar-right">
        <span v-if="agentMode" class="skills-pill" :title="t('chat.skillsActiveHint')">
          <span class="dot"></span>
          {{ t('chat.skillsActive', { count: enabledSkillCount }) }}
        </span>
        <label class="model">
          <span class="muted model-label">{{ t('chat.modelLabel') }}</span>
          <select v-if="hasModels" v-model="selectedModel" class="input model-select">
            <optgroup v-for="g in modelGroups" :key="g.provider" :label="g.provider">
              <option v-for="m in g.models" :key="m.id" :value="m.id">{{ m.id }}</option>
            </optgroup>
          </select>
          <span v-else class="muted no-models">{{ t('chat.noModels') }}</span>
        </label>
      </div>
    </header>

    <div ref="listEl" class="messages">
      <p v-if="messages.length === 0" class="muted empty">
        {{ agentMode ? t('chat.emptySkills') : t('chat.empty') }}
      </p>

      <div v-for="(m, i) in messages" :key="i" class="msg" :class="m.role">
        <div class="msg-role muted">
          {{ m.role === 'user' ? t('chat.you') : t('chat.assistant') }}
        </div>

        <ul v-if="m.steps && m.steps.length" class="run-steps">
          <li v-for="(s, si) in m.steps" :key="si" class="run-step" :class="s.status">
            <span v-if="s.status === 'running'" class="spinner spinner-sm"></span>
            <span v-else class="step-icon" aria-hidden="true">{{
              s.status === 'ok' ? '✓' : '✕'
            }}</span>
            <span class="step-text">{{ s.summary }}</span>
          </li>
        </ul>

        <div v-if="m.role === 'assistant' && (m.content || !m.steps?.length)" class="msg-body">
          <MessageText :content="m.content" />
          <button v-if="m.content" class="copy" type="button" @click="copyMessage(i, m.content)">
            {{ copiedIndex === i ? t('chat.copied') : t('chat.copy') }}
          </button>
        </div>
        <div v-else-if="m.role === 'user'" class="msg-body">{{ m.content }}</div>

        <div v-if="m.artifacts && m.artifacts.length" class="artifacts">
          <div class="artifacts-label muted">{{ t('chat.created') }}</div>
          <div v-for="(a, ai) in m.artifacts" :key="ai" class="artifact-card" @click="reveal(a)">
            <span class="artifact-icon" aria-hidden="true">📄</span>
            <span class="artifact-name">{{ fileName(a) }}</span>
            <button class="btn-link artifact-reveal" type="button" @click.stop="reveal(a)">
              {{ t('chat.reveal') }}
            </button>
          </div>
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
        :placeholder="agentMode ? t('chat.placeholderSkills') : t('chat.placeholder')"
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

    <div v-if="showConsent" class="consent-overlay" role="dialog" aria-modal="true">
      <div class="consent-card">
        <div class="consent-icon" aria-hidden="true">🔒</div>
        <h2 class="consent-title">{{ t('chat.consentTitle') }}</h2>
        <p class="consent-body">{{ t('chat.consentBody') }}</p>
        <ul class="consent-points">
          <li>{{ t('chat.consentPoint1') }}</li>
          <li>{{ t('chat.consentPoint2') }}</li>
          <li>{{ t('chat.consentPoint3') }}</li>
        </ul>
        <div class="consent-actions">
          <button class="btn btn-ghost" type="button" @click="confirmConsent(false)">
            {{ t('chat.consentNotNow') }}
          </button>
          <button class="btn btn-primary" type="button" @click="confirmConsent(true)">
            {{ t('chat.consentAllow') }}
          </button>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.chat {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  position: relative;
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

.toolbar-right {
  display: inline-flex;
  align-items: center;
  gap: 0.8rem;
}

.skills-pill {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.76rem;
  padding: 0.22rem 0.6rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--accent) 14%, transparent);
  color: var(--accent);
  font-weight: 600;
}

.skills-pill .dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--ok, #22c55e);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--ok, #22c55e) 25%, transparent);
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
  max-width: 240px;
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
  max-width: 360px;
}

.msg {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
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

.run-steps {
  list-style: none;
  margin: 0;
  padding: 0.5rem 0.7rem;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  background: color-mix(in srgb, var(--accent) 6%, var(--bg-card));
  border: 1px solid var(--border);
  border-radius: var(--radius);
  font-size: 0.83rem;
}

.run-step {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.run-step.error .step-text {
  color: var(--danger);
}

.step-icon {
  width: 16px;
  text-align: center;
  font-weight: 800;
}

.run-step.ok .step-icon {
  color: var(--ok, #22c55e);
}

.run-step.error .step-icon {
  color: var(--danger);
}

.spinner-sm {
  width: 13px;
  height: 13px;
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

.artifacts {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.artifacts-label {
  font-size: 0.72rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.artifact-card {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.55rem 0.7rem;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg-card);
  cursor: pointer;
  transition:
    border-color 0.12s ease,
    transform 0.12s ease;
}

.artifact-card:hover {
  border-color: var(--accent);
  transform: translateY(-1px);
}

.artifact-icon {
  font-size: 1.1rem;
}

.artifact-name {
  flex: 1;
  font-size: 0.86rem;
  font-weight: 550;
  overflow-wrap: anywhere;
}

.artifact-reveal {
  font-size: 0.78rem;
  white-space: nowrap;
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

.consent-overlay {
  position: absolute;
  inset: 0;
  background: color-mix(in srgb, var(--bg) 70%, transparent);
  backdrop-filter: blur(3px);
  display: grid;
  place-items: center;
  padding: 1.5rem;
  z-index: 20;
}

.consent-card {
  max-width: 420px;
  width: 100%;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: calc(var(--radius) * 1.4);
  padding: 1.6rem;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.28);
  text-align: center;
}

.consent-icon {
  font-size: 2rem;
}

.consent-title {
  margin: 0.5rem 0 0.4rem;
  font-size: 1.15rem;
}

.consent-body {
  margin: 0 0 0.9rem;
  color: var(--txt-secondary);
  font-size: 0.9rem;
}

.consent-points {
  text-align: left;
  margin: 0 0 1.2rem;
  padding-left: 1.1rem;
  font-size: 0.85rem;
  color: var(--txt-secondary);
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}

.consent-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.6rem;
}
</style>
