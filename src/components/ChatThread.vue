<script setup lang="ts">
import { nextTick, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import * as api from '@/services/tauri'
import ChatActivity, { type ActivityPhase } from '@/components/ChatActivity.vue'
import MessageText from '@/components/MessageText.vue'
import TaskStudio from '@/components/TaskStudio.vue'
import type { TaskCard } from '@/composables/useTaskStudio'
import type { UiMessage } from '@/composables/useChatActivity'

defineProps<{
  messages: UiMessage[]
  showStudio: boolean
  studioCards: TaskCard[]
  projectSkills: api.Skill[]
  showWelcome: boolean
  researchAvailable?: boolean
  projectLabel: string
  agentMode: boolean
  showActivity: boolean
  activityPhase: ActivityPhase
  streamingReply: boolean
}>()

const emit = defineEmits<{
  pick: [card: TaskCard]
  save: [tiles: string[]]
  focusComposer: []
  addFiles: []
  newNote: []
  research: []
  keep: [index: number]
  openNote: [name: string]
  failed: [error: unknown]
}>()

const { t } = useI18n()
const copiedIndex = ref<number | null>(null)
const listEl = ref<HTMLElement | null>(null)
let copiedTimer: ReturnType<typeof setTimeout> | undefined

async function scrollToBottom(): Promise<void> {
  await nextTick()
  if (listEl.value) {
    listEl.value.scrollTop = listEl.value.scrollHeight
  }
}

async function copyMessage(index: number, content: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(content)
    copiedIndex.value = index
    clearTimeout(copiedTimer)
    copiedTimer = setTimeout(() => {
      copiedIndex.value = null
    }, 1500)
  } catch {
    // Clipboard unavailable; ignore.
  }
}

function previewSrc(artifact: api.ChatArtifact): string {
  return api.localFileUrl(artifact.path)
}

async function openArtifact(artifact: api.ChatArtifact): Promise<void> {
  try {
    await api.openPath(artifact.path)
  } catch (error) {
    emit('failed', error)
  }
}

async function revealFolder(path: string): Promise<void> {
  try {
    await api.revealPath(path)
  } catch (error) {
    emit('failed', error)
  }
}

onUnmounted(() => clearTimeout(copiedTimer))

defineExpose({ scrollToBottom })
</script>

<template>
  <div ref="listEl" class="messages">
    <TaskStudio
      v-if="showStudio"
      :cards="studioCards"
      :skills="projectSkills"
      @pick="emit('pick', $event)"
      @save="emit('save', $event)"
    />

    <div v-else-if="showWelcome" class="welcome" data-testid="chat-welcome">
      <div class="welcome-emoji" aria-hidden="true">👋</div>
      <h2 class="welcome-title">
        {{ t('chat.welcomeTitle', { name: projectLabel }) }}
      </h2>
      <p class="muted welcome-lead">
        {{ agentMode ? t('chat.emptySkills') : t('chat.welcomeLead') }}
      </p>
      <div class="welcome-actions">
        <button
          class="welcome-card"
          type="button"
          data-testid="welcome-ask"
          @click="emit('focusComposer')"
        >
          <span class="welcome-card-icon" aria-hidden="true">💬</span>
          <span class="welcome-card-title">{{ t('chat.welcomeAsk') }}</span>
          <span class="welcome-card-lead muted">{{ t('chat.welcomeAskLead') }}</span>
        </button>
        <button
          class="welcome-card"
          type="button"
          data-testid="welcome-files"
          @click="emit('addFiles')"
        >
          <span class="welcome-card-icon" aria-hidden="true">📎</span>
          <span class="welcome-card-title">{{ t('chat.welcomeFiles') }}</span>
          <span class="welcome-card-lead muted">{{ t('chat.welcomeFilesLead') }}</span>
        </button>
        <button
          class="welcome-card"
          type="button"
          data-testid="welcome-note"
          @click="emit('newNote')"
        >
          <span class="welcome-card-icon" aria-hidden="true">📝</span>
          <span class="welcome-card-title">{{ t('chat.welcomeNote') }}</span>
          <span class="welcome-card-lead muted">{{ t('chat.welcomeNoteLead') }}</span>
        </button>
        <button
          v-if="researchAvailable"
          class="welcome-card"
          type="button"
          data-testid="welcome-research"
          @click="emit('research')"
        >
          <span class="welcome-card-icon" aria-hidden="true">🔎</span>
          <span class="welcome-card-title">{{ t('research.tile') }}</span>
          <span class="welcome-card-lead muted">{{ t('research.tileLead') }}</span>
        </button>
      </div>
      <p class="muted welcome-foot">{{ t('chat.welcomeFoot') }}</p>
    </div>

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
        <div v-if="m.content" class="msg-actions">
          <button class="copy" type="button" @click="copyMessage(i, m.content)">
            {{ copiedIndex === i ? t('chat.copied') : t('chat.copy') }}
          </button>
          <button
            v-if="m.noteName"
            class="copy kept"
            type="button"
            :data-testid="`msg-kept-${i}`"
            @click="emit('openNote', m.noteName)"
          >
            {{ t('chat.kept') }}
          </button>
          <button
            v-else
            class="copy"
            type="button"
            :data-testid="`msg-keep-${i}`"
            @click="emit('keep', i)"
          >
            {{ t('chat.keep') }}
          </button>
        </div>
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
          <img
            v-if="a.kind === 'image' && previewSrc(a)"
            :src="previewSrc(a)"
            :alt="a.name"
            class="artifact-image"
          />
          <audio
            v-else-if="a.kind === 'audio' && previewSrc(a)"
            :src="previewSrc(a)"
            class="artifact-audio"
            controls
            @click.stop
          />
          <video
            v-else-if="a.kind === 'video' && previewSrc(a)"
            :src="previewSrc(a)"
            class="artifact-video"
            controls
            @click.stop
          />
          <span v-else class="artifact-icon" aria-hidden="true">📄</span>
          <span class="artifact-name">{{ a.name }}</span>
          <button class="btn-link artifact-reveal" type="button" @click.stop="revealFolder(a.path)">
            {{ t('chat.reveal') }}
          </button>
        </div>
      </div>
    </div>

    <div v-if="showActivity" class="msg assistant" data-testid="chat-working">
      <div v-if="!streamingReply" class="msg-role muted">{{ t('chat.assistant') }}</div>
      <ChatActivity :phase="activityPhase" :compact="streamingReply" />
    </div>
  </div>
</template>

<style scoped>
/* ---- welcome card for an empty project chat ---- */
.welcome {
  margin: auto;
  max-width: 620px;
  width: 100%;
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
}

.welcome-emoji {
  font-size: 2.2rem;
}

.welcome-title {
  font-size: 1.3rem;
}

.welcome-lead {
  margin: 0;
  max-width: 440px;
}

.welcome-actions {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(9.5rem, 1fr));
  gap: 0.7rem;
  width: 100%;
  margin-top: 0.8rem;
}

.welcome-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.3rem;
  padding: 1rem 0.8rem;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg-card);
  color: var(--txt);
  font: inherit;
  cursor: pointer;
  box-shadow: var(--shadow);
  transition:
    border-color 0.12s ease,
    transform 0.12s ease;
}

.welcome-card:hover {
  border-color: var(--accent);
  transform: translateY(-2px);
}

.welcome-card-icon {
  font-size: 1.6rem;
}

.welcome-card-title {
  font-weight: 650;
  font-size: 0.92rem;
  text-align: center;
  overflow-wrap: anywhere;
}

.welcome-card-lead {
  font-size: 0.78rem;
  text-align: center;
  overflow-wrap: anywhere;
}

.welcome-foot {
  margin: 0.8rem 0 0;
  font-size: 0.78rem;
}

@container (max-width: 420px) {
  .welcome-actions {
    grid-template-columns: 1fr;
  }
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

.msg-actions {
  display: flex;
  gap: 0.8rem;
}

.copy.kept {
  color: var(--accent);
  opacity: 1;
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

.artifact-image,
.artifact-video {
  max-width: min(480px, 100%);
  max-height: 360px;
  border-radius: var(--radius);
  object-fit: contain;
  background: var(--bg);
}

.artifact-audio {
  width: min(360px, 100%);
}

.artifact-card:has(.artifact-image),
.artifact-card:has(.artifact-video),
.artifact-card:has(.artifact-audio) {
  flex-wrap: wrap;
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
</style>
