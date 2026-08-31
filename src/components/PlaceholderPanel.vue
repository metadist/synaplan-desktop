<script setup lang="ts">
import { useI18n } from 'vue-i18n'

defineProps<{ title: string; body: string }>()

const { t } = useI18n()
</script>

<template>
  <section class="panel">
    <header class="panel-header">
      <h1>{{ title }}</h1>
      <span class="badge">{{ t('common.inDevelopment') }}</span>
    </header>
    <div class="panel-body">
      <p class="muted lead">{{ body }}</p>
      <slot />
    </div>
  </section>
</template>

<style scoped>
.panel {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.panel-header {
  display: flex;
  align-items: center;
  gap: 0.7rem;
  padding: 0.9rem 1.2rem;
  border-bottom: 1px solid var(--border);
}

.badge {
  font-size: 0.7rem;
  font-weight: 600;
  padding: 0.15rem 0.5rem;
  border-radius: 999px;
  background: var(--accent-soft);
  color: var(--accent);
}

.panel-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 2rem;
  text-align: center;
}

.lead {
  max-width: 460px;
  font-size: 0.95rem;
}
</style>
