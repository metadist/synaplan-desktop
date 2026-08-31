<script setup lang="ts">
import { onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '@/stores/config'
import PairView from '@/views/PairView.vue'
import ChatView from '@/views/ChatView.vue'

const { t } = useI18n()
const config = useConfigStore()

onMounted(() => config.load())
</script>

<template>
  <header class="app-header">
    <div class="brand">
      <span class="brand-name">{{ t('app.name') }}</span>
      <span class="brand-tag muted">{{ t('app.tagline') }}</span>
    </div>
    <div v-if="config.paired" class="header-right">
      <span class="conn muted" :title="config.apiBaseUrl ?? ''">
        {{ t('status.connectedTo', { url: config.apiBaseUrl }) }}
      </span>
      <button class="btn btn-ghost" @click="config.signOut()">{{ t('status.signOut') }}</button>
    </div>
  </header>

  <p v-if="config.keyIsPlaintext" class="banner banner-warn plaintext">
    {{ t('status.plaintextWarning') }}
  </p>

  <main class="app-main">
    <div v-if="config.loading" class="center muted">
      <span class="spinner"></span>
      <span>{{ t('common.loading') }}</span>
    </div>
    <PairView v-else-if="!config.paired" />
    <ChatView v-else />
  </main>
</template>

<style scoped>
.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.7rem 1.1rem;
  border-bottom: 1px solid var(--border);
  background: var(--bg-card);
}

.brand {
  display: flex;
  flex-direction: column;
  line-height: 1.2;
}

.brand-name {
  font-weight: 700;
  letter-spacing: -0.01em;
}

.brand-tag {
  font-size: 0.75rem;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  min-width: 0;
}

.conn {
  font-size: 0.78rem;
  max-width: 260px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.plaintext {
  margin: 0.6rem 1.1rem 0;
}

.app-main {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.center {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.6rem;
}
</style>
