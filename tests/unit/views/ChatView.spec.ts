import { beforeEach, describe, expect, it, vi } from 'vitest'
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
  getLastChatModel: vi.fn().mockResolvedValue(null),
  setLastChatModel: vi.fn().mockResolvedValue(undefined),
  getStudioTiles: vi.fn().mockResolvedValue([]),
  setStudioTiles: vi.fn(async (tiles: string[]) => tiles),
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
import * as api from '@/services/tauri'
import type { Skill } from '@/services/tauri'

function skill(name: string, extra: Partial<Skill> = {}): Skill {
  return {
    name,
    description: name,
    dir: `/tmp/${name}`,
    bundled: true,
    enabled: true,
    source: 'bundled',
    license: 'Apache-2.0',
    compatibilityWarning: false,
    allowUnattended: false,
    blocked: false,
    blockedReason: null,
    version: null,
    url: null,
    sha: null,
    needsPython: true,
    needsNode: false,
    needsLibreoffice: false,
    pythonImports: [],
    ...extra,
  }
}

function factory() {
  const pinia = createPinia()
  setActivePinia(pinia)
  const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages })
  return mount(ChatView, { global: { plugins: [pinia, i18n] } })
}

describe('ChatView', () => {
  beforeEach(() => {
    vi.mocked(api.sendChat).mockClear()
    vi.mocked(api.sendAgentChat).mockClear()
    vi.mocked(api.listSkills).mockResolvedValue([])
    vi.mocked(api.getStudioTiles).mockResolvedValue([])
    vi.mocked(api.setStudioTiles).mockClear()
  })

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

  it('restores the last selected model instead of the default', async () => {
    vi.mocked(api.listModels).mockResolvedValueOnce([
      { id: 'gpt-4o-mini', provider: 'openai' },
      { id: 'claude-fable-5-1', provider: 'anthropic' },
    ])
    vi.mocked(api.getLastChatModel).mockResolvedValueOnce('claude-fable-5-1')
    const wrapper = factory()
    await flushPromises()

    const select = wrapper.get('select.model-select').element as HTMLSelectElement
    expect(select.value).toBe('claude-fable-5-1')
  })

  it('shows the disconnected copy on an unauthorized stream error', async () => {
    const wrapper = factory()
    await flushPromises()

    h.errorCb?.({ code: 'unauthorized', message: 'gone' })
    await flushPromises()

    expect(wrapper.find('.banner-error').text()).toBe(messages.en.errors.unauthorized)
  })

  it('shows three default example tiles for ready skills', async () => {
    vi.mocked(api.listSkills).mockResolvedValueOnce([
      skill('email-draft'),
      skill('calendar-event'),
      skill('vcard'),
      skill('slides'),
      skill('pptx', { blocked: true }),
    ])
    const wrapper = factory()
    await flushPromises()

    expect(wrapper.get('[data-testid="task-studio"]').text()).toContain('What can I do here')
    expect(wrapper.text()).toContain('Follow up in Outlook')
    expect(wrapper.text()).toContain('Put it on the calendar')
    expect(wrapper.text()).toContain('Save a contact')
    expect(wrapper.text()).not.toContain('Pitch it in five slides')
    expect(wrapper.text()).not.toContain('While you are away')
    expect(wrapper.findAll('.studio-card')).toHaveLength(3)
  })

  it('fills the composer from a card without sending', async () => {
    vi.mocked(api.listSkills).mockResolvedValueOnce([skill('email-draft')])
    const wrapper = factory()
    await flushPromises()

    await wrapper.get('[data-task="followupEmail"]').trigger('click')

    const textarea = wrapper.get('textarea').element as HTMLTextAreaElement
    expect(textarea.value).toContain('.eml')
    expect(api.sendAgentChat).not.toHaveBeenCalled()
    expect(api.sendChat).not.toHaveBeenCalled()
  })

  it('shows the user-saved example tiles', async () => {
    vi.mocked(api.getStudioTiles).mockResolvedValueOnce(['slides', 'invoice', 'chart'])
    vi.mocked(api.listSkills).mockResolvedValueOnce([
      skill('email-draft'),
      skill('slides'),
      skill('invoice'),
      skill('chart'),
    ])
    const wrapper = factory()
    await flushPromises()

    expect(wrapper.text()).toContain('Pitch it in five slides')
    expect(wrapper.text()).toContain('Send a clean invoice')
    expect(wrapper.text()).toContain('Chart these numbers')
    expect(wrapper.text()).not.toContain('Follow up in Outlook')
    expect(wrapper.findAll('.studio-card')).toHaveLength(3)
  })

  it('lets the user change which example tiles are shown', async () => {
    vi.mocked(api.listSkills).mockResolvedValueOnce([
      skill('email-draft'),
      skill('calendar-event'),
      skill('vcard'),
      skill('slides'),
    ])
    const wrapper = factory()
    await flushPromises()

    await wrapper.get('[data-testid="btn-choose-tiles"]').trigger('click')
    const slides = wrapper.get('[data-skill="slides"] input')
    expect((slides.element as HTMLInputElement).disabled).toBe(true)

    await wrapper.get('[data-skill="vcard"] input').setValue(false)
    await flushPromises()
    expect((slides.element as HTMLInputElement).disabled).toBe(false)

    await slides.setValue(true)
    await wrapper.get('[data-testid="btn-save-tiles"]').trigger('click')
    await flushPromises()

    expect(api.setStudioTiles).toHaveBeenCalledWith(['email-draft', 'calendar-event', 'slides'])
    expect(wrapper.text()).toContain('Pitch it in five slides')
    expect(wrapper.text()).not.toContain('Save a contact')
  })
})
