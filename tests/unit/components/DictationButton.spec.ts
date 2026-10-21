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

function factory(language = 'auto', attach = false) {
  const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages })
  return mount(DictationButton, {
    props: { projectId: 'p1', language },
    attachTo: attach ? document.body : undefined,
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

  it('moves through the list with the keyboard and returns focus on Escape', async () => {
    const wrapper = factory('en', true)
    const trigger = wrapper.get('[data-testid="dictation-language"]')
    await trigger.trigger('keydown', { key: 'ArrowDown' })
    const menu = wrapper.get('[data-testid="dictation-language-menu"]')
    expect(document.activeElement).toBe(wrapper.get('[data-lang="en"]').element)
    await menu.trigger('keydown', { key: 'ArrowDown' })
    expect(document.activeElement).toBe(wrapper.get('[data-lang="de"]').element)
    await menu.trigger('keydown', { key: 'Home' })
    expect(document.activeElement).toBe(wrapper.get('[data-lang="auto"]').element)
    await menu.trigger('keydown', { key: 'Escape' })
    expect(wrapper.find('[data-testid="dictation-language-menu"]').exists()).toBe(false)
    expect(document.activeElement).toBe(trigger.element)
    wrapper.unmount()
  })

  it('opens downward when the control is near the top of the window', async () => {
    const previous = window.innerHeight
    window.innerHeight = 800
    const wrapper = factory('auto')
    const trigger = wrapper.get('[data-testid="dictation-language"]')
    trigger.element.getBoundingClientRect = () =>
      ({
        top: 12,
        bottom: 40,
        left: 0,
        right: 80,
        width: 80,
        height: 28,
        x: 0,
        y: 12,
        toJSON: () => ({}),
      }) as DOMRect
    await trigger.trigger('click')
    expect(wrapper.get('[data-testid="dictation-language-menu"]').classes()).toContain('down')
    window.innerHeight = previous
  })
})
