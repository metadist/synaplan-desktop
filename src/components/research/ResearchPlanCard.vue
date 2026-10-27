<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ResearchPoint } from '@/lib/researchRun'

const props = defineProps<{
  points: ResearchPoint[]
  editable: boolean
  disclosures: string[]
  estimate: string
  fileCount: number
  web: boolean
}>()

const emit = defineEmits<{
  start: []
  cancel: []
  edit: [points: { id?: string; title?: string }[]]
}>()

const { t } = useI18n()
const added = ref('')

const sourcesLine = computed(() =>
  [
    props.web ? t('research.web') : '',
    props.fileCount > 0 ? t('research.files', { count: props.fileCount }) : '',
  ]
    .filter(Boolean)
    .join(' · '),
)

function pointsPayload(): { id?: string; title?: string }[] {
  return props.points.map((point) => ({ id: point.id, title: point.title }))
}

function remove(id: string): void {
  emit(
    'edit',
    props.points
      .filter((point) => point.id !== id)
      .map((point) => ({ id: point.id, title: point.title })),
  )
}

function add(): void {
  const title = added.value.trim()
  if (!title) {
    return
  }
  emit('edit', [...pointsPayload(), { title }])
  added.value = ''
}
</script>

<template>
  <section class="card" data-testid="research-plan">
    <h2>{{ t('research.planTitle') }}</h2>
    <p>{{ t('research.planLead') }}</p>
    <ol>
      <li v-for="(point, index) in points" :key="point.id || index">
        {{ point.title }}
        <button v-if="editable" class="btn-link" type="button" @click="remove(point.id)">
          {{ t('research.removePoint') }}
        </button>
      </li>
    </ol>
    <form v-if="editable" class="add" @submit.prevent="add">
      <input v-model="added" class="input" type="text" :placeholder="t('research.addPoint')" />
      <button class="btn btn-ghost" type="submit">{{ t('research.addPoint') }}</button>
    </form>
    <p v-if="sourcesLine">{{ t('research.sourcesLabel', { list: sourcesLine }) }}</p>
    <p v-for="(line, index) in disclosures" :key="index">{{ line }}</p>
    <p v-if="estimate">{{ t('research.disclosureCost', { amount: estimate }) }}</p>
    <p>{{ t('research.nothingEmailed') }}</p>
    <div class="actions">
      <template v-if="editable">
        <button class="btn btn-ghost" type="button" @click="emit('cancel')">
          {{ t('common.cancel') }}
        </button>
        <button
          class="btn btn-primary"
          type="button"
          data-testid="research-plan-start"
          @click="emit('start')"
        >
          {{ t('research.start') }}
        </button>
      </template>
      <button v-else class="btn btn-ghost" type="button" @click="emit('cancel')">
        {{ t('research.stop') }}
      </button>
    </div>
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
.add {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}
</style>
