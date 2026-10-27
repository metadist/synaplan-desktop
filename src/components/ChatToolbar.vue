<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { Assistant } from '@/services/tauri'
import type { PanelTab } from '@/components/ProjectPanel.vue'

defineProps<{
  projectLabel: string
  threadTitle: string
  boundAssistants: Assistant[]
  threadAssistantId: number | null
  defaultAssistantName: string
  showAssistantChip: boolean
  assistantInForce: number | null
  assistantLabel: string
  sending: boolean
  agentMode: boolean
  enabledSkillCount: number
  hasChatModel: boolean
  chatModelLabel: string
  historyOpen: boolean
  chatCount: number
  noteCount: number
  fileCount: number
  panelTab: PanelTab | null
  notesBump: boolean
  picking: boolean
}>()

const emit = defineEmits<{
  openProjectSettings: []
  pickAssistant: [event: Event]
  openAgents: []
  openModels: []
  toggleHistory: []
  togglePanel: [tab: PanelTab]
  newNote: []
  addFiles: []
}>()

const { t } = useI18n()
</script>

<template>
  <header class="chat-toolbar">
    <div class="heading">
      <div class="title-row">
        <h1 class="title">{{ projectLabel }}</h1>
        <button
          class="gear"
          type="button"
          :title="t('settings.projectSettings')"
          :aria-label="t('settings.projectSettings')"
          data-testid="chat-project-settings"
          @click="emit('openProjectSettings')"
        >
          ⚙
        </button>
      </div>
      <p class="muted subtitle">
        {{ threadTitle || t('chat.newChat') }}
      </p>
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
          @change="emit('pickAssistant', $event)"
        >
          <option value="">
            {{
              t('chat.assistantDefault', {
                name: defaultAssistantName || t('assistants.noneShort'),
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
        @click="emit('openAgents')"
      >
        <span class="muted model-label">{{ t('assistants.one') }}</span>
        <span class="model-name">{{ assistantLabel }}</span>
      </button>
      <span v-if="agentMode" class="skills-pill" :title="t('chat.skillsActiveHint')">
        <span class="dot"></span>
        {{ t('chat.skillsActive', { count: enabledSkillCount }, enabledSkillCount) }}
      </span>
      <button
        class="model-chip"
        type="button"
        :class="{ unset: !hasChatModel }"
        :title="t('chat.modelHint')"
        data-testid="chat-model-chip"
        @click="emit('openModels')"
      >
        <span class="muted model-label">{{ t('chat.modelLabel') }}</span>
        <span class="model-name">{{ hasChatModel ? chatModelLabel : t('models.unset') }}</span>
      </button>
    </div>
  </header>

  <!-- What belongs to this project, one click away -->
  <div class="project-bar" data-testid="project-bar">
    <button
      class="pill"
      :class="{ active: historyOpen }"
      type="button"
      :title="t('chat.pillChatsHint')"
      data-testid="pill-chats"
      @click="emit('toggleHistory')"
    >
      <span class="pill-icon" aria-hidden="true">💬</span>
      {{ t('chat.pillChats', { count: chatCount }, chatCount) }}
    </button>
    <button
      class="pill"
      :class="{ active: panelTab === 'notes', bump: notesBump }"
      type="button"
      :title="t('chat.pillNotesHint')"
      data-testid="pill-notes"
      @click="emit('togglePanel', 'notes')"
    >
      <span class="pill-icon" aria-hidden="true">📝</span>
      {{ t('chat.pillNotes', { count: noteCount }, noteCount) }}
    </button>
    <button
      class="pill"
      :class="{ active: panelTab === 'files' }"
      type="button"
      :title="t('chat.pillFilesHint')"
      data-testid="pill-files"
      @click="emit('togglePanel', 'files')"
    >
      <span class="pill-icon" aria-hidden="true">📎</span>
      {{ t('chat.pillFiles', { count: fileCount }, fileCount) }}
    </button>
    <span class="bar-spacer"></span>
    <button
      class="pill pill-action"
      type="button"
      data-testid="bar-new-note"
      @click="emit('newNote')"
    >
      + {{ t('notes.newNote') }}
    </button>
    <button
      class="pill pill-action"
      type="button"
      :disabled="picking"
      data-testid="bar-add-files"
      @click="emit('addFiles')"
    >
      + {{ t('panel.addFiles') }}
    </button>
  </div>
</template>

<style scoped>
.chat-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 0.55rem 1rem;
  padding: 0.7rem 1.2rem;
  border-bottom: 1px solid var(--border);
}

.title {
  font-size: 1rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 100%;
}

.subtitle {
  margin: 0.05rem 0 0;
  font-size: 0.78rem;
}

.title-row {
  display: flex;
  align-items: center;
  gap: 0.35rem;
}

.gear {
  border: none;
  background: transparent;
  color: var(--txt-secondary);
  font: inherit;
  font-size: 0.95rem;
  line-height: 1;
  padding: 0.1rem 0.3rem;
  border-radius: 6px;
  cursor: pointer;
  opacity: 0.6;
}

.gear:hover {
  opacity: 1;
  background: var(--bg-elevated);
  color: var(--txt);
}

.toolbar-right {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: flex-end;
  gap: 0.45rem 0.8rem;
  flex: 1 1 16rem;
  min-width: 0;
}

.heading {
  flex: 1 1 12rem;
  min-width: 0;
}

/* ---- project bar: chats / notes / files at a glance ---- */
.project-bar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.4rem;
  padding: 0.5rem 1.2rem;
  border-bottom: 1px solid var(--border);
  background: var(--bg-card);
}

.bar-spacer {
  flex: 1;
}

.pill {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.28rem 0.7rem;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--bg);
  color: var(--txt);
  font: inherit;
  font-size: 0.8rem;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  transition:
    border-color 0.12s ease,
    background 0.12s ease;
}

.pill:hover {
  border-color: var(--accent);
}

.pill.active {
  background: var(--accent-soft);
  border-color: var(--accent);
  color: var(--accent);
}

.pill.bump {
  animation: pill-bump 0.5s ease;
}

@keyframes pill-bump {
  0% {
    transform: scale(1);
  }
  35% {
    transform: scale(1.12);
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  100% {
    transform: scale(1);
  }
}

.pill-action {
  background: transparent;
  border-style: dashed;
  color: var(--txt-secondary);
}

.pill-action:hover {
  color: var(--accent);
}

.pill-icon {
  font-size: 0.9rem;
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
  white-space: nowrap;
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
  max-width: min(280px, 100%);
  min-width: 0;
  flex-shrink: 1;
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
</style>
