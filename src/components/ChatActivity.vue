<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'

/**
 * Honest, lively feedback while a turn is in flight. Instead of a silent pause,
 * the person always sees what the system is doing right now — waiting for the
 * answer, searching the web, or generating an image / audio / video — with a
 * moving indicator and, after a moment, how long it has taken. It never claims
 * a percentage it does not have: server-side work shows an indeterminate bar,
 * quick waits show pulsing dots.
 */
export type ActivityPhase = 'thinking' | 'web' | 'image' | 'audio' | 'video' | 'document'

const props = withDefaults(
  defineProps<{
    phase?: ActivityPhase
    /** A slim inline chip under a streaming answer, instead of the full card. */
    compact?: boolean
  }>(),
  { phase: 'thinking', compact: false },
)

const { t } = useI18n()

const ICON: Record<ActivityPhase, string> = {
  thinking: '💭',
  web: '🌐',
  image: '🎨',
  audio: '🎧',
  video: '🎬',
  document: '📝',
}

/** Phases that run on the server for an unknown time deserve a moving bar. */
const BAR_PHASES: readonly ActivityPhase[] = ['web', 'image', 'audio', 'video']

const icon = computed(() => ICON[props.phase])
const label = computed(() => t(`chat.activity.${props.phase}`))
const showBar = computed(() => !props.compact && BAR_PHASES.includes(props.phase))

const elapsed = ref(0)
let timer: ReturnType<typeof setInterval> | undefined
onMounted(() => {
  timer = setInterval(() => {
    elapsed.value += 1
  }, 1000)
})
onUnmounted(() => {
  if (timer !== undefined) {
    clearInterval(timer)
  }
})

/** Show the seconds only after a moment, so a fast answer stays clean. */
const showElapsed = computed(() => elapsed.value >= 3)
const elapsedLabel = computed(() => t('chat.activity.elapsed', { seconds: elapsed.value }))
/** After a longer wait, say we are still on it — respect for the person's time. */
const slow = computed(() => elapsed.value >= 20)
</script>

<template>
  <div
    class="activity"
    :class="{ compact }"
    role="status"
    aria-live="polite"
    data-testid="chat-activity"
  >
    <span class="activity-icon" aria-hidden="true">{{ icon }}</span>
    <div class="activity-main">
      <div class="activity-line">
        <span class="activity-label">{{ label }}</span>
        <span class="dots" aria-hidden="true"><i></i><i></i><i></i></span>
        <span v-if="showElapsed" class="activity-elapsed" data-testid="chat-activity-elapsed">
          {{ elapsedLabel }}
        </span>
      </div>
      <div v-if="showBar" class="activity-bar" aria-hidden="true"><span></span></div>
      <p v-if="slow && !compact" class="activity-slow">{{ t('chat.activity.stillWorking') }}</p>
    </div>
  </div>
</template>

<style scoped>
.activity {
  display: flex;
  align-items: flex-start;
  gap: 0.6rem;
  padding: 0.55rem 0.8rem;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: color-mix(in srgb, var(--accent) 5%, var(--bg-card));
  max-width: max-content;
}

.activity.compact {
  padding: 0.3rem 0.5rem;
  border-color: transparent;
  background: transparent;
}

.activity-icon {
  font-size: 1.1rem;
  line-height: 1.3;
  animation: activity-bob 1.6s ease-in-out infinite;
}

.activity.compact .activity-icon {
  font-size: 0.95rem;
}

.activity-main {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  min-width: 220px;
}

.activity.compact .activity-main {
  min-width: 0;
  gap: 0;
}

.activity-line {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.activity-label {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--txt);
}

.activity.compact .activity-label {
  font-size: 0.8rem;
  font-weight: 550;
  color: var(--txt-secondary);
}

.activity-elapsed {
  font-size: 0.72rem;
  color: var(--txt-secondary);
  font-variant-numeric: tabular-nums;
}

.activity-slow {
  margin: 0;
  font-size: 0.75rem;
  color: var(--txt-secondary);
}

.dots {
  display: inline-flex;
  align-items: center;
  gap: 3px;
}

.dots i {
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: var(--accent);
  display: inline-block;
  animation: dot-pulse 1.2s ease-in-out infinite;
}

.dots i:nth-child(2) {
  animation-delay: 0.2s;
}

.dots i:nth-child(3) {
  animation-delay: 0.4s;
}

.activity-bar {
  position: relative;
  height: 4px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--accent) 16%, transparent);
  overflow: hidden;
}

.activity-bar span {
  position: absolute;
  top: 0;
  left: -40%;
  height: 100%;
  width: 40%;
  border-radius: 999px;
  background: var(--accent);
  animation: bar-slide 1.3s ease-in-out infinite;
}

@keyframes dot-pulse {
  0%,
  80%,
  100% {
    transform: scale(0.6);
    opacity: 0.4;
  }
  40% {
    transform: scale(1);
    opacity: 1;
  }
}

@keyframes bar-slide {
  0% {
    left: -40%;
  }
  100% {
    left: 100%;
  }
}

@keyframes activity-bob {
  0%,
  100% {
    transform: translateY(0);
  }
  50% {
    transform: translateY(-2px);
  }
}

@media (prefers-reduced-motion: reduce) {
  .activity-icon,
  .dots i,
  .activity-bar span {
    animation: none;
  }
  .activity-bar span {
    left: 0;
    width: 100%;
    opacity: 0.5;
  }
}
</style>
