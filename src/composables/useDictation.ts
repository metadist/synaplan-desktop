import { onUnmounted, ref, type Ref } from 'vue'
import * as api from '@/services/tauri'

/** Session PCM: 16 kHz mono 16-bit. */
export const SAMPLE_RATE = 16_000
/** Silence this long ends a phrase. */
export const PAUSE_MS = 700
/** Shorter phrases are recognised poorly (3 s). */
export const MIN_CHUNK_SAMPLES = 48_000
/** Cap for someone who never pauses (15 s). */
export const MAX_CHUNK_SAMPLES = 240_000
/** How often the live text is re-read. */
export const POLL_MS = 1800
/** RMS below this counts as silence. */
export const SILENCE_RMS = 0.012

export type DictationState = 'idle' | 'starting' | 'recording' | 'finishing'

const MIC_ERRORS = new Set([
  'NotAllowedError',
  'NotFoundError',
  'SecurityError',
  'NotReadableError',
])

/** A refused or missing microphone becomes a plain-language, code-bearing error. */
export function asMicrophoneError(e: unknown): unknown {
  if (e && typeof e === 'object' && 'name' in e && MIC_ERRORS.has(String(e.name))) {
    return {
      code: 'microphone_denied',
      message: String((e as { message?: string }).message ?? e.name),
    }
  }
  return e
}

/** Commit when a pause closes a long-enough phrase, or the cap is reached. */
export function shouldCommit(samples: number, pausedMs: number): boolean {
  return (pausedMs >= PAUSE_MS && samples >= MIN_CHUNK_SAMPLES) || samples >= MAX_CHUNK_SAMPLES
}

/** The one-shot result wins; the live text is only the fallback. */
export function pickFinalText(oneShot: string, live: string): string {
  const shot = oneShot.trim()
  return shot !== '' ? shot : live.trim()
}

export function rms(frame: Float32Array): number {
  if (frame.length === 0) {
    return 0
  }
  let sum = 0
  for (let i = 0; i < frame.length; i++) {
    sum += frame[i] * frame[i]
  }
  return Math.sqrt(sum / frame.length)
}

/** Nearest-sample downsampling; good enough for speech going into Whisper. */
export function downsample(input: Float32Array, fromRate: number, toRate: number): Float32Array {
  if (fromRate === toRate) {
    return input
  }
  const ratio = fromRate / toRate
  const length = Math.floor(input.length / ratio)
  const out = new Float32Array(length)
  for (let i = 0; i < length; i++) {
    out[i] = input[Math.floor(i * ratio)]
  }
  return out
}

export function floatToPcm16(input: Float32Array): Uint8Array {
  const out = new Uint8Array(input.length * 2)
  const view = new DataView(out.buffer)
  for (let i = 0; i < input.length; i++) {
    const clamped = Math.max(-1, Math.min(1, input[i]))
    view.setInt16(i * 2, clamped < 0 ? clamped * 0x8000 : clamped * 0x7fff, true)
  }
  return out
}

/** Audio-capture seams the composable needs; injectable so unit tests stay silent. */
export interface DictationCapture {
  /** Start capturing; call `onFrame` with mono float frames at `sampleRate`. */
  start(onFrame: (frame: Float32Array, sampleRate: number) => void): Promise<void>
  /** Stop and hand back the whole take as one encoded blob (may be empty). */
  stop(): Promise<{ bytes: Uint8Array; mime: string }>
}

/** Real capture: ScriptProcessor for PCM frames + MediaRecorder for the take. */
export function browserCapture(): DictationCapture {
  let stream: MediaStream | null = null
  let context: AudioContext | null = null
  let processor: ScriptProcessorNode | null = null
  let recorder: MediaRecorder | null = null
  const parts: BlobPart[] = []
  let mime = 'audio/webm'

  return {
    async start(onFrame) {
      stream = await navigator.mediaDevices.getUserMedia({
        audio: { channelCount: 1, echoCancellation: true, noiseSuppression: true },
      })
      context = new AudioContext()
      const source = context.createMediaStreamSource(stream)
      processor = context.createScriptProcessor(4096, 1, 1)
      const rate = context.sampleRate
      processor.onaudioprocess = (event) => {
        onFrame(new Float32Array(event.inputBuffer.getChannelData(0)), rate)
      }
      source.connect(processor)
      processor.connect(context.destination)
      if (typeof MediaRecorder !== 'undefined') {
        recorder = new MediaRecorder(stream)
        mime = recorder.mimeType || mime
        recorder.ondataavailable = (event) => {
          if (event.data.size > 0) {
            parts.push(event.data)
          }
        }
        recorder.start()
      }
    },
    async stop() {
      const stopped = new Promise<void>((resolve) => {
        if (!recorder || recorder.state === 'inactive') {
          resolve()
          return
        }
        recorder.onstop = () => resolve()
        recorder.stop()
      })
      processor?.disconnect()
      processor = null
      await context?.close()
      context = null
      stream?.getTracks().forEach((track) => track.stop())
      stream = null
      await stopped
      const blob = new Blob(parts, { type: mime })
      parts.length = 0
      return { bytes: new Uint8Array(await blob.arrayBuffer()), mime }
    },
  }
}

export interface DictationOptions {
  /** Generic note-taking prompt from i18n (never hardcoded). */
  prompt: () => string
  capture?: () => DictationCapture
}

/**
 * One dictation take at a time for a project: live PCM to a session for
 * interim text, the whole take to the one-shot route on stop, one-shot wins.
 * All HTTP goes through Rust; this only moves audio and text.
 */
export function useDictation(projectId: Ref<string>, options: DictationOptions) {
  const state = ref<DictationState>('idle')
  const interim = ref('')
  const error = ref<unknown>(null)

  let capture: DictationCapture | null = null
  let sessionId = ''
  let takeProjectId = ''
  let pollTimer: ReturnType<typeof setInterval> | null = null
  let chunk: Float32Array[] = []
  let chunkSamples = 0
  let pausedMs = 0
  let sending: Promise<void> = Promise.resolve()

  function resetChunk(): void {
    chunk = []
    chunkSamples = 0
    pausedMs = 0
  }

  function queueChunk(commit: boolean): void {
    if (chunkSamples === 0 || !sessionId) {
      resetChunk()
      return
    }
    const joined = new Float32Array(chunkSamples)
    let offset = 0
    for (const part of chunk) {
      joined.set(part, offset)
      offset += part.length
    }
    resetChunk()
    const pcm = floatToPcm16(joined)
    const id = sessionId
    sending = sending
      .then(() => api.dictationChunk(id, pcm, commit))
      .catch((e) => {
        error.value = e
      })
  }

  function onFrame(frame: Float32Array, rate: number): void {
    if (state.value !== 'recording') {
      return
    }
    const mono = downsample(frame, rate, SAMPLE_RATE)
    const silent = rms(mono) < SILENCE_RMS
    // Never start a phrase with silence; silence inside a phrase is kept.
    if (silent && chunkSamples === 0) {
      return
    }
    chunk.push(mono)
    chunkSamples += mono.length
    pausedMs = silent ? pausedMs + (mono.length / SAMPLE_RATE) * 1000 : 0
    if (shouldCommit(chunkSamples, pausedMs)) {
      queueChunk(true)
    }
  }

  function stopPolling(): void {
    if (pollTimer !== null) {
      clearInterval(pollTimer)
      pollTimer = null
    }
  }

  async function poll(): Promise<void> {
    const id = sessionId
    if (!id) {
      return
    }
    try {
      const text = await api.dictationPoll(id)
      if (state.value === 'recording' && sessionId === id) {
        interim.value = text
      }
    } catch {
      // Interim text is best effort; the one-shot result closes the take.
    }
  }

  async function start(): Promise<boolean> {
    if (state.value !== 'idle' || !projectId.value) {
      return false
    }
    state.value = 'starting'
    error.value = null
    interim.value = ''
    takeProjectId = projectId.value
    resetChunk()
    try {
      // Model + language are checked on the Rust side before the mic opens.
      sessionId = (await api.dictationStart(takeProjectId, options.prompt())).sessionId
      capture = (options.capture ?? browserCapture)()
      await capture.start(onFrame)
      state.value = 'recording'
      pollTimer = setInterval(() => void poll(), POLL_MS)
      return true
    } catch (e) {
      error.value = asMicrophoneError(e)
      await cleanup()
      return false
    }
  }

  async function cleanup(): Promise<void> {
    stopPolling()
    if (capture) {
      try {
        await capture.stop()
      } catch {
        // Already stopped.
      }
      capture = null
    }
    if (sessionId) {
      const id = sessionId
      sessionId = ''
      void api.dictationClose(id).catch(() => undefined)
    }
    resetChunk()
    state.value = 'idle'
  }

  /** Stop the take and return the final text (one-shot preferred). */
  async function stop(): Promise<string> {
    if (state.value !== 'recording' || !capture) {
      return ''
    }
    state.value = 'finishing'
    stopPolling()
    // Whatever is still buffered closes the last phrase.
    queueChunk(true)
    let take: { bytes: Uint8Array; mime: string } = { bytes: new Uint8Array(), mime: '' }
    try {
      take = await capture.stop()
    } catch (e) {
      error.value = e
    }
    capture = null
    await sending

    const id = sessionId
    let live = ''
    let oneShot = ''
    try {
      live = id ? await api.dictationCommit(id) : ''
    } catch (e) {
      error.value = e
    }
    try {
      oneShot =
        take.bytes.length > 0
          ? await api.dictationTranscribe(takeProjectId, options.prompt(), take.bytes, take.mime)
          : ''
    } catch (e) {
      error.value = e
    }
    const text = pickFinalText(oneShot, live || interim.value)
    interim.value = ''
    sessionId = ''
    if (id) {
      void api.dictationClose(id).catch(() => undefined)
    }
    state.value = 'idle'
    return text
  }

  /** Abandon the take without inserting anything. */
  async function cancel(): Promise<void> {
    await cleanup()
    interim.value = ''
  }

  onUnmounted(() => {
    void cleanup()
  })

  return { state, interim, error, start, stop, cancel }
}
