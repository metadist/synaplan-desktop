import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent, h, nextTick, ref } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'

vi.mock('@/services/tauri', () => ({
  dictationStart: vi.fn(),
  dictationChunk: vi.fn(),
  dictationPoll: vi.fn(),
  dictationCommit: vi.fn(),
  dictationClose: vi.fn(),
  dictationTranscribe: vi.fn(),
}))

import * as api from '@/services/tauri'
import {
  CORRECTION_WINDOW_SAMPLES,
  MAX_CHUNK_SAMPLES,
  MIN_CHUNK_SAMPLES,
  PAUSE_MS,
  SAMPLE_RATE,
  asMicrophoneError,
  correctionWindows,
  floatToPcm16,
  pickFinalText,
  shouldCommit,
  useDictation,
  type DictationCapture,
} from '@/composables/useDictation'

describe('dictation commit math', () => {
  it('does not commit 4.9 s of speech even after a pause', () => {
    expect(shouldCommit(SAMPLE_RATE * 4.9, PAUSE_MS)).toBe(false)
  })

  it('commits 5.0 s of speech once the pause is long enough', () => {
    expect(shouldCommit(MIN_CHUNK_SAMPLES, PAUSE_MS)).toBe(true)
    expect(shouldCommit(MIN_CHUNK_SAMPLES, PAUSE_MS - 1)).toBe(false)
  })

  it('commits 40 s of speech with no pause at all', () => {
    expect(shouldCommit(MAX_CHUNK_SAMPLES, 0)).toBe(true)
    expect(shouldCommit(MAX_CHUNK_SAMPLES - 1, 0)).toBe(false)
  })

  it('splits a long take into 35 s windows and folds a short leftover', () => {
    const seventy = correctionWindows(new Float32Array(SAMPLE_RATE * 70))
    expect(seventy).toHaveLength(2)
    expect(seventy[0].length).toBe(CORRECTION_WINDOW_SAMPLES)
    expect(seventy[1].length).toBe(SAMPLE_RATE * 35)

    const thirtySix = correctionWindows(new Float32Array(SAMPLE_RATE * 36))
    expect(thirtySix).toHaveLength(1)
    expect(thirtySix[0].length).toBe(SAMPLE_RATE * 36)
  })

  it('prefers the correction text and falls back to the live text', () => {
    expect(pickFinalText('Full take.', 'full take')).toBe('Full take.')
    expect(pickFinalText('   ', 'live words')).toBe('live words')
    expect(pickFinalText('', '')).toBe('')
  })

  it('encodes floats as little-endian 16-bit PCM', () => {
    const pcm = floatToPcm16(new Float32Array([0, 1, -1]))
    expect(Array.from(pcm)).toEqual([0, 0, 0xff, 0x7f, 0x00, 0x80])
  })

  it('turns a refused microphone into a code the UI can name', () => {
    const denied = Object.assign(new Error('Permission denied'), { name: 'NotAllowedError' })
    expect(asMicrophoneError(denied)).toEqual({
      code: 'microphone_denied',
      message: 'Permission denied',
    })
    const other = { code: 'network', message: 'x' }
    expect(asMicrophoneError(other)).toBe(other)
  })
})

/** A capture that lets the test push frames and hand back a fixed take. */
function fakeCapture(take: Uint8Array) {
  let onFrame: ((frame: Float32Array, rate: number) => void) | null = null
  const capture: DictationCapture = {
    async start(cb) {
      onFrame = cb
    },
    async stop() {
      onFrame = null
      return { bytes: take, mime: 'audio/webm' }
    },
  }
  return {
    capture,
    speak(seconds: number) {
      const frame = new Float32Array(Math.round(seconds * SAMPLE_RATE)).fill(0.3)
      onFrame?.(frame, SAMPLE_RATE)
    },
    silence(seconds: number) {
      onFrame?.(new Float32Array(Math.round(seconds * SAMPLE_RATE)), SAMPLE_RATE)
    },
  }
}

function harness(take: Uint8Array, capture?: DictationCapture) {
  const fake = fakeCapture(take)
  const projectId = ref('p1')
  let dictation!: ReturnType<typeof useDictation>
  const Host = defineComponent({
    setup() {
      dictation = useDictation(projectId, {
        prompt: () => 'Notes.',
        capture: () => capture ?? fake.capture,
      })
      return () => h('div')
    },
  })
  const wrapper = mount(Host)
  return { wrapper, fake, dictation: dictation! }
}

describe('useDictation take', () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] })
    vi.mocked(api.dictationStart).mockReset().mockResolvedValue({ sessionId: 's1' })
    vi.mocked(api.dictationChunk).mockReset().mockResolvedValue(undefined)
    vi.mocked(api.dictationPoll).mockReset().mockResolvedValue('live so far')
    vi.mocked(api.dictationCommit).mockReset().mockResolvedValue('live so far and more')
    vi.mocked(api.dictationClose).mockReset().mockResolvedValue(undefined)
    vi.mocked(api.dictationTranscribe).mockReset().mockResolvedValue('The whole take, cleanly.')
    let starts = 0
    vi.mocked(api.dictationStart).mockImplementation(async () => {
      starts += 1
      return { sessionId: starts === 1 ? 's1' : 's2' }
    })
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('opens the session through Rust before the mic, commits at phrase ends, and prefers the correction pass', async () => {
    const take = new Uint8Array([1, 2, 3])
    const { wrapper, fake, dictation } = harness(take)
    vi.mocked(api.dictationCommit).mockImplementation(async (id: string) =>
      id === 's2' ? 'The whole take, cleanly.' : 'live so far and more',
    )
    vi.mocked(api.dictationPoll).mockImplementation(async (id: string) =>
      id === 's2' ? 'The whole take, cleanly.' : 'live so far',
    )

    expect(await dictation.start()).toBe(true)
    expect(api.dictationStart).toHaveBeenCalledWith('p1', 'Notes.')
    expect(dictation.state.value).toBe('recording')

    // Leading silence is never sent.
    fake.silence(1)
    fake.speak(4.9)
    fake.silence(0.5)
    await flushPromises()
    expect(api.dictationChunk).not.toHaveBeenCalled()

    // 4.9 s speech + 0.5 s + 0.3 s silence = over 5 s and a 800 ms pause → commit.
    fake.silence(0.3)
    await flushPromises()
    expect(api.dictationChunk).toHaveBeenCalledTimes(1)
    expect(vi.mocked(api.dictationChunk).mock.calls[0][0]).toBe('s1')
    expect(vi.mocked(api.dictationChunk).mock.calls[0][2]).toBe(true)

    // 40 s without a pause commits on its own.
    fake.speak(40)
    await flushPromises()
    expect(api.dictationChunk).toHaveBeenCalledTimes(2)

    // Interim text is polled while recording.
    await vi.advanceTimersByTimeAsync(1800)
    await nextTick()
    expect(dictation.interim.value).toBe('live so far')

    const text = await dictation.stop()
    expect(text).toBe('The whole take, cleanly.')
    expect(api.dictationStart).toHaveBeenCalledTimes(2)
    const correctionChunks = vi.mocked(api.dictationChunk).mock.calls.filter((c) => c[0] === 's2')
    expect(correctionChunks.length).toBeGreaterThanOrEqual(2)
    expect(correctionChunks.every((c) => c[2] === true)).toBe(true)
    expect(api.dictationTranscribe).not.toHaveBeenCalled()
    expect(api.dictationCommit).toHaveBeenCalledWith('s1')
    expect(api.dictationCommit).toHaveBeenCalledWith('s2')
    expect(api.dictationClose).toHaveBeenCalledWith('s1')
    expect(api.dictationClose).toHaveBeenCalledWith('s2')
    expect(dictation.state.value).toBe('idle')
    expect(dictation.interim.value).toBe('')
    wrapper.unmount()
  })

  it('does not apply a stale poll after the session changes', async () => {
    vi.mocked(api.dictationPoll).mockImplementation(async (id: string) =>
      id === 's1' ? 'old session' : 'new session',
    )
    const { wrapper, dictation } = harness(new Uint8Array([1]))
    await dictation.start()
    await dictation.cancel()
    vi.mocked(api.dictationStart).mockResolvedValue({ sessionId: 's2' })
    await dictation.start()
    await vi.advanceTimersByTimeAsync(1800)
    await nextTick()
    expect(dictation.interim.value).not.toBe('old session')
    wrapper.unmount()
  })

  it('falls back to the live text when the correction pass yields nothing', async () => {
    vi.mocked(api.dictationCommit).mockImplementation(async (id: string) =>
      id === 's2' ? '' : 'live so far and more',
    )
    vi.mocked(api.dictationTranscribe).mockResolvedValue('')
    const { wrapper, fake, dictation } = harness(new Uint8Array([9]))
    await dictation.start()
    fake.speak(6)
    expect(await dictation.stop()).toBe('live so far and more')
    wrapper.unmount()
  })

  it('falls back to the encoded take when the correction session fails', async () => {
    vi.mocked(api.dictationStart).mockImplementation(async () => {
      const n = vi.mocked(api.dictationStart).mock.calls.length
      if (n === 1) {
        return { sessionId: 's1' }
      }
      throw { code: 'network', message: 'down' }
    })
    const take = new Uint8Array([9])
    const { wrapper, fake, dictation } = harness(take)
    await dictation.start()
    fake.speak(6)
    expect(await dictation.stop()).toBe('The whole take, cleanly.')
    expect(api.dictationTranscribe).toHaveBeenCalledWith('p1', 'Notes.', take, 'audio/webm')
    wrapper.unmount()
  })

  it('never opens the mic when the project has no Dictation model', async () => {
    vi.mocked(api.dictationStart).mockRejectedValue({ code: 'voice_model_unset', message: '' })
    let micOpened = false
    const { wrapper, dictation } = harness(new Uint8Array(), {
      async start() {
        micOpened = true
      },
      async stop() {
        return { bytes: new Uint8Array(), mime: '' }
      },
    })

    expect(await dictation.start()).toBe(false)
    expect(micOpened).toBe(false)
    expect(dictation.state.value).toBe('idle')
    expect((dictation.error.value as { code: string }).code).toBe('voice_model_unset')
    wrapper.unmount()
  })
})
