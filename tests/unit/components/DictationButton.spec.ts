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
  it('shows which language dictation will use, resolved (never a raw key)', () => {
    const auto = factory('auto').get('[data-testid="dictation-language"]')
      .element as HTMLSelectElement
    expect(auto.value).toBe('auto')
    const de = factory('de').get('[data-testid="dictation-language"]').element as HTMLSelectElement
    expect(de.value).toBe('de')
    // The visible options are real words, not i18n keys.
    const labels = factory('auto')
      .findAll('option')
      .map((o) => o.text())
    expect(labels).toContain('Auto-detect')
    expect(labels).toContain('German')
    expect(labels.some((l) => l.includes('dictation.'))).toBe(false)
    // The mic says what it will do, so translation is never a surprise.
    expect(
      factory('en').get('[data-testid="dictation-toggle"]').attributes('aria-label'),
    ).toContain('English')
  })

  it('emits the picked language so the parent can persist it on the project', async () => {
    const wrapper = factory('en')
    await wrapper.get('[data-testid="dictation-language"]').setValue('auto')
    expect(wrapper.emitted('update:language')?.[0]).toEqual(['auto'])
  })

  it('does not re-emit when the active language is chosen again', async () => {
    const wrapper = factory('de')
    await wrapper.get('[data-testid="dictation-language"]').setValue('de')
    expect(wrapper.emitted('update:language')).toBeUndefined()
  })
})
