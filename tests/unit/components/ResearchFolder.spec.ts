import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { messages } from '@/i18n'
import FolderAddDialog from '@/components/research/FolderAddDialog.vue'
import ResearchTeam from '@/components/research/ResearchTeam.vue'

function i18n() {
  return createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages })
}

describe('folder add dialog', () => {
  it('summarises the walked folder', () => {
    const wrapper = mount(FolderAddDialog, {
      props: {
        walk: {
          root: '/Users/me/Notes',
          files: [
            { path: '/Users/me/Notes/a.pdf', name: 'a.pdf', kind: 'pdf', bytes: 10 },
            { path: '/Users/me/Notes/b.docx', name: 'b.docx', kind: 'word', bytes: 10 },
          ],
          skipped: [{ name: 'clip.mp4', reason: 'kind' }],
          pdf: 1,
          word: 1,
          excel: 0,
          other: 0,
          totalBytes: 20,
          truncated: false,
        },
      },
      global: { plugins: [i18n()] },
    })
    expect(wrapper.text()).toContain('/Users/me/Notes')
    expect(wrapper.text()).toContain('2 files')
    expect(wrapper.text()).toContain('1 PDF')
    expect(wrapper.text()).toContain('1 file')
    expect(wrapper.find('[data-testid="folder-truncated"]').exists()).toBe(false)
  })

  it('asks for a new walk when subfolders are switched on', async () => {
    const wrapper = mount(FolderAddDialog, {
      props: {
        walk: {
          root: '/Users/me/Notes',
          files: [],
          skipped: [],
          pdf: 0,
          word: 0,
          excel: 0,
          other: 0,
          totalBytes: 0,
          truncated: true,
        },
      },
      global: { plugins: [i18n()] },
    })
    expect(wrapper.find('[data-testid="folder-truncated"]').exists()).toBe(true)
    await wrapper.get('input[type="checkbox"]').setValue(true)
    expect(wrapper.emitted('toggleSub')).toEqual([[true]])
  })
})

describe('research team', () => {
  it('resets custom picks back to the recommended empty keys', async () => {
    const wrapper = mount(ResearchTeam, {
      props: {
        lead: '',
        reader: '',
        checker: '',
        chat: 'chat-model',
        docs: 'docs-model',
        options: [{ id: 'chat-model', name: 'Chat model', available: true, reason: null }],
      },
      global: { plugins: [i18n()] },
    })
    expect(wrapper.text()).toContain('Recommended')
    await wrapper.get('[data-testid="research-team-reset"]').trigger('click')
    expect(wrapper.emitted('reset')).toHaveLength(1)
  })
})
