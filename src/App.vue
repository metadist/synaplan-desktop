<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '@/stores/config'
import { useProjectsStore } from '@/stores/projects'
import { useAssistantsStore } from '@/stores/assistants'
import { useUiStore } from '@/stores/ui'
import * as api from '@/services/tauri'
import AppSidebar from '@/components/AppSidebar.vue'
import PairView from '@/views/PairView.vue'
import ChatView from '@/views/ChatView.vue'
import NotesView from '@/views/NotesView.vue'
import FilesView from '@/views/FilesView.vue'
import AgentsView from '@/views/AgentsView.vue'
import ModelsView from '@/views/ModelsView.vue'
import ProjectConfigView from '@/views/ProjectConfigView.vue'
import SkillsView from '@/views/SkillsView.vue'
import ComputerView from '@/views/ComputerView.vue'
import DoctorView from '@/views/DoctorView.vue'
import SettingsView from '@/views/SettingsView.vue'

const { t, locale } = useI18n()
const config = useConfigStore()
const projects = useProjectsStore()
const assistants = useAssistantsStore()
const ui = useUiStore()

let stopPoll: (() => void) | undefined
let stopRevoked: (() => void) | undefined
let stopClose: (() => void) | undefined
const closeAsk = ref(false)

function pushTrayCopy(): void {
  void api.setTrayCopy({
    connected: t('tray.connected'),
    notConnected: t('tray.notConnected'),
    lastCheckin: t('tray.lastCheckin'),
    noCheckin: t('tray.noCheckin'),
    noJobs: t('tray.noJobs'),
    jobsWaiting: t('tray.jobsWaiting'),
    quit: t('tray.quit'),
  })
}

async function keepRunning(): Promise<void> {
  closeAsk.value = false
  await ui.setCloseHides(true)
  await api.hideMainWindow()
}

async function quitApp(): Promise<void> {
  await api.quitApp()
}

onMounted(() => {
  void ui.loadPrefs()
  void config.load()
  void api
    .onPairingRevoked(() => {
      config.markRevoked()
      void config.refresh()
    })
    .then((unlisten) => {
      stopRevoked = unlisten
    })
  void api
    .onCloseRequested(() => {
      closeAsk.value = true
    })
    .then((unlisten) => {
      stopClose = unlisten
    })
})

watch(locale, () => pushTrayCopy(), { immediate: true })

watch(
  () => config.paired,
  async (paired) => {
    stopPoll?.()
    stopPoll = undefined
    if (!paired) {
      projects.reset()
      assistants.reset()
      return
    }
    // Projects live on this computer; the list is loaded once the shell shows.
    if (!projects.loaded) {
      try {
        await projects.load()
      } catch {
        // The switcher shows the error on its own next action.
      }
    }
    await config.loadPoll()
    stopPoll = await api.onPollStatus((status) => config.setPollStatus(status))
    if (config.pollStatus?.lastErrorCode === 'unauthorized') {
      config.markRevoked()
      await config.refresh()
    }
  },
)

watch(
  () => config.pollStatus?.lastErrorCode,
  (code) => {
    if (code === 'unauthorized' && config.paired) {
      config.markRevoked()
      void config.refresh()
    }
  },
)

onUnmounted(() => {
  stopPoll?.()
  stopRevoked?.()
  stopClose?.()
})

const current = computed(() => {
  switch (ui.view) {
    case 'notes':
      return NotesView
    case 'files':
      return FilesView
    case 'agents':
      return AgentsView
    case 'models':
      return ModelsView
    case 'project':
      return ProjectConfigView
    case 'skills':
      return SkillsView
    case 'computer':
      return ComputerView
    case 'doctor':
      return DoctorView
    case 'settings':
      return SettingsView
    default:
      return ChatView
  }
})
</script>

<template>
  <div
    v-if="closeAsk"
    class="close-ask"
    role="dialog"
    aria-modal="true"
    aria-labelledby="close-ask-title"
  >
    <div class="card close-card">
      <h2 id="close-ask-title">{{ t('computer.backgroundTitle') }}</h2>
      <p>{{ t('computer.backgroundBody') }}</p>
      <div class="close-actions">
        <button class="btn btn-ghost" type="button" @click="quitApp">
          {{ t('computer.backgroundQuit') }}
        </button>
        <button class="btn btn-primary" type="button" @click="keepRunning">
          {{ t('computer.backgroundKeep') }}
        </button>
      </div>
    </div>
  </div>

  <div v-if="config.loading" class="center muted">
    <span class="spinner"></span>
    <span>{{ t('common.loading') }}</span>
  </div>

  <PairView v-else-if="!config.paired" />

  <div v-else class="shell">
    <AppSidebar />
    <div class="content">
      <p v-if="config.pairNotice" class="banner banner-warn pair-notice">
        {{ t('pair.confirmed', { who: config.pairNotice }) }}
        <button class="btn-link" type="button" @click="config.dismissPairNotice()">
          {{ t('common.close') }}
        </button>
      </p>
      <p v-if="config.keyIsPlaintext" class="banner banner-warn plaintext">
        {{ t('status.plaintextWarning') }}
      </p>
      <keep-alive>
        <component :is="current" />
      </keep-alive>
    </div>
  </div>
</template>

<style scoped>
.shell {
  flex: 1;
  min-height: 0;
  display: flex;
}

.content {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  position: relative;
}

.plaintext {
  margin: 0.6rem 0.9rem 0;
}

.center {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.6rem;
}

.close-ask {
  position: fixed;
  inset: 0;
  z-index: 40;
  display: grid;
  place-items: center;
  background: color-mix(in srgb, var(--bg) 55%, transparent);
  padding: 1rem;
}

.close-card {
  max-width: 28rem;
  padding: 1.1rem 1.2rem;
}

.close-card h2 {
  margin: 0 0 0.4rem;
  font-size: 1.05rem;
}

.close-card p {
  margin: 0 0 0.9rem;
}

.close-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
}
</style>
