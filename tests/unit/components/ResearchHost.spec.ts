import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { messages } from '@/i18n'
import type { ResearchEventPayload, ResearchThreadRef } from '@/services/tauri'

const h = vi.hoisted(() => ({
  eventCb: null as ((event: ResearchEventPayload) => void) | null,
}))

vi.mock('@/services/tauri', () => ({
  researchCapabilities: vi.fn().mockResolvedValue({
    enabled: true,
    researchProtocol: 1,
    depths: ['standard', 'thorough'],
    webSearchAvailable: true,
    webSearchProvider: null,
    knowledgeAvailable: true,
    readDocuments: true,
    strictWorld: false,
    maxActiveThorough: 1,
    limits: { standardPerDay: null, thoroughPerDay: null },
  }),
  onResearchEvent: vi.fn(async (cb: (event: ResearchEventPayload) => void) => {
    h.eventCb = cb
    return () => {}
  }),
  researchState: vi.fn(),
  researchAttach: vi.fn().mockResolvedValue(undefined),
  researchFollowUp: vi.fn(),
  researchSaveReport: vi.fn().mockResolvedValue({ path: '/out/r.md', sourcesPath: '', version: 1 }),
  researchListVersions: vi.fn().mockResolvedValue([1]),
  researchSetTray: vi.fn().mockResolvedValue(undefined),
  researchNotify: vi.fn().mockResolvedValue(undefined),
  asCommandError: (err: { code?: string; message?: string }) => ({
    code: err?.code ?? 'unexpected',
    message: err?.message ?? '',
  }),
}))

import * as api from '@/services/tauri'
import ResearchHost from '@/components/research/ResearchHost.vue'

function i18n() {
  return createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages })
}

const complete = {
  id: 'rr_a',
  status: 'complete',
  depth: 'standard',
  question: 'Which suppliers?',
  report: { markdown: 'Price [1].', version: 1 },
  sources: [{ id: 's1', title: 'Price list' }],
  outcome: { code: 'complete', text: '', params: { statements: 1, supported: 1 } },
  check: { statements: 1, supported: 1, removed: 0 },
}

function saved(runId: string, status: string): ResearchThreadRef {
  return { runId, startedAt: '2026-10-27T10:00:00Z', status, reportPath: '', version: 1 }
}

function mountHost(savedRef: ResearchThreadRef | null, chatId = 'c1') {
  return mount(ResearchHost, {
    props: {
      open: true,
      projectId: 'p1',
      chatId,
      webOn: true,
      fileCount: 0,
      skills: [],
      saved: savedRef,
    },
    global: { plugins: [i18n()] },
  })
}

describe('research host', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    h.eventCb = null
  })

  it('shows a finished run saved on the chat and saves its report', async () => {
    vi.mocked(api.researchState).mockResolvedValue(complete)
    const wrapper = mountHost(saved('rr_a', 'complete'))
    await flushPromises()
    expect(wrapper.find('[data-testid="research-result"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('1 of 1 statements are backed by a source.')
    expect(api.researchSaveReport).toHaveBeenCalledOnce()
    expect(api.researchAttach).not.toHaveBeenCalled()
  })

  it('follows the new run a follow-up starts', async () => {
    vi.mocked(api.researchState).mockResolvedValue(complete)
    vi.mocked(api.researchFollowUp).mockResolvedValue({
      id: 'rr_b',
      status: 'briefing',
      researchProtocol: 1,
    })
    const wrapper = mountHost(saved('rr_a', 'complete'))
    await flushPromises()
    await wrapper.get('[data-testid="research-result"] button[aria-expanded]').trigger('click')
    const deeper = wrapper
      .findAll('[data-testid="research-more"] button')
      .find((button) => button.text() === 'Go deeper')
    await deeper!.trigger('click')
    await flushPromises()
    expect(api.researchFollowUp).toHaveBeenCalledWith('rr_a', 'deeper', null, expect.any(String))
    expect(api.researchAttach).toHaveBeenCalledWith('rr_b')
    expect(wrapper.emitted('remember')?.at(-1)?.[0]).toMatchObject({ runId: 'rr_b' })
    expect(wrapper.find('[data-testid="research-progress"]').exists()).toBe(true)
  })

  it('ignores events of another run and clears the card on a chat switch', async () => {
    vi.mocked(api.researchState).mockResolvedValue({
      ...complete,
      status: 'gathering',
      report: null,
    })
    const wrapper = mountHost(saved('rr_a', 'gathering'))
    await flushPromises()
    expect(api.researchAttach).toHaveBeenCalledWith('rr_a')
    h.eventCb?.({ runId: 'rr_other', seq: 9, name: 'complete', data: {} })
    await flushPromises()
    expect(wrapper.find('[data-testid="research-progress"]').exists()).toBe(true)

    await wrapper.setProps({ chatId: 'c2', saved: null })
    await flushPromises()
    expect(wrapper.find('[data-testid="research-progress"]').exists()).toBe(false)
    h.eventCb?.({ runId: 'rr_a', seq: 10, name: 'complete', data: {} })
    await flushPromises()
    expect(wrapper.find('[data-testid="research-result"]').exists()).toBe(false)
  })

  it('says so when following a run stops for good', async () => {
    vi.mocked(api.researchState).mockResolvedValue({
      ...complete,
      status: 'gathering',
      report: null,
    })
    const wrapper = mountHost(saved('rr_a', 'gathering'))
    await flushPromises()
    h.eventCb?.({
      runId: 'rr_a',
      seq: null,
      name: 'detached',
      data: { code: 'unauthorized', message: 'x' },
    })
    await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toContain('Pair it again')
  })
})
