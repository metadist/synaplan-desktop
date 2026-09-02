import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { messages } from '@/i18n'

vi.mock('@/services/tauri', () => ({
  defaultDeviceName: vi.fn().mockResolvedValue('Test Box'),
  pair: vi.fn(),
  pairWithKey: vi.fn(),
  asCommandError: (e: unknown) =>
    e && typeof e === 'object' && 'code' in e ? e : { code: 'unexpected', message: String(e) },
}))

import PairView from '@/views/PairView.vue'
import * as api from '@/services/tauri'
import { useConfigStore } from '@/stores/config'

function factory() {
  const pinia = createPinia()
  setActivePinia(pinia)
  const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages })
  return mount(PairView, { global: { plugins: [pinia, i18n] } })
}

describe('PairView', () => {
  beforeEach(() => {
    vi.mocked(api.pair).mockReset()
    vi.mocked(api.pairWithKey).mockReset()
    vi.mocked(api.defaultDeviceName).mockResolvedValue('Test Box')
  })

  it('pre-fills the local API address in development', async () => {
    const wrapper = factory()
    await flushPromises()
    expect((wrapper.find('#address').element as HTMLInputElement).value).toBe(
      'http://localhost:8000',
    )
  })

  it('shows the invalid-code message when the server rejects the code', async () => {
    vi.mocked(api.pair).mockRejectedValue({ code: 'invalid_code', message: 'nope' })
    const wrapper = factory()
    await flushPromises()

    await wrapper.find('#address').setValue('https://web.synaplan.com')
    await wrapper.find('#code').setValue('BADCODE')
    await wrapper.find('form').trigger('submit')
    await flushPromises()

    expect(wrapper.find('.banner-error').text()).toBe(messages.en.errors.invalid_code)
  })

  it('stores the paired status on success', async () => {
    const status = {
      paired: true,
      apiBaseUrl: 'https://web.synaplan.com',
      deviceId: 1,
      keyBackend: 'memory',
      keyIsPlaintext: false,
      plaintextKeyPath: null,
    }
    vi.mocked(api.pair).mockResolvedValue(status)
    const wrapper = factory()
    await flushPromises()

    await wrapper.find('#address').setValue('https://web.synaplan.com')
    await wrapper.find('#code').setValue('ABCD1234')
    await wrapper.find('form').trigger('submit')
    await flushPromises()

    expect(useConfigStore().paired).toBe(true)
  })

  it('pre-fills the computer name from the OS hostname', async () => {
    const wrapper = factory()
    await flushPromises()
    expect((wrapper.find('#name').element as HTMLInputElement).value).toBe('Test Box')
  })

  it('pairs with a pasted API key from the key tab', async () => {
    const status = {
      paired: true,
      apiBaseUrl: 'http://localhost:8000',
      deviceId: null,
      keyBackend: 'plaintext',
      keyIsPlaintext: true,
      plaintextKeyPath: '/tmp/key.plaintext',
    }
    vi.mocked(api.pairWithKey).mockResolvedValue(status)
    const wrapper = factory()
    await flushPromises()

    await wrapper.get('[data-testid="tab-use-key"]').trigger('click')
    await wrapper.find('#address2').setValue('http://localhost:8000')
    await wrapper.find('#key').setValue('sk_test_local')
    await wrapper.get('[data-testid="form-use-key"]').trigger('submit')
    await flushPromises()

    expect(api.pairWithKey).toHaveBeenCalledWith('http://localhost:8000', 'sk_test_local')
    expect(useConfigStore().paired).toBe(true)
  })
})
