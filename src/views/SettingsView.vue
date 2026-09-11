<script setup lang="ts">
import { computed, onActivated, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import * as api from '@/services/tauri'
import type { StorageInfo } from '@/services/tauri'
import { supportedLanguages, type SupportedLanguage, detectLocale } from '@/i18n'
import { useConfigStore } from '@/stores/config'
import { useProjectsStore } from '@/stores/projects'
import { useUiStore } from '@/stores/ui'
import { useErrorText } from '@/composables/useErrorText'
import { useProjectName } from '@/composables/useProjectName'
import { useDictationLanguages } from '@/composables/useDictationLanguages'
import ProjectFormDialog from '@/components/ProjectFormDialog.vue'
import ProjectDeleteDialog from '@/components/ProjectDeleteDialog.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'

/**
 * Settings: the Synaplan account this computer is connected to, the interface
 * language, where this install keeps things, and the current project's name,
 * dictation language and folders. Paths are shown platform-native and only
 * ever revealed — never built on in the webview.
 */
const { t, locale } = useI18n()
const config = useConfigStore()
const projects = useProjectsStore()
const ui = useUiStore()
const errorText = useErrorText()
const projectName = useProjectName()
const dictation = useDictationLanguages()

const storage = ref<StorageInfo | null>(null)
const error = ref('')
const dialog = ref<'rename' | 'delete' | 'signout' | null>(null)
const busy = ref(false)

const project = computed(() => projects.active)
const canDelete = computed(() => projects.projects.length > 1)

/** Language names in their own language, so everyone finds theirs. */
const languageOptions = computed(() => {
  let names: Intl.DisplayNames | null = null
  try {
    names = new Intl.DisplayNames([locale.value, 'en'], { type: 'language' })
  } catch {
    names = null
  }
  return supportedLanguages.map((code) => {
    let own = code as string
    try {
      own = new Intl.DisplayNames([code], { type: 'language' }).of(code) ?? code
    } catch {
      own = names?.of(code) ?? code
    }
    return { code, label: own.charAt(0).toUpperCase() + own.slice(1) }
  })
})

const systemLanguageLabel = computed(
  () => languageOptions.value.find((o) => o.code === detectLocale())?.label ?? detectLocale(),
)

async function loadStorage(): Promise<void> {
  try {
    storage.value = await api.getStorageInfo()
  } catch (e) {
    error.value = errorText(e)
  }
}

onMounted(loadStorage)
onActivated(loadStorage)

function onLanguage(event: Event): void {
  const raw = (event.target as HTMLSelectElement).value
  const next = (supportedLanguages as readonly string[]).includes(raw)
    ? (raw as SupportedLanguage)
    : null
  void ui.setLanguage(next)
}

function reveal(path: string | undefined): void {
  if (path) {
    void api.revealPath(path)
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

async function signOut(): Promise<void> {
  busy.value = true
  error.value = ''
  try {
    await config.signOut()
    dialog.value = null
  } catch (e) {
    error.value = errorText(e)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <section class="view" data-testid="settings-view">
    <header class="view-header">
      <h1>{{ t('settings.title') }}</h1>
    </header>

    <div class="view-body">
      <p class="muted intro">{{ t('settings.intro') }}</p>

      <p v-if="error && !dialog" class="banner banner-error" role="alert">{{ error }}</p>

      <!-- Account -->
      <div class="card section" data-testid="settings-account">
        <div class="section-title">{{ t('settings.account') }}</div>
        <div class="kv">
          <span class="k muted">{{ t('settings.workspace') }}</span>
          <span class="v">
            <span class="dot" :class="{ ok: config.paired }"></span>
            <code class="path">{{ config.apiBaseUrl }}</code>
          </span>
        </div>
        <div v-if="config.status?.deviceId" class="kv">
          <span class="k muted">{{ t('settings.device') }}</span>
          <span class="v">#{{ config.status.deviceId }}</span>
        </div>
        <div class="kv">
          <span class="k muted">{{ t('settings.key') }}</span>
          <span class="v">{{
            config.keyIsPlaintext ? t('settings.keyPlaintext') : t('settings.keySecure')
          }}</span>
        </div>
        <div class="actions">
          <button
            class="btn btn-secondary"
            type="button"
            :disabled="!config.apiBaseUrl"
            @click="api.openUrl(config.apiBaseUrl ?? '')"
          >
            {{ t('settings.openWorkspace') }}
          </button>
          <button
            class="btn btn-ghost"
            type="button"
            data-testid="settings-signout"
            @click="dialog = 'signout'"
          >
            {{ t('status.signOut') }}
          </button>
        </div>
      </div>

      <!-- Language -->
      <div class="card section" data-testid="settings-language">
        <div class="section-title">{{ t('settings.language') }}</div>
        <label class="field">
          <span class="muted">{{ t('settings.languageHint') }}</span>
          <select
            class="input"
            :value="ui.language ?? ''"
            data-testid="settings-language-select"
            @change="onLanguage"
          >
            <option value="">
              {{ t('settings.languageSystem', { name: systemLanguageLabel }) }}
            </option>
            <option v-for="opt in languageOptions" :key="opt.code" :value="opt.code">
              {{ opt.label }}
            </option>
          </select>
        </label>
      </div>

      <!-- Storage -->
      <div class="card section" data-testid="settings-storage">
        <div class="section-title">{{ t('settings.storage') }}</div>
        <p class="muted hint">{{ t('settings.storageHint') }}</p>
        <ul v-if="storage" class="folders">
          <li class="folder">
            <span class="folder-name">{{ t('settings.projectsFolder') }}</span>
            <code class="path">{{ storage.projectsDir }}</code>
            <button class="btn-link" type="button" @click="reveal(storage.projectsDir)">
              {{ t('computer.reveal') }}
            </button>
          </li>
          <li class="folder">
            <span class="folder-name">{{ t('settings.outboxFolder') }}</span>
            <code class="path">{{ storage.outboxDir }}</code>
            <button class="btn-link" type="button" @click="reveal(storage.outboxDir)">
              {{ t('computer.reveal') }}
            </button>
          </li>
          <li class="folder">
            <span class="folder-name">{{ t('settings.skillsFolder') }}</span>
            <code class="path">{{ storage.skillsDir }}</code>
            <button class="btn-link" type="button" @click="reveal(storage.skillsDir)">
              {{ t('computer.reveal') }}
            </button>
          </li>
          <li class="folder">
            <span class="folder-name">{{ t('settings.configFolder') }}</span>
            <code class="path">{{ storage.configDir }}</code>
            <button class="btn-link" type="button" @click="reveal(storage.configDir)">
              {{ t('computer.reveal') }}
            </button>
          </li>
        </ul>
        <button class="btn-link more" type="button" @click="ui.setView('computer')">
          {{ t('settings.openComputer') }} →
        </button>
      </div>

      <!-- This project -->
      <div v-if="project" class="card section" data-testid="settings-project">
        <div class="section-title">
          {{ t('settings.project', { name: projectName(project) }) }}
        </div>
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
            data-testid="settings-rename"
            @click="dialog = 'rename'"
          >
            {{ t('projects.renameAction') }}
          </button>
          <button class="btn btn-ghost" type="button" @click="ui.setView('models')">
            {{ t('settings.openModels') }}
          </button>
          <button class="btn btn-ghost" type="button" @click="ui.setView('agents')">
            {{ t('settings.openAgents') }}
          </button>
        </div>

        <p class="muted hint folders-hint">{{ t('settings.projectFoldersHint') }}</p>
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

        <div class="danger-row">
          <button
            class="btn-link danger"
            type="button"
            :disabled="!canDelete"
            :title="canDelete ? '' : t('projects.lastProject')"
            data-testid="settings-delete"
            @click="dialog = 'delete'"
          >
            {{ t('projects.deleteAction') }}
          </button>
        </div>
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
      <ConfirmDialog
        v-if="dialog === 'signout'"
        :title="t('settings.signOutTitle')"
        :body="t('settings.signOutBody', { url: config.apiBaseUrl ?? '' })"
        :confirm-label="t('status.signOut')"
        danger
        :busy="busy"
        :error="error"
        @cancel="dialog = null"
        @confirm="signOut"
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

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--border-strong);
  flex-shrink: 0;
}

.dot.ok {
  background: var(--ok);
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

.field {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  font-size: 0.85rem;
  max-width: 360px;
}

.hint {
  margin: 0 0 0.6rem;
  font-size: 0.82rem;
}

.folders-hint {
  margin-top: 0.9rem;
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

.more {
  margin-top: 0.6rem;
  font-size: 0.82rem;
}

.danger-row {
  margin-top: 0.8rem;
  padding-top: 0.6rem;
  border-top: 1px solid var(--border);
}

.btn-link.danger {
  color: var(--danger);
}

.btn-link:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
