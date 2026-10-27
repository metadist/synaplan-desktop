import type { ComputedRef, Ref } from 'vue'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { useI18n } from 'vue-i18n'
import * as api from '@/services/tauri'
import type { ActivityPhase } from '@/components/ChatActivity.vue'
import { useConfigStore } from '@/stores/config'
import { useErrorText } from '@/composables/useErrorText'
import type { RetryAttempt, UiMessage } from '@/composables/useChatActivity'

/** Guards one in-flight turn. A newer turn or a project switch bumps `turnSeq`. */
export interface TurnClock {
  turnSeq: number
  streamTurn: number
  pendingDocument: boolean
  currentAttempt: RetryAttempt | null
}

export function createTurnClock(): TurnClock {
  return { turnSeq: 0, streamTurn: 0, pendingDocument: false, currentAttempt: null }
}

export interface StreamTurnDeps {
  messages: Ref<UiMessage[]>
  sending: Ref<boolean>
  activity: Ref<ActivityPhase | null>
  error: Ref<string>
  errorCode: Ref<string>
  retryAttempt: Ref<RetryAttempt | null>
  clock: TurnClock
  projectId: ComputedRef<string>
  chatModel: ComputedRef<string>
  threadAssistantId: Ref<number | null>
  webSearchOn: ComputedRef<boolean>
  scrollToBottom: () => Promise<void>
  persistThread: () => Promise<void>
  refreshKnowledge: () => void
  clearInput: () => void
  sendAgent: (
    projectId: string,
    wire: api.ChatMessage[],
    allowExec: boolean,
    assistantId: number | null,
  ) => Promise<void>
}

/**
 * The plain chat turn: streaming tokens, pictures and sounds, and saving a
 * reply the person asked for as a document. Skill turns call `sendAgent`.
 */
export function useStreamTurn(deps: StreamTurnDeps) {
  const { t } = useI18n()
  const config = useConfigStore()
  const errorText = useErrorText()

  function bumpTurn(): number {
    deps.clock.turnSeq += 1
    deps.clock.streamTurn = deps.clock.turnSeq
    return deps.clock.turnSeq
  }

  function noteFailure(code: string, message: string, attempt: RetryAttempt | null): void {
    deps.errorCode.value = code
    deps.error.value = message
    if (attempt) {
      deps.retryAttempt.value = attempt
    }
    if (code === 'unauthorized') {
      config.markRevoked()
      void config.refresh()
    }
  }

  function currentAssistant(): UiMessage {
    const last = deps.messages.value[deps.messages.value.length - 1]
    if (last && last.role === 'assistant') {
      return last
    }
    const created: UiMessage = {
      role: 'assistant',
      content: '',
      model: deps.chatModel.value,
      createdAt: new Date().toISOString(),
    }
    deps.messages.value.push(created)
    return created
  }

  function appendAssistantText(text: string): void {
    if (deps.clock.streamTurn !== deps.clock.turnSeq) {
      return
    }
    const msg = currentAssistant()
    msg.content += text
    void deps.scrollToBottom()
  }

  function pushArtifact(msg: UiMessage, artifact: api.ChatArtifact): void {
    if (!msg.artifacts) {
      msg.artifacts = []
    }
    if (!msg.artifacts.some((item) => item.path === artifact.path)) {
      msg.artifacts.push(artifact)
    }
  }

  /** A reply that ends on a question is the assistant asking for details, not the document. */
  function asksBack(text: string): boolean {
    const lastLine =
      text
        .split('\n')
        .map((line) => line.trim())
        .filter((line) => line.length > 0)
        .pop() ?? ''
    return lastLine.endsWith('?')
  }

  async function saveReplyAsDocument(): Promise<void> {
    const project = deps.projectId.value
    const msg = deps.messages.value[deps.messages.value.length - 1]
    if (!project || !msg || msg.role !== 'assistant') {
      return
    }
    if (msg.artifacts && msg.artifacts.length > 0) {
      return
    }
    const text = msg.content.trim()
    if (text.length < 8 || asksBack(text)) {
      return
    }
    try {
      const artifact = await api.saveTextArtifact(project, text, text.slice(0, 48))
      pushArtifact(msg, artifact)
      void deps.persistThread()
      deps.refreshKnowledge()
    } catch (e) {
      if (!deps.error.value) {
        deps.error.value = errorText(e)
      }
    }
  }

  function finishTurn(): void {
    if (deps.clock.streamTurn !== deps.clock.turnSeq) {
      return
    }
    deps.sending.value = false
    deps.activity.value = null
    // A history-save failure has no turn code and must stay visible.
    if (deps.errorCode.value) {
      deps.error.value = ''
      deps.errorCode.value = ''
      deps.retryAttempt.value = null
      deps.clock.currentAttempt = null
    }
    if (deps.clock.pendingDocument) {
      deps.clock.pendingDocument = false
      void saveReplyAsDocument()
    }
    void deps.persistThread()
    deps.refreshKnowledge()
  }

  function onStreamError(error: api.StreamError): void {
    if (deps.clock.streamTurn !== deps.clock.turnSeq) {
      return
    }
    deps.sending.value = false
    deps.activity.value = null
    noteFailure(error.code, errorText(error), deps.clock.currentAttempt)
    void deps.persistThread()
  }

  async function dispatchSend(text: string, useAgent: boolean, allowExec: boolean): Promise<void> {
    const turn = bumpTurn()
    const sendProject = deps.projectId.value
    if (!sendProject) {
      return
    }
    deps.error.value = ''
    deps.errorCode.value = ''
    deps.retryAttempt.value = null
    deps.clock.currentAttempt = { text, useAgent, allowExec }
    deps.clock.pendingDocument = false
    deps.messages.value.push({ role: 'user', content: text, createdAt: new Date().toISOString() })
    deps.clearInput()
    deps.sending.value = true
    // Name the wait right away; the media branch below refines it once the
    // classifier says the person asked for a picture, sound, or video.
    deps.activity.value = deps.webSearchOn.value ? 'web' : 'thinking'
    void deps.scrollToBottom()
    // The user's message is on disk before the answer streams.
    await deps.persistThread()
    if (turn !== deps.clock.turnSeq || deps.projectId.value !== sendProject) {
      return
    }

    let kind: api.GenerationKind | null = null
    try {
      kind = await api.classifyGeneration(text)
    } catch {
      // A classifier hiccup must never eat the message: it is a plain chat turn.
      kind = null
    }
    if (turn !== deps.clock.turnSeq || deps.projectId.value !== sendProject) {
      return
    }

    if (kind === 'image' || kind === 'audio' || kind === 'video') {
      deps.clock.streamTurn = turn
      deps.activity.value = kind
      try {
        const artifact = await api.generateAndAttach(sendProject, kind, text)
        if (turn !== deps.clock.turnSeq || deps.projectId.value !== sendProject) {
          return
        }
        const msg = currentAssistant()
        msg.content = t(`chat.generated.${artifact.kind}`)
        pushArtifact(msg, artifact)
        finishTurn()
      } catch (e) {
        if (turn !== deps.clock.turnSeq) {
          return
        }
        deps.sending.value = false
        deps.activity.value = null
        if (!deps.error.value) {
          const commandError = api.asCommandError(e)
          noteFailure(commandError.code, errorText(e), deps.clock.currentAttempt)
        }
        void deps.persistThread()
      }
      return
    }
    // Skills write real .docx/.xlsx/.pptx files themselves; only a plain chat
    // reply is kept as a text document.
    deps.clock.pendingDocument = kind === 'document' && !useAgent

    const wire: api.ChatMessage[] = deps.messages.value.map((message) => ({
      role: message.role,
      content: message.content,
    }))
    try {
      if (useAgent) {
        await deps.sendAgent(sendProject, wire, allowExec, deps.threadAssistantId.value)
      } else {
        await api.sendChat(sendProject, wire, deps.threadAssistantId.value)
      }
    } catch (e) {
      if (turn !== deps.clock.turnSeq) {
        return
      }
      deps.sending.value = false
      deps.activity.value = null
      deps.clock.pendingDocument = false
      if (!deps.error.value) {
        const commandError = api.asCommandError(e)
        noteFailure(commandError.code, errorText(e), deps.clock.currentAttempt)
      }
    }
  }

  function resend(): void {
    const attempt = deps.retryAttempt.value
    if (!attempt || deps.sending.value) {
      return
    }
    const last = deps.messages.value[deps.messages.value.length - 1]
    if (last?.role === 'assistant' && !last.content.trim()) {
      deps.messages.value.pop()
    }
    const user = deps.messages.value[deps.messages.value.length - 1]
    if (user?.role === 'user' && user.content === attempt.text) {
      deps.messages.value.pop()
    }
    void dispatchSend(attempt.text, attempt.useAgent, attempt.allowExec)
  }

  async function stop(): Promise<void> {
    try {
      await api.cancelChat()
    } catch {
      // Ignore: the turn ends on its own shortly after.
    }
  }

  async function listen(): Promise<UnlistenFn[]> {
    return [
      await api.onChatToken((token) => appendAssistantText(token)),
      await api.onChatTruncated(() => appendAssistantText(`\n\n${t('chat.answerTruncated')}`)),
      await api.onChatDone(() => finishTurn()),
      await api.onChatError((error) => onStreamError(error)),
    ]
  }

  return {
    bumpTurn,
    currentAssistant,
    appendAssistantText,
    pushArtifact,
    finishTurn,
    onStreamError,
    dispatchSend,
    resend,
    stop,
    listen,
  }
}
