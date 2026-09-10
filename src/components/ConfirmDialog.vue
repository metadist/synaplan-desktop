<script setup lang="ts">
import { useI18n } from 'vue-i18n'

/** A small modal confirm. Destructive actions pass `danger` for the red button. */
defineProps<{
  title: string
  body: string
  confirmLabel: string
  danger?: boolean
  busy?: boolean
  error?: string
}>()

const emit = defineEmits<{
  cancel: []
  confirm: []
}>()

const { t } = useI18n()
</script>

<template>
  <Teleport to="body">
    <div class="consent-overlay" role="dialog" aria-modal="true" @click.self="emit('cancel')">
      <div class="consent-card">
        <h2 class="consent-title">{{ title }}</h2>
        <p class="consent-body">{{ body }}</p>

        <p v-if="error" class="banner banner-error" role="alert">{{ error }}</p>

        <div class="consent-actions">
          <button class="btn btn-ghost" type="button" :disabled="busy" @click="emit('cancel')">
            {{ t('common.cancel') }}
          </button>
          <button
            class="btn"
            :class="danger ? 'btn-danger' : 'btn-primary'"
            type="button"
            :disabled="busy"
            data-testid="confirm-dialog-confirm"
            @click="emit('confirm')"
          >
            {{ confirmLabel }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.consent-overlay {
  position: fixed;
  inset: 0;
  background: color-mix(in srgb, var(--bg) 70%, transparent);
  backdrop-filter: blur(3px);
  display: grid;
  place-items: center;
  padding: 1.5rem;
  z-index: 30;
}
.consent-card {
  max-width: 440px;
  width: 100%;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: calc(var(--radius) * 1.4);
  padding: 1.6rem;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.28);
}
.consent-title {
  margin: 0 0 0.4rem;
  font-size: 1.15rem;
}
.consent-body {
  margin: 0 0 0.9rem;
  color: var(--txt-secondary);
  font-size: 0.9rem;
  white-space: pre-line;
}
.consent-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
  margin-top: 1rem;
}
</style>
