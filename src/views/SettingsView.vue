<script setup lang="ts">
import { computed, onActivated, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import * as api from '@/services/tauri'
import type { DebugLogSettings, StorageInfo } from '@/services/tauri'
import { supportedLanguages, type SupportedLanguage, detectLocale } from '@/i18n'
import { useConfigStore } from '@/stores/config'
import { useUiStore } from '@/stores/ui'
import { useErrorText } from '@/composables/useErrorText'
import ConfirmDialog from '@/components/ConfirmDialog.vue'

/**
 * Settings: the global things — the Synaplan account this computer is connected
 * to, the interface language, where this install keeps things, and the debug
 * log. Anything tied to one project (its name, models, folders) lives in
 * Project config instead. Paths are shown platform-native and only ever
 * revealed — never built on in the webview.
 */
const { t, locale } = useI18n()
const config = useConfigStore()
const ui = useUiStore()
const errorText = useErrorText()

const storage = ref<StorageInfo | null>(null)
const error = ref('')
const dialog = ref<'signout' | null>(null)
const busy = ref(false)

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

// ---- debug log --------------------------------------------------------------

const debugLog = ref<DebugLogSettings | null>(null)
const debugBusy = ref(false)

async function loadDebugLog(): Promise<void> {
  try {
    debugLog.value = await api.getDebugLog()
  } catch {
    // The card simply stays hidden; nothing else depends on it.
    debugLog.value = null
  }
}

async function toggleDebugLog(): Promise<void> {
  if (!debugLog.value || debugBusy.value) {
    return
  }
  debugBusy.value = true
  error.value = ''
  try {
    debugLog.value = await api.setDebugLog(!debugLog.value.enabled)
  } catch (e) {
    error.value = errorText(e)
  } finally {
    debugBusy.value = false
  }
}

function load(): void {
  void loadStorage()
  void loadDebugLog()
}

onMounted(load)
onActivated(load)

function onLanguage(event: Event): void {
  const raw = (event.target as HTMLSelectElement).value
  const next = (supportedLanguages as readonly string[]).includes(raw)
    ? (raw as SupportedLanguage)
    : null
  void ui.setLanguage(next)
}

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
      <p class="muted sub">{{ t('settings.subtitle') }}</p>
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

      <!-- Debugging -->
      <div v-if="debugLog" class="card section" data-testid="settings-debug">
        <div class="section-title">{{ t('settings.debug') }}</div>
        <label class="switch-row">
          <input
            type="checkbox"
            :checked="debugLog.enabled"
            :disabled="debugBusy"
            data-testid="settings-debug-toggle"
            @change="toggleDebugLog"
          />
          <span class="switch-text">
            <span class="switch-title">{{ t('settings.debugToggle') }}</span>
            <span class="muted">{{ t('settings.debugHint') }}</span>
          </span>
        </label>
        <div class="kv debug-path">
          <span class="k muted">{{ t('settings.debugFile') }}</span>
          <span class="v">
            <code class="path" data-testid="settings-debug-path">{{ debugLog.path }}</code>
            <button
              class="btn-link"
              type="button"
              :disabled="!debugLog.enabled"
              :title="debugLog.enabled ? '' : t('settings.debugOffNote')"
              data-testid="settings-debug-reveal"
              @click="reveal(debugLog.path)"
            >
              {{ t('computer.reveal') }}
            </button>
          </span>
        </div>
        <p v-if="!debugLog.enabled" class="muted hint">{{ t('settings.debugOffNote') }}</p>
      </div>

      <!-- Where the project settings moved to -->
      <div class="card section" data-testid="settings-project-pointer">
        <div class="section-title">{{ t('settings.projectSettings') }}</div>
        <p class="muted hint">{{ t('settings.projectPointerHint') }}</p>
        <button
          class="btn btn-secondary"
          type="button"
          data-testid="settings-open-project"
          @click="ui.setView('project')"
        >
          {{ t('settings.openProjectConfig') }}
        </button>
      </div>
    </div>

    <Teleport to="body">
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

.switch-row {
  display: flex;
  align-items: flex-start;
  gap: 0.65rem;
  cursor: pointer;
  font-size: 0.88rem;
}

.switch-row input {
  margin-top: 0.2rem;
}

.switch-text {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.switch-title {
  font-weight: 600;
}

.debug-path {
  margin-top: 0.6rem;
}

.sub {
  margin: 0.15rem 0 0;
  font-size: 0.85rem;
}

.btn-link:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
