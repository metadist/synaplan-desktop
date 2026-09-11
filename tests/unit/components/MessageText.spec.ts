import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { messages } from '@/i18n'
import MessageText from '@/components/MessageText.vue'

function factory(content: string) {
  const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages })
  return mount(MessageText, {
    props: { content },
    global: { plugins: [i18n] },
  })
}

describe('MessageText', () => {
  it('renders a trusted markdown image so it can display in chat', () => {
    const wrapper = factory('Here is a cat:\n\n![Kaize](https://images.pexels.com/cat.jpg)')
    const img = wrapper.get('img.chat-image')
    expect(img.attributes('src')).toBe('https://images.pexels.com/cat.jpg')
    expect(img.attributes('alt')).toBe('Kaize')
  })

  it('turns an untrusted image source into a caption instead of a broken chip', () => {
    const wrapper = factory('![Kaize](cat.jpg)')
    expect(wrapper.find('img').exists()).toBe(false)
    expect(wrapper.get('.chat-image-fallback').text()).toBe('Kaize')
  })

  it('still renders ordinary markdown', () => {
    const wrapper = factory('**Hello** and a [link](https://synaplan.com)')
    expect(wrapper.html()).toContain('<strong>Hello</strong>')
    expect(wrapper.get('a').attributes('href')).toBe('https://synaplan.com')
  })
})
