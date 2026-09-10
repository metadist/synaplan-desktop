<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { MODEL_SLOTS, type ModelSlot } from '@/services/tauri'
import { useProjectsStore } from '@/stores/projects'
import { useProjectName } from '@/composables/useProjectName'

/** "This project's models": the eight slots. Picking arrives with the catalog. */
const { t } = useI18n()
const projects = useProjectsStore()
const projectName = useProjectName()

const project = computed(() => projects.active)

/** Show the provider id, never the engineers' `service:providerId:tag` key. */
function displayBinding(value: string): string {
  const parts = value.split(':')
  return parts.length >= 3 ? parts.slice(1, -1).join(':') : value
}

const rows = computed(() =>
  MODEL_SLOTS.map((slot: ModelSlot) => ({
    slot,
    label: t(`models.slots.${slot}.label`),
    hint: t(`models.slots.${slot}.hint`),
    value: project.value?.models[slot] ?? '',
  })),
)
</script>

<template>
  <section class="view">
    <header class="view-header">
      <div>
        <h1>{{ t('models.title') }}</h1>
        <p class="muted subtitle">{{ projectName(project) }}</p>
      </div>
    </header>

    <div class="view-body">
      <p class="sovereignty">{{ t('models.sovereignty') }}</p>
      <p class="muted intro">{{ t('models.intro') }}</p>

      <div v-if="project" class="slots">
        <div
          v-for="row in rows"
          :key="row.slot"
          class="card slot"
          :data-testid="`slot-${row.slot}`"
        >
          <div class="slot-main">
            <div class="slot-label">{{ row.label }}</div>
            <div class="slot-hint muted">{{ row.hint }}</div>
          </div>
          <div class="slot-value" :class="{ unset: !row.value }">
            {{ row.value ? displayBinding(row.value) : t('models.unset') }}
          </div>
        </div>
      </div>
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
.sovereignty {
  margin: 0 0 0.35rem;
  font-weight: 600;
}
.intro {
  margin: 0 0 1rem;
  font-size: 0.9rem;
}
.slots {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  max-width: 720px;
}
.slot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.8rem 1rem;
}
.slot-main {
  min-width: 0;
}
.slot-label {
  font-weight: 650;
}
.slot-hint {
  font-size: 0.8rem;
}
.slot-value {
  font-size: 0.85rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  text-align: right;
  overflow-wrap: anywhere;
}
.slot-value.unset {
  font-family: inherit;
  color: var(--txt-secondary);
}
</style>
