<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Project } from '@/services/tauri'
import { useProjectsStore } from '@/stores/projects'
import { useUiStore } from '@/stores/ui'
import { useErrorText } from '@/composables/useErrorText'
import { useProjectName } from '@/composables/useProjectName'
import ProjectFormDialog from '@/components/ProjectFormDialog.vue'
import ProjectDeleteDialog from '@/components/ProjectDeleteDialog.vue'

/**
 * Always-visible answer to "which project am I in?", plus create/rename/delete.
 * `compact` is the folded rail: a round badge with the project's initial, and
 * the menu unfolds to the right.
 */
const props = withDefaults(defineProps<{ compact?: boolean }>(), { compact: false })

const { t } = useI18n()
const projects = useProjectsStore()
const ui = useUiStore()
const errorText = useErrorText()
const projectName = useProjectName()

const open = ref(false)
const dialog = ref<'create' | 'rename' | 'delete' | null>(null)
const busy = ref(false)
const error = ref('')
const root = ref<HTMLElement | null>(null)
const menuEl = ref<HTMLElement | null>(null)
/** Where the folded rail's menu unfolds (fixed, so the rail's scroll box cannot clip it). */
const menuAt = ref({ top: 0, left: 0 })

const active = computed(() => projects.active)
const canDelete = computed(() => projects.projects.length > 1)
const initial = computed(() => {
  const name = projectName(active.value) || t('projects.none')
  return [...name][0]?.toUpperCase() ?? '?'
})

function toggle(): void {
  if (!open.value && props.compact) {
    const rect = root.value?.getBoundingClientRect()
    if (rect) {
      menuAt.value = { top: rect.top, left: rect.right + 8 }
    }
  }
  open.value = !open.value
}

function close(): void {
  open.value = false
}

function onDocumentClick(event: MouseEvent): void {
  const target = event.target as Node
  if (root.value?.contains(target) || menuEl.value?.contains(target)) {
    return
  }
  close()
}

function onKey(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    close()
    dialog.value = null
  }
}

onMounted(() => {
  document.addEventListener('mousedown', onDocumentClick)
  document.addEventListener('keydown', onKey)
})
onUnmounted(() => {
  document.removeEventListener('mousedown', onDocumentClick)
  document.removeEventListener('keydown', onKey)
})

async function pick(project: Project): Promise<void> {
  close()
  error.value = ''
  try {
    await projects.select(project.id)
    if (!['chat', 'notes', 'files', 'agents', 'models'].includes(ui.view)) {
      ui.setView('chat')
    }
  } catch (e) {
    error.value = errorText(e)
  }
}

function openDialog(kind: 'create' | 'rename' | 'delete'): void {
  close()
  error.value = ''
  dialog.value = kind
}

async function submitForm(payload: {
  name: string
  dictationLanguage: string
  copyModelsFrom: string | null
}): Promise<void> {
  busy.value = true
  error.value = ''
  try {
    if (dialog.value === 'create') {
      await projects.create(payload.name, payload.dictationLanguage, payload.copyModelsFrom)
      ui.setView('chat')
    } else if (active.value) {
      await projects.update(active.value.id, {
        name: payload.name,
        dictationLanguage: payload.dictationLanguage,
      })
    }
    dialog.value = null
  } catch (e) {
    error.value = errorText(e)
  } finally {
    busy.value = false
  }
}

async function confirmDelete(removeFiles: boolean): Promise<void> {
  if (!active.value) {
    return
  }
  busy.value = true
  error.value = ''
  try {
    await projects.remove(active.value.id, removeFiles)
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
  <div ref="root" class="switcher" :class="{ compact }">
    <button
      v-if="compact"
      class="badge"
      type="button"
      :aria-expanded="open"
      aria-haspopup="menu"
      :title="`${t('projects.current')}: ${projectName(active) || t('projects.none')}`"
      data-testid="project-switcher"
      @click="toggle"
    >
      {{ initial }}
    </button>
    <button
      v-else
      class="switcher-btn"
      type="button"
      :aria-expanded="open"
      aria-haspopup="menu"
      data-testid="project-switcher"
      @click="toggle"
    >
      <span class="switcher-kicker">{{ t('projects.current') }}</span>
      <span class="switcher-name">{{ projectName(active) || t('projects.none') }}</span>
      <span class="chevron" aria-hidden="true">⌄</span>
    </button>

    <Teleport to="body" :disabled="!compact">
      <div
        v-if="open"
        ref="menuEl"
        class="menu card"
        :class="{ flyout: compact }"
        :style="compact ? { top: `${menuAt.top}px`, left: `${menuAt.left}px` } : undefined"
        role="menu"
        data-testid="project-menu"
      >
        <button
          v-for="p in projects.projects"
          :key="p.id"
          class="menu-item"
          :class="{ active: p.id === projects.activeId }"
          type="button"
          role="menuitemradio"
          :aria-checked="p.id === projects.activeId"
          @click="pick(p)"
        >
          <span class="menu-check" aria-hidden="true">{{
            p.id === projects.activeId ? '✓' : ''
          }}</span>
          <span class="menu-label">{{ projectName(p) }}</span>
        </button>
        <div class="menu-sep" role="separator"></div>
        <button class="menu-item" type="button" role="menuitem" @click="openDialog('create')">
          <span class="menu-check" aria-hidden="true">+</span>
          <span class="menu-label">{{ t('projects.new') }}</span>
        </button>
        <button
          class="menu-item"
          type="button"
          role="menuitem"
          :disabled="!active"
          @click="openDialog('rename')"
        >
          <span class="menu-check" aria-hidden="true"></span>
          <span class="menu-label">{{ t('projects.renameAction') }}</span>
        </button>
        <button
          class="menu-item danger"
          type="button"
          role="menuitem"
          :disabled="!active || !canDelete"
          :title="canDelete ? '' : t('projects.lastProject')"
          @click="openDialog('delete')"
        >
          <span class="menu-check" aria-hidden="true"></span>
          <span class="menu-label">{{ t('projects.deleteAction') }}</span>
        </button>
      </div>
    </Teleport>

    <p v-if="error && !dialog" class="banner banner-error switch-error" role="alert">{{ error }}</p>

    <Teleport to="body">
      <ProjectFormDialog
        v-if="dialog === 'create' || dialog === 'rename'"
        :mode="dialog"
        :project="active"
        :busy="busy"
        :error="error"
        @cancel="dialog = null"
        @submit="submitForm"
      />
      <ProjectDeleteDialog
        v-if="dialog === 'delete' && active"
        :project="active"
        :busy="busy"
        :error="error"
        @cancel="dialog = null"
        @confirm="confirmDelete"
      />
    </Teleport>
  </div>
</template>

<style scoped>
.switcher {
  position: relative;
  margin-bottom: 0.6rem;
}

.switcher-btn {
  width: 100%;
  display: grid;
  grid-template-columns: 1fr auto;
  grid-template-rows: auto auto;
  column-gap: 0.4rem;
  text-align: left;
  padding: 0.5rem 0.65rem;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-elevated);
  color: var(--txt);
  font: inherit;
  cursor: pointer;
}

.switcher-btn:hover {
  border-color: var(--border-strong);
}

.badge {
  display: grid;
  place-items: center;
  width: 40px;
  height: 40px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--accent-soft);
  color: var(--accent);
  font: inherit;
  font-size: 1rem;
  font-weight: 750;
  cursor: pointer;
}

.badge:hover {
  border-color: var(--accent);
}

.menu.flyout {
  position: fixed;
  right: auto;
  width: 220px;
}

.switcher-kicker {
  grid-column: 1;
  font-size: 0.68rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--txt-secondary);
}

.switcher-name {
  grid-column: 1;
  font-weight: 650;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chevron {
  grid-column: 2;
  grid-row: 1 / span 2;
  align-self: center;
  color: var(--txt-secondary);
  line-height: 1;
}

.menu {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  right: 0;
  z-index: 25;
  padding: 0.3rem;
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
  box-shadow: var(--shadow-lg);
}

.menu-item {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.45rem 0.5rem;
  border: none;
  background: transparent;
  border-radius: var(--radius-sm);
  color: var(--txt);
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.menu-item:hover:not(:disabled) {
  background: var(--bg-elevated);
}

.menu-item.active {
  color: var(--accent);
  font-weight: 600;
}

.menu-item.danger {
  color: var(--danger);
}

.menu-item:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.menu-check {
  width: 1rem;
  flex-shrink: 0;
  text-align: center;
  font-size: 0.8rem;
}

.menu-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.menu-sep {
  height: 1px;
  background: var(--border);
  margin: 0.25rem 0.3rem;
}

.switch-error {
  margin: 0.5rem 0 0;
  font-size: 0.78rem;
}
</style>
