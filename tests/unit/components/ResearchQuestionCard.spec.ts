import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { messages } from '@/i18n'
import ResearchQuestionCard from '@/components/research/ResearchQuestionCard.vue'

const question = {
  id: 'q1',
  text: 'Which country?',
  options: ['Germany', 'France'],
  allowText: true,
}

function mountCard() {
  const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages })
  return mount(ResearchQuestionCard, { props: { question }, global: { plugins: [i18n] } })
}

describe('ResearchQuestionCard', () => {
  it('sends a choice, an own answer, or a skip', async () => {
    const wrapper = mountCard()
    await wrapper.findAll('button')[0].trigger('click')
    expect(wrapper.emitted('answer')?.[0]).toEqual([{ choice: 'Germany' }])

    await wrapper.get('[data-testid="research-own-answer"]').setValue('Spain')
    await wrapper.get('form').trigger('submit')
    expect(wrapper.emitted('answer')?.[1]).toEqual([{ text: 'Spain' }])

    await wrapper.get('[data-testid="research-skip"]').trigger('click')
    expect(wrapper.emitted('skip')).toHaveLength(1)
  })
})
