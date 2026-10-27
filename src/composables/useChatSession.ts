import { computed, nextTick, ref, type ComputedRef, type Ref } from 'vue'
import { useI18n } from 'vue-i18n'
import * as api from '@/services/tauri'
import { useProjectsStore } from '@/stores/projects'
import { useUiStore } from '@/stores/ui'
import { useErrorText } from '@/composables/useErrorText'
import { useChatThreads } from '@/composables/useChatThreads'
import { useNotes } from '@/composables/useNotes'
import { useKnowledgeFiles } from '@/composables/useKnowledgeFiles'
import { hasStudioCopy, resolveStudioTiles, type TaskCard } from '@/composables/useTaskStudio'
import type { RetryAttempt, UiMessage } from '@/composables/useChatActivity'
import type { TurnClock } from '@/composables/useStreamTurn'
import type { PanelTab } from '@/components/ProjectPanel.vue'

/**
 * Notes, files, history and example tiles that sit beside one chat.
 * The turn runners stay separate; this only owns the project shelf around them.
 */
export function useChatSession(options: {
  projectId: ComputedRef<string>
  messages: Ref<UiMessage[]>
  input: Ref<string>
  sending: Ref<boolean>
  error: Ref<string>
  errorCode: Ref<string>
  retryAttempt: Ref<RetryAttempt | null>
  clock: TurnClock
  threadAssistantId: Ref<number | null>
  chatModel: ComputedRef<string>
  scrollToBottom: () => Promise<void>
  focusComposer: () => void
}) {
  const { t, locale } = useI18n()
  const projects = useProjectsStore()
  const ui = useUiStore()
  const errorText = useErrorText()
  const threads = useChatThreads(options.projectId)
  const notes = useNotes(options.projectId)
  const knowledge = useKnowledgeFiles(options.projectId)

  const panelTab = ref<PanelTab | null>(null)
  const dragging = ref(false)
  const picking = ref(false)
  const noteCount = computed(() => notes.notes.value.length)
  const notesBump = ref(false)
  let bumpTimer: ReturnType<typeof setTimeout> | undefined
  const fileCount = computed(() => knowledge.files.value.length)
  const chatCount = computed(() => threads.threads.value.length)

  const skills = ref<api.Skill[]>([])
  const studioPicks = ref<string[]>([])
  const executionConsent = ref(false)
  const composerArmed = ref(false)
  let armedTimer: ReturnType<typeof setTimeout> | undefined
  const webSearchBusy = ref(false)

  const projectSkills = computed(() => {
    const overlay = new Set(projects.active?.enabledSkills ?? [])
    return skills.value.filter(
      (skill) => skill.enabled && !skill.blocked && overlay.has(skill.name),
    )
  })
  const enabledSkillCount = computed(() => projectSkills.value.length)
  const agentMode = computed(() => enabledSkillCount.value > 0)
  const studioCards = computed(() => resolveStudioTiles(projectSkills.value, studioPicks.value))
  const showStudio = computed(
    () => options.messages.value.length === 0 && studioCards.value.length > 0,
  )
  const showWelcome = computed(() => options.messages.value.length === 0 && !showStudio.value)

  function togglePanel(tab: PanelTab): void {
    panelTab.value = panelTab.value === tab ? null : tab
  }

  function openPanel(tab: PanelTab): void {
    panelTab.value = tab
  }

  async function newNote(): Promise<void> {
    await notes.create()
    if (notes.current.value) {
      openPanel('notes')
    }
  }

  function bumpNotes(): void {
    clearTimeout(bumpTimer)
    notesBump.value = true
    bumpTimer = setTimeout(() => (notesBump.value = false), 700)
  }

  /**
   * One click turns live web search on or off for this project. Persisted on the
   * project so every later turn — plain chat and skill runs — carries it.
   */
  async function toggleWebSearch(): Promise<void> {
    const project = projects.active
    if (!project || webSearchBusy.value) {
      return
    }
    webSearchBusy.value = true
    options.error.value = ''
    try {
      await projects.update(project.id, { webSearch: !project.webSearch })
    } catch (failure) {
      options.error.value = errorText(failure)
    } finally {
      webSearchBusy.value = false
    }
  }

  /**
   * The composer is the fastest note pad there is: with text in it the note
   * button keeps that text as a note and clears the box; empty, it opens a new
   * note next to the chat.
   */
  async function onNoteButton(): Promise<void> {
    if (options.input.value.trim() === '') {
      await newNote()
      return
    }
    const summary = await notes.keep(options.input.value)
    if (summary) {
      options.input.value = ''
      bumpNotes()
    }
  }

  /** One click keeps an answer as a note; the note remembers which chat it came from. */
  async function keepMessage(index: number): Promise<void> {
    const message = options.messages.value[index]
    if (!message || message.noteName) {
      return
    }
    const footer = t('notes.keptFrom', {
      title: threads.current.value?.title || t('chat.newChat'),
      date: new Intl.DateTimeFormat(locale.value, { dateStyle: 'medium' }).format(new Date()),
    })
    const summary = await notes.keep(message.content, footer)
    if (summary && options.messages.value[index] === message) {
      message.noteName = summary.name
      bumpNotes()
    }
  }

  /** A kept answer links to its note: open it next to the chat. */
  async function openKeptNote(name: string): Promise<void> {
    openPanel('notes')
    await notes.open(name)
  }

  /** Native file picker → the project's knowledge folder; the panel shows progress. */
  async function addFiles(): Promise<void> {
    if (picking.value) {
      return
    }
    picking.value = true
    try {
      const paths = await api.pickFiles(t('files.pickTitle'))
      if (paths.length > 0) {
        openPanel('files')
        void knowledge.add(paths)
      }
    } finally {
      picking.value = false
    }
  }

  /** OS drag-and-drop anywhere over the chat adds to this project. Only while Chat is showing. */
  function onDrop(event: api.FileDropEvent): void {
    if (ui.view !== 'chat') {
      dragging.value = false
      return
    }
    if (event.type === 'enter' || event.type === 'over') {
      dragging.value = true
      return
    }
    dragging.value = false
    if (event.type === 'drop' && event.paths.length > 0) {
      openPanel('files')
      void knowledge.add(event.paths)
    }
  }

  function toStored(): api.StoredChatMessage[] {
    return options.messages.value.map((message) => ({
      role: message.role,
      content: message.content,
      model: message.role === 'assistant' ? (message.model ?? options.chatModel.value) : '',
      createdAt: message.createdAt ?? new Date().toISOString(),
      artifacts: message.artifacts,
    }))
  }

  async function persistThread(): Promise<void> {
    if (options.messages.value.length === 0) {
      return
    }
    try {
      await threads.persist(toStored(), options.threadAssistantId.value)
    } catch (failure) {
      if (!options.error.value) {
        options.error.value = errorText(failure)
      }
    }
  }

  function refreshKnowledge(): void {
    void knowledge.refresh()
  }

  async function openThread(chatId: string): Promise<void> {
    if (options.sending.value) {
      return
    }
    options.error.value = ''
    options.errorCode.value = ''
    options.retryAttempt.value = null
    options.clock.currentAttempt = null
    try {
      const thread = await threads.open(chatId)
      options.threadAssistantId.value = thread.assistantId
      options.messages.value = thread.messages.map((message) => ({
        role: message.role,
        content: message.content,
        model: message.model,
        createdAt: message.createdAt,
        artifacts: message.artifacts,
      }))
      void options.scrollToBottom()
    } catch (failure) {
      options.error.value = errorText(failure)
    }
  }

  async function removeThread(chatId: string): Promise<void> {
    if (options.sending.value) {
      return
    }
    try {
      const wasOpen = threads.current.value?.id === chatId
      await threads.remove(chatId)
      if (wasOpen) {
        options.messages.value = []
      }
    } catch (failure) {
      options.error.value = errorText(failure)
    }
  }

  async function refreshSkillState(): Promise<void> {
    try {
      skills.value = await api.listSkills()
    } catch {
      skills.value = []
    }
    try {
      executionConsent.value = await api.getExecutionConsent()
    } catch {
      // Unknown consent is no consent: the next skill turn asks again.
      executionConsent.value = false
    }
    try {
      studioPicks.value = await api.getStudioTiles()
    } catch {
      studioPicks.value = []
    }
  }

  function armComposer(): void {
    composerArmed.value = true
    if (armedTimer !== undefined) {
      clearTimeout(armedTimer)
    }
    armedTimer = setTimeout(() => {
      composerArmed.value = false
    }, 900)
  }

  function applyTaskPrompt(card: TaskCard): void {
    options.input.value = hasStudioCopy(card)
      ? t(`chat.studio.cards.${card.id}.prompt`)
      : t('chat.studio.genericPrompt', { name: card.skill })
    armComposer()
    void nextTick(() => options.focusComposer())
  }

  async function saveStudioTiles(tiles: string[]): Promise<void> {
    try {
      studioPicks.value = await api.setStudioTiles(tiles)
    } catch {
      studioPicks.value = tiles
    }
  }

  function newChat(): void {
    if (options.sending.value) {
      return
    }
    threads.startNew()
    options.messages.value = []
    options.error.value = ''
    options.errorCode.value = ''
    options.retryAttempt.value = null
    options.clock.currentAttempt = null
    options.input.value = ''
    options.threadAssistantId.value = null
    composerArmed.value = false
  }

  function dispose(): void {
    if (armedTimer !== undefined) {
      clearTimeout(armedTimer)
    }
    clearTimeout(bumpTimer)
  }

  return {
    threads,
    notes,
    knowledge,
    panelTab,
    dragging,
    picking,
    noteCount,
    notesBump,
    fileCount,
    chatCount,
    skills,
    executionConsent,
    composerArmed,
    webSearchBusy,
    projectSkills,
    enabledSkillCount,
    agentMode,
    studioCards,
    showStudio,
    showWelcome,
    togglePanel,
    openPanel,
    newNote,
    toggleWebSearch,
    onNoteButton,
    keepMessage,
    openKeptNote,
    addFiles,
    onDrop,
    persistThread,
    refreshKnowledge,
    openThread,
    removeThread,
    refreshSkillState,
    armComposer,
    applyTaskPrompt,
    saveStudioTiles,
    newChat,
    dispose,
  }
}
