<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { Skill } from '@/services/tauri'

/**
 * Per-project skill overlay: every skill installed on this computer with a
 * "use in this project" toggle. A project can only narrow what the computer
 * allows — skills switched off or blocked at computer level cannot be turned on
 * here; the row says why and points to the computer-level page.
 */
const props = defineProps<{
  skills: Skill[]
  enabledHere: string[]
  busy: boolean
}>()

const emit = defineEmits<{
  change: [enabledHere: string[]]
  openSkills: []
}>()

const { t } = useI18n()

function isOn(skill: Skill): boolean {
  return props.enabledHere.includes(skill.name)
}

function runnable(skill: Skill): boolean {
  return skill.enabled && !skill.blocked
}

function toggle(skill: Skill): void {
  const next = new Set(props.enabledHere)
  if (next.has(skill.name)) {
    next.delete(skill.name)
  } else {
    next.add(skill.name)
  }
  emit('change', [...next])
}

function status(skill: Skill): string {
  if (skill.blocked) {
    return skill.blockedReason || t('projectSkills.blocked')
  }
  if (!skill.enabled) {
    return t('projectSkills.offOnComputer')
  }
  return ''
}
</script>

<template>
  <ul class="rows" data-testid="project-skill-list">
    <li
      v-for="skill in skills"
      :key="skill.name"
      class="row card"
      :class="{ on: isOn(skill) && runnable(skill), dim: !runnable(skill) }"
      :data-testid="`skill-${skill.name}`"
    >
      <label class="use">
        <input
          type="checkbox"
          :checked="isOn(skill)"
          :disabled="busy || !runnable(skill)"
          :data-testid="`skill-${skill.name}-toggle`"
          @change="toggle(skill)"
        />
        <span class="use-text">
          <span class="name">{{ skill.name }}</span>
          <span class="muted description">{{ skill.description }}</span>
        </span>
      </label>
      <span v-if="status(skill)" class="status muted" :data-testid="`skill-${skill.name}-status`">
        {{ status(skill) }}
        <button
          v-if="!skill.enabled && !skill.blocked"
          class="btn-link"
          type="button"
          @click="emit('openSkills')"
        >
          {{ t('projectSkills.manage') }} →
        </button>
      </span>
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
  gap: 0.45rem;
}
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.6rem 0.9rem;
}
.row.on {
  border-color: var(--accent);
}
.row.dim .use-text {
  opacity: 0.7;
}
.use {
  display: flex;
  align-items: flex-start;
  gap: 0.65rem;
  cursor: pointer;
  min-width: 0;
}
.use input {
  margin-top: 0.2rem;
}
.use-text {
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
.status {
  font-size: 0.78rem;
  text-align: right;
  flex-shrink: 0;
  display: inline-flex;
  flex-direction: column;
  align-items: flex-end;
}
</style>
