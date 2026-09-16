import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { messages } from '@/i18n'

vi.mock('@/services/tauri', () => ({
  runDoctor: vi.fn(),
  openUrl: vi.fn(),
  asCommandError: (e: unknown) =>
    e && typeof e === 'object' && 'code' in e ? e : { code: 'unexpected', message: String(e) },
}))

import DoctorView from '@/views/DoctorView.vue'
import * as api from '@/services/tauri'

async function factory() {
  const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages })
  const wrapper = mount(DoctorView, { global: { plugins: [i18n] } })
  await flushPromises()
  return wrapper
}

describe('DoctorView', () => {
  beforeEach(() => {
    vi.mocked(api.runDoctor).mockReset()
    vi.mocked(api.runDoctor).mockResolvedValue([
      {
        id: 'python',
        name: 'Python',
        found: true,
        path: '/usr/bin/python3',
        version: '3.12.0',
        hint: '',
      },
      {
        id: 'node',
        name: 'Node.js',
        found: false,
        path: null,
        version: null,
        hint: 'Install Node.js to run JavaScript skills.',
      },
    ])
  })

  it('shows which local tools are present and which are missing', async () => {
    const wrapper = await factory()
    expect(wrapper.text()).toContain('Python')
    expect(wrapper.text()).toContain('Found')
    expect(wrapper.text()).toContain('3.12.0')
    expect(wrapper.text()).toContain('Node.js')
    expect(wrapper.text()).toContain('Missing')
    expect(wrapper.text()).toContain('Install Node.js')
  })
})
