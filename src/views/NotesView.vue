<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useProjectsStore } from '@/stores/projects'
import { useProjectName } from '@/composables/useProjectName'
import * as api from '@/services/tauri'

const { t } = useI18n()
const projects = useProjectsStore()
const projectName = useProjectName()

const project = computed(() => projects.active)
</script>

<template>
  <section class="view">
    <header class="view-header">
      <div>
        <h1>{{ t('notes.title') }}</h1>
        <p class="muted subtitle">{{ projectName(project) }}</p>
      </div>
    </header>

    <div class="view-body">
      <div v-if="project" class="empty card">
        <h2 class="empty-title">{{ t('notes.emptyTitle') }}</h2>
        <p class="muted">{{ t('notes.emptyBody') }}</p>
        <code class="path">{{ project.notesDir }}</code>
        <div class="actions">
          <button class="btn btn-ghost" type="button" @click="api.revealPath(project.notesDir)">
            {{ t('notes.revealFolder') }}
          </button>
        </div>
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
.path {
  display: block;
  margin: 0.8rem 0;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.76rem;
  color: var(--txt-secondary);
  overflow-wrap: anywhere;
}
.actions {
  display: flex;
  gap: 0.5rem;
}
</style>
