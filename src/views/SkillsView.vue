<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import * as api from '@/services/tauri'
import { useErrorText } from '@/composables/useErrorText'
import { useUiStore } from '@/stores/ui'
import { DOCS, FIND_SKILLS_URL } from '@/constants'

type InstallKind = 'folder' | 'zip' | 'url'

const { t } = useI18n()
const errorText = useErrorText()
const ui = useUiStore()

const skills = ref<api.Skill[]>([])
const tools = ref<api.Tool[]>([])
const error = ref('')
const busy = ref(false)
const installKind = ref<InstallKind | null>(null)
const installValue = ref('')
const preview = ref<api.InstallPreview | null>(null)
const pendingSource = ref('')
const confirmRemove = ref<api.Skill | null>(null)
const confirmUnattended = ref<api.Skill | null>(null)

const pythonMissing = computed(() => !tools.value.find((tool) => tool.id === 'python')?.found)
const hasEnabled = computed(() => skills.value.some((s) => s.enabled && !s.blocked))

onMounted(load)

async function load(): Promise<void> {
  try {
    skills.value = await api.listSkills()
  } catch (e) {
    error.value = errorText(e)
  }
  try {
    tools.value = await api.runDoctor()
  } catch {
    // Doctor is best-effort here.
  }
}

function readiness(skill: api.Skill): string {
  if (skill.blocked && skill.blockedReason) {
    return skill.blockedReason
  }
  const missing: string[] = []
  if (skill.needsPython && pythonMissing.value) {
    missing.push(t('skills.needsPython'))
  }
  if (skill.needsNode && !tools.value.find((tool) => tool.id === 'node')?.found) {
    missing.push(t('skills.needsNode'))
  }
  if (skill.needsLibreoffice && !tools.value.find((tool) => tool.id === 'libreoffice')?.found) {
    missing.push(t('skills.needsLibreoffice'))
  }
  if (missing.length) {
    return missing.join(' · ')
  }
  return t('skills.ready')
}

async function toggle(skill: api.Skill): Promise<void> {
  error.value = ''
  try {
    skills.value = await api.setSkillEnabled(skill.name, !skill.enabled)
  } catch (e) {
    error.value = errorText(e)
  }
}

function startInstall(kind: InstallKind): void {
  installKind.value = kind
  installValue.value = ''
  preview.value = null
  error.value = ''
}

async function runPreview(): Promise<void> {
  const value = installValue.value.trim()
  if (!value || !installKind.value || busy.value) {
    return
  }
  error.value = ''
  busy.value = true
  try {
    if (installKind.value === 'folder') {
      preview.value = await api.previewSkillFolder(value)
    } else if (installKind.value === 'zip') {
      preview.value = await api.previewSkillZip(value)
    } else {
      preview.value = await api.previewSkillUrl(value)
    }
    pendingSource.value = value
  } catch (e) {
    error.value = errorText(e)
    preview.value = null
  } finally {
    busy.value = false
  }
}

async function confirmInstall(): Promise<void> {
  if (!preview.value || !installKind.value || busy.value) {
    return
  }
  error.value = ''
  busy.value = true
  try {
    if (installKind.value === 'folder') {
      skills.value = await api.installSkillFromFolder(pendingSource.value)
    } else if (installKind.value === 'zip') {
      skills.value = await api.installSkillFromZip(pendingSource.value)
    } else {
      skills.value = await api.installSkillFromUrl(pendingSource.value, preview.value.cachePath)
    }
    skills.value = await api.setSkillEnabled(preview.value.name, true)
    cancelInstall()
  } catch (e) {
    error.value = errorText(e)
  } finally {
    busy.value = false
  }
}

function cancelInstall(): void {
  installKind.value = null
  installValue.value = ''
  preview.value = null
  pendingSource.value = ''
}

async function doRemove(): Promise<void> {
  const skill = confirmRemove.value
  if (!skill) {
    return
  }
  error.value = ''
  try {
    skills.value = await api.removeSkill(skill.name)
    confirmRemove.value = null
  } catch (e) {
    error.value = errorText(e)
  }
}

async function doUnattended(allow: boolean): Promise<void> {
  const skill = confirmUnattended.value
  if (!skill) {
    return
  }
  error.value = ''
  try {
    skills.value = await api.setSkillUnattended(skill.name, allow)
    confirmUnattended.value = null
  } catch (e) {
    error.value = errorText(e)
  }
}

function askUnattended(skill: api.Skill): void {
  if (skill.allowUnattended) {
    void api.setSkillUnattended(skill.name, false).then((next) => {
      skills.value = next
    })
    return
  }
  confirmUnattended.value = skill
}

function placeholder(): string {
  if (installKind.value === 'folder') {
    return t('skills.folderPlaceholder')
  }
  if (installKind.value === 'zip') {
    return t('skills.zipPlaceholder')
  }
  return t('skills.urlPlaceholder')
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

      <div v-if="pythonMissing && hasEnabled" class="banner banner-warn python-warn" role="status">
        <span>{{ t('skills.pythonMissing') }}</span>
        <button class="btn-link" type="button" @click="ui.setView('doctor')">
          {{ t('skills.openDoctor') }} →
        </button>
      </div>

      <div class="install-actions">
        <button class="btn btn-secondary" type="button" @click="startInstall('folder')">
          {{ t('skills.fromFolder') }}
        </button>
        <button class="btn btn-secondary" type="button" @click="startInstall('zip')">
          {{ t('skills.fromZip') }}
        </button>
        <button class="btn btn-secondary" type="button" @click="startInstall('url')">
          {{ t('skills.fromUrl') }}
        </button>
      </div>

      <div v-if="installKind && !preview" class="card install-box">
        <label class="install-label" :for="'skill-source'">{{ t('skills.sourceLabel') }}</label>
        <div class="add-row">
          <input
            id="skill-source"
            v-model="installValue"
            class="input"
            type="text"
            :placeholder="placeholder()"
            @keydown.enter="runPreview"
          />
          <button
            class="btn btn-primary"
            type="button"
            :disabled="busy || !installValue.trim()"
            @click="runPreview"
          >
            {{ busy ? t('skills.checking') : t('skills.preview') }}
          </button>
          <button class="btn btn-ghost" type="button" @click="cancelInstall">
            {{ t('common.cancel') }}
          </button>
        </div>
      </div>

      <p v-if="skills.length === 0" class="muted empty">{{ t('skills.empty') }}</p>

      <div v-for="skill in skills" :key="skill.name" class="card skill">
        <div class="skill-main">
          <div class="skill-head">
            <span class="skill-name">{{ skill.name }}</span>
            <span class="tag">{{
              skill.bundled ? t('skills.included') : t('skills.installedByYou')
            }}</span>
            <span v-if="skill.blocked" class="tag blocked">{{ t('skills.blocked') }}</span>
          </div>
          <p class="skill-desc muted">{{ skill.description }}</p>
          <p v-if="skill.license" class="skill-note">
            {{ t('skills.license', { license: skill.license }) }}
          </p>
          <p class="skill-note">{{ t('skills.mayRunPrograms') }}</p>
          <p class="skill-note">{{ readiness(skill) }}</p>
          <p v-if="skill.compatibilityWarning" class="skill-warn">
            {{ t('skills.otherAssistant') }}
          </p>
          <div class="skill-actions">
            <label class="toggle">
              <input
                type="checkbox"
                :checked="skill.allowUnattended"
                @change="askUnattended(skill)"
              />
              <span>{{ t('skills.unattended') }}</span>
            </label>
            <button
              v-if="!skill.bundled"
              class="btn-link danger"
              type="button"
              @click="confirmRemove = skill"
            >
              {{ t('skills.remove') }}
            </button>
          </div>
        </div>
        <label class="toggle">
          <input type="checkbox" :checked="skill.enabled" @change="toggle(skill)" />
          <span>{{ skill.enabled ? t('skills.enabled') : t('skills.disabled') }}</span>
        </label>
      </div>

      <button class="btn-link learn-more" type="button" @click="api.openUrl(FIND_SKILLS_URL)">
        {{ t('skills.findPublic') }} →
      </button>
      <button class="btn-link learn-more" type="button" @click="api.openUrl(DOCS.skills)">
        {{ t('common.learnMore') }} →
      </button>
    </div>

    <div v-if="preview" class="consent-overlay" role="dialog" aria-modal="true">
      <div class="consent-card">
        <h2 class="consent-title">{{ t('skills.confirmTitle', { name: preview.name }) }}</h2>
        <p class="consent-body">{{ preview.description }}</p>
        <ul class="consent-points">
          <li>{{ t('skills.confirmPoint1') }}</li>
          <li>{{ t('skills.confirmPoint2') }}</li>
          <li>{{ t('skills.confirmPoint3') }}</li>
        </ul>
        <p v-if="preview.license" class="skill-note">
          {{ t('skills.license', { license: preview.license }) }}
        </p>
        <p v-if="preview.compatibilityWarning" class="skill-warn">
          {{ t('skills.otherAssistant') }}
        </p>
        <p class="file-list-label">{{ t('skills.fileList') }}</p>
        <ul class="file-list">
          <li v-for="file in preview.files" :key="file">{{ file }}</li>
        </ul>
        <div class="consent-actions">
          <button class="btn btn-ghost" type="button" :disabled="busy" @click="cancelInstall">
            {{ t('common.cancel') }}
          </button>
          <button class="btn btn-primary" type="button" :disabled="busy" @click="confirmInstall">
            {{ t('skills.confirmInstall') }}
          </button>
        </div>
      </div>
    </div>

    <div v-if="confirmRemove" class="consent-overlay" role="dialog" aria-modal="true">
      <div class="consent-card">
        <h2 class="consent-title">{{ t('skills.removeTitle', { name: confirmRemove.name }) }}</h2>
        <p class="consent-body">{{ t('skills.removeBody') }}</p>
        <div class="consent-actions">
          <button class="btn btn-ghost" type="button" @click="confirmRemove = null">
            {{ t('common.cancel') }}
          </button>
          <button class="btn btn-danger" type="button" @click="doRemove">
            {{ t('skills.remove') }}
          </button>
        </div>
      </div>
    </div>

    <div v-if="confirmUnattended" class="consent-overlay" role="dialog" aria-modal="true">
      <div class="consent-card">
        <h2 class="consent-title">
          {{ t('skills.unattendedTitle', { name: confirmUnattended.name }) }}
        </h2>
        <p class="consent-body">{{ t('skills.unattendedBody') }}</p>
        <div class="consent-actions">
          <button class="btn btn-ghost" type="button" @click="confirmUnattended = null">
            {{ t('common.cancel') }}
          </button>
          <button class="btn btn-primary" type="button" @click="doUnattended(true)">
            {{ t('skills.unattendedAllow') }}
          </button>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.view {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  position: relative;
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

.python-warn {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.8rem;
  margin-bottom: 1rem;
}

.empty {
  font-size: 0.9rem;
}

.install-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  margin-bottom: 0.9rem;
}

.install-box {
  padding: 0.9rem 1rem;
  margin-bottom: 0.9rem;
}

.install-label {
  display: block;
  font-size: 0.8rem;
  font-weight: 650;
  color: var(--txt-secondary);
  margin-bottom: 0.45rem;
}

.add-row {
  display: flex;
  gap: 0.5rem;
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
  flex-wrap: wrap;
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

.tag.blocked {
  background: var(--danger-bg);
  color: var(--danger);
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

.skill-warn {
  margin: 0.25rem 0 0;
  font-size: 0.78rem;
  color: var(--danger);
}

.skill-actions {
  display: flex;
  align-items: center;
  gap: 0.9rem;
  margin-top: 0.55rem;
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

.btn-link.danger {
  color: var(--danger);
}

.learn-more {
  display: block;
  margin-top: 0.55rem;
  font-size: 0.88rem;
  font-weight: 550;
}

.consent-overlay {
  position: absolute;
  inset: 0;
  background: color-mix(in srgb, var(--bg) 70%, transparent);
  backdrop-filter: blur(3px);
  display: grid;
  place-items: center;
  padding: 1.5rem;
  z-index: 20;
}

.consent-card {
  max-width: 460px;
  width: 100%;
  max-height: 90%;
  overflow-y: auto;
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
}

.consent-points {
  text-align: left;
  margin: 0 0 0.8rem;
  padding-left: 1.1rem;
}

.file-list-label {
  margin: 0.6rem 0 0.25rem;
  font-size: 0.78rem;
  font-weight: 650;
  color: var(--txt-secondary);
}

.file-list {
  margin: 0 0 1rem;
  padding-left: 1.1rem;
  font-size: 0.78rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  max-height: 8rem;
  overflow-y: auto;
}

.consent-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
}
</style>
