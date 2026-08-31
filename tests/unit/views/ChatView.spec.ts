import { describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { messages } from '@/i18n'

// Capture the event callbacks the view registers so a test can drive them.
const h = vi.hoisted(() => ({
  tokenCb: null as ((t: string) => void) | null,
  doneCb: null as (() => void) | null,
  errorCb: null as ((e: { code: string; message: string }) => void) | null,
}))

vi.mock('@/services/tauri', () => ({
  onChatToken: vi.fn(async (cb: (t: string) => void) => {
    h.tokenCb = cb
    return () => {}
  }),
  onChatDone: vi.fn(async (cb: () => void) => {
    h.doneCb = cb
    return () => {}
  }),
  onChatError: vi.fn(async (cb: (e: { code: string; message: string }) => void) => {
    h.errorCb = cb
    return () => {}
  }),
  onAgentText: vi.fn(async () => () => {}),
  onAgentTool: vi.fn(async () => () => {}),
  onAgentDone: vi.fn(async () => () => {}),
  onAgentError: vi.fn(async () => () => {}),
  listModels: vi.fn().mockResolvedValue([{ id: 'gpt-4o-mini', provider: 'openai' }]),
  sendChat: vi.fn().mockResolvedValue(undefined),
  sendAgentChat: vi.fn().mockResolvedValue(undefined),
  cancelChat: vi.fn().mockResolvedValue(undefined),
  listSkills: vi.fn().mockResolvedValue([]),
  getExecutionConsent: vi.fn().mockResolvedValue(false),
  setExecutionConsent: vi.fn().mockResolvedValue(undefined),
  revealPath: vi.fn().mockResolvedValue(undefined),
  getStatus: vi.fn().mockResolvedValue({
    paired: false,
    apiBaseUrl: null,
    deviceId: null,
    keyBackend: 'memory',
    keyIsPlaintext: false,
  }),
  asCommandError: (e: unknown) =>
    e && typeof e === 'object' && 'code' in e ? e : { code: 'unexpected', message: String(e) },
}))

import ChatView from '@/views/ChatView.vue'

function factory() {
  const pinia = createPinia()
  setActivePinia(pinia)
  const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages })
  return mount(ChatView, { global: { plugins: [pinia, i18n] } })
}

describe('ChatView', () => {
  it('renders streamed tokens into an assistant message', async () => {
    const wrapper = factory()
    await flushPromises()

    await wrapper.find('textarea').setValue('Ping')
    await wrapper.find('button.btn-primary').trigger('click')
    await flushPromises()

    h.tokenCb?.('PO')
    h.tokenCb?.('NG')
    await flushPromises()

    expect(wrapper.text()).toContain('PONG')
  })

  it('shows the disconnected copy on an unauthorized stream error', async () => {
    const wrapper = factory()
    await flushPromises()

    h.errorCb?.({ code: 'unauthorized', message: 'gone' })
    await flushPromises()

    expect(wrapper.find('.banner-error').text()).toBe(messages.en.errors.unauthorized)
  })
})
