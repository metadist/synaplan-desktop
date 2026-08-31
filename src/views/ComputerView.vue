<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import * as api from '@/services/tauri'
import { useErrorText } from '@/composables/useErrorText'
import { DOCS } from '@/constants'

const { t } = useI18n()
const errorText = useErrorText()

const policy = ref<api.FilesystemPolicy | null>(null)
const newFolder = ref('')
const error = ref('')
const busy = ref(false)

onMounted(load)

async function load(): Promise<void> {
  try {
    policy.value = await api.getFilesystemPolicy()
  } catch (e) {
    error.value = errorText(e)
  }
}

async function addFolder(): Promise<void> {
  const path = newFolder.value.trim()
  if (!path || busy.value) {
    return
  }
  error.value = ''
  busy.value = true
  try {
    policy.value = await api.addReadFolder(path)
    newFolder.value = ''
  } catch (e) {
    error.value = errorText(e)
  } finally {
    busy.value = false
  }
}

async function removeFolder(path: string): Promise<void> {
  error.value = ''
  try {
    policy.value = await api.removeReadFolder(path)
  } catch (e) {
    error.value = errorText(e)
  }
}

async function reveal(path: string): Promise<void> {
  try {
    await api.revealPath(path)
  } catch (e) {
    error.value = errorText(e)
  }
}
</script>

<template>
  <section class="view">
    <header class="view-header">
      <h1>{{ t('computer.title') }}</h1>
    </header>

    <div class="view-body">
      <p class="muted intro">{{ t('computer.intro') }}</p>

      <p v-if="error" class="banner banner-error" role="alert">{{ error }}</p>

      <div v-if="policy" class="card section">
        <div class="section-title">{{ t('computer.outboxLabel') }}</div>
        <div class="path-row">
          <code class="path">{{ policy.outbox }}</code>
          <button class="btn btn-ghost" type="button" @click="reveal(policy.outbox)">
            {{ t('computer.reveal') }}
          </button>
        </div>
      </div>

      <div v-if="policy" class="card section">
        <div class="section-title">{{ t('computer.readLabel') }}</div>
        <ul v-if="policy.read.length" class="folder-list">
          <li v-for="f in policy.read" :key="f" class="folder-row">
            <code class="path">{{ f }}</code>
            <span class="folder-actions">
              <button class="btn-link" type="button" @click="reveal(f)">
                {{ t('computer.reveal') }}
              </button>
              <button class="btn-link danger" type="button" @click="removeFolder(f)">
                {{ t('computer.remove') }}
              </button>
            </span>
          </li>
        </ul>
        <p v-else class="muted empty">{{ t('computer.emptyReadFolders') }}</p>

        <div class="add-row">
          <input
            v-model="newFolder"
            class="input"
            type="text"
            :placeholder="t('computer.folderPlaceholder')"
            @keydown.enter="addFolder"
          />
          <button
            class="btn btn-primary"
            type="button"
            :disabled="busy || !newFolder.trim()"
            @click="addFolder"
          >
            {{ t('computer.add') }}
          </button>
        </div>
      </div>

      <div v-if="policy" class="card section">
        <div class="section-title">{{ t('computer.denyLabel') }}</div>
        <div class="deny-list">
          <code v-for="d in policy.deny" :key="d" class="deny-item">{{ d }}</code>
        </div>
      </div>

      <button class="btn-link learn-more" type="button" @click="api.openUrl(DOCS.folders)">
        {{ t('common.learnMore') }} →
      </button>
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
  padding: 0.9rem 1.2rem;
  border-bottom: 1px solid var(--border);
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

.path {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.82rem;
  overflow-wrap: anywhere;
}

.path-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.6rem;
}

.folder-list {
  list-style: none;
  margin: 0 0 0.7rem;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.folder-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.6rem;
}

.folder-actions {
  display: inline-flex;
  gap: 0.75rem;
  flex-shrink: 0;
}

.btn-link.danger {
  color: var(--danger);
}

.empty {
  font-size: 0.85rem;
  margin: 0 0 0.7rem;
}

.add-row {
  display: flex;
  gap: 0.5rem;
}

.deny-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}

.deny-item {
  font-size: 0.72rem;
  padding: 0.15rem 0.4rem;
  border-radius: 5px;
  background: var(--bg-elevated);
  color: var(--txt-secondary);
}

.learn-more {
  font-size: 0.88rem;
  font-weight: 550;
}
</style>
