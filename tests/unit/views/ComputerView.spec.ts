import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { messages } from '@/i18n'
import type { FilesystemPolicy } from '@/services/tauri'

const policy: FilesystemPolicy = {
  read: ['/home/u/Documents'],
  outbox: '/home/u/Synaplan/out',
  deny: ['/etc'],
  maxFileBytes: 100,
}

vi.mock('@/services/tauri', () => ({
  getFilesystemPolicy: vi.fn(),
  addReadFolder: vi.fn(),
  removeReadFolder: vi.fn(),
  pickFolder: vi.fn(),
  revealPath: vi.fn().mockResolvedValue(undefined),
  openUrl: vi.fn(),
  asCommandError: (e: unknown) =>
    e && typeof e === 'object' && 'code' in e ? e : { code: 'unexpected', message: String(e) },
}))

import ComputerView from '@/views/ComputerView.vue'
import * as api from '@/services/tauri'

async function factory() {
  const pinia = createPinia()
  setActivePinia(pinia)
  const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages })
  const wrapper = mount(ComputerView, { global: { plugins: [pinia, i18n] } })
  await flushPromises()
  return wrapper
}

describe('ComputerView', () => {
  beforeEach(() => {
    vi.mocked(api.getFilesystemPolicy).mockReset()
    vi.mocked(api.getFilesystemPolicy).mockResolvedValue(policy)
    vi.mocked(api.addReadFolder).mockReset()
    vi.mocked(api.removeReadFolder).mockReset()
  })

  it('lists the out-box, allowed folders, and blocked paths', async () => {
    const wrapper = await factory()
    expect(wrapper.text()).toContain('/home/u/Synaplan/out')
    expect(wrapper.text()).toContain('/home/u/Documents')
    expect(wrapper.text()).toContain('/etc')
    expect(wrapper.text()).toContain('This computer has not checked in yet.')
  })

  it('adds a typed folder to the allowlist', async () => {
    vi.mocked(api.addReadFolder).mockResolvedValue({
      ...policy,
      read: ['/home/u/Documents', '/home/u/Pictures'],
    })
    const wrapper = await factory()
    await wrapper.get('.add-row input').setValue('/home/u/Pictures')
    await wrapper.get('.add-row .btn-ghost').trigger('click')
    await flushPromises()
    expect(api.addReadFolder).toHaveBeenCalledWith('/home/u/Pictures')
    expect(wrapper.text()).toContain('/home/u/Pictures')
  })
})
