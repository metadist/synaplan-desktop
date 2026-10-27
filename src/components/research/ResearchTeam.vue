<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  lead: string
  reader: string
  checker: string
  chat: string
  docs: string
  options: { id: string; name: string; available: boolean; reason: string | null }[]
}>()

const emit = defineEmits<{
  customize: [role: 'lead' | 'reader' | 'checker', key: string]
  reset: []
}>()

const { t } = useI18n()
const custom = ref(false)

const rows = computed(() => [
  {
    role: 'lead' as const,
    label: t('research.roleLead'),
    value: props.lead || props.chat,
    hint: t('research.sameAsChat'),
  },
  {
    role: 'reader' as const,
    label: t('research.roleReader'),
    value: props.reader || props.docs,
    hint: t('research.recommended'),
  },
  {
    role: 'checker' as const,
    label: t('research.roleChecker'),
    value: props.checker,
    hint: t('research.recommended'),
  },
])

function nameOf(id: string): string {
  return props.options.find((option) => option.id === id)?.name || id || t('research.recommended')
}
</script>

<template>
  <section class="team" data-testid="research-team">
    <h2>{{ t('research.team') }}</h2>
    <p>{{ t('research.world') }}</p>
    <div v-for="row in rows" :key="row.role" class="row">
      <span>{{ row.label }}</span>
      <select
        v-if="custom"
        :value="row.value"
        @change="emit('customize', row.role, ($event.target as HTMLSelectElement).value)"
      >
        <option value="">{{ t('research.recommended') }}</option>
        <option
          v-for="option in options"
          :key="option.id"
          :value="option.id"
          :disabled="!option.available"
        >
          {{ option.name }}{{ option.available ? '' : ` — ${option.reason || ''}` }}
        </option>
      </select>
      <span v-else
        >{{ nameOf(row.value) }} <span class="muted">{{ row.hint }}</span></span
      >
    </div>
    <button class="btn btn-ghost" type="button" @click="custom = !custom">
      {{ t('research.customize') }}
    </button>
    <button
      class="btn btn-ghost"
      type="button"
      data-testid="research-team-reset"
      @click="emit('reset')"
    >
      {{ t('research.reset') }}
    </button>
  </section>
</template>

<style scoped>
.team {
  margin-top: 1.2rem;
  padding-top: 1rem;
  border-top: 1px solid var(--border);
}
.row {
  display: flex;
  justify-content: space-between;
  gap: 0.8rem;
  padding: 0.35rem 0;
}
.muted {
  color: var(--muted);
}
</style>
