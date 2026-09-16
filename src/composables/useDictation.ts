import { onUnmounted, ref, type Ref } from 'vue'
import * as api from '@/services/tauri'

/** Session PCM: 16 kHz mono 16-bit. */
export const SAMPLE_RATE = 16_000
/** Silence this long ends a phrase — then we post the snippet. */
export const PAUSE_MS = 700
/** Whisper needs a real sentence; shorter live snippets are recognised poorly. */
export const MIN_CHUNK_SAMPLES = SAMPLE_RATE * 5
/** Cap for someone who never pauses (40 s). */
export const MAX_CHUNK_SAMPLES = SAMPLE_RATE * 40
/** After stop, the whole take is re-transcribed in this window (30–40 s). */
export const CORRECTION_WINDOW_SAMPLES = SAMPLE_RATE * 35
/** A leftover shorter than this is folded into the previous correction window. */
export const CORRECTION_TAIL_SAMPLES = SAMPLE_RATE * 2
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

/** The correction pass wins; the live text is only the fallback. */
export function pickFinalText(correction: string, live: string): string {
  const shot = correction.trim()
  return shot !== '' ? shot : live.trim()
}

/** Join 16 kHz frames into one buffer. */
export function concatFloat32(parts: Float32Array[]): Float32Array {
  let n = 0
  for (const part of parts) {
    n += part.length
  }
  const out = new Float32Array(n)
  let offset = 0
  for (const part of parts) {
    out.set(part, offset)
    offset += part.length
  }
  return out
}

/**
 * Split the whole take into ~35 s windows for the quality pass. A short
 * leftover is appended to the previous window so Whisper still sees context.
 */
export function correctionWindows(
  pcm: Float32Array,
  window = CORRECTION_WINDOW_SAMPLES,
  tail = CORRECTION_TAIL_SAMPLES,
): Float32Array[] {
  if (pcm.length === 0) {
    return []
  }
  if (pcm.length <= window) {
    return [pcm]
  }
  const out: Float32Array[] = []
  let i = 0
  while (i < pcm.length) {
    const remaining = pcm.length - i
    if (remaining <= window) {
      if (remaining < tail && out.length > 0) {
        const last = out[out.length - 1]
        const merged = new Float32Array(last.length + remaining)
        merged.set(last)
        merged.set(pcm.subarray(i), last.length)
        out[out.length - 1] = merged
      } else {
        out.push(pcm.subarray(i))
      }
      break
    }
    out.push(pcm.subarray(i, i + window))
    i += window
  }
  return out
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
 * One dictation take at a time for a project. Live PCM is posted after a
 * pause (longer snippets than a 2 s metronome). On stop the whole take is
 * committed again as 35 s windows so Whisper can correct the live text.
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
  let take: Float32Array[] = []
  let sending: Promise<void> = Promise.resolve()

  function resetChunk(): void {
    chunk = []
    chunkSamples = 0
    pausedMs = 0
  }

  function resetTake(): void {
    take = []
    resetChunk()
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
    take.push(mono)
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
      // Interim text is best effort; the correction pass closes the take.
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
    resetTake()
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
    resetTake()
    state.value = 'idle'
  }

  async function correctTake(pcm: Float32Array): Promise<string> {
    const windows = correctionWindows(pcm).filter((w) => rms(w) >= SILENCE_RMS)
    if (windows.length === 0) {
      return ''
    }
    const session = await api.dictationStart(takeProjectId, options.prompt())
    const id = session.sessionId
    try {
      for (const window of windows) {
        await api.dictationChunk(id, floatToPcm16(window), true)
        try {
          const text = await api.dictationPoll(id)
          if (state.value === 'finishing') {
            interim.value = text
          }
        } catch {
          // Partial correction text is best effort.
        }
      }
      return await api.dictationCommit(id)
    } finally {
      void api.dictationClose(id).catch(() => undefined)
    }
  }

  /** Stop the take and return the final text (correction pass preferred). */
  async function stop(): Promise<string> {
    if (state.value !== 'recording' || !capture) {
      return ''
    }
    state.value = 'finishing'
    stopPolling()
    // Whatever is still buffered closes the last live phrase.
    queueChunk(true)
    const whole = concatFloat32(take)
    take = []
    let blob: { bytes: Uint8Array; mime: string } = { bytes: new Uint8Array(), mime: '' }
    try {
      blob = await capture.stop()
    } catch (e) {
      error.value = e
    }
    capture = null
    await sending

    const id = sessionId
    sessionId = ''
    let live = ''
    try {
      live = id ? await api.dictationCommit(id) : ''
    } catch (e) {
      error.value = e
    }
    if (id) {
      void api.dictationClose(id).catch(() => undefined)
    }

    let corrected = ''
    try {
      corrected = await correctTake(whole)
    } catch (e) {
      error.value = e
    }
    if (corrected === '' && blob.bytes.length > 0) {
      try {
        corrected = await api.dictationTranscribe(
          takeProjectId,
          options.prompt(),
          blob.bytes,
          blob.mime,
        )
      } catch (e) {
        error.value = e
      }
    }
    const text = pickFinalText(corrected, live || interim.value)
    interim.value = ''
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
