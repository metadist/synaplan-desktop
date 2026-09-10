<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { UnlistenFn } from '@tauri-apps/api/event'
import * as api from '@/services/tauri'
import { useConfigStore } from '@/stores/config'
import { useProjectsStore } from '@/stores/projects'
import { useAssistantsStore } from '@/stores/assistants'
import { useUiStore } from '@/stores/ui'
import { useErrorText } from '@/composables/useErrorText'
import { useChatThreads } from '@/composables/useChatThreads'
import { useProjectName } from '@/composables/useProjectName'
import { hasStudioCopy, resolveStudioTiles, type TaskCard } from '@/composables/useTaskStudio'
import ChatThreadList from '@/components/ChatThreadList.vue'
import DictationButton from '@/components/DictationButton.vue'
import MessageText from '@/components/MessageText.vue'
import TaskStudio from '@/components/TaskStudio.vue'

interface RunStep {
  summary: string
  status: 'running' | 'ok' | 'error'
}

interface UiMessage {
  role: 'user' | 'assistant'
  content: string
  model?: string
  createdAt?: string
  steps?: RunStep[]
  artifacts?: string[]
}

const { t } = useI18n()
const config = useConfigStore()
const projects = useProjectsStore()
const assistants = useAssistantsStore()
const ui = useUiStore()
const errorText = useErrorText()
const projectName = useProjectName()

const projectId = computed(() => projects.active?.id ?? '')
const threads = useChatThreads(projectId)

const messages = ref<UiMessage[]>([])
const input = ref('')
const sending = ref(false)
const error = ref('')
const listEl = ref<HTMLElement | null>(null)
const composerInput = ref<HTMLTextAreaElement | null>(null)
const copiedIndex = ref<number | null>(null)
const composerArmed = ref(false)

const skills = ref<api.Skill[]>([])
const studioPicks = ref<string[]>([])
const executionConsent = ref(false)
const showConsent = ref(false)
const pendingText = ref('')

/** The project's Chat model, shown as the provider id (never the full key). */
const chatModel = computed(() => projects.active?.models.chat ?? '')
const chatModelLabel = computed(() => {
  const parts = chatModel.value.split(':')
  return parts.length >= 3 ? parts.slice(1, -1).join(':') : chatModel.value
})
const hasChatModel = computed(() => chatModel.value !== '')

/** The Assistant pinned on the open thread; `null` follows the project default. */
const threadAssistantId = ref<number | null>(null)
const boundAssistants = computed(() =>
  (projects.active?.assistantIds ?? [])
    .map((id) => assistants.byId.get(id))
    .filter((a): a is api.Assistant => a !== undefined),
)
const assistantInForce = computed<number | null>(
  () => threadAssistantId.value ?? projects.active?.defaultAssistantId ?? null,
)
const assistantLabel = computed(() =>
  assistantInForce.value === null
    ? ''
    : assistants.name(assistantInForce.value) || t('assistants.one'),
)
const showAssistantChip = computed(() => (projects.active?.assistantIds.length ?? 0) > 0)

function onPickAssistant(event: Event): void {
  const raw = (event.target as HTMLSelectElement).value
  threadAssistantId.value = raw === '' ? null : Number(raw)
  if (threads.current.value) {
    void persistThread()
  }
}
/** Skills this project may run: the project's overlay ∩ enabled on this computer ∩ not blocked. */
const projectSkills = computed(() => {
  const overlay = new Set(projects.active?.enabledSkills ?? [])
  return skills.value.filter((s) => s.enabled && !s.blocked && overlay.has(s.name))
})
const enabledSkillCount = computed(() => projectSkills.value.length)
const agentMode = computed(() => enabledSkillCount.value > 0)
const studioCards = computed(() => resolveStudioTiles(projectSkills.value, studioPicks.value))
const showStudio = computed(() => messages.value.length === 0 && studioCards.value.length > 0)
const showWorking = computed(
  () => sending.value && messages.value[messages.value.length - 1]?.role === 'user',
)

let armedTimer: ReturnType<typeof setTimeout> | undefined

const unlisteners: UnlistenFn[] = []

onMounted(async () => {
  unlisteners.push(
    await api.onChatToken((token) => appendAssistantText(token)),
    await api.onChatDone(() => finishTurn()),
    await api.onChatError((e) => onStreamError(e)),
    await api.onAgentText((text) => appendAssistantText(text)),
    await api.onAgentTool((ev) => applyToolEvent(ev)),
    await api.onAgentDone(() => finishTurn()),
    await api.onAgentError((e) => onStreamError(e)),
  )
  await refreshSkillState()
  void assistants.load()
  await threads.refresh().catch(() => undefined)
})

// Switching project: stop any stream and show that project's (empty) chat.
watch(projectId, async () => {
  if (sending.value) {
    await stop()
    sending.value = false
  }
  messages.value = []
  error.value = ''
  input.value = ''
  threadAssistantId.value = null
})

onUnmounted(() => {
  unlisteners.forEach((u) => u())
  if (armedTimer !== undefined) {
    clearTimeout(armedTimer)
  }
})

function toStored(): api.StoredChatMessage[] {
  return messages.value.map((m) => ({
    role: m.role,
    content: m.content,
    model: m.role === 'assistant' ? (m.model ?? chatModel.value) : '',
    createdAt: m.createdAt ?? new Date().toISOString(),
  }))
}

async function persistThread(): Promise<void> {
  if (messages.value.length === 0) {
    return
  }
  try {
    await threads.persist(toStored(), threadAssistantId.value)
  } catch (e) {
    if (!error.value) {
      error.value = errorText(e)
    }
  }
}

function finishTurn(): void {
  sending.value = false
  void persistThread()
}

async function openThread(chatId: string): Promise<void> {
  if (sending.value) {
    return
  }
  error.value = ''
  try {
    const thread = await threads.open(chatId)
    threadAssistantId.value = thread.assistantId
    messages.value = thread.messages.map((m) => ({
      role: m.role,
      content: m.content,
      model: m.model,
      createdAt: m.createdAt,
    }))
    void scrollToBottom()
  } catch (e) {
    error.value = errorText(e)
  }
}

async function removeThread(chatId: string): Promise<void> {
  if (sending.value) {
    return
  }
  try {
    const wasOpen = threads.current.value?.id === chatId
    await threads.remove(chatId)
    if (wasOpen) {
      messages.value = []
    }
  } catch (e) {
    error.value = errorText(e)
  }
}

async function refreshSkillState(): Promise<void> {
  try {
    skills.value = await api.listSkills()
    executionConsent.value = await api.getExecutionConsent()
  } catch {
    skills.value = []
  }
  try {
    studioPicks.value = await api.getStudioTiles()
  } catch {
    studioPicks.value = []
  }
}

function applyTaskPrompt(card: TaskCard): void {
  input.value = hasStudioCopy(card)
    ? t(`chat.studio.cards.${card.id}.prompt`)
    : t('chat.studio.genericPrompt', { name: card.skill })
  composerArmed.value = true
  if (armedTimer !== undefined) {
    clearTimeout(armedTimer)
  }
  armedTimer = setTimeout(() => {
    composerArmed.value = false
  }, 900)
  void nextTick(() => composerInput.value?.focus())
}

async function saveStudioTiles(tiles: string[]): Promise<void> {
  try {
    studioPicks.value = await api.setStudioTiles(tiles)
  } catch {
    studioPicks.value = tiles
  }
}

function newChat(): void {
  if (sending.value) {
    return
  }
  threads.startNew()
  messages.value = []
  error.value = ''
  input.value = ''
  threadAssistantId.value = null
  composerArmed.value = false
}

function onStreamError(e: api.StreamError): void {
  sending.value = false
  error.value = errorText(e)
  if (e.code === 'unauthorized') {
    void config.refresh()
  }
  void persistThread()
}

function currentAssistant(): UiMessage {
  const last = messages.value[messages.value.length - 1]
  if (last && last.role === 'assistant') {
    return last
  }
  const created: UiMessage = {
    role: 'assistant',
    content: '',
    model: chatModel.value,
    createdAt: new Date().toISOString(),
  }
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
  if (!text || sending.value || !hasChatModel.value || !projectId.value) {
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
  messages.value.push({ role: 'user', content: text, createdAt: new Date().toISOString() })
  input.value = ''
  sending.value = true
  void scrollToBottom()
  // The user's message is on disk before the answer streams.
  await persistThread()

  const wire: api.ChatMessage[] = messages.value.map((m) => ({ role: m.role, content: m.content }))
  try {
    if (useAgent) {
      await api.sendAgentChat(projectId.value, wire, allowExec, threadAssistantId.value)
    } else {
      await api.sendChat(projectId.value, wire, threadAssistantId.value)
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

function parentDir(path: string): string {
  return path.replace(/[/\\][^/\\]*$/, '') || path
}

async function openArtifact(path: string): Promise<void> {
  try {
    await api.revealPath(path)
  } catch {
    // Ignore.
  }
}

async function revealFolder(path: string): Promise<void> {
  try {
    await api.revealPath(parentDir(path))
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

// ---- dictation into the composer -------------------------------------------
// The take appends to whatever was typed; interim readings replace only the
// take's own span, the final text replaces it once more. Nothing is sent.
const hasVoiceModel = computed(() => (projects.active?.models.voice ?? '') !== '')
const dictating = ref(false)
let takeBase = ''

function onDictationStart(): void {
  dictating.value = true
  takeBase = input.value
}

function writeTake(text: string): void {
  const glue = takeBase !== '' && !/\s$/.test(takeBase) ? ' ' : ''
  input.value = text ? takeBase + glue + text : takeBase
}

function onDictationDone(text: string): void {
  writeTake(text)
  dictating.value = false
  void nextTick(() => composerInput.value?.focus())
}

function onDictationError(e: unknown): void {
  dictating.value = false
  error.value = errorText(e)
}
</script>

<template>
  <section class="chat">
    <ChatThreadList
      :threads="threads.threads.value"
      :current-id="threads.current.value?.id ?? null"
      :busy="sending"
      @open="openThread"
      @new="newChat"
      @remove="removeThread"
    />

    <div class="chat-main">
      <header class="chat-toolbar">
        <div>
          <h1 class="title">{{ threads.current.value?.title || t('nav.chat') }}</h1>
          <p class="muted subtitle">{{ projectName(projects.active) }}</p>
        </div>
        <div class="toolbar-right">
          <label
            v-if="showAssistantChip && boundAssistants.length > 1"
            class="model-chip"
            :title="t('chat.assistantHint')"
          >
            <span class="muted model-label">{{ t('assistants.one') }}</span>
            <select
              class="assistant-select"
              :value="threadAssistantId ?? ''"
              :disabled="sending"
              data-testid="chat-assistant-select"
              @change="onPickAssistant"
            >
              <option value="">
                {{
                  t('chat.assistantDefault', {
                    name:
                      assistants.name(projects.active?.defaultAssistantId ?? null) ||
                      t('assistants.noneShort'),
                  })
                }}
              </option>
              <option v-for="a in boundAssistants" :key="a.id" :value="a.id">{{ a.name }}</option>
            </select>
          </label>
          <button
            v-else-if="showAssistantChip && assistantInForce !== null"
            class="model-chip"
            type="button"
            :title="t('chat.assistantHint')"
            data-testid="chat-assistant-chip"
            @click="ui.setView('agents')"
          >
            <span class="muted model-label">{{ t('assistants.one') }}</span>
            <span class="model-name">{{ assistantLabel }}</span>
          </button>
          <span v-if="agentMode" class="skills-pill" :title="t('chat.skillsActiveHint')">
            <span class="dot"></span>
            {{ t('chat.skillsActive', { count: enabledSkillCount }) }}
          </span>
          <button
            class="model-chip"
            type="button"
            :class="{ unset: !hasChatModel }"
            :title="t('chat.modelHint')"
            data-testid="chat-model-chip"
            @click="ui.setView('models')"
          >
            <span class="muted model-label">{{ t('chat.modelLabel') }}</span>
            <span class="model-name">{{ hasChatModel ? chatModelLabel : t('models.unset') }}</span>
          </button>
        </div>
      </header>

      <p v-if="!hasChatModel" class="banner banner-warn no-model" data-testid="chat-no-model">
        {{ t('chat.noChatModel') }}
        <button class="btn-link" type="button" @click="ui.setView('models')">
          {{ t('chat.pickModel') }} →
        </button>
      </p>

      <div ref="listEl" class="messages">
        <TaskStudio
          v-if="showStudio"
          :cards="studioCards"
          :skills="projectSkills"
          @pick="applyTaskPrompt"
          @save="saveStudioTiles"
        />
        <p v-else-if="messages.length === 0" class="muted empty">
          {{
            agentMode
              ? t('chat.emptySkills')
              : t('chat.emptyProject', { name: projectName(projects.active) })
          }}
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
            <div
              v-for="(a, ai) in m.artifacts"
              :key="ai"
              class="artifact-card"
              :title="t('chat.open')"
              @click="openArtifact(a)"
            >
              <span class="artifact-icon" aria-hidden="true">📄</span>
              <span class="artifact-name">{{ fileName(a) }}</span>
              <button class="btn-link artifact-reveal" type="button" @click.stop="revealFolder(a)">
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

      <div class="composer" :class="{ 'studio-open': showStudio, armed: composerArmed }">
        <textarea
          ref="composerInput"
          v-model="input"
          class="input composer-input"
          rows="1"
          :placeholder="agentMode ? t('chat.placeholderSkills') : t('chat.placeholder')"
          :disabled="!hasChatModel"
          @keydown="onKeydown"
        ></textarea>
        <DictationButton
          v-if="hasVoiceModel"
          :project-id="projectId"
          :disabled="sending || !hasChatModel"
          @start="onDictationStart"
          @interim="writeTake"
          @done="onDictationDone"
          @error="onDictationError"
        />
        <button v-if="sending" class="btn btn-ghost" type="button" @click="stop">
          {{ t('chat.stop') }}
        </button>
        <button
          v-else
          class="btn btn-primary"
          type="button"
          :disabled="!hasChatModel || dictating || input.trim().length === 0"
          @click="send"
        >
          {{ t('chat.send') }}
        </button>
      </div>
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
  position: relative;
}

.chat-main {
  flex: 1;
  min-width: 0;
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
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 420px;
}

.subtitle {
  margin: 0.05rem 0 0;
  font-size: 0.78rem;
}

.toolbar-right {
  display: inline-flex;
  align-items: center;
  gap: 0.8rem;
  flex-shrink: 0;
}

.no-model {
  margin: 0.7rem 1.2rem 0;
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
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

.model-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  padding: 0.3rem 0.65rem;
  border: 1px solid var(--border-strong);
  border-radius: 999px;
  background: var(--bg-card);
  color: var(--txt);
  font: inherit;
  cursor: pointer;
  max-width: 280px;
}

.model-chip:hover {
  border-color: var(--accent);
}

.model-chip.unset {
  border-style: dashed;
  color: var(--txt-secondary);
}

.model-label {
  font-size: 0.74rem;
}

.assistant-select {
  font: inherit;
  font-size: 0.82rem;
  font-weight: 600;
  border: none;
  background: transparent;
  color: var(--txt);
  max-width: 200px;
  cursor: pointer;
}

.model-name {
  font-size: 0.82rem;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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

.composer.studio-open {
  background: var(--accent-soft);
  border-top: 2px solid var(--accent);
  padding: 1rem 1.2rem 1.05rem;
}

.composer-input {
  flex: 1;
  resize: none;
  max-height: 140px;
  min-height: 40px;
}

.composer.studio-open .composer-input {
  min-height: 72px;
  border-color: var(--accent);
  border-width: 2px;
  background: var(--bg-card);
}

.composer.armed .composer-input {
  animation: composer-armed 0.9s ease;
}

@keyframes composer-armed {
  0% {
    box-shadow: 0 0 0 0 color-mix(in srgb, var(--accent) 45%, transparent);
  }
  40% {
    box-shadow: 0 0 0 6px color-mix(in srgb, var(--accent) 22%, transparent);
  }
  100% {
    box-shadow: 0 0 0 0 transparent;
  }
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
