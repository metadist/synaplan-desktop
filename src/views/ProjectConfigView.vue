<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import * as api from '@/services/tauri'
import { useProjectsStore } from '@/stores/projects'
import { useUiStore } from '@/stores/ui'
import { useErrorText } from '@/composables/useErrorText'
import { useProjectName } from '@/composables/useProjectName'
import { useDictationLanguages } from '@/composables/useDictationLanguages'
import ProjectFormDialog from '@/components/ProjectFormDialog.vue'
import ProjectDeleteDialog from '@/components/ProjectDeleteDialog.vue'

/**
 * Project config: everything that belongs to the current project — its name,
 * dictation language, the models and assistants that process its data, its
 * folders on this computer, and deleting it. Account, language and machine
 * storage are global and live in Settings instead. Paths are shown
 * platform-native and only ever revealed — never built on in the webview.
 */
const { t } = useI18n()
const projects = useProjectsStore()
const ui = useUiStore()
const errorText = useErrorText()
const projectName = useProjectName()
const dictation = useDictationLanguages()

const error = ref('')
const dialog = ref<'rename' | 'delete' | null>(null)
const busy = ref(false)

const project = computed(() => projects.active)
const canDelete = computed(() => projects.projects.length > 1)

async function reveal(path: string | undefined): Promise<void> {
  if (!path) {
    return
  }
  try {
    await api.revealPath(path)
  } catch (e) {
    error.value = errorText(e)
  }
}

async function rename(payload: { name: string; dictationLanguage: string }): Promise<void> {
  if (!project.value) {
    return
  }
  busy.value = true
  error.value = ''
  try {
    await projects.update(project.value.id, {
      name: payload.name,
      dictationLanguage: payload.dictationLanguage,
    })
    dialog.value = null
  } catch (e) {
    error.value = errorText(e)
  } finally {
    busy.value = false
  }
}

async function removeProject(removeFiles: boolean): Promise<void> {
  if (!project.value) {
    return
  }
  busy.value = true
  error.value = ''
  try {
    await projects.remove(project.value.id, removeFiles)
    dialog.value = null
    ui.setView('chat')
  } catch (e) {
    error.value = errorText(e)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <section class="view" data-testid="project-config-view">
    <header class="view-header">
      <h1>{{ t('projectConfig.title') }}</h1>
      <p v-if="project" class="muted sub">{{ projectName(project) }}</p>
    </header>

    <div class="view-body">
      <p class="muted intro">{{ t('projectConfig.intro') }}</p>

      <p v-if="error && !dialog" class="banner banner-error" role="alert">{{ error }}</p>

      <div v-if="project" class="card section" data-testid="project-config-basics">
        <div class="section-title">{{ t('projectConfig.basics') }}</div>
        <div class="kv">
          <span class="k muted">{{ t('projects.nameLabel') }}</span>
          <span class="v">{{ projectName(project) }}</span>
        </div>
        <div class="kv">
          <span class="k muted">{{ t('projects.dictationLanguage') }}</span>
          <span class="v">{{ dictation.labelFor(project.dictationLanguage) }}</span>
        </div>
        <div class="actions">
          <button
            class="btn btn-secondary"
            type="button"
            data-testid="project-config-rename"
            @click="dialog = 'rename'"
          >
            {{ t('projects.renameAction') }}
          </button>
        </div>
      </div>

      <!-- Which models and assistants process this project's data -->
      <div v-if="project" class="card section" data-testid="project-config-models">
        <div class="section-title">{{ t('projectConfig.processing') }}</div>
        <p class="muted hint">{{ t('projectConfig.processingHint') }}</p>
        <div class="actions">
          <button
            class="btn btn-secondary"
            type="button"
            data-testid="project-config-open-models"
            @click="ui.setView('models')"
          >
            {{ t('settings.openModels') }}
          </button>
          <button
            class="btn btn-ghost"
            type="button"
            data-testid="project-config-open-agents"
            @click="ui.setView('agents')"
          >
            {{ t('settings.openAgents') }}
          </button>
        </div>
      </div>

      <!-- Folders on this computer -->
      <div v-if="project" class="card section" data-testid="project-config-folders">
        <div class="section-title">{{ t('projectConfig.folders') }}</div>
        <p class="muted hint">{{ t('settings.projectFoldersHint') }}</p>
        <ul class="folders">
          <li class="folder">
            <span class="folder-name">{{ t('settings.projectFolder') }}</span>
            <code class="path">{{ project.projectDir }}</code>
            <button class="btn-link" type="button" @click="reveal(project.projectDir)">
              {{ t('computer.reveal') }}
            </button>
          </li>
          <li class="folder">
            <span class="folder-name">{{ t('nav.notes') }}</span>
            <code class="path">{{ project.notesDir }}</code>
            <button class="btn-link" type="button" @click="reveal(project.notesDir)">
              {{ t('computer.reveal') }}
            </button>
          </li>
          <li class="folder">
            <span class="folder-name">{{ t('settings.projectOut') }}</span>
            <code class="path">{{ project.outDir }}</code>
            <button class="btn-link" type="button" @click="reveal(project.outDir)">
              {{ t('computer.reveal') }}
            </button>
          </li>
        </ul>
        <p class="muted hint">
          {{ t('settings.projectFilesHint', { name: projectName(project) }) }}
        </p>
      </div>

      <!-- Delete this project -->
      <div v-if="project" class="card section" data-testid="project-config-danger">
        <div class="section-title">{{ t('projectConfig.danger') }}</div>
        <p class="muted hint">{{ t('projectConfig.dangerHint') }}</p>
        <button
          class="btn btn-danger"
          type="button"
          :disabled="!canDelete"
          :title="canDelete ? '' : t('projects.lastProject')"
          data-testid="project-config-delete"
          @click="dialog = 'delete'"
        >
          {{ t('projects.deleteAction') }}
        </button>
      </div>
    </div>

    <Teleport to="body">
      <ProjectFormDialog
        v-if="dialog === 'rename' && project"
        mode="rename"
        :project="project"
        :busy="busy"
        :error="error"
        @cancel="dialog = null"
        @submit="rename"
      />
      <ProjectDeleteDialog
        v-if="dialog === 'delete' && project"
        :project="project"
        :busy="busy"
        :error="error"
        @cancel="dialog = null"
        @confirm="removeProject"
      />
    </Teleport>
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
  padding: 0.9rem 1.2rem;
  border-bottom: 1px solid var(--border);
}

.sub {
  margin: 0.15rem 0 0;
  font-size: 0.85rem;
}

.view-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 1.1rem 1.2rem;
  max-width: 820px;
}

.intro {
  margin: 0 0 1rem;
  font-size: 0.9rem;
}

.section {
  padding: 0.9rem 1rem;
  margin-bottom: 0.9rem;
}

.section-title {
  font-size: 0.8rem;
  font-weight: 650;
  color: var(--txt-secondary);
  margin-bottom: 0.6rem;
}

.kv {
  display: grid;
  grid-template-columns: 150px 1fr;
  gap: 0.6rem;
  align-items: center;
  padding: 0.25rem 0;
  font-size: 0.9rem;
}

.k {
  font-size: 0.82rem;
}

.v {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  min-width: 0;
}

.path {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.8rem;
  overflow-wrap: anywhere;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  margin-top: 0.7rem;
}

.hint {
  margin: 0 0 0.6rem;
  font-size: 0.82rem;
}

.folders {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.folder {
  display: grid;
  grid-template-columns: 150px 1fr auto;
  gap: 0.6rem;
  align-items: center;
  font-size: 0.85rem;
}

.folder-name {
  font-weight: 550;
}
</style>
