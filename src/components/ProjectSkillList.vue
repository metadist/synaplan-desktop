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
  try: [skill: Skill]
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
      @click="emit('try', skill)"
      @dblclick="emit('try', skill)"
    >
      <label class="use" @click.prevent="emit('try', skill)">
        <input
          type="checkbox"
          :checked="isOn(skill)"
          :disabled="busy || !runnable(skill)"
          :data-testid="`skill-${skill.name}-toggle`"
          @click.stop
          @change="toggle(skill)"
        />
        <span class="use-text">
          <span class="name">{{ skill.name }}</span>
          <span class="muted description">{{ skill.description }}</span>
        </span>
      </label>
      <span class="side">
        <button
          class="btn btn-primary small"
          type="button"
          :data-testid="`skill-${skill.name}-try`"
          @click.stop="emit('try', skill)"
        >
          {{ t('agents.try') }}
        </button>
        <span v-if="status(skill)" class="status muted" :data-testid="`skill-${skill.name}-status`">
          {{ status(skill) }}
          <button
            v-if="!skill.enabled && !skill.blocked"
            class="btn-link"
            type="button"
            @click.stop="emit('openSkills')"
          >
            {{ t('projectSkills.manage') }} →
          </button>
        </span>
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
  cursor: pointer;
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
.side {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 0.35rem;
}
.small {
  padding: 0.35rem 0.75rem;
  font-size: 0.78rem;
}
.status {
  font-size: 0.78rem;
  text-align: right;
  display: inline-flex;
  flex-direction: column;
  align-items: flex-end;
}
</style>
