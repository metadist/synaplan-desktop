<script setup lang="ts">
import { computed, onMounted, onUnmounted, watch } from 'vue'
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
import SkillsView from '@/views/SkillsView.vue'
import ComputerView from '@/views/ComputerView.vue'
import DoctorView from '@/views/DoctorView.vue'

const { t } = useI18n()
const config = useConfigStore()
const projects = useProjectsStore()
const assistants = useAssistantsStore()
const ui = useUiStore()

let stopPoll: (() => void) | undefined

onMounted(() => config.load())

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
  },
)

onUnmounted(() => stopPoll?.())

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
    case 'skills':
      return SkillsView
    case 'computer':
      return ComputerView
    case 'doctor':
      return DoctorView
    default:
      return ChatView
  }
})
</script>

<template>
  <div v-if="config.loading" class="center muted">
    <span class="spinner"></span>
    <span>{{ t('common.loading') }}</span>
  </div>

  <PairView v-else-if="!config.paired" />

  <div v-else class="shell">
    <AppSidebar />
    <div class="content">
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
</style>
