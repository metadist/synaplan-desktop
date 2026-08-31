<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '@/stores/config'
import { useUiStore } from '@/stores/ui'
import AppSidebar from '@/components/AppSidebar.vue'
import PairView from '@/views/PairView.vue'
import ChatView from '@/views/ChatView.vue'
import SkillsView from '@/views/SkillsView.vue'
import ComputerView from '@/views/ComputerView.vue'
import DoctorView from '@/views/DoctorView.vue'

const { t } = useI18n()
const config = useConfigStore()
const ui = useUiStore()

onMounted(() => config.load())

const current = computed(() => {
  switch (ui.view) {
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
