<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '@/stores/config'
import { useUiStore, type View } from '@/stores/ui'
import { openUrl } from '@/services/tauri'
import { DOCS } from '@/constants'
import ProjectSwitcher from '@/components/ProjectSwitcher.vue'
import birdDark from '@/assets/single_bird-dark.svg'
import birdLight from '@/assets/single_bird-light.svg'

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
  { id: 'settings', label: t('nav.settings'), icon: 'settings' },
])

const collapsed = computed(() => ui.sidebarCollapsed)
const toggleLabel = computed(() => (collapsed.value ? t('nav.expand') : t('nav.collapse')))
</script>

<template>
  <aside class="sidebar" :class="{ collapsed }" data-testid="sidebar">
    <div class="brand">
      <picture class="brand-mark">
        <source media="(prefers-color-scheme: dark)" :srcset="birdLight" />
        <img :src="birdDark" :alt="t('app.name')" width="28" height="34" />
      </picture>
      <span v-if="!collapsed" class="brand-name">{{ t('app.name') }}</span>
      <button
        class="fold"
        type="button"
        :title="toggleLabel"
        :aria-label="toggleLabel"
        :aria-expanded="!collapsed"
        data-testid="sidebar-toggle"
        @click="ui.toggleSidebar()"
      >
        <span
          class="nav-icon"
          :data-icon="collapsed ? 'expand' : 'collapse'"
          aria-hidden="true"
        ></span>
      </button>
    </div>

    <ProjectSwitcher :compact="collapsed" />

    <nav class="nav" :aria-label="t('nav.projectSection')">
      <button
        v-for="item in projectItems"
        :key="item.id"
        class="nav-item"
        :class="{ active: ui.view === item.id }"
        type="button"
        :title="collapsed ? item.label : undefined"
        :aria-label="item.label"
        :data-testid="`nav-${item.id}`"
        @click="ui.setView(item.id)"
      >
        <span class="nav-icon" :data-icon="item.icon" aria-hidden="true"></span>
        <span v-if="!collapsed" class="nav-label">{{ item.label }}</span>
      </button>
    </nav>

    <div class="sidebar-footer">
      <nav class="nav machine" :aria-label="t('nav.machineSection')">
        <span v-if="!collapsed" class="section-kicker">{{ t('nav.machineSection') }}</span>
        <button
          v-for="item in machineItems"
          :key="item.id"
          class="nav-item small"
          :class="{ active: ui.view === item.id }"
          type="button"
          :title="collapsed ? item.label : undefined"
          :aria-label="item.label"
          :data-testid="`nav-${item.id}`"
          @click="ui.setView(item.id)"
        >
          <span class="nav-icon" :data-icon="item.icon" aria-hidden="true"></span>
          <span v-if="!collapsed" class="nav-label">{{ item.label }}</span>
        </button>
      </nav>
      <button
        v-if="!collapsed"
        class="btn-link docs-link"
        type="button"
        @click="openUrl(DOCS.overview)"
      >
        {{ t('common.documentation') }}
      </button>
      <button
        class="conn"
        type="button"
        :title="config.apiBaseUrl ?? ''"
        :aria-label="t('nav.settings')"
        @click="ui.setView('settings')"
      >
        <span class="dot" :class="{ ok: config.paired }"></span>
        <span v-if="!collapsed" class="conn-url">{{ config.apiBaseUrl }}</span>
      </button>
      <template v-if="!collapsed">
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
      </template>
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
  overflow-x: hidden;
  transition: width 0.16s ease;
}

.sidebar.collapsed {
  width: 60px;
  padding: 0.9rem 0.5rem;
  align-items: center;
}

.brand {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  padding: 0.2rem 0.4rem 0.9rem;
}

.collapsed .brand {
  flex-direction: column;
  gap: 0.3rem;
  padding: 0 0 0.6rem;
}

.brand-mark {
  display: grid;
  place-items: center;
  width: 28px;
  height: 34px;
  flex-shrink: 0;
}

.brand-mark img {
  display: block;
  width: 28px;
  height: 34px;
}

.fold {
  margin-left: auto;
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--txt-secondary);
  cursor: pointer;
}

.collapsed .fold {
  margin-left: 0;
}

.fold:hover {
  background: var(--bg-elevated);
  color: var(--txt);
}

.fold .nav-icon {
  width: 15px;
  height: 15px;
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

.collapsed .nav {
  width: 100%;
  align-items: center;
}

.collapsed .nav-item {
  width: 40px;
  height: 40px;
  justify-content: center;
  padding: 0;
}

.collapsed .nav-item.small {
  width: 36px;
  height: 36px;
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
.nav-icon[data-icon='settings'] {
  -webkit-mask-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><circle cx='12' cy='12' r='3'/><path d='M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z'/></svg>");
  mask-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><circle cx='12' cy='12' r='3'/><path d='M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z'/></svg>");
}
.nav-icon[data-icon='collapse'] {
  -webkit-mask-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2.2' stroke-linecap='round' stroke-linejoin='round'><path d='M11 17l-5-5 5-5M18 17l-5-5 5-5'/></svg>");
  mask-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2.2' stroke-linecap='round' stroke-linejoin='round'><path d='M11 17l-5-5 5-5M18 17l-5-5 5-5'/></svg>");
}
.nav-icon[data-icon='expand'] {
  -webkit-mask-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2.2' stroke-linecap='round' stroke-linejoin='round'><path d='M13 17l5-5-5-5M6 17l5-5-5-5'/></svg>");
  mask-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2.2' stroke-linecap='round' stroke-linejoin='round'><path d='M13 17l5-5-5-5M6 17l5-5-5-5'/></svg>");
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
  min-width: 0;
  padding: 0.25rem 0.6rem;
  border: none;
  border-radius: 8px;
  background: transparent;
  font: inherit;
  font-size: 0.75rem;
  color: var(--txt-secondary);
  cursor: pointer;
  text-align: left;
}

.conn:hover {
  background: var(--bg-elevated);
  color: var(--txt);
}

.collapsed .conn {
  justify-content: center;
  padding: 0.4rem;
}

.collapsed .sidebar-footer {
  width: 100%;
  align-items: center;
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
