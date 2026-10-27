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
import type { ResearchExport } from '@/lib/researchExports'
import { exportInstruction } from '@/lib/researchExports'
import {
  applyResearchEvent,
  applyResearchState,
  emptyResearch,
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

const { t } = useI18n()
const caps = ref<api.ResearchCapabilities | null>(null)
const ui = ref<ResearchUi>(emptyResearch())
const question = ref('')
const depth = ref('standard')
const useWeb = ref(true)
const useFiles = ref(false)
const busy = ref(false)
const reportOpen = ref(false)
const cited = ref<string | null>(null)
const versions = ref<number[]>([1])
const savedPath = ref('')
let unlisten: (() => void) | null = null

const running = computed(() =>
  ['briefing', 'planning', 'gathering', 'writing', 'checking'].includes(ui.value.status),
)
const waitingPlan = computed(() => ui.value.status === 'waiting_plan')
const done = computed(() => ['complete', 'stopped', 'failed'].includes(ui.value.status))
const source = computed(() => ui.value.sources.find((item) => item.id === cited.value) ?? null)
const quotes = computed(() => ui.value.citations.filter((item) => item.source === cited.value))

async function probe(): Promise<void> {
  try {
    caps.value = await api.researchCapabilities()
  } catch {
    caps.value = null
  }
  emit('available', !!caps.value?.enabled)
}

async function attach(runId: string): Promise<void> {
  await api.researchAttach(runId)
}

onMounted(async () => {
  await probe()
  unlisten = await api.onResearchEvent((event) => {
    if (ui.value.runId && event.runId !== ui.value.runId) {
      return
    }
    ui.value = applyResearchEvent(ui.value, event)
    if (done.value) {
      void settle()
    }
  })
  if (props.saved?.runId && !['complete', 'stopped', 'failed'].includes(props.saved.status)) {
    ui.value.runId = props.saved.runId
    ui.value.status = props.saved.status
    savedPath.value = props.saved.reportPath
    await attach(props.saved.runId)
  }
})

onUnmounted(() => unlisten?.())

watch(
  () => props.saved?.runId,
  (runId) => {
    if (runId && runId !== ui.value.runId) {
      ui.value.runId = runId
      ui.value.status = props.saved?.status ?? ''
      void attach(runId)
    }
  },
)

function remember(): void {
  if (!ui.value.runId) {
    return
  }
  emit('remember', {
    runId: ui.value.runId,
    startedAt: new Date().toISOString(),
    status: ui.value.status,
    reportPath: savedPath.value,
    version: ui.value.version,
  })
}

async function start(): Promise<void> {
  if (!props.projectId) {
    return
  }
  busy.value = true
  try {
    const key = `desk-${Date.now()}`
    ui.value = emptyResearch(question.value.trim(), depth.value)
    const ref = await api.researchStart({
      projectId: props.projectId,
      question: ui.value.question,
      depth: depth.value,
      web: useWeb.value && !!caps.value?.webSearchAvailable,
      useFiles: useFiles.value && props.fileCount > 0,
      idempotencyKey: key,
    })
    ui.value.runId = ref.id
    ui.value.status = ref.status
    remember()
    await attach(ref.id)
    void api.researchSetTray(t('research.progress', { depth: depth.value }))
  } finally {
    busy.value = false
  }
}

async function sendInput(input: Record<string, unknown>): Promise<void> {
  if (!ui.value.runId) {
    return
  }
  const state = (await api.researchInput(ui.value.runId, input)) as Parameters<
    typeof applyResearchState
  >[1]
  ui.value = applyResearchState(ui.value, state)
  remember()
}

async function settle(): Promise<void> {
  try {
    const state = (await api.researchState(ui.value.runId)) as Parameters<
      typeof applyResearchState
    >[1]
    ui.value = applyResearchState(ui.value, state)
  } catch {
    // The card still shows the outcome from the event.
  }
  await finish()
}

async function finish(): Promise<void> {
  void api.researchSetTray('')
  if (ui.value.markdown && props.projectId) {
    const saved = await api.researchSaveReport({
      projectId: props.projectId,
      question: ui.value.question,
      version: ui.value.version,
      markdown: ui.value.markdown,
      sourcesJson: JSON.stringify({ sources: ui.value.sources, citations: ui.value.citations }),
    })
    savedPath.value = saved.path
    versions.value = await api.researchListVersions(props.projectId, ui.value.question)
  }
  remember()
  if (document.hidden && ui.value.outcome) {
    await api.researchNotify(
      ui.value.outcome.text || t(`research.outcome.${ui.value.outcome.code}`),
      props.projectId,
      props.chatId,
    )
  }
}

async function onExport(kind: ResearchExport): Promise<void> {
  if (!savedPath.value) {
    await finish()
  }
  emit('export', exportInstruction(kind, savedPath.value, ui.value.version))
}

function copyQuote(text: string): void {
  void navigator.clipboard?.writeText(text)
}

async function removeReport(): Promise<void> {
  if (ui.value.runId) {
    await api.researchDelete(ui.value.runId)
  }
  ui.value = emptyResearch()
  savedPath.value = ''
  emit('remember', null)
}

defineExpose({ probe })
</script>

<template>
  <div v-if="caps">
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
      :depth="ui.depth"
      :editable="waitingPlan"
      :disclosures="ui.disclosures"
      :estimate="ui.estimateCost !== null ? String(ui.estimateCost) : ''"
      :file-count="fileCount"
      :web="useWeb"
      @start="sendInput({ type: 'approve_plan' })"
      @cancel="ui.runId ? sendInput({ type: 'stop_gathering' }) : undefined"
      @edit="sendInput({ type: 'edit_plan', points: $event })"
    />
    <ResearchProgressCard
      v-if="running"
      :depth="ui.depth"
      :points="ui.points"
      :found="ui.found"
      :read="ui.read"
      :closed="ui.closed"
      :now="ui.now"
      :queued-since="ui.queuedSince"
      :details="ui.details"
      @stop="api.researchCancel(ui.runId)"
      @write-now="sendInput({ type: 'stop_gathering' })"
    />
    <ResearchResultCard
      v-if="done"
      :title="ui.question"
      :preview="ui.markdown.slice(0, 280)"
      :check="ui.check"
      :saved-path="savedPath"
      :outcome="ui.outcome ? t(`research.outcome.${ui.outcome.code}`, ui.outcome.params) : ''"
      :failed="ui.status !== 'complete'"
      :skills="skills"
      :version="ui.version"
      @open="reportOpen = true"
      @export="onExport"
      @deeper="api.researchFollowUp(ui.runId, 'deeper', null, `deep-${Date.now()}`)"
      @refresh="api.researchFollowUp(ui.runId, 'refresh', null, `refresh-${Date.now()}`)"
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
      @close="reportOpen = false"
      @cite="cited = $event"
      @ask="api.researchFollowUp(ui.runId, 'question', $event, `ask-${Date.now()}`)"
      @deeper="api.researchFollowUp(ui.runId, 'deeper', null, `deep-${Date.now()}`)"
      @restore="api.researchRestoreVersion(projectId, ui.question, $event)"
      @revise="api.researchFollowUp(ui.runId, 'revise', $event, `rev-${Date.now()}`)"
    />
    <SourceDrawer
      :source="source"
      :quotes="quotes"
      :number="ui.sources.findIndex((item) => item.id === cited) + 1"
      @close="cited = null"
      @open-page="source?.url && api.openUrl(source.url)"
      @copy-quote="copyQuote"
    />
  </div>
</template>
