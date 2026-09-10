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
const embedSet = computed(() => (project.value?.models.embed ?? '') !== '')
</script>

<template>
  <section class="view">
    <header class="view-header">
      <div>
        <h1>{{ t('files.title') }}</h1>
        <p class="muted subtitle">{{ projectName(project) }}</p>
      </div>
    </header>

    <div class="view-body">
      <div v-if="project" class="empty card">
        <h2 class="empty-title">{{ t('files.emptyTitle') }}</h2>
        <p class="muted">{{ t('files.emptyBody') }}</p>
        <p v-if="!embedSet" class="banner banner-warn">
          {{ t('files.embedUnset') }}
          <button class="btn-link" type="button" @click="ui.setView('models')">
            {{ t('files.openModels') }} →
          </button>
        </p>
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
.empty {
  max-width: 560px;
  padding: 1.4rem 1.5rem;
}
.empty-title {
  margin: 0 0 0.35rem;
  font-size: 1.05rem;
}
.banner {
  margin: 0.9rem 0 0;
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}
</style>
