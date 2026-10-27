<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ResearchExport } from '@/lib/researchExports'
import { exportBlock, exportSkill } from '@/lib/researchExports'
import type { Skill } from '@/services/tauri'

const props = defineProps<{
  title: string
  preview: string
  hasReport: boolean
  check: { statements: number; supported: number; removed: number } | null
  savedPath: string
  outcome: string
  failed: boolean
  skills: Skill[]
  version: number
  busy: boolean
}>()

const emit = defineEmits<{
  open: []
  export: [kind: ResearchExport]
  deeper: []
  refresh: []
  delete: []
  doctor: []
}>()

const { t } = useI18n()
const more = ref(false)

const kinds: ResearchExport[] = ['docx', 'pptx', 'xlsx', 'email-draft', 'web-report']

function skillFor(kind: ResearchExport) {
  return props.skills.find((skill) => skill.name === exportSkill(kind))
}

const blocks = computed(() =>
  Object.fromEntries(kinds.map((kind) => [kind, exportBlock(skillFor(kind))])),
)
</script>

<template>
  <section class="card" :data-testid="failed ? 'research-failed' : 'research-result'">
    <h2>{{ failed ? t('research.stoppedTitle') : t('research.reportReady') }}</h2>
    <p v-if="outcome">{{ outcome }}</p>
    <p v-if="title" class="title">{{ title }}</p>
    <p v-if="preview">{{ preview }}</p>
    <p v-if="check">
      {{ t('research.backed', { supported: check.supported, statements: check.statements }) }}
      <template v-if="check.removed > 0">
        {{ t('research.removed', { removed: check.removed }) }}
      </template>
    </p>
    <div class="actions">
      <button
        v-if="hasReport"
        class="btn btn-primary"
        type="button"
        data-testid="research-open"
        @click="emit('open')"
      >
        {{ t('research.openReport') }}
      </button>
      <button
        v-if="hasReport"
        class="btn btn-ghost"
        type="button"
        :disabled="!!blocks.docx"
        @click="emit('export', 'docx')"
      >
        {{ t('research.word') }}
      </button>
      <button class="btn btn-ghost" type="button" :aria-expanded="more" @click="more = !more">
        {{ t('research.more') }}
      </button>
    </div>
    <p v-if="hasReport && blocks.docx" class="hint">
      {{ blocks.docx.startsWith('research.') ? t(blocks.docx) : blocks.docx }}
      <button class="btn-link" type="button" @click="emit('doctor')">
        {{ t('research.checkComputer') }}
      </button>
    </p>
    <div v-if="more" class="more" data-testid="research-more">
      <template v-if="hasReport">
        <button
          v-for="kind in kinds.slice(1)"
          :key="kind"
          class="btn btn-ghost"
          type="button"
          :disabled="!!blocks[kind]"
          @click="emit('export', kind)"
        >
          {{ t(`research.export.${kind}`) }}
        </button>
        <button class="btn btn-ghost" type="button" :disabled="busy" @click="emit('deeper')">
          {{ t('research.deeper') }}
        </button>
        <button class="btn btn-ghost" type="button" :disabled="busy" @click="emit('refresh')">
          {{ t('research.refreshSources') }}
        </button>
      </template>
      <button
        class="btn btn-ghost"
        type="button"
        data-testid="research-delete"
        @click="emit('delete')"
      >
        {{ t('research.deleteReport') }}
      </button>
    </div>
    <p v-if="savedPath">{{ t('research.savedIn', { path: savedPath }) }}</p>
    <p v-if="hasReport" class="muted">{{ t('research.madeFrom', { version }) }}</p>
  </section>
</template>

<style scoped>
.card {
  margin: 0.8rem 1.2rem;
  padding: 0.9rem 1rem;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-card);
}
.actions,
.more {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}
.hint,
.muted {
  color: var(--muted);
}
</style>
