<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Skill } from '@/services/tauri'
import { starterPromptKey } from '@/composables/useTaskStudio'

/**
 * First-use helper: a skill does not run when you click its name. This window
 * explains that, prepares a starter message, and sends the person to Chat.
 */
const props = defineProps<{
  skill: Skill
  enabledHere: boolean
}>()

const emit = defineEmits<{
  cancel: []
  run: [prompt: string, autoSend: boolean, enableHere: boolean]
}>()

const { t, te } = useI18n()

const prompt = ref('')

function defaultPrompt(): string {
  const key = starterPromptKey(props.skill.name)
  return key && te(key) ? t(key) : t('chat.studio.genericPrompt', { name: props.skill.name })
}

watch(
  () => props.skill.name,
  () => {
    prompt.value = defaultPrompt()
  },
  { immediate: true },
)

const canRun = computed(() => props.skill.enabled && !props.skill.blocked)
const blockedReason = computed(() => {
  if (props.skill.blocked) {
    return props.skill.blockedReason || t('projectSkills.blocked')
  }
  if (!props.skill.enabled) {
    return t('projectSkills.offOnComputer')
  }
  return ''
})
</script>

<template>
  <div class="overlay" role="dialog" aria-modal="true" data-testid="skill-try-dialog">
    <div class="card dialog">
      <h2 class="title">{{ t('agents.tryTitle', { name: skill.name }) }}</h2>
      <p class="muted body">{{ t('agents.tryBody') }}</p>
      <p class="desc">{{ skill.description }}</p>
      <p v-if="blockedReason" class="banner banner-warn notice">{{ blockedReason }}</p>
      <p v-else-if="!enabledHere" class="muted notice">{{ t('agents.tryWillEnable') }}</p>
      <label class="prompt-label" for="skill-try-prompt">{{ t('agents.tryPromptLabel') }}</label>
      <textarea
        id="skill-try-prompt"
        v-model="prompt"
        class="input prompt"
        rows="5"
        data-testid="skill-try-prompt"
      />
      <div class="actions">
        <button class="btn btn-ghost" type="button" data-testid="skill-try-cancel" @click="emit('cancel')">
          {{ t('common.cancel') }}
        </button>
        <button
          class="btn btn-secondary"
          type="button"
          :disabled="!canRun || prompt.trim() === ''"
          data-testid="skill-try-fill"
          @click="emit('run', prompt.trim(), false, !enabledHere)"
        >
          {{ t('agents.tryFill') }}
        </button>
        <button
          class="btn btn-primary"
          type="button"
          :disabled="!canRun || prompt.trim() === ''"
          data-testid="skill-try-run"
          @click="emit('run', prompt.trim(), true, !enabledHere)"
        >
          {{ t('agents.tryRun') }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: absolute;
  inset: 0;
  background: color-mix(in srgb, var(--bg) 70%, transparent);
  backdrop-filter: blur(3px);
  display: grid;
  place-items: center;
  padding: 1.5rem;
  z-index: 20;
}
.dialog {
  max-width: 480px;
  width: 100%;
  padding: 1.4rem 1.5rem;
}
.title {
  margin: 0 0 0.4rem;
  font-size: 1.15rem;
}
.body,
.desc,
.notice {
  margin: 0 0 0.7rem;
  font-size: 0.88rem;
}
.prompt-label {
  display: block;
  font-size: 0.8rem;
  font-weight: 650;
  color: var(--txt-secondary);
  margin-bottom: 0.35rem;
}
.prompt {
  width: 100%;
  resize: vertical;
  min-height: 6rem;
  margin-bottom: 1rem;
}
.actions {
  display: flex;
  justify-content: flex-end;
  flex-wrap: wrap;
  gap: 0.5rem;
}
</style>
