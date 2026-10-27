import type { Ref } from 'vue'
import type { UnlistenFn } from '@tauri-apps/api/event'
import * as api from '@/services/tauri'
import type { UiMessage } from '@/composables/useChatActivity'
import type { TurnClock } from '@/composables/useStreamTurn'

export interface AgentTurnDeps {
  clock: TurnClock
  messages: Ref<UiMessage[]>
  scrollToBottom: () => Promise<void>
  currentAssistant: () => UiMessage
  pushArtifact: (message: UiMessage, artifact: api.ChatArtifact) => void
  appendAssistantText: (text: string) => void
  finishTurn: () => void
  onStreamError: (error: api.StreamError) => void
}

/**
 * The skill turn: tool steps, files the skill wrote, and the agent stream.
 * Plain chat tokens stay in `useStreamTurn`; both share one assistant bubble.
 */
export function useAgentTurn(deps: AgentTurnDeps) {
  function fileName(path: string): string {
    return path.split(/[/\\]/).pop() || path
  }

  function kindFromPath(path: string): string {
    const ext = (path.split('.').pop() || '').toLowerCase()
    if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'heic', 'heif'].includes(ext)) {
      return 'image'
    }
    if (['mp3', 'wav', 'ogg', 'm4a'].includes(ext)) {
      return 'audio'
    }
    if (['mp4', 'webm', 'mov', 'avi', 'mkv'].includes(ext)) {
      return 'video'
    }
    if (
      [
        'pdf',
        'docx',
        'doc',
        'xlsx',
        'xls',
        'pptx',
        'ppt',
        'txt',
        'md',
        'csv',
        'odt',
        'rtf',
      ].includes(ext)
    ) {
      return 'document'
    }
    return 'file'
  }

  function applyToolEvent(ev: api.AgentToolEvent): void {
    if (deps.clock.streamTurn !== deps.clock.turnSeq) {
      return
    }
    const msg = deps.currentAssistant()
    if (!msg.steps) {
      msg.steps = []
    }
    if (ev.phase === 'start') {
      msg.steps.push({ summary: ev.summary, status: 'running' })
    } else {
      const running = [...msg.steps].reverse().find((step) => step.status === 'running')
      if (running) {
        running.summary = ev.summary
        running.status = ev.ok ? 'ok' : 'error'
      } else {
        msg.steps.push({ summary: ev.summary, status: ev.ok ? 'ok' : 'error' })
      }
      if (ev.artifact) {
        deps.pushArtifact(msg, {
          path: ev.artifact,
          name: fileName(ev.artifact),
          kind: kindFromPath(ev.artifact),
        })
      }
    }
    void deps.scrollToBottom()
  }

  function send(
    projectId: string,
    wire: api.ChatMessage[],
    allowExec: boolean,
    assistantId: number | null,
  ): Promise<void> {
    return api.sendAgentChat(projectId, wire, allowExec, assistantId)
  }

  async function listen(): Promise<UnlistenFn[]> {
    return [
      await api.onAgentText((text) => deps.appendAssistantText(text)),
      await api.onAgentTool((ev) => applyToolEvent(ev)),
      await api.onAgentDone(() => deps.finishTurn()),
      await api.onAgentError((error) => deps.onStreamError(error)),
    ]
  }

  return { applyToolEvent, send, listen }
}
