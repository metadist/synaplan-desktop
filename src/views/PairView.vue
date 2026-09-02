<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import * as api from '@/services/tauri'
import { useConfigStore } from '@/stores/config'
import { useErrorText } from '@/composables/useErrorText'

const { t } = useI18n()
const config = useConfigStore()
const errorText = useErrorText()

const DEFAULT_DEV_URL = 'http://localhost:8000'

// Dev-only convenience: pre-fill the address so a developer does not retype
// their local instance each run. Never the runtime source for a shipped build.
const address = ref(
  (import.meta.env.VITE_SYNAPLAN_DEV_URL || (import.meta.env.DEV ? DEFAULT_DEV_URL : '')).trim(),
)
const code = ref('')
const name = ref('')
const apiKey = ref('')
const useKey = ref(false)
const submitting = ref(false)
const error = ref('')

onMounted(async () => {
  try {
    name.value = await api.defaultDeviceName()
  } catch {
    // Leave empty; the field has a friendly placeholder.
  }
})

async function submitPair(): Promise<void> {
  error.value = ''
  submitting.value = true
  try {
    const status = await api.pair(address.value, code.value, name.value)
    config.setStatus(status)
  } catch (e) {
    error.value = errorText(e)
  } finally {
    submitting.value = false
  }
}

async function submitKey(): Promise<void> {
  error.value = ''
  submitting.value = true
  try {
    const status = await api.pairWithKey(address.value, apiKey.value)
    config.setStatus(status)
  } catch (e) {
    error.value = errorText(e)
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <div class="pair-wrap">
    <div class="card pair-card">
      <h1>{{ t('pair.title') }}</h1>
      <p class="muted intro">{{ t('pair.intro') }}</p>

      <p v-if="error" class="banner banner-error" role="alert">{{ error }}</p>

      <div class="mode" role="tablist">
        <button
          class="mode-btn"
          type="button"
          role="tab"
          :aria-selected="!useKey"
          :class="{ active: !useKey }"
          @click="useKey = false"
        >
          {{ t('pair.modeCode') }}
        </button>
        <button
          class="mode-btn"
          type="button"
          role="tab"
          :aria-selected="useKey"
          :class="{ active: useKey }"
          data-testid="tab-use-key"
          @click="useKey = true"
        >
          {{ t('pair.modeKey') }}
        </button>
      </div>

      <form v-if="!useKey" @submit.prevent="submitPair">
        <div class="field">
          <label class="label" for="address">{{ t('pair.addressLabel') }}</label>
          <input
            id="address"
            v-model="address"
            class="input"
            type="text"
            inputmode="url"
            autocomplete="off"
            :placeholder="t('pair.addressPlaceholder')"
          />
          <span class="muted hint">{{ t('pair.addressHint') }}</span>
        </div>
        <div class="field">
          <label class="label" for="code">{{ t('pair.codeLabel') }}</label>
          <input
            id="code"
            v-model="code"
            class="input code-input"
            type="text"
            autocomplete="off"
            autocapitalize="characters"
            spellcheck="false"
            :placeholder="t('pair.codePlaceholder')"
          />
        </div>
        <div class="field">
          <label class="label" for="name">{{ t('pair.nameLabel') }}</label>
          <input
            id="name"
            v-model="name"
            class="input"
            type="text"
            :placeholder="t('pair.namePlaceholder')"
          />
        </div>
        <button class="btn btn-primary submit" type="submit" :disabled="submitting">
          <span v-if="submitting" class="spinner"></span>
          <span>{{ submitting ? t('pair.submitting') : t('pair.submit') }}</span>
        </button>
      </form>

      <form v-else data-testid="form-use-key" @submit.prevent="submitKey">
        <div class="field">
          <label class="label" for="address2">{{ t('pair.addressLabel') }}</label>
          <input
            id="address2"
            v-model="address"
            class="input"
            type="text"
            inputmode="url"
            autocomplete="off"
            :placeholder="t('pair.addressPlaceholder')"
          />
          <span class="muted hint">{{ t('pair.addressHint') }}</span>
        </div>
        <div class="field">
          <label class="label" for="key">{{ t('pair.keyLabel') }}</label>
          <input
            id="key"
            v-model="apiKey"
            class="input"
            type="password"
            autocomplete="off"
            spellcheck="false"
            :placeholder="t('pair.keyPlaceholder')"
          />
          <span class="muted hint">{{ t('pair.keyHint') }}</span>
        </div>
        <button class="btn btn-primary submit" type="submit" :disabled="submitting">
          <span v-if="submitting" class="spinner"></span>
          <span>{{ submitting ? t('pair.submitting') : t('pair.keySubmit') }}</span>
        </button>
      </form>
    </div>
  </div>
</template>

<style scoped>
.pair-wrap {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
  overflow: auto;
}

.pair-card {
  width: 100%;
  max-width: 420px;
  padding: 1.6rem;
}

.intro {
  margin: 0.4rem 0 1.2rem;
  font-size: 0.88rem;
}

.mode {
  display: flex;
  gap: 0.35rem;
  margin: 0 0 1rem;
  padding: 0.2rem;
  border-radius: 10px;
  background: color-mix(in srgb, var(--border) 55%, transparent);
}

.mode-btn {
  flex: 1;
  border: 0;
  background: transparent;
  color: var(--muted, #64748b);
  font: inherit;
  font-size: 0.82rem;
  font-weight: 600;
  padding: 0.45rem 0.5rem;
  border-radius: 8px;
  cursor: pointer;
}

.mode-btn.active {
  background: var(--surface, #fff);
  color: var(--text, #0f172a);
  box-shadow: 0 1px 2px rgb(15 23 42 / 8%);
}

.code-input {
  letter-spacing: 0.12em;
  text-transform: uppercase;
  font-variant-numeric: tabular-nums;
}

.submit {
  width: 100%;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  margin-top: 0.3rem;
}

.hint {
  font-size: 0.76rem;
}
</style>
