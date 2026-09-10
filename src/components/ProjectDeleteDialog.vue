<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Project } from '@/services/tauri'
import { useProjectName } from '@/composables/useProjectName'

/**
 * Danger confirm for deleting a project. Chats and settings are removed; the
 * notes folder is deleted only when the user opts in. Files already sent to
 * Synaplan are never touched from here — the copy says so.
 */
defineProps<{
  project: Project
  busy: boolean
  error: string
}>()

const emit = defineEmits<{
  cancel: []
  confirm: [removeFiles: boolean]
}>()

const { t } = useI18n()
const projectName = useProjectName()
const removeFiles = ref(false)
</script>

<template>
  <div class="consent-overlay" role="dialog" aria-modal="true" @click.self="emit('cancel')">
    <div class="consent-card">
      <h2 class="consent-title">{{ t('projects.deleteTitle', { name: projectName(project) }) }}</h2>
      <p class="consent-body">{{ t('projects.deleteBody') }}</p>

      <label class="check">
        <input v-model="removeFiles" type="checkbox" data-testid="project-delete-files" />
        <span>
          {{ t('projects.deleteNotesToo') }}
          <code class="path">{{ project.notesDir }}</code>
        </span>
      </label>

      <p class="consent-note">{{ t('projects.deleteSynaplanNote') }}</p>

      <p v-if="error" class="banner banner-error" role="alert">{{ error }}</p>

      <div class="consent-actions">
        <button class="btn btn-ghost" type="button" :disabled="busy" @click="emit('cancel')">
          {{ t('common.cancel') }}
        </button>
        <button
          class="btn btn-danger"
          type="button"
          :disabled="busy"
          data-testid="project-delete-confirm"
          @click="emit('confirm', removeFiles)"
        >
          {{ t('projects.delete') }}
        </button>
      </div>
    </div>
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

.check {
  display: flex;
  align-items: flex-start;
  gap: 0.5rem;
  margin: 0 0 0.8rem;
  font-size: 0.88rem;
}

.check input {
  margin-top: 0.2rem;
}

.path {
  display: block;
  margin-top: 0.2rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.74rem;
  color: var(--txt-secondary);
  overflow-wrap: anywhere;
}

.consent-note {
  margin: 0 0 0.9rem;
  font-size: 0.82rem;
  color: var(--txt-secondary);
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
