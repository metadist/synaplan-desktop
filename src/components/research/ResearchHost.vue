<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import ResearchComposer from '@/components/research/ResearchComposer.vue'
import ResearchPlanCard from '@/components/research/ResearchPlanCard.vue'
import ResearchProgressCard from '@/components/research/ResearchProgressCard.vue'
import ResearchQuestionCard from '@/components/research/ResearchQuestionCard.vue'
import ResearchReport from '@/components/research/ResearchReport.vue'
import ResearchResultCard from '@/components/research/ResearchResultCard.vue'
import SourceDrawer from '@/components/research/SourceDrawer.vue'
import { useErrorText } from '@/composables/useErrorText'
import type { ResearchExport } from '@/lib/researchExports'
import { exportInstruction } from '@/lib/researchExports'
import {
  RESEARCH_RUNNING,
  applyResearchEvent,
  applyResearchState,
  emptyResearch,
  isResearchDone,
  researchMinutes,
  type ResearchOutcome,
  type ResearchStateBody,
  type ResearchUi,
} from '@/lib/researchRun'
import * as api from '@/services/tauri'
import type { ResearchThreadRef, Skill } from '@/services/tauri'

const props = defineProps<{
  open: boolean
  projectId: string
  chatId: string
  webOn: boolean
  fileCount: number
  skills: Skill[]
  saved: ResearchThreadRef | null
}>()

const emit = defineEmits<{
  available: [value: boolean]
  export: [text: string]
  remember: [ref: ResearchThreadRef | null]
  doctor: []
}>()

type FollowUpKind = 'question' | 'revise' | 'deeper' | 'refresh'

const { t, te, locale } = useI18n()
const errorText = useErrorText()
const caps = ref<api.ResearchCapabilities | null>(null)
const ui = ref<ResearchUi>(emptyResearch())
const question = ref('')
const depth = ref('standard')
const useWeb = ref(true)
const useFiles = ref(false)
const busy = ref(false)
const error = ref('')
const reportOpen = ref(false)
const cited = ref<string | null>(null)
const versions = ref<number[]>([1])
const savedPath = ref('')
const startedAt = ref('')
let unlisten: (() => void) | null = null
let disposed = false

const running = computed(() => RESEARCH_RUNNING.includes(ui.value.status))
const waitingPlan = computed(() => ui.value.status === 'waiting_plan')
const done = computed(() => isResearchDone(ui.value.status))
const source = computed(() => ui.value.sources.find((item) => item.id === cited.value) ?? null)
const quotes = computed(() => ui.value.citations.filter((item) => item.source === cited.value))
const estimate = computed(() =>
  ui.value.estimateCost === null
    ? ''
    : new Intl.NumberFormat(locale.value, { style: 'currency', currency: 'USD' }).format(
        ui.value.estimateCost,
      ),
)

function depthLabel(value: string): string {
  return t(`research.depth.${value}`, { minutes: researchMinutes(value) })
}

const ROLE_KEYS: Record<string, string> = {
  lead: 'research.roleLead',
  reader: 'research.roleReader',
  checker: 'research.roleChecker',
}

function outcomeText(outcome: ResearchOutcome | null): string {
  if (!outcome?.code) {
    return outcome?.text ?? ''
  }
  const params = { ...outcome.params }
  if (typeof params.role === 'string' && ROLE_KEYS[params.role]) {
    params.role = t(ROLE_KEYS[params.role])
  }
  const key = `research.outcome.${outcome.code}`
  return te(key) ? t(key, params) : outcome.text
}

function newKey(): string {
  return `desk-${crypto.randomUUID()}`
}

function fail(failure: unknown): void {
  error.value = errorText(failure)
}

/** `true` while `runId` is still the run on the card (no chat switch, no new run). */
function current(runId: string): boolean {
  return !disposed && ui.value.runId === runId
}

async function probe(): Promise<void> {
  try {
    caps.value = await api.researchCapabilities()
  } catch {
    caps.value = null
  }
  emit('available', !!caps.value?.enabled)
}

function remember(): void {
  if (!ui.value.runId) {
    return
  }
  emit('remember', {
    runId: ui.value.runId,
    startedAt: startedAt.value || new Date().toISOString(),
    status: ui.value.status,
    reportPath: savedPath.value,
    version: ui.value.version,
  })
}

function reset(): void {
  ui.value = emptyResearch()
  error.value = ''
  reportOpen.value = false
  cited.value = null
  savedPath.value = ''
  startedAt.value = ''
  versions.value = [1]
}

/** Show the run saved on this chat: read its state, then follow it if it is still going. */
async function restore(saved: ResearchThreadRef): Promise<void> {
  ui.value = { ...emptyResearch(), runId: saved.runId, status: saved.status }
  savedPath.value = saved.reportPath
  startedAt.value = saved.startedAt
  try {
    const state = (await api.researchState(saved.runId)) as unknown as ResearchStateBody
    if (!current(saved.runId)) {
      return
    }
    ui.value = applyResearchState(ui.value, state)
  } catch (failure) {
    if (!current(saved.runId)) {
      return
    }
    // The server keeps a run for two days; the saved report stays on this computer.
    if (api.asCommandError(failure).code !== 'not_found') {
      fail(failure)
    }
    if (savedPath.value) {
      ui.value.status = isResearchDone(saved.status) ? saved.status : 'stopped'
    }
    return
  }
  if (done.value) {
    await finish(false)
  } else {
    await follow(saved.runId)
  }
}

async function follow(runId: string): Promise<void> {
  try {
    await api.researchAttach(runId)
  } catch (failure) {
    if (current(runId)) {
      fail(failure)
    }
  }
}

onMounted(async () => {
  unlisten = await api.onResearchEvent((event) => {
    if (!ui.value.runId || event.runId !== ui.value.runId) {
      return
    }
    if (event.name === 'detached') {
      error.value = errorText(event.data)
      return
    }
    const wasDone = done.value
    ui.value = applyResearchEvent(ui.value, event)
    if (done.value && !wasDone) {
      void settle(ui.value.runId)
    }
  })
  if (disposed) {
    unlisten()
    return
  }
  await probe()
  if (props.saved?.runId) {
    await restore(props.saved)
  }
})

onUnmounted(() => {
  disposed = true
  unlisten?.()
})

// Another chat (or project) is open: show its run, or nothing.
watch(
  () => [props.chatId, props.saved?.runId ?? ''] as const,
  ([, runId]) => {
    if (runId === ui.value.runId) {
      return
    }
    reset()
    if (runId && props.saved) {
      void restore(props.saved)
    }
  },
)

async function start(): Promise<void> {
  if (!props.projectId || busy.value) {
    return
  }
  busy.value = true
  error.value = ''
  try {
    const next = emptyResearch(question.value.trim(), depth.value)
    const ref = await api.researchStart({
      projectId: props.projectId,
      question: next.question,
      depth: depth.value,
      web: useWeb.value && !!caps.value?.webSearchAvailable,
      useFiles: useFiles.value && props.fileCount > 0,
      idempotencyKey: newKey(),
    })
    ui.value = { ...next, runId: ref.id, status: ref.status }
    savedPath.value = ''
    startedAt.value = new Date().toISOString()
    question.value = ''
    remember()
    void api.researchSetTray(t('research.progress', { depth: depthLabel(depth.value) }))
    await follow(ref.id)
  } catch (failure) {
    fail(failure)
  } finally {
    busy.value = false
  }
}

async function sendInput(input: Record<string, unknown>): Promise<void> {
  const runId = ui.value.runId
  if (!runId) {
    return
  }
  error.value = ''
  try {
    const state = (await api.researchInput(runId, input)) as unknown as ResearchStateBody
    if (current(runId)) {
      ui.value = applyResearchState(ui.value, state)
      remember()
    }
  } catch (failure) {
    if (current(runId)) {
      fail(failure)
    }
  }
}

async function stop(): Promise<void> {
  const runId = ui.value.runId
  if (!runId) {
    return
  }
  error.value = ''
  try {
    const state = (await api.researchCancel(runId)) as unknown as ResearchStateBody
    if (current(runId)) {
      ui.value = applyResearchState(ui.value, state)
      remember()
    }
  } catch (failure) {
    if (current(runId)) {
      fail(failure)
    }
  }
}

async function settle(runId: string): Promise<void> {
  try {
    const state = (await api.researchState(runId)) as unknown as ResearchStateBody
    if (!current(runId)) {
      return
    }
    ui.value = applyResearchState(ui.value, state)
  } catch {
    // The card still shows the outcome from the event.
  }
  if (current(runId)) {
    await finish(true)
  }
}

async function finish(notify: boolean): Promise<void> {
  const runId = ui.value.runId
  void api.researchSetTray('')
  if (ui.value.markdown && props.projectId) {
    try {
      const saved = await api.researchSaveReport({
        projectId: props.projectId,
        question: ui.value.question,
        version: ui.value.version,
        markdown: ui.value.markdown,
        sourcesJson: JSON.stringify({ sources: ui.value.sources, citations: ui.value.citations }),
      })
      const list = await api.researchListVersions(props.projectId, ui.value.question)
      if (!current(runId)) {
        return
      }
      savedPath.value = saved.path
      versions.value = list.length > 0 ? list : [ui.value.version]
    } catch (failure) {
      if (current(runId)) {
        fail(failure)
      }
    }
  }
  if (!current(runId)) {
    return
  }
  remember()
  if (notify && document.hidden && ui.value.outcome) {
    void api
      .researchNotify(outcomeText(ui.value.outcome), props.projectId, props.chatId)
      .catch(() => undefined)
  }
}

/** A follow-up is a new run that continues this report; the card follows it. */
async function followUp(kind: FollowUpKind, text: string | null): Promise<void> {
  const parent = ui.value.runId
  if (!parent || busy.value) {
    return
  }
  busy.value = true
  error.value = ''
  try {
    const ref = await api.researchFollowUp(parent, kind, text, newKey())
    if (!current(parent)) {
      return
    }
    const nextDepth = kind === 'deeper' ? 'thorough' : ui.value.depth
    ui.value = {
      ...emptyResearch(ui.value.question, nextDepth),
      runId: ref.id,
      status: ref.status,
      version: ui.value.version,
    }
    reportOpen.value = false
    cited.value = null
    startedAt.value = new Date().toISOString()
    remember()
    void api.researchSetTray(t('research.progress', { depth: depthLabel(nextDepth) }))
    await follow(ref.id)
  } catch (failure) {
    if (current(parent)) {
      fail(failure)
    }
  } finally {
    busy.value = false
  }
}

async function restoreVersion(version: number): Promise<void> {
  const runId = ui.value.runId
  try {
    const restored = await api.researchRestoreVersion(props.projectId, ui.value.question, version)
    if (!current(runId)) {
      return
    }
    ui.value = { ...ui.value, markdown: restored.markdown, version: restored.version }
    savedPath.value = restored.path
    remember()
  } catch (failure) {
    fail(failure)
  }
}

async function onExport(kind: ResearchExport): Promise<void> {
  if (!savedPath.value) {
    await finish(false)
  }
  if (savedPath.value) {
    emit('export', exportInstruction(kind, savedPath.value, ui.value.version))
  }
}

function copyQuote(text: string): void {
  void navigator.clipboard?.writeText(text)
}

async function openSource(): Promise<void> {
  if (source.value?.url) {
    await api.openUrl(source.value.url).catch(fail)
  }
}

async function removeReport(): Promise<void> {
  const runId = ui.value.runId
  if (runId) {
    try {
      await api.researchDelete(runId)
    } catch (failure) {
      if (api.asCommandError(failure).code !== 'not_found') {
        fail(failure)
        return
      }
    }
  }
  reset()
  emit('remember', null)
}

defineExpose({ probe })
</script>

<template>
  <div v-if="caps">
    <p v-if="error" class="banner banner-error research-error" role="alert">{{ error }}</p>
    <ResearchComposer
      v-if="open && !ui.runId"
      :question="question"
      :depth="depth"
      :web="useWeb"
      :files="useFiles"
      :web-available="caps.webSearchAvailable"
      :files-ready="fileCount"
      :depths="
        caps.depths.filter((item) => item === 'standard' || item === 'thorough' || item === 'quick')
      "
      :thorough-busy="false"
      :standard-left="caps.limits.standardPerDay"
      :thorough-left="caps.limits.thoroughPerDay"
      :busy="busy"
      @update:question="question = $event"
      @update:depth="depth = $event"
      @update:web="useWeb = $event"
      @update:files="useFiles = $event"
      @start="start"
    />
    <ResearchQuestionCard
      v-if="ui.questionCard"
      :question="ui.questionCard"
      @answer="sendInput({ type: 'answer', question: ui.questionCard?.id, ...$event })"
      @skip="sendInput({ type: 'skip_question', question: ui.questionCard?.id })"
    />
    <ResearchPlanCard
      v-if="waitingPlan || (ui.points.length > 0 && !running && !done && !ui.questionCard)"
      :points="ui.points"
      :editable="waitingPlan"
      :disclosures="ui.disclosures"
      :estimate="estimate"
      :file-count="fileCount"
      :web="useWeb"
      @start="sendInput({ type: 'approve_plan' })"
      @cancel="stop"
      @edit="sendInput({ type: 'edit_plan', points: $event })"
    />
    <ResearchProgressCard
      v-if="running"
      :depth-label="depthLabel(ui.depth)"
      :points="ui.points"
      :found="ui.found"
      :read="ui.read"
      :closed="ui.closed"
      :now="ui.now"
      :queued-since="ui.queuedSince"
      :details="ui.details"
      @stop="stop"
      @write-now="sendInput({ type: 'stop_gathering' })"
    />
    <ResearchResultCard
      v-if="done"
      :title="ui.question"
      :preview="ui.markdown.slice(0, 280)"
      :has-report="ui.markdown !== ''"
      :check="ui.check"
      :saved-path="savedPath"
      :outcome="outcomeText(ui.outcome)"
      :failed="ui.status !== 'complete'"
      :skills="skills"
      :version="ui.version"
      :busy="busy"
      @open="reportOpen = true"
      @export="onExport"
      @deeper="followUp('deeper', null)"
      @refresh="followUp('refresh', null)"
      @delete="removeReport"
      @doctor="emit('doctor')"
    />
    <ResearchReport
      v-if="reportOpen"
      :markdown="ui.markdown"
      :sources="ui.sources"
      :citations="ui.citations"
      :version="ui.version"
      :versions="versions"
      :busy="busy"
      @close="reportOpen = false"
      @cite="cited = $event"
      @ask="followUp('question', $event)"
      @deeper="followUp('deeper', null)"
      @restore="restoreVersion"
      @revise="followUp('revise', $event)"
    />
    <SourceDrawer
      :source="source"
      :quotes="quotes"
      :number="ui.sources.findIndex((item) => item.id === cited) + 1"
      @close="cited = null"
      @open-page="openSource"
      @copy-quote="copyQuote"
    />
  </div>
</template>

<style scoped>
.research-error {
  margin: 0.8rem 1.2rem 0;
}
</style>
