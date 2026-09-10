<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import * as api from '@/services/tauri'
import { useProjectsStore } from '@/stores/projects'
import { useAssistantsStore } from '@/stores/assistants'
import { useUiStore } from '@/stores/ui'
import { useErrorText } from '@/composables/useErrorText'
import { useKnowledgeFiles } from '@/composables/useKnowledgeFiles'
import { useProjectName } from '@/composables/useProjectName'
import AssistantList from '@/components/AssistantList.vue'
import InOutBoard from '@/components/InOutBoard.vue'
import ProjectSkillList from '@/components/ProjectSkillList.vue'

const { t } = useI18n()
const projects = useProjectsStore()
const assistants = useAssistantsStore()
const ui = useUiStore()
const errorText = useErrorText()
const projectName = useProjectName()

const project = computed(() => projects.active)
const projectId = computed(() => project.value?.id ?? '')
const saving = ref(false)
const saveError = ref('')

const skills = ref<api.Skill[]>([])
const skillsError = ref('')

const knowledge = useKnowledgeFiles(projectId)
const outFiles = ref<api.OutFile[]>([])
const outLoading = ref(false)

onMounted(async () => {
  void assistants.load()
  await Promise.all([loadSkills(), knowledge.refresh(), loadOut()])
})

watch(projectId, () => {
  outFiles.value = []
  void loadOut()
})

async function loadSkills(): Promise<void> {
  try {
    skills.value = await api.listSkills()
    skillsError.value = ''
  } catch (e) {
    skills.value = []
    skillsError.value = errorText(e)
  }
}

async function loadOut(): Promise<void> {
  const id = projectId.value
  if (!id) {
    return
  }
  outLoading.value = true
  try {
    const list = await api.listOutFiles(id)
    if (projectId.value === id) {
      outFiles.value = list
    }
  } catch {
    if (projectId.value === id) {
      outFiles.value = []
    }
  } finally {
    outLoading.value = false
  }
}

async function patch(update: api.ProjectPatch): Promise<void> {
  if (!project.value) {
    return
  }
  saving.value = true
  saveError.value = ''
  try {
    await projects.update(project.value.id, update)
  } catch (e) {
    saveError.value = errorText(e)
  } finally {
    saving.value = false
  }
}

function onBindingChange(boundIds: number[], defaultId: number | null): void {
  void patch({ assistantIds: boundIds, defaultAssistantId: defaultId })
}

function onSkillsChange(enabledHere: string[]): void {
  void patch({ enabledSkills: enabledHere })
}

async function reveal(path: string): Promise<void> {
  try {
    await api.revealPath(path)
  } catch {
    // The folder may have been removed meanwhile; nothing to report.
  }
}
</script>

<template>
  <section class="view">
    <header class="view-header">
      <div>
        <h1>{{ t('agents.title') }}</h1>
        <p class="muted subtitle">{{ projectName(project) }}</p>
      </div>
    </header>

    <div v-if="project" class="view-body">
      <p class="muted intro">{{ t('agents.intro') }}</p>

      <section class="block">
        <h2 class="block-title">{{ t('inout.title') }}</h2>
        <p class="muted block-hint">{{ t('inout.hint') }}</p>
        <InOutBoard
          :files="knowledge.files.value"
          :files-loading="knowledge.loading.value"
          :out-files="outFiles"
          :out-loading="outLoading"
          @open-files="ui.setView('files')"
          @reveal="reveal"
          @reveal-out="reveal(project.outDir)"
        />
      </section>

      <section class="block">
        <div class="block-head">
          <h2 class="block-title">{{ t('assistants.title') }}</h2>
          <button
            v-if="assistants.state !== 'loading'"
            class="btn btn-ghost small"
            type="button"
            data-testid="assistants-refresh"
            @click="assistants.load(true)"
          >
            {{ t('common.retry') }}
          </button>
        </div>
        <p class="muted block-hint">{{ t('assistants.hint') }}</p>

        <p v-if="assistants.state === 'loading'" class="muted" data-testid="assistants-loading">
          <span class="spinner small"></span> {{ t('assistants.loading') }}
        </p>
        <p
          v-else-if="assistants.state === 'disabled'"
          class="banner banner-warn"
          data-testid="assistants-disabled"
        >
          {{ t('assistants.disabled') }}
        </p>
        <p
          v-else-if="assistants.state === 'error'"
          class="banner banner-error"
          data-testid="assistants-error"
        >
          {{ errorText(assistants.error) }}
        </p>
        <p v-else-if="assistants.list.length === 0" class="muted" data-testid="assistants-empty">
          {{ t('assistants.none') }}
        </p>
        <AssistantList
          v-else
          :assistants="assistants.list"
          :bound-ids="project.assistantIds"
          :default-id="project.defaultAssistantId"
          :models="project.models"
          :busy="saving"
          @change="onBindingChange"
        />
      </section>

      <section class="block">
        <div class="block-head">
          <h2 class="block-title">{{ t('projectSkills.title') }}</h2>
          <button class="btn-link" type="button" @click="ui.setView('skills')">
            {{ t('agents.installSkills') }} →
          </button>
        </div>
        <p class="muted block-hint">{{ t('projectSkills.hint') }}</p>
        <p v-if="skillsError" class="banner banner-error">{{ skillsError }}</p>
        <p v-else-if="skills.length === 0" class="muted" data-testid="project-skills-empty">
          {{ t('projectSkills.none') }}
        </p>
        <ProjectSkillList
          v-else
          :skills="skills"
          :enabled-here="project.enabledSkills"
          :busy="saving"
          @change="onSkillsChange"
          @open-skills="ui.setView('skills')"
        />
      </section>

      <p v-if="saveError" class="banner banner-error" data-testid="agents-save-error">
        {{ saveError }}
      </p>
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
.block {
  max-width: 760px;
  margin-bottom: 1.6rem;
}
.block-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.block-title {
  margin: 0;
  font-size: 1rem;
}
.block-hint {
  margin: 0.2rem 0 0.7rem;
  font-size: 0.82rem;
}
.small {
  padding: 0.3rem 0.65rem;
  font-size: 0.78rem;
}
.spinner.small {
  width: 12px;
  height: 12px;
  vertical-align: middle;
}
</style>
