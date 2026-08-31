<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import * as api from '@/services/tauri'
import { useErrorText } from '@/composables/useErrorText'
import { DOCS } from '@/constants'

const { t } = useI18n()
const errorText = useErrorText()

const tools = ref<api.Tool[]>([])
const loading = ref(true)
const error = ref('')

onMounted(check)

async function check(): Promise<void> {
  loading.value = true
  error.value = ''
  try {
    tools.value = await api.runDoctor()
  } catch (e) {
    error.value = errorText(e)
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <section class="view">
    <header class="view-header">
      <h1>{{ t('doctor.title') }}</h1>
      <button class="btn btn-ghost" type="button" :disabled="loading" @click="check">
        {{ t('doctor.recheck') }}
      </button>
    </header>

    <div class="view-body">
      <p class="muted intro">{{ t('doctor.intro') }}</p>

      <p v-if="error" class="banner banner-error" role="alert">{{ error }}</p>

      <div v-if="loading" class="center muted">
        <span class="spinner"></span>
        <span>{{ t('doctor.checking') }}</span>
      </div>

      <div v-else class="tools">
        <div v-for="tool in tools" :key="tool.id" class="card tool">
          <div class="tool-status" :class="{ ok: tool.found }" aria-hidden="true">
            {{ tool.found ? '✓' : '✕' }}
          </div>
          <div class="tool-main">
            <div class="tool-head">
              <span class="tool-name">{{ tool.name }}</span>
              <span class="tool-state" :class="{ ok: tool.found }">
                {{ tool.found ? t('doctor.found') : t('doctor.missing') }}
              </span>
            </div>
            <div v-if="tool.found" class="tool-detail muted">
              <div v-if="tool.version">{{ tool.version }}</div>
              <code v-if="tool.path" class="tool-path">{{ tool.path }}</code>
            </div>
            <div v-else class="tool-hint">{{ tool.hint }}</div>
          </div>
        </div>
      </div>

      <button class="btn-link learn-more" type="button" @click="api.openUrl(DOCS.tools)">
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
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
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
.center {
  display: flex;
  gap: 0.6rem;
  align-items: center;
  justify-content: center;
  padding: 2rem;
}
.tools {
  display: flex;
  flex-direction: column;
  gap: 0.7rem;
}
.tool {
  display: flex;
  gap: 0.8rem;
  align-items: flex-start;
  padding: 0.9rem 1rem;
}
.tool-status {
  width: 26px;
  height: 26px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  font-weight: 800;
  color: var(--danger);
  background: var(--danger-bg);
  flex-shrink: 0;
}
.tool-status.ok {
  color: var(--ok);
  background: color-mix(in srgb, var(--ok) 15%, transparent);
}
.tool-main {
  flex: 1;
  min-width: 0;
}
.tool-head {
  display: flex;
  align-items: center;
  gap: 0.55rem;
}
.tool-name {
  font-weight: 650;
}
.tool-state {
  font-size: 0.72rem;
  color: var(--danger);
}
.tool-state.ok {
  color: var(--ok);
}
.tool-detail {
  margin-top: 0.25rem;
  font-size: 0.82rem;
}
.tool-path {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.76rem;
  overflow-wrap: anywhere;
}
.tool-hint {
  margin-top: 0.25rem;
  font-size: 0.83rem;
  color: var(--txt-secondary);
}
.learn-more {
  display: block;
  margin-top: 1rem;
  font-size: 0.88rem;
  font-weight: 550;
}
</style>
