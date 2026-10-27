import { computed, type Ref } from 'vue'
import type { ActivityPhase } from '@/components/ChatActivity.vue'
import type { ChatArtifact } from '@/services/tauri'

export interface RunStep {
  summary: string
  status: 'running' | 'ok' | 'error'
}

export interface UiMessage {
  role: 'user' | 'assistant'
  content: string
  model?: string
  createdAt?: string
  steps?: RunStep[]
  artifacts?: ChatArtifact[]
  /** Set once the answer was kept as a note; opens that note from the message. */
  noteName?: string
}

export interface RetryAttempt {
  text: string
  useAgent: boolean
  allowExec: boolean
}

/**
 * "The system is working" feedback for one turn.
 * A turn must never look frozen. `activity` names what is happening now so the
 * indicator can say "Searching the web…", "Generating an image…", etc.; it is
 * set when a turn starts and cleared the moment the turn reaches a terminal
 * state (done, error, or cancelled).
 */
export function useChatActivity(
  messages: Ref<UiMessage[]>,
  sending: Ref<boolean>,
  activity: Ref<ActivityPhase | null>,
) {
  const lastMessage = computed(() => messages.value[messages.value.length - 1])
  /** A tool step already spins its own indicator, so the card steps aside then. */
  const hasRunningStep = computed(
    () => lastMessage.value?.steps?.some((step) => step.status === 'running') ?? false,
  )
  /** The answer bubble is already on screen and still growing (or paused). */
  const streamingReply = computed(
    () => lastMessage.value?.role === 'assistant' && (lastMessage.value.content ?? '') !== '',
  )
  const activityPhase = computed<ActivityPhase>(() => activity.value ?? 'thinking')
  const showActivity = computed(() => sending.value && !hasRunningStep.value)

  return { lastMessage, hasRunningStep, streamingReply, activityPhase, showActivity }
}
