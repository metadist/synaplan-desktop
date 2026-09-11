<script setup lang="ts">
import { computed, onActivated, onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useProjectsStore } from '@/stores/projects'
import { useUiStore } from '@/stores/ui'
import { useProjectName } from '@/composables/useProjectName'
import { useKnowledgeFiles } from '@/composables/useKnowledgeFiles'
import { useErrorText } from '@/composables/useErrorText'
import * as api from '@/services/tauri'
import type { KnowledgeFile } from '@/services/tauri'
import type { UnlistenFn } from '@tauri-apps/api/event'
import KnowledgeFileList from '@/components/KnowledgeFileList.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'

/**
 * The project's knowledge folder: files dropped here leave this computer, land
 * in the paired workspace and are indexed with the workspace search model
 * (DEFAULTMODEL.VECTORIZE). Project Embed is not required and is not sent.
 */
const { t } = useI18n()
const projects = useProjectsStore()
const ui = useUiStore()
const projectName = useProjectName()
const errorText = useErrorText()

const project = computed(() => projects.active)
const projectId = computed(() => project.value?.id ?? '')
const knowledge = useKnowledgeFiles(projectId)
void knowledge.refresh()
// Files can also be added from the Chat view; pick those up on return.
onActivated(() => void knowledge.refresh())

const dragging = ref(false)
const typedPath = ref('')
const confirmRemove = ref<KnowledgeFile | null>(null)
const removing = ref(false)

let unlistenDrop: UnlistenFn | null = null

onMounted(async () => {
  unlistenDrop = await api.onFileDrop((event) => {
    // The view stays alive behind other pages; only the page on screen takes the drop.
    if (ui.view !== 'files') {
      dragging.value = false
      return
    }
    if (event.type === 'enter' || event.type === 'over') {
      dragging.value = true
    } else if (event.type === 'leave') {
      dragging.value = false
    } else {
      dragging.value = false
      void knowledge.add(event.paths)
    }
  })
})

onUnmounted(() => {
  unlistenDrop?.()
})

function addTyped(): void {
  const path = typedPath.value.trim()
  if (!path) {
    return
  }
  typedPath.value = ''
  void knowledge.add([path])
}

const picking = ref(false)

async function pick(): Promise<void> {
  if (picking.value) {
    return
  }
  picking.value = true
  try {
    const paths = await api.pickFiles(t('files.pickTitle'))
    if (paths.length > 0) {
      void knowledge.add(paths)
    }
  } finally {
    picking.value = false
  }
}

async function performRemove(): Promise<void> {
  const target = confirmRemove.value
  if (!target) {
    return
  }
  removing.value = true
  try {
    await knowledge.remove(target.id)
  } finally {
    removing.value = false
    confirmRemove.value = null
  }
}

function pendingErrorText(err: unknown): string {
  const code = api.asCommandError(err).code
  if (code === 'file_outside_allowed') {
    return t('files.outsideAllowed')
  }
  return errorText(err)
}

function isOutsideAllowed(err: unknown): boolean {
  return api.asCommandError(err).code === 'file_outside_allowed'
}
</script>

<template>
  <section class="view">
    <header class="view-header">
      <div>
        <h1>{{ t('files.title') }}</h1>
        <p class="muted subtitle">{{ projectName(project) }}</p>
      </div>
      <p class="muted index-note" data-testid="files-index-model">
        {{ t('files.indexedWith') }}
      </p>
    </header>

    <div class="view-body">
      <div class="dropzone card" :class="{ active: dragging }" data-testid="files-dropzone">
        <p class="drop-title">{{ t('files.dropHere') }}</p>
        <p class="muted drop-body">{{ t('files.emptyBody') }}</p>
        <button
          class="btn btn-primary pick-btn"
          type="button"
          :disabled="picking"
          data-testid="files-pick"
          @click="pick"
        >
          {{ t('files.pickFiles') }}
        </button>
        <form class="path-row" @submit.prevent="addTyped">
          <input
            v-model="typedPath"
            class="input path-input"
            type="text"
            :placeholder="t('files.pathPlaceholder')"
            data-testid="files-path"
          />
          <button
            class="btn btn-ghost"
            type="submit"
            :disabled="typedPath.trim() === ''"
            data-testid="files-add"
          >
            {{ t('files.add') }}
          </button>
        </form>
        <p class="muted allow-note">
          {{ t('files.allowedFoldersNote') }}
          <button class="btn-link" type="button" @click="ui.setView('computer')">
            {{ t('nav.computer') }} →
          </button>
        </p>
      </div>

      <ul v-if="knowledge.pending.value.length" class="pending" data-testid="files-pending">
        <li
          v-for="row in knowledge.pending.value"
          :key="row.path"
          class="pending-row card"
          :class="{ failed: row.error !== null }"
        >
          <span class="pending-name">{{ row.name }}</span>
          <span v-if="row.error === null" class="muted">
            <span class="spinner small"></span> {{ t('files.sending') }}
          </span>
          <span v-else class="pending-error">
            {{ pendingErrorText(row.error) }}
            <button
              v-if="isOutsideAllowed(row.error)"
              class="btn-link"
              type="button"
              @click="ui.setView('computer')"
            >
              {{ t('nav.computer') }} →
            </button>
            <button class="btn-link dismiss" type="button" @click="knowledge.dismiss(row.path)">
              {{ t('files.dismiss') }}
            </button>
          </span>
        </li>
      </ul>

      <p v-if="knowledge.error.value" class="banner banner-error notice" data-testid="files-error">
        {{ errorText(knowledge.error.value) }}
        <button class="btn-link" type="button" @click="knowledge.refresh()">
          {{ t('common.retry') }}
        </button>
      </p>

      <KnowledgeFileList
        :files="knowledge.files.value"
        :loading="knowledge.loading.value"
        @remove="confirmRemove = $event"
        @refresh="knowledge.refresh()"
      />
    </div>

    <ConfirmDialog
      v-if="confirmRemove"
      :title="t('files.removeTitle', { name: confirmRemove.name })"
      :body="t('files.removeBody')"
      :confirm-label="t('files.remove')"
      danger
      :busy="removing"
      @cancel="confirmRemove = null"
      @confirm="performRemove"
    />
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
.index-note {
  margin: 0;
  font-size: 0.78rem;
  text-align: right;
  max-width: 320px;
}
.view-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 1.1rem 1.2rem;
  display: flex;
  flex-direction: column;
  gap: 0.9rem;
}
.notice {
  max-width: 720px;
  margin: 0;
}
.dropzone {
  max-width: 720px;
  padding: 1.2rem 1.4rem;
  border: 1px dashed var(--border-strong);
  box-shadow: none;
  transition:
    border-color 0.12s ease,
    background 0.12s ease;
}
.dropzone.active {
  border-color: var(--accent);
  background: var(--accent-soft);
}
.drop-title {
  margin: 0 0 0.3rem;
  font-weight: 650;
}
.drop-body {
  margin: 0 0 0.8rem;
  font-size: 0.88rem;
}
.path-row {
  display: flex;
  gap: 0.5rem;
}
.pick-btn {
  align-self: center;
  margin-bottom: 0.75rem;
}

.path-input {
  flex: 1;
  min-width: 0;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.82rem;
}
.allow-note {
  margin: 0.6rem 0 0;
  font-size: 0.78rem;
}
.pending {
  list-style: none;
  margin: 0;
  padding: 0;
  max-width: 720px;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}
.pending-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.55rem 0.9rem;
  font-size: 0.85rem;
}
.pending-row.failed {
  border-color: color-mix(in srgb, var(--danger) 40%, transparent);
}
.pending-name {
  font-weight: 550;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.pending-error {
  color: var(--danger);
  text-align: right;
}
.dismiss {
  margin-left: 0.6rem;
  color: var(--txt-secondary);
}
.spinner.small {
  width: 12px;
  height: 12px;
  vertical-align: -2px;
}
</style>
