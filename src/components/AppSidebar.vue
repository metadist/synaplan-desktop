<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '@/stores/config'
import { useUiStore, type View } from '@/stores/ui'
import { openUrl } from '@/services/tauri'
import { DOCS } from '@/constants'
import ProjectSwitcher from '@/components/ProjectSwitcher.vue'

const { t } = useI18n()
const config = useConfigStore()
const ui = useUiStore()

interface NavItem {
  id: View
  label: string
  icon: string
}

/** The five project views — never more (05_ux_and_i18n.md §1). */
const projectItems = computed<NavItem[]>(() => [
  { id: 'chat', label: t('nav.chat'), icon: 'chat' },
  { id: 'notes', label: t('nav.notes'), icon: 'notes' },
  { id: 'files', label: t('nav.files'), icon: 'files' },
  { id: 'agents', label: t('nav.agents'), icon: 'agents' },
  { id: 'models', label: t('nav.models'), icon: 'models' },
])

/** Machine-level screens live in the footer, not on the rail. */
const machineItems = computed<NavItem[]>(() => [
  { id: 'computer', label: t('nav.computer'), icon: 'computer' },
  { id: 'doctor', label: t('nav.doctor'), icon: 'doctor' },
  { id: 'skills', label: t('nav.skills'), icon: 'skills' },
])
</script>

<template>
  <aside class="sidebar">
    <div class="brand">
      <span class="brand-mark">S</span>
      <span class="brand-name">{{ t('app.name') }}</span>
    </div>

    <ProjectSwitcher />

    <nav class="nav" :aria-label="t('nav.projectSection')">
      <button
        v-for="item in projectItems"
        :key="item.id"
        class="nav-item"
        :class="{ active: ui.view === item.id }"
        type="button"
        :data-testid="`nav-${item.id}`"
        @click="ui.setView(item.id)"
      >
        <span class="nav-icon" :data-icon="item.icon" aria-hidden="true"></span>
        <span class="nav-label">{{ item.label }}</span>
      </button>
    </nav>

    <div class="sidebar-footer">
      <nav class="nav machine" :aria-label="t('nav.machineSection')">
        <span class="section-kicker">{{ t('nav.machineSection') }}</span>
        <button
          v-for="item in machineItems"
          :key="item.id"
          class="nav-item small"
          :class="{ active: ui.view === item.id }"
          type="button"
          :data-testid="`nav-${item.id}`"
          @click="ui.setView(item.id)"
        >
          <span class="nav-icon" :data-icon="item.icon" aria-hidden="true"></span>
          <span class="nav-label">{{ item.label }}</span>
        </button>
      </nav>
      <button class="btn-link docs-link" type="button" @click="openUrl(DOCS.overview)">
        {{ t('common.documentation') }}
      </button>
      <div class="conn">
        <span class="dot" :class="{ ok: config.paired }"></span>
        <span class="conn-url" :title="config.apiBaseUrl ?? ''">{{ config.apiBaseUrl }}</span>
      </div>
      <p v-if="config.pollStatus?.plaintextBlocked" class="poll-foot">
        {{ t('computer.pollPlaintext') }}
      </p>
      <p v-else-if="config.pollStatus?.lastCheckinUnix" class="poll-foot">
        {{
          t('computer.pollLast', {
            time: new Date(config.pollStatus.lastCheckinUnix * 1000).toLocaleString(),
          })
        }}
      </p>
      <p v-else-if="config.paired" class="poll-foot">{{ t('computer.pollNever') }}</p>
      <button class="btn btn-ghost btn-block" type="button" @click="config.signOut()">
        {{ t('status.signOut') }}
      </button>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  width: 224px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg-card);
  border-right: 1px solid var(--border);
  padding: 0.9rem 0.7rem;
  overflow-y: auto;
}

.brand {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  padding: 0.2rem 0.4rem 0.9rem;
}

.brand-mark {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  border-radius: 8px;
  background: var(--accent);
  color: var(--accent-contrast);
  font-weight: 800;
}

.brand-name {
  font-weight: 700;
  letter-spacing: -0.01em;
}

.nav {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.nav:not(.machine) {
  flex: 1;
}

.section-kicker {
  font-size: 0.68rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--txt-secondary);
  padding: 0.2rem 0.6rem 0.1rem;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.55rem 0.6rem;
  border: none;
  background: transparent;
  border-radius: 8px;
  color: var(--txt-secondary);
  font: inherit;
  font-weight: 550;
  cursor: pointer;
  text-align: left;
}

.nav-item.small {
  padding: 0.4rem 0.6rem;
  font-size: 0.85rem;
}

.nav-item:hover {
  background: var(--bg-elevated);
  color: var(--txt);
}

.nav-item.active {
  background: var(--accent-soft);
  color: var(--accent);
}

.nav-icon {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
  background-color: currentColor;
  -webkit-mask-repeat: no-repeat;
  mask-repeat: no-repeat;
  -webkit-mask-position: center;
  mask-position: center;
}

.nav-item.small .nav-icon {
  width: 15px;
  height: 15px;
}

/* Simple inline mask icons so we avoid an icon dependency. */
.nav-icon[data-icon='chat'] {
  -webkit-mask-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><path d='M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z'/></svg>");
  mask-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><path d='M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z'/></svg>");
}
.nav-icon[data-icon='notes'] {
  -webkit-mask-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><path d='M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z'/><path d='M14 2v6h6M8 13h8M8 17h5'/></svg>");
  mask-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><path d='M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z'/><path d='M14 2v6h6M8 13h8M8 17h5'/></svg>");
}
.nav-icon[data-icon='files'] {
  -webkit-mask-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><path d='M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z'/></svg>");
  mask-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><path d='M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z'/></svg>");
}
.nav-icon[data-icon='agents'] {
  -webkit-mask-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><circle cx='12' cy='8' r='4'/><path d='M4 21a8 8 0 0 1 16 0'/></svg>");
  mask-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><circle cx='12' cy='8' r='4'/><path d='M4 21a8 8 0 0 1 16 0'/></svg>");
}
.nav-icon[data-icon='models'] {
  -webkit-mask-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><path d='M12 2 2 7l10 5 10-5z'/><path d='M2 17l10 5 10-5M2 12l10 5 10-5'/></svg>");
  mask-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><path d='M12 2 2 7l10 5 10-5z'/><path d='M2 17l10 5 10-5M2 12l10 5 10-5'/></svg>");
}
.nav-icon[data-icon='skills'] {
  -webkit-mask-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><polygon points='12 2 15 9 22 9 16 14 18 21 12 17 6 21 8 14 2 9 9 9'/></svg>");
  mask-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><polygon points='12 2 15 9 22 9 16 14 18 21 12 17 6 21 8 14 2 9 9 9'/></svg>");
}
.nav-icon[data-icon='computer'] {
  -webkit-mask-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><rect x='2' y='3' width='20' height='14' rx='2'/><path d='M8 21h8M12 17v4'/></svg>");
  mask-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><rect x='2' y='3' width='20' height='14' rx='2'/><path d='M8 21h8M12 17v4'/></svg>");
}
.nav-icon[data-icon='doctor'] {
  -webkit-mask-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><path d='M22 12h-4l-3 9L9 3l-3 9H2'/></svg>");
  mask-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><path d='M22 12h-4l-3 9L9 3l-3 9H2'/></svg>");
}

.sidebar-footer {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding-top: 0.6rem;
  border-top: 1px solid var(--border);
}

.conn {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.75rem;
  color: var(--txt-secondary);
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--border-strong);
  flex-shrink: 0;
}
.dot.ok {
  background: var(--ok);
}

.conn-url {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.btn-block {
  width: 100%;
}

.docs-link {
  font-size: 0.78rem;
  text-align: left;
  padding: 0 0.6rem;
}

.poll-foot {
  margin: 0;
  font-size: 0.7rem;
  color: var(--txt-secondary);
  line-height: 1.35;
}
</style>
