<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useProjectsStore } from '@/stores/projects'
import { useUiStore } from '@/stores/ui'
import { useProjectName } from '@/composables/useProjectName'

const { t } = useI18n()
const projects = useProjectsStore()
const ui = useUiStore()
const projectName = useProjectName()

const project = computed(() => projects.active)
</script>

<template>
  <section class="view">
    <header class="view-header">
      <div>
        <h1>{{ t('agents.title') }}</h1>
        <p class="muted subtitle">{{ projectName(project) }}</p>
      </div>
    </header>

    <div class="view-body">
      <p class="muted intro">{{ t('agents.intro') }}</p>
      <div v-if="project" class="empty card">
        <h2 class="empty-title">{{ t('agents.emptyTitle') }}</h2>
        <p class="muted">{{ t('agents.emptyBody') }}</p>
        <button class="btn-link" type="button" @click="ui.setView('skills')">
          {{ t('agents.installSkills') }} →
        </button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.view {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.view-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.9rem 1.2rem;
  border-bottom: 1px solid var(--border);
}
.subtitle {
  margin: 0.1rem 0 0;
  font-size: 0.82rem;
}
.view-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 1.1rem 1.2rem;
}
.intro {
  margin: 0 0 1rem;
  font-size: 0.9rem;
}
.empty {
  max-width: 560px;
  padding: 1.4rem 1.5rem;
}
.empty-title {
  margin: 0 0 0.35rem;
  font-size: 1.05rem;
}
</style>
