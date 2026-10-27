<script setup lang="ts">
import { computed, nextTick, onActivated, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { UnlistenFn } from '@tauri-apps/api/event'
import * as api from '@/services/tauri'
import { useConfigStore } from '@/stores/config'
import { useProjectsStore } from '@/stores/projects'
import { useAssistantsStore } from '@/stores/assistants'
import { useUiStore } from '@/stores/ui'
import { useErrorText } from '@/composables/useErrorText'
import { useProjectName } from '@/composables/useProjectName'
import { turnNeedsFileTools } from '@/lib/turnRouting'
import { useChatActivity, type RetryAttempt, type UiMessage } from '@/composables/useChatActivity'
import { useChatSession } from '@/composables/useChatSession'
import { createTurnClock, useStreamTurn } from '@/composables/useStreamTurn'
import { useAgentTurn } from '@/composables/useAgentTurn'
import ChatComposer from '@/components/ChatComposer.vue'
import ChatThread from '@/components/ChatThread.vue'
import ChatThreadList from '@/components/ChatThreadList.vue'
import ChatToolbar from '@/components/ChatToolbar.vue'
import ProjectPanel from '@/components/ProjectPanel.vue'
import type { ActivityPhase } from '@/components/ChatActivity.vue'

const { t } = useI18n()
const config = useConfigStore()
const projects = useProjectsStore()
const assistants = useAssistantsStore()
const ui = useUiStore()
const errorText = useErrorText()
const projectName = useProjectName()

const projectId = computed(() => projects.active?.id ?? '')
const messages = ref<UiMessage[]>([])
const input = ref('')
const sending = ref(false)
const error = ref('')
const errorCode = ref('')
const retryAttempt = ref<RetryAttempt | null>(null)
const activity = ref<ActivityPhase | null>(null)
const clock = createTurnClock()
const threadAssistantId = ref<number | null>(null)
const threadRef = ref<{ scrollToBottom: () => Promise<void> } | null>(null)
const composerRef = ref<{ focus: () => void; cancelTake: () => Promise<void> } | null>(null)
const composerDrafts = new Map<string, string>()
const showConsent = ref(false)
const consentDeclineRef = ref<HTMLButtonElement | null>(null)
const pendingText = ref('')
/** True when Agents handed us a starter — send as a skill turn even if tiles are still loading. */
const pendingAgent = ref(false)

/** The history column is a 60px rail when folded; the pill and the rail's own button flip it. */
const historyOpen = computed(() => !ui.historyCollapsed)
function toggleHistory(): void {
  ui.setHistoryCollapsed(!ui.historyCollapsed)
}

/** The project's Chat model, shown as the provider id (never the full key). */
const chatModel = computed(() => projects.active?.models.chat ?? '')
const chatModelLabel = computed(() => {
  const parts = chatModel.value.split(':')
  return parts.length >= 3 ? parts.slice(1, -1).join(':') : chatModel.value
})
const hasChatModel = computed(() => chatModel.value !== '')
const webSearchOn = computed(() => projects.active?.webSearch === true)

/** The Assistant pinned on the open thread; `null` follows the project default. */
const boundAssistants = computed(() =>
  (projects.active?.assistantIds ?? [])
    .map((id) => assistants.byId.get(id))
    .filter((assistant): assistant is api.Assistant => assistant !== undefined),
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

function focusComposer(): void {
  composerRef.value?.focus()
}

async function scrollToBottom(): Promise<void> {
  await threadRef.value?.scrollToBottom()
}

function clearInput(): void {
  input.value = ''
}

const {
  threads,
  notes,
  knowledge,
  panelTab,
  dragging,
  picking,
  noteCount,
  notesBump,
  fileCount,
  chatCount,
  executionConsent,
  composerArmed,
  webSearchBusy,
  projectSkills,
  enabledSkillCount,
  agentMode,
  studioCards,
  showStudio,
  showWelcome,
  togglePanel,
  openPanel,
  newNote,
  toggleWebSearch,
  onNoteButton,
  keepMessage,
  openKeptNote,
  addFiles,
  onDrop,
  persistThread,
  refreshKnowledge,
  openThread,
  removeThread,
  refreshSkillState,
  armComposer,
  applyTaskPrompt,
  saveStudioTiles,
  newChat,
  dispose,
} = useChatSession({
  projectId,
  messages,
  input,
  sending,
  error,
  errorCode,
  retryAttempt,
  clock,
  threadAssistantId,
  chatModel,
  scrollToBottom,
  focusComposer,
})

/** A starter belongs to the chat it was queued for; a new chat or project drops it. */
function onNewChat(): void {
  pendingAgent.value = false
  newChat()
}

function onPickAssistant(assistantId: number | null): void {
  threadAssistantId.value = assistantId
  if (threads.current.value) {
    void persistThread()
  }
}

const { showActivity, activityPhase, streamingReply } = useChatActivity(messages, sending, activity)

const stream = useStreamTurn({
  messages,
  sending,
  activity,
  error,
  errorCode,
  retryAttempt,
  clock,
  projectId,
  chatModel,
  threadAssistantId,
  webSearchOn,
  scrollToBottom,
  persistThread,
  refreshKnowledge,
  clearInput,
})
const agent = useAgentTurn({
  clock,
  messages,
  scrollToBottom,
  currentAssistant: stream.currentAssistant,
  pushArtifact: stream.pushArtifact,
  appendAssistantText: stream.appendAssistantText,
  finishTurn: stream.finishTurn,
  onStreamError: stream.onStreamError,
})

const unlisteners: UnlistenFn[] = []

onMounted(async () => {
  unlisteners.push(
    ...(await stream.listen()),
    ...(await agent.listen()),
    await api.onFileDrop((event) => onDrop(event)),
  )
  await refreshSkillState()
  void assistants.load()
  await threads.refresh().catch(() => undefined)
  void notes.refresh()
  void knowledge.refresh()
  consumePendingChat()
})

// Coming back from Notes / Files / Agents: refresh pills and take a starter prompt.
onActivated(() => {
  void notes.refresh()
  void knowledge.refresh()
  void refreshSkillState().then(() => consumePendingChat())
})

watch(
  () => ui.pendingChat,
  (pending) => {
    if (pending && ui.view === 'chat') {
      consumePendingChat()
    }
  },
)

function consumePendingChat(): void {
  const pending = ui.takePendingChat()
  if (!pending) {
    return
  }
  input.value = pending.prompt
  pendingAgent.value = true
  armComposer()
  void nextTick(() => {
    focusComposer()
    if (pending.autoSend) {
      send()
    }
  })
}

// Switching project: stop any stream/dictation and restore that project's draft.
watch(projectId, async (next, prev) => {
  stream.bumpTurn()
  if (sending.value) {
    await stream.stop()
    sending.value = false
  }
  showConsent.value = false
  activity.value = null
  pendingText.value = ''
  pendingAgent.value = false
  if (composerRef.value) {
    await composerRef.value.cancelTake()
  }
  if (prev) {
    composerDrafts.set(prev, input.value)
  }
  messages.value = []
  error.value = ''
  errorCode.value = ''
  retryAttempt.value = null
  clock.currentAttempt = null
  clock.pendingDocument = false
  input.value = next ? (composerDrafts.get(next) ?? '') : ''
  threadAssistantId.value = null
  dragging.value = false
})

onUnmounted(() => {
  unlisteners.forEach((unlisten) => unlisten())
  dispose()
})

watch(
  () => config.pollStatus?.lastErrorCode,
  (code, prev) => {
    if (prev && !code && (errorCode.value === 'unreachable' || errorCode.value === 'network')) {
      error.value = ''
      errorCode.value = ''
    }
  },
)

function send(): void {
  const text = input.value.trim()
  if (!text || sending.value || !hasChatModel.value || !projectId.value) {
    return
  }
  const useAgent = agentMode.value || pendingAgent.value || turnNeedsFileTools(text)
  // A skill turn asks for execution consent once. Naming a file does not:
  // list, read and write need no skill, and run_program stays behind consent.
  if ((agentMode.value || pendingAgent.value) && !executionConsent.value) {
    pendingText.value = text
    showConsent.value = true
    // The safe choice holds focus so Enter can never grant consent by accident.
    void nextTick(() => consentDeclineRef.value?.focus())
    return
  }
  pendingAgent.value = false
  void stream.dispatchSend(text, useAgent, executionConsent.value)
}

async function confirmConsent(allow: boolean): Promise<void> {
  const text = pendingText.value
  const sendProject = projectId.value
  showConsent.value = false
  pendingText.value = ''
  if (!text || !sendProject) {
    return
  }
  if (allow) {
    try {
      await api.setExecutionConsent()
      executionConsent.value = true
    } catch {
      // If persisting fails we still proceed for this turn without exec.
    }
  }
  pendingAgent.value = false
  if (projectId.value !== sendProject) {
    return
  }
  await stream.dispatchSend(text, true, allow)
}

/** Escape or a click beside the card: nothing is sent and the text stays in the composer. */
function dismissConsent(): void {
  showConsent.value = false
  pendingText.value = ''
  focusComposer()
}

function onCommandFailed(failure: unknown): void {
  error.value = errorText(failure)
}

const hasVoiceModel = computed(() => (projects.active?.models.voice ?? '') !== '')
/** The project's dictation language; empty legacy value shows as auto-detect. */
const dictationLanguage = computed(() => projects.active?.dictationLanguage || 'auto')

/** Persist the dictation language on the project so every take (and the label) uses it. */
async function onSetDictationLanguage(code: string): Promise<void> {
  const project = projects.active
  if (!project || code === project.dictationLanguage) {
    return
  }
  try {
    await projects.update(project.id, { dictationLanguage: code })
  } catch (failure) {
    error.value = errorText(failure)
  }
}

function onDictationError(failure: unknown): void {
  error.value = errorText(failure)
}
</script>

<template>
  <section class="chat" :class="{ dragging }">
    <ChatThreadList
      :threads="threads.threads.value"
      :current-id="threads.current.value?.id ?? null"
      :busy="sending"
      :collapsed="!historyOpen"
      @open="openThread"
      @new="onNewChat"
      @remove="removeThread"
      @toggle="toggleHistory"
    />

    <div class="chat-main">
      <ChatToolbar
        :project-label="projectName(projects.active)"
        :thread-title="threads.current.value?.title || ''"
        :bound-assistants="boundAssistants"
        :thread-assistant-id="threadAssistantId"
        :default-assistant-name="assistants.name(projects.active?.defaultAssistantId ?? null)"
        :show-assistant-chip="showAssistantChip"
        :assistant-in-force="assistantInForce"
        :assistant-label="assistantLabel"
        :sending="sending"
        :agent-mode="agentMode"
        :enabled-skill-count="enabledSkillCount"
        :has-chat-model="hasChatModel"
        :chat-model-label="chatModelLabel"
        :history-open="historyOpen"
        :chat-count="chatCount"
        :note-count="noteCount"
        :file-count="fileCount"
        :panel-tab="panelTab"
        :notes-bump="notesBump"
        :picking="picking"
        @open-project-settings="ui.setView('project')"
        @pick-assistant="onPickAssistant"
        @open-agents="ui.setView('agents')"
        @open-models="ui.setView('models')"
        @toggle-history="toggleHistory"
        @toggle-panel="togglePanel"
        @new-note="newNote"
        @add-files="addFiles"
      />

      <p v-if="!hasChatModel" class="banner banner-warn no-model" data-testid="chat-no-model">
        {{ t('chat.noChatModel') }}
        <button class="btn-link" type="button" @click="ui.setView('models')">
          {{ t('chat.pickModel') }} →
        </button>
      </p>

      <ChatThread
        ref="threadRef"
        :messages="messages"
        :show-studio="showStudio"
        :studio-cards="studioCards"
        :project-skills="projectSkills"
        :show-welcome="showWelcome"
        :project-label="projectName(projects.active)"
        :agent-mode="agentMode"
        :show-activity="showActivity"
        :activity-phase="activityPhase"
        :streaming-reply="streamingReply"
        @pick="applyTaskPrompt"
        @save="saveStudioTiles"
        @focus-composer="focusComposer"
        @add-files="addFiles"
        @new-note="newNote"
        @keep="keepMessage"
        @open-note="openKeptNote"
        @failed="onCommandFailed"
      />

      <div v-if="error || retryAttempt" class="chat-error-row">
        <p v-if="error" class="banner banner-error chat-error" role="alert">{{ error }}</p>
        <button
          v-if="retryAttempt && !sending"
          class="btn btn-ghost"
          type="button"
          data-testid="chat-resend"
          @click="stream.resend()"
        >
          {{ t('chat.resend') }}
        </button>
      </div>

      <ChatComposer
        ref="composerRef"
        v-model="input"
        :picking="picking"
        :web-search-on="webSearchOn"
        :web-search-busy="webSearchBusy"
        :project-id="projectId"
        :sending="sending"
        :has-chat-model="hasChatModel"
        :agent-mode="agentMode"
        :has-voice-model="hasVoiceModel"
        :dictation-language="dictationLanguage"
        :armed="composerArmed"
        :studio-open="showStudio"
        @send="send"
        @stop="stream.stop()"
        @add-files="addFiles"
        @note="onNoteButton"
        @toggle-web="toggleWebSearch"
        @update:language="onSetDictationLanguage"
        @dictation-error="onDictationError"
      />
    </div>

    <ProjectPanel
      v-if="panelTab"
      :tab="panelTab"
      :project="projects.active"
      :notes="notes"
      :knowledge="knowledge"
      @close="panelTab = null"
      @tab="openPanel"
      @add-files="addFiles"
    />

    <div v-if="dragging" class="drop-overlay" data-testid="chat-drop-overlay">
      <div class="drop-card">
        <span class="drop-icon" aria-hidden="true">📎</span>
        {{ t('chat.dropHere', { name: projectName(projects.active) }) }}
      </div>
    </div>

    <div
      v-if="showConsent"
      class="consent-overlay"
      role="dialog"
      aria-modal="true"
      aria-labelledby="chat-consent-title"
      aria-describedby="chat-consent-body"
      data-testid="chat-consent"
      @click.self="dismissConsent"
      @keydown.esc.stop="dismissConsent"
    >
      <div class="consent-card">
        <div class="consent-icon" aria-hidden="true">🔒</div>
        <h2 id="chat-consent-title" class="consent-title">{{ t('chat.consentTitle') }}</h2>
        <p id="chat-consent-body" class="consent-body">{{ t('chat.consentBody') }}</p>
        <ul class="consent-points">
          <li>{{ t('chat.consentPoint1') }}</li>
          <li>{{ t('chat.consentPoint2') }}</li>
          <li>{{ t('chat.consentPoint3') }}</li>
        </ul>
        <div class="consent-actions">
          <button
            ref="consentDeclineRef"
            class="btn btn-ghost"
            type="button"
            data-testid="chat-consent-not-now"
            @click="confirmConsent(false)"
          >
            {{ t('chat.consentNotNow') }}
          </button>
          <button
            class="btn btn-primary"
            type="button"
            data-testid="chat-consent-allow"
            @click="confirmConsent(true)"
          >
            {{ t('chat.consentAllow') }}
          </button>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped src="./chat-view.css"></style>
