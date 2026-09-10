<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Project } from '@/services/tauri'
import { useDictationLanguages } from '@/composables/useDictationLanguages'
import { useProjectName } from '@/composables/useProjectName'

/**
 * Create or rename a project. Rename only changes the display name; the folder
 * name and the knowledge folder stay the same (the copy says so).
 */
const props = defineProps<{
  mode: 'create' | 'rename'
  /** Project being renamed (rename) or the one models may be copied from (create). */
  project: Project | null
  busy: boolean
  error: string
}>()

const emit = defineEmits<{
  cancel: []
  submit: [payload: { name: string; dictationLanguage: string; copyModelsFrom: string | null }]
}>()

const { t } = useI18n()
const languages = useDictationLanguages()
const projectName = useProjectName()

const name = ref(props.mode === 'rename' ? (props.project?.name ?? '') : '')
const language = ref(
  props.mode === 'rename'
    ? (props.project?.dictationLanguage ?? languages.defaultCode.value)
    : languages.defaultCode.value,
)
const sourceHasModels = computed(() => {
  const m = props.project?.models
  return !!m && Object.values(m).some((v) => typeof v === 'string' && v.length > 0)
})
const copyModels = ref(sourceHasModels.value)
const nameInput = ref<HTMLInputElement | null>(null)

const canSubmit = computed(() => name.value.trim().length > 0 && !props.busy)
const title = computed(() =>
  props.mode === 'create'
    ? t('projects.createTitle')
    : t('projects.renameTitle', { name: projectName(props.project) }),
)

onMounted(() => nameInput.value?.focus())

function submit(): void {
  if (!canSubmit.value) {
    return
  }
  emit('submit', {
    name: name.value.trim(),
    dictationLanguage: language.value,
    copyModelsFrom:
      props.mode === 'create' && copyModels.value && props.project ? props.project.id : null,
  })
}
</script>

<template>
  <div class="consent-overlay" role="dialog" aria-modal="true" @click.self="emit('cancel')">
    <form class="consent-card" @submit.prevent="submit">
      <h2 class="consent-title">{{ title }}</h2>
      <p class="consent-body">
        {{ mode === 'create' ? t('projects.createBody') : t('projects.renameBody') }}
      </p>

      <label class="field">
        <span class="label">{{ t('projects.nameLabel') }}</span>
        <input
          ref="nameInput"
          v-model="name"
          class="input"
          type="text"
          maxlength="120"
          :placeholder="t('projects.namePlaceholder')"
          data-testid="project-name"
        />
      </label>

      <label class="field">
        <span class="label">{{ t('projects.dictationLanguage') }}</span>
        <select v-model="language" class="input" data-testid="project-language">
          <option v-for="opt in languages.options.value" :key="opt.code" :value="opt.code">
            {{ opt.label }}
          </option>
        </select>
        <span class="hint">{{ t('projects.dictationLanguageHint') }}</span>
      </label>

      <label v-if="mode === 'create' && sourceHasModels" class="check">
        <input v-model="copyModels" type="checkbox" data-testid="project-copy-models" />
        <span>{{ t('projects.copyModels', { project: projectName(project) }) }}</span>
      </label>
      <p v-else-if="mode === 'create'" class="hint">{{ t('projects.modelsLater') }}</p>

      <p v-if="error" class="banner banner-error" role="alert">{{ error }}</p>

      <div class="consent-actions">
        <button class="btn btn-ghost" type="button" :disabled="busy" @click="emit('cancel')">
          {{ t('common.cancel') }}
        </button>
        <button
          class="btn btn-primary"
          type="submit"
          :disabled="!canSubmit"
          data-testid="project-submit"
        >
          {{ mode === 'create' ? t('projects.create') : t('projects.rename') }}
        </button>
      </div>
    </form>
  </div>
</template>

<style scoped>
.consent-overlay {
  position: fixed;
  inset: 0;
  background: color-mix(in srgb, var(--bg) 70%, transparent);
  backdrop-filter: blur(3px);
  display: grid;
  place-items: center;
  padding: 1.5rem;
  z-index: 30;
}

.consent-card {
  max-width: 440px;
  width: 100%;
  max-height: 90%;
  overflow-y: auto;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: calc(var(--radius) * 1.4);
  padding: 1.6rem;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.28);
  margin: 0;
}

.consent-title {
  margin: 0 0 0.4rem;
  font-size: 1.15rem;
}

.consent-body {
  margin: 0 0 0.9rem;
  color: var(--txt-secondary);
  font-size: 0.9rem;
}

.hint {
  font-size: 0.78rem;
  color: var(--txt-secondary);
  line-height: 1.4;
}

.check {
  display: flex;
  align-items: flex-start;
  gap: 0.5rem;
  margin: 0 0 0.9rem;
  font-size: 0.88rem;
}

.check input {
  margin-top: 0.2rem;
}

.banner {
  margin: 0 0 0.9rem;
}

.consent-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
}
</style>
