<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { MODEL_SLOTS, type ModelSlot, type ProjectModels } from '@/services/tauri'
import { useProjectsStore } from '@/stores/projects'
import { useProjectName } from '@/composables/useProjectName'
import { useModelCatalog, isCatalogKey } from '@/composables/useModelCatalog'
import { useErrorText } from '@/composables/useErrorText'
import ModelSlotRow from '@/components/ModelSlotRow.vue'

/**
 * "This project's models": eight slots bound to what the paired workspace
 * advertises. A fresh project starts with the workspace's recommended model in
 * every slot that has one (see `applyDefaultModels`); this panel is where the
 * person changes those picks. Embed is not a project pick: index and chat
 * search both use workspace VECTORIZE, so that slot is shown locked. When the
 * workspace has no catalog yet the panel says so and keeps the picks — nothing
 * is invented client-side.
 */
const { t } = useI18n()
const projects = useProjectsStore()
const projectName = useProjectName()
const errorText = useErrorText()

const project = computed(() => projects.active)
const projectId = computed(() => project.value?.id ?? '')
const catalog = useModelCatalog(projectId)

const saving = ref<ModelSlot | null>(null)
const saveError = ref('')
/** The "Stay in this world" chip explains itself when clicked. */
const worldOpen = ref(false)

const catalogMissing = computed(() => catalog.catalog.value?.catalogMissing ?? false)
const hasFallback = computed(() => {
  const sources = catalog.catalog.value?.sources
  return (
    !!sources && Object.values(sources).some((s) => s === 'flat_models' || s === 'audio_models')
  )
})
const allUsedSet = computed(() => {
  const m = project.value?.models
  return !!m && m.chat !== '' && m.voice !== '' && m.embed !== '' && m.docs !== ''
})

function sourceFor(slot: ModelSlot) {
  return catalog.catalog.value?.sources[slot] ?? 'none'
}

async function pick(slot: ModelSlot, value: string): Promise<void> {
  const p = project.value
  if (!p) {
    return
  }
  const models: ProjectModels = { ...p.models, [slot]: value }
  if (slot === 'chat') {
    // A pick from the flat list is a bare provider id; remember it as such so
    // it can be upgraded to the catalog key once the workspace offers one.
    models.chatLegacyProviderId = value !== '' && !isCatalogKey(value) ? value : null
  }
  saving.value = slot
  saveError.value = ''
  try {
    await projects.update(p.id, { models })
  } catch (e) {
    saveError.value = errorText(e)
  } finally {
    saving.value = null
  }
}
</script>

<template>
  <section class="view">
    <header class="view-header">
      <div>
        <h1>{{ t('models.title') }}</h1>
        <p class="muted subtitle">{{ projectName(project) }}</p>
      </div>
      <button
        v-if="allUsedSet && !catalogMissing && !catalog.error.value"
        class="world-chip"
        :class="{ open: worldOpen }"
        type="button"
        :aria-expanded="worldOpen"
        :title="t('models.worldChipHint')"
        data-testid="world-chip"
        @click="worldOpen = !worldOpen"
      >
        ✓ {{ t('models.worldChip') }}
      </button>
    </header>

    <div class="view-body">
      <div v-if="worldOpen" class="world-card" data-testid="world-card">
        <p class="world-lead">{{ t('models.worldExplain') }}</p>
        <p class="muted world-foot">{{ t('models.worldTaste') }}</p>
      </div>
      <p class="sovereignty">{{ t('models.sovereignty') }}</p>
      <p class="muted intro">{{ t('models.intro') }}</p>

      <p v-if="catalog.loading.value && !catalog.catalog.value" class="muted loading">
        <span class="spinner"></span> {{ t('models.loading') }}
      </p>

      <div v-if="catalog.error.value" class="banner banner-error notice" data-testid="models-error">
        {{ t('models.loadFailed') }} {{ errorText(catalog.error.value) }}
        <button class="btn-link" type="button" @click="catalog.load()">
          {{ t('models.retry') }}
        </button>
      </div>

      <div
        v-else-if="catalogMissing"
        class="banner banner-warn notice"
        data-testid="models-missing"
      >
        {{ t('models.catalogMissing') }}
        <span v-if="hasFallback"> {{ t('models.fallbackNote') }}</span>
      </div>

      <div v-if="saveError" class="banner banner-error notice">{{ saveError }}</div>

      <div v-if="project" class="slots">
        <ModelSlotRow
          v-for="slot in MODEL_SLOTS"
          :key="slot"
          :slot-id="slot"
          :value="project.models[slot]"
          :entries="catalog.entries(slot)"
          :source="sourceFor(slot)"
          :disabled="saving !== null || catalog.loading.value || slot === 'embed'"
          :legacy="slot === 'chat' && project.models.chatLegacyProviderId !== null"
          @change="pick(slot, $event)"
        />
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
.world-chip {
  font: inherit;
  font-size: 0.78rem;
  font-weight: 600;
  color: var(--ok);
  background: transparent;
  border: 1px solid color-mix(in srgb, var(--ok) 40%, transparent);
  border-radius: 999px;
  padding: 0.2rem 0.65rem;
  cursor: pointer;
}

.world-chip:hover,
.world-chip.open {
  background: color-mix(in srgb, var(--ok) 12%, transparent);
  border-color: var(--ok);
}

.world-card {
  margin: 0 0 1rem;
  padding: 0.8rem 1rem;
  border: 1px solid color-mix(in srgb, var(--ok) 40%, transparent);
  border-left: 4px solid var(--ok);
  border-radius: var(--radius-sm);
  background: color-mix(in srgb, var(--ok) 7%, var(--bg-card));
}

.world-lead {
  margin: 0;
  font-size: 0.92rem;
  line-height: 1.5;
}

.world-foot {
  margin: 0.35rem 0 0;
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
.loading {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin: 0 0 0.8rem;
}
.notice {
  max-width: 720px;
  margin: 0 0 0.9rem;
}
.slots {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
  max-width: 720px;
}
</style>
