<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Assistant, ProjectModels } from '@/services/tauri'
import { worldMismatches, type WorldMismatch } from '@/composables/useAssistantWorld'
import { displayModelId } from '@/composables/useModelCatalog'

/**
 * Bind Assistants from the workspace to the project and pick one default.
 * Emits the new binding; the parent persists it. The world warning compares
 * the recipe's models to the project's — the project's models always win.
 */
const props = defineProps<{
  assistants: Assistant[]
  boundIds: number[]
  defaultId: number | null
  models: ProjectModels
  busy: boolean
}>()

const emit = defineEmits<{
  change: [boundIds: number[], defaultId: number | null]
}>()

const { t } = useI18n()

const bound = computed(() => new Set(props.boundIds))

function isBound(id: number): boolean {
  return bound.value.has(id)
}

function toggle(id: number): void {
  const next = new Set(bound.value)
  let defaultId = props.defaultId
  if (next.has(id)) {
    next.delete(id)
    if (defaultId === id) {
      defaultId = null
    }
  } else {
    next.add(id)
    if (defaultId === null) {
      defaultId = id
    }
  }
  emit('change', [...next], defaultId)
}

function setDefault(id: number | null): void {
  if (id !== null && !bound.value.has(id)) {
    return
  }
  emit('change', props.boundIds, id)
}

function mismatches(assistant: Assistant): WorldMismatch[] {
  return isBound(assistant.id) ? worldMismatches(assistant, props.models) : []
}

function mismatchLine(m: WorldMismatch): string {
  const slot = t(`models.slots.${m.slot}.label`)
  return m.recipe === null
    ? t('assistants.worldDefault', { slot })
    : t('assistants.worldOther', { slot, model: displayModelId(m.recipe) })
}
</script>

<template>
  <ul class="rows" data-testid="assistant-list">
    <li
      v-for="a in assistants"
      :key="a.id"
      class="row card"
      :class="{ bound: isBound(a.id) }"
      :data-testid="`assistant-${a.id}`"
    >
      <label class="bind">
        <input
          type="checkbox"
          :checked="isBound(a.id)"
          :disabled="busy"
          :data-testid="`assistant-${a.id}-bind`"
          @change="toggle(a.id)"
        />
        <span class="bind-text">
          <span class="name">{{ a.name }}</span>
          <span v-if="a.description" class="muted description">{{ a.description }}</span>
        </span>
      </label>

      <div class="row-side">
        <span
          v-if="defaultId === a.id"
          class="pill default"
          :data-testid="`assistant-${a.id}-default`"
        >
          {{ t('assistants.default') }}
        </span>
        <button
          v-else-if="isBound(a.id)"
          class="btn btn-ghost small"
          type="button"
          :disabled="busy"
          :data-testid="`assistant-${a.id}-make-default`"
          @click="setDefault(a.id)"
        >
          {{ t('assistants.makeDefault') }}
        </button>
      </div>

      <div
        v-if="mismatches(a).length > 0"
        class="banner banner-warn world"
        :data-testid="`assistant-${a.id}-world`"
      >
        <p class="world-title">{{ t('assistants.worldWarning') }}</p>
        <ul class="world-list">
          <li v-for="m in mismatches(a)" :key="m.slot">{{ mismatchLine(m) }}</li>
        </ul>
      </div>
    </li>
  </ul>
</template>

<style scoped>
.rows {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.row {
  display: grid;
  grid-template-columns: 1fr auto;
  align-items: center;
  gap: 0.5rem 1rem;
  padding: 0.7rem 0.9rem;
}
.row.bound {
  border-color: var(--accent);
}
.bind {
  display: flex;
  align-items: flex-start;
  gap: 0.65rem;
  cursor: pointer;
  min-width: 0;
}
.bind input {
  margin-top: 0.2rem;
}
.bind-text {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.name {
  font-weight: 600;
}
.description {
  font-size: 0.8rem;
}
.row-side {
  display: flex;
  align-items: center;
}
.small {
  padding: 0.3rem 0.65rem;
  font-size: 0.78rem;
}
.pill.default {
  font-size: 0.74rem;
  font-weight: 600;
  padding: 0.2rem 0.55rem;
  border-radius: 999px;
  background: var(--accent-soft);
  color: var(--accent);
}
.world {
  grid-column: 1 / -1;
  margin: 0;
  padding: 0.55rem 0.75rem;
  font-size: 0.82rem;
}
.world-title {
  margin: 0 0 0.25rem;
}
.world-list {
  margin: 0;
  padding-left: 1.1rem;
}
</style>
