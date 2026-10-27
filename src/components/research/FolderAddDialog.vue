<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { FolderWalk } from '@/services/tauri'

const props = withDefaults(defineProps<{ walk: FolderWalk | null; sending?: boolean }>(), {
  sending: false,
})
const emit = defineEmits<{
  cancel: []
  confirm: [subfolders: boolean]
  toggleSub: [value: boolean]
}>()
const { t } = useI18n()
const sub = ref(false)

const summary = computed(() => {
  const walk = props.walk
  if (!walk) {
    return ''
  }
  return t('research.folderSummary', {
    count: walk.files.length,
    pdf: walk.pdf,
    word: walk.word,
    excel: walk.excel,
  })
})

function setSub(value: boolean): void {
  sub.value = value
  emit('toggleSub', value)
}
</script>

<template>
  <div
    v-if="walk"
    class="overlay"
    role="dialog"
    aria-modal="true"
    data-testid="folder-add"
    @click.self="emit('cancel')"
  >
    <div class="card">
      <h2>{{ t('research.folderTitle') }}</h2>
      <p class="path">{{ walk.root }}</p>
      <p>{{ summary }}</p>
      <p>{{ t('research.folderConsent') }}</p>
      <label>
        <input
          type="checkbox"
          :checked="sub"
          @change="setSub(($event.target as HTMLInputElement).checked)"
        />
        {{ t('research.includeSubfolders') }}
      </label>
      <p v-if="walk.skipped.length">
        {{ t('research.skippedKind', { count: walk.skipped.length }) }}
      </p>
      <div class="actions">
        <button class="btn btn-ghost" type="button" @click="emit('cancel')">
          {{ t('common.cancel') }}
        </button>
        <button
          class="btn btn-primary"
          type="button"
          :disabled="sending || walk.files.length === 0"
          data-testid="folder-send"
          @click="emit('confirm', sub)"
        >
          {{ t('research.sendFiles', { count: walk.files.length }) }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  display: grid;
  place-items: center;
  background: color-mix(in srgb, var(--bg) 40%, transparent);
}
.card {
  width: min(32rem, calc(100% - 2rem));
  padding: 1rem;
  border-radius: 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
}
.path {
  font-family: var(--font-mono, ui-monospace, monospace);
  word-break: break-all;
}
.actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.4rem;
}
</style>
