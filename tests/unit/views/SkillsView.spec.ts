import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { messages } from '@/i18n'
import type { Skill } from '@/services/tauri'

vi.mock('@/services/tauri', () => ({
  listSkills: vi.fn(),
  runDoctor: vi.fn(),
  setSkillEnabled: vi.fn(),
  setSkillUnattended: vi.fn(),
  openUrl: vi.fn(),
  asCommandError: (e: unknown) =>
    e && typeof e === 'object' && 'code' in e ? e : { code: 'unexpected', message: String(e) },
}))

import SkillsView from '@/views/SkillsView.vue'
import * as api from '@/services/tauri'

function skill(name: string, extra: Partial<Skill> = {}): Skill {
  return {
    name,
    description: `${name} skill`,
    dir: `/tmp/${name}`,
    bundled: true,
    enabled: true,
    source: 'bundled',
    license: 'Apache-2.0',
    compatibilityWarning: false,
    allowUnattended: false,
    blocked: false,
    blockedReason: null,
    version: '1.0.0',
    url: null,
    sha: null,
    needsPython: false,
    needsNode: false,
    needsLibreoffice: false,
    pythonImports: [],
    ...extra,
  }
}

async function factory() {
  const pinia = createPinia()
  setActivePinia(pinia)
  const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages })
  const wrapper = mount(SkillsView, { global: { plugins: [pinia, i18n] } })
  await flushPromises()
  return wrapper
}

describe('SkillsView', () => {
  beforeEach(() => {
    vi.mocked(api.listSkills).mockReset()
    vi.mocked(api.runDoctor).mockReset()
    vi.mocked(api.setSkillEnabled).mockReset()
    vi.mocked(api.listSkills).mockResolvedValue([
      skill('pptx'),
      skill('email-draft', { enabled: false }),
    ])
    vi.mocked(api.runDoctor).mockResolvedValue([
      {
        id: 'python',
        name: 'Python',
        found: true,
        path: '/usr/bin/python3',
        version: '3',
        hint: '',
      },
    ])
  })

  it('lists installed skills and their on/off state', async () => {
    const wrapper = await factory()
    expect(wrapper.text()).toContain('pptx')
    expect(wrapper.text()).toContain('email-draft')
    expect(wrapper.text()).toContain('Enabled')
    expect(wrapper.text()).toContain('Disabled')
  })

  it('opens the from-folder installer', async () => {
    const wrapper = await factory()
    const buttons = wrapper.findAll('button')
    const fromFolder = buttons.find((b) => b.text() === 'From a folder')
    expect(fromFolder).toBeDefined()
    await fromFolder!.trigger('click')
    expect(wrapper.get('#skill-source').attributes('placeholder')).toBe(
      'Path to a folder that contains SKILL.md',
    )
  })
})
