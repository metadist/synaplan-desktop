import { describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { messages } from '@/i18n'

// The composable imports the tauri bridge; no call happens at mount, but the
// module must resolve in the test environment.
vi.mock('@/services/tauri', () => ({
  dictationStart: vi.fn(),
  dictationChunk: vi.fn(),
  dictationPoll: vi.fn(),
  dictationCommit: vi.fn(),
  dictationClose: vi.fn(),
  dictationTranscribe: vi.fn(),
}))

import DictationButton from '@/components/DictationButton.vue'

function factory(language = 'auto') {
  const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages })
  return mount(DictationButton, {
    props: { projectId: 'p1', language },
    global: { plugins: [i18n] },
  })
}

describe('DictationButton language control', () => {
  it('shows which language dictation will use, resolved (never a raw key)', async () => {
    expect(factory('auto').get('[data-testid="dictation-language"]').text()).toContain(
      'Auto-detect',
    )
    expect(factory('de').get('[data-testid="dictation-language"]').text()).toContain('German')
    const open = factory('auto')
    await open.get('[data-testid="dictation-language"]').trigger('click')
    const labels = open
      .get('[data-testid="dictation-language-menu"]')
      .findAll('button')
      .map((o) => o.text())
    expect(labels).toContain('Auto-detect')
    expect(labels).toContain('German')
    expect(labels.some((l) => l.includes('dictation.'))).toBe(false)
    expect(
      factory('en').get('[data-testid="dictation-toggle"]').attributes('aria-label'),
    ).toContain('English')
  })

  it('emits the picked language so the parent can persist it on the project', async () => {
    const wrapper = factory('en')
    await wrapper.get('[data-testid="dictation-language"]').trigger('click')
    await wrapper.get('[data-lang="auto"]').trigger('click')
    expect(wrapper.emitted('update:language')?.[0]).toEqual(['auto'])
  })

  it('does not re-emit when the active language is chosen again', async () => {
    const wrapper = factory('de')
    await wrapper.get('[data-testid="dictation-language"]').trigger('click')
    await wrapper.get('[data-lang="de"]').trigger('click')
    expect(wrapper.emitted('update:language')).toBeUndefined()
  })
})
