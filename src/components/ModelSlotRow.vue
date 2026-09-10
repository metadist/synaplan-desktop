<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { CatalogEntry, ModelSlot, SlotSource } from '@/services/tauri'
import { displayModelId } from '@/composables/useModelCatalog'

/**
 * One row of "This project's models": the slot, its current pick, and a picker
 * fed only by what the workspace advertises. A pick that the workspace no
 * longer offers stays visible as such — it is never swapped for another model.
 */
const props = defineProps<{
  slotId: ModelSlot
  value: string
  entries: CatalogEntry[]
  source: SlotSource
  disabled: boolean
  legacy: boolean
}>()

const emit = defineEmits<{ change: [value: string] }>()

const { t } = useI18n()

const current = computed(() => props.entries.find((e) => e.id === props.value) ?? null)
const missingFromList = computed(
  () => props.value !== '' && current.value === null && props.source !== 'none',
)

interface ProviderGroup {
  service: string
  entries: CatalogEntry[]
}

const groups = computed<ProviderGroup[]>(() => {
  const byService = new Map<string, CatalogEntry[]>()
  for (const entry of props.entries) {
    const list = byService.get(entry.service) ?? []
    list.push(entry)
    byService.set(entry.service, list)
  }
  return [...byService.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([service, entries]) => ({
      service,
      entries: [...entries].sort((a, b) => a.providerId.localeCompare(b.providerId)),
    }))
})

const hasChoices = computed(() => props.entries.length > 0)

const statusText = computed(() => {
  if (props.value === '') {
    return ''
  }
  if (missingFromList.value) {
    return t('models.notInList', { model: displayModelId(props.value) })
  }
  if (current.value && !current.value.available) {
    return current.value.unavailableReason || t('models.notReady')
  }
  if (props.legacy) {
    return t('models.legacyPick')
  }
  return ''
})

const statusKind = computed<'warn' | 'ok' | ''>(() => {
  if (props.value === '') {
    return ''
  }
  if (missingFromList.value || (current.value && !current.value.available)) {
    return 'warn'
  }
  return current.value?.available && props.source === 'catalog' ? 'ok' : ''
})

function onChange(event: Event): void {
  emit('change', (event.target as HTMLSelectElement).value)
}

function optionLabel(entry: CatalogEntry): string {
  const base =
    entry.name && entry.name !== entry.providerId
      ? `${entry.providerId} · ${entry.name}`
      : entry.providerId
  return entry.available ? base : `${base} — ${entry.unavailableReason || t('models.notReady')}`
}
</script>

<template>
  <div class="card slot" :data-testid="`slot-${slotId}`">
    <div class="slot-main">
      <div class="slot-label">{{ t(`models.slots.${slotId}.label`) }}</div>
      <div class="slot-hint muted">{{ t(`models.slots.${slotId}.hint`) }}</div>
      <div
        v-if="statusText"
        class="slot-status"
        :class="statusKind"
        :data-testid="`slot-${slotId}-status`"
      >
        {{ statusText }}
      </div>
    </div>

    <div class="slot-pick">
      <select
        class="input slot-select"
        :value="value"
        :disabled="disabled"
        :aria-label="t(`models.slots.${slotId}.label`)"
        :data-testid="`slot-${slotId}-select`"
        @change="onChange"
      >
        <option value="">{{ t('models.unset') }}</option>
        <option v-if="missingFromList" :value="value">
          {{ displayModelId(value) }} — {{ t('models.notInListShort') }}
        </option>
        <option v-if="!hasChoices && value !== '' && !missingFromList" :value="value">
          {{ displayModelId(value) }}
        </option>
        <optgroup v-for="group in groups" :key="group.service" :label="group.service">
          <option
            v-for="entry in group.entries"
            :key="entry.id"
            :value="entry.id"
            :disabled="!entry.available"
          >
            {{ optionLabel(entry) }}
          </option>
        </optgroup>
      </select>
      <div class="slot-meta">
        <span v-if="current" class="meta-provider">{{ current.service }}</span>
        <span v-if="source === 'flat_models' || source === 'audio_models'" class="meta-basic muted">
          {{ t('models.basicList') }}
        </span>
        <span v-else-if="!hasChoices && source !== 'none'" class="muted">
          {{ t('models.noneForSlot') }}
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.slot {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.8rem 1rem;
}
.slot-main {
  min-width: 0;
  flex: 1;
}
.slot-label {
  font-weight: 650;
}
.slot-hint {
  font-size: 0.8rem;
}
.slot-status {
  margin-top: 0.3rem;
  font-size: 0.78rem;
  color: var(--txt-secondary);
}
.slot-status.warn {
  color: var(--danger);
}
.slot-status.ok {
  color: var(--ok);
}
.slot-pick {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 0.25rem;
  width: 280px;
  flex-shrink: 0;
}
.slot-select {
  padding: 0.45rem 0.6rem;
  font-size: 0.85rem;
}
.slot-meta {
  display: flex;
  gap: 0.5rem;
  font-size: 0.75rem;
  min-height: 1.1em;
}
.meta-provider {
  color: var(--txt-secondary);
  text-transform: capitalize;
}
</style>
