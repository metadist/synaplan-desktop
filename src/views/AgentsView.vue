<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useProjectsStore } from '@/stores/projects'
import { useAssistantsStore } from '@/stores/assistants'
import { useUiStore } from '@/stores/ui'
import { useErrorText } from '@/composables/useErrorText'
import { useProjectName } from '@/composables/useProjectName'
import AssistantList from '@/components/AssistantList.vue'

const { t } = useI18n()
const projects = useProjectsStore()
const assistants = useAssistantsStore()
const ui = useUiStore()
const errorText = useErrorText()
const projectName = useProjectName()

const project = computed(() => projects.active)
const saving = ref(false)
const saveError = ref('')

onMounted(() => {
  void assistants.load()
})

async function onBindingChange(boundIds: number[], defaultId: number | null): Promise<void> {
  if (!project.value) {
    return
  }
  saving.value = true
  saveError.value = ''
  try {
    await projects.update(project.value.id, {
      assistantIds: boundIds,
      defaultAssistantId: defaultId,
    })
  } catch (e) {
    saveError.value = errorText(e)
  } finally {
    saving.value = false
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

    <div class="view-body">
      <p class="muted intro">{{ t('agents.intro') }}</p>

      <section v-if="project" class="block">
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
        <p v-if="saveError" class="banner banner-error" data-testid="assistants-save-error">
          {{ saveError }}
        </p>
      </section>

      <section v-if="project" class="block">
        <h2 class="block-title">{{ t('agents.skillsTitle') }}</h2>
        <p class="muted block-hint">{{ t('agents.emptyBody') }}</p>
        <button class="btn-link" type="button" @click="ui.setView('skills')">
          {{ t('agents.installSkills') }} →
        </button>
      </section>
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
  max-width: 720px;
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
