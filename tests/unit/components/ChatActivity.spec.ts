import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { messages } from '@/i18n'
import ChatActivity, { type ActivityPhase } from '@/components/ChatActivity.vue'

function factory(phase: ActivityPhase = 'thinking', compact = false) {
  const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages })
  return mount(ChatActivity, {
    props: { phase, compact },
    global: { plugins: [i18n] },
  })
}

describe('ChatActivity', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  it('names each phase in plain language', () => {
    expect(factory('thinking').text()).toContain('Working on your answer')
    expect(factory('web').text()).toContain('Searching the web')
    expect(factory('image').text()).toContain('Generating an image')
    expect(factory('audio').text()).toContain('Generating audio')
    expect(factory('video').text()).toContain('Creating a video')
    expect(factory('document').text()).toContain('Writing your document')
  })

  it('shows a moving progress bar for server-side work, dots for a quick wait', () => {
    expect(factory('web').find('.activity-bar').exists()).toBe(true)
    expect(factory('image').find('.activity-bar').exists()).toBe(true)
    // A plain answer streams in shortly; dots are honest, a bar would over-promise.
    expect(factory('thinking').find('.activity-bar').exists()).toBe(false)
    expect(factory('thinking').find('.dots').exists()).toBe(true)
  })

  it('reveals elapsed time after a moment and reassures on a long wait', async () => {
    const wrapper = factory('image')
    expect(wrapper.find('[data-testid="chat-activity-elapsed"]').exists()).toBe(false)

    vi.advanceTimersByTime(3000)
    await wrapper.vm.$nextTick()
    expect(wrapper.get('[data-testid="chat-activity-elapsed"]').text()).toContain('3s')
    expect(wrapper.text()).not.toContain('Still working')

    vi.advanceTimersByTime(17000)
    await wrapper.vm.$nextTick()
    expect(wrapper.get('[data-testid="chat-activity-elapsed"]').text()).toContain('20s')
    expect(wrapper.text()).toContain('Still working')
  })

  it('is a slim chip with no bar or reassurance when compact', () => {
    const wrapper = factory('web', true)
    expect(wrapper.get('[data-testid="chat-activity"]').classes()).toContain('compact')
    expect(wrapper.find('.activity-bar').exists()).toBe(false)
    vi.advanceTimersByTime(25000)
    expect(wrapper.text()).not.toContain('Still working')
  })

  it('stops its timer on unmount', () => {
    const clear = vi.spyOn(globalThis, 'clearInterval')
    factory('thinking').unmount()
    expect(clear).toHaveBeenCalled()
  })
})
