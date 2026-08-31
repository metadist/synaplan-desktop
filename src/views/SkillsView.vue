<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import * as api from '@/services/tauri'
import { useErrorText } from '@/composables/useErrorText'
import { DOCS } from '@/constants'

const { t } = useI18n()
const errorText = useErrorText()

const skills = ref<api.Skill[]>([])
const error = ref('')

onMounted(load)

async function load(): Promise<void> {
  try {
    skills.value = await api.listSkills()
  } catch (e) {
    error.value = errorText(e)
  }
}

async function toggle(skill: api.Skill): Promise<void> {
  error.value = ''
  try {
    skills.value = await api.setSkillEnabled(skill.name, !skill.enabled)
  } catch (e) {
    error.value = errorText(e)
  }
}
</script>

<template>
  <section class="view">
    <header class="view-header">
      <h1>{{ t('skills.title') }}</h1>
    </header>

    <div class="view-body">
      <p class="muted intro">{{ t('skills.intro') }}</p>

      <p v-if="error" class="banner banner-error" role="alert">{{ error }}</p>

      <p v-if="skills.length === 0" class="muted empty">{{ t('skills.empty') }}</p>

      <div v-for="skill in skills" :key="skill.name" class="card skill">
        <div class="skill-main">
          <div class="skill-head">
            <span class="skill-name">{{ skill.name }}</span>
            <span class="tag">{{
              skill.bundled ? t('skills.included') : t('skills.installedByYou')
            }}</span>
          </div>
          <p class="skill-desc muted">{{ skill.description }}</p>
          <p class="skill-note">{{ t('skills.mayRunPrograms') }}</p>
        </div>
        <label class="toggle">
          <input type="checkbox" :checked="skill.enabled" @change="toggle(skill)" />
          <span>{{ skill.enabled ? t('skills.enabled') : t('skills.disabled') }}</span>
        </label>
      </div>

      <p class="muted install-note">{{ t('skills.installComing') }}</p>
      <button class="btn-link learn-more" type="button" @click="api.openUrl(DOCS.skills)">
        {{ t('common.learnMore') }} →
      </button>
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
  padding: 0.9rem 1.2rem;
  border-bottom: 1px solid var(--border);
}

.view-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 1.1rem 1.2rem;
}

.intro {
  margin: 0 0 1rem;
  font-size: 0.9rem;
}

.empty {
  font-size: 0.9rem;
}

.skill {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.9rem 1rem;
  margin-bottom: 0.7rem;
}

.skill-head {
  display: flex;
  align-items: center;
  gap: 0.55rem;
}

.skill-name {
  font-weight: 650;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}

.tag {
  font-size: 0.68rem;
  font-weight: 600;
  padding: 0.1rem 0.45rem;
  border-radius: 999px;
  background: var(--accent-soft);
  color: var(--accent);
}

.skill-desc {
  margin: 0.35rem 0 0.3rem;
  font-size: 0.86rem;
}

.skill-note {
  margin: 0;
  font-size: 0.74rem;
  color: var(--txt-secondary);
}

.toggle {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  flex-shrink: 0;
  font-size: 0.8rem;
  color: var(--txt-secondary);
  cursor: pointer;
}

.install-note {
  margin: 1rem 0 0.4rem;
  font-size: 0.85rem;
}

.learn-more {
  font-size: 0.88rem;
  font-weight: 550;
}
</style>
