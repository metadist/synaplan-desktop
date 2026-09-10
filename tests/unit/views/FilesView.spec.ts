import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { messages } from '@/i18n'
import type { FileDropEvent, KnowledgeFile, Project } from '@/services/tauri'

const h = vi.hoisted(() => ({
  dropCb: null as ((e: FileDropEvent) => void) | null,
}))

vi.mock('@/services/tauri', () => ({
  listProjects: vi.fn(),
  listProjectFiles: vi.fn(),
  uploadProjectFile: vi.fn(),
  deleteProjectFile: vi.fn(),
  onFileDrop: vi.fn(async (cb: (e: FileDropEvent) => void) => {
    h.dropCb = cb
    return () => {}
  }),
  asCommandError: (e: unknown) =>
    e && typeof e === 'object' && 'code' in e ? e : { code: 'unexpected', message: String(e) },
}))

import FilesView from '@/views/FilesView.vue'
import { useProjectsStore } from '@/stores/projects'
import { useUiStore } from '@/stores/ui'
import * as api from '@/services/tauri'

function project(id: string, embed: string): Project {
  return {
    id,
    slug: id,
    name: 'Work',
    kind: 'project',
    createdAt: '2026-09-10T00:00:00Z',
    updatedAt: '2026-09-10T00:00:00Z',
    dictationLanguage: 'en',
    defaultAssistantId: null,
    assistantIds: [],
    enabledSkills: [],
    models: {
      chat: 'openai:gpt-4o-mini:chat',
      voice: '',
      speak: '',
      vision: '',
      image: '',
      video: '',
      embed,
      docs: '',
      chatLegacyProviderId: null,
    },
    knowledgeFolder: `DESKTOP:${id}`,
    projectDir: `/home/u/Synaplan/projects/${id}`,
    notesDir: `/home/u/Synaplan/projects/${id}/notes`,
    outDir: `/home/u/Synaplan/projects/${id}/out`,
  }
}

function file(
  id: number,
  name: string,
  state: KnowledgeFile['state'],
  detail: string | null = null,
) {
  return { id, name, size: 2048, state, detail, uploadedAt: '2026-09-10T10:00:00Z' }
}

async function factory(active: Project) {
  const pinia = createPinia()
  setActivePinia(pinia)
  const i18n = createI18n({ legacy: false, locale: 'en', fallbackLocale: 'en', messages })
  vi.mocked(api.listProjects).mockResolvedValue({
    projects: [active],
    activeId: active.id,
    personalId: active.id,
  })
  await useProjectsStore().load()
  const wrapper = mount(FilesView, {
    attachTo: document.body,
    global: { plugins: [pinia, i18n] },
  })
  await flushPromises()
  return wrapper
}

describe('FilesView', () => {
  beforeEach(() => {
    h.dropCb = null
    vi.mocked(api.listProjectFiles).mockReset()
    vi.mocked(api.listProjectFiles).mockResolvedValue([])
    vi.mocked(api.uploadProjectFile).mockReset()
    vi.mocked(api.deleteProjectFile).mockReset()
    vi.mocked(api.deleteProjectFile).mockResolvedValue(undefined)
  })

  afterEach(() => {
    document.body.innerHTML = ''
  })

  it('refuses to send anything while the project has no index model', async () => {
    const wrapper = await factory(project('p1', ''))

    expect(wrapper.find('[data-testid="files-embed-unset"]').exists()).toBe(true)
    expect((wrapper.get('[data-testid="files-path"]').element as HTMLInputElement).disabled).toBe(
      true,
    )
    h.dropCb?.({ type: 'drop', paths: ['/home/u/Documents/report.pdf'] })
    await flushPromises()
    expect(api.uploadProjectFile).not.toHaveBeenCalled()
    expect(wrapper.text()).not.toMatch(/DESKTOP:|group_key|VECTORIZE/)
  })

  it('uploads dropped files through Rust and lists the result in plain language', async () => {
    vi.mocked(api.uploadProjectFile).mockResolvedValue(file(11, 'report.pdf', 'sent'))
    const wrapper = await factory(project('p1', 'ollama:bge-m3:vectorize'))

    expect(wrapper.get('[data-testid="files-index-model"]').text()).toContain('bge-m3')
    expect(wrapper.text()).not.toContain('ollama:bge-m3:vectorize')

    h.dropCb?.({ type: 'enter', paths: ['/home/u/Documents/report.pdf'] })
    await flushPromises()
    expect(wrapper.get('[data-testid="files-dropzone"]').classes()).toContain('active')

    h.dropCb?.({ type: 'drop', paths: ['/home/u/Documents/report.pdf'] })
    await flushPromises()

    expect(api.uploadProjectFile).toHaveBeenCalledWith('p1', '/home/u/Documents/report.pdf')
    expect(wrapper.get('[data-testid="file-11-state"]').text()).toBe('Sent to Synaplan')
    expect(wrapper.find('[data-testid="files-pending"]').exists()).toBe(false)
  })

  it('adds a typed path and shows a file outside the allowed folders as such', async () => {
    vi.mocked(api.uploadProjectFile).mockRejectedValue({
      code: 'file_outside_allowed',
      message: 'nope',
    })
    const wrapper = await factory(project('p1', 'ollama:bge-m3:vectorize'))

    await wrapper.get('[data-testid="files-path"]').setValue('/tmp/elsewhere/x.pdf')
    await wrapper.get('[data-testid="files-add"]').trigger('submit')
    await flushPromises()

    expect(api.uploadProjectFile).toHaveBeenCalledWith('p1', '/tmp/elsewhere/x.pdf')
    const pending = wrapper.get('[data-testid="files-pending"]')
    expect(pending.text()).toContain('x.pdf')
    expect(pending.text()).toContain('not in a folder this app may use')
    const ui = useUiStore()
    await pending
      .findAll('button')
      .find((b) => b.text().includes('This computer'))!
      .trigger('click')
    expect(ui.view).toBe('computer')
  })

  it('shows the lifecycle states and the failure reason from the workspace', async () => {
    vi.mocked(api.listProjectFiles).mockResolvedValue([
      file(1, 'a.pdf', 'ready'),
      file(2, 'b.docx', 'indexing'),
      file(3, 'c.txt', 'failed', 'Text extraction failed'),
    ])
    const wrapper = await factory(project('p1', 'ollama:bge-m3:vectorize'))

    expect(wrapper.get('[data-testid="file-1-state"]').text()).toBe('Ready for chat')
    expect(wrapper.get('[data-testid="file-2-state"]').text()).toContain('Indexing')
    expect(wrapper.get('[data-testid="file-3-state"]').text()).toBe('Could not index')
    expect(wrapper.get('[data-testid="file-3"]').text()).toContain('Text extraction failed')
  })

  it('removes a file from the workspace only after confirmation', async () => {
    vi.mocked(api.listProjectFiles).mockResolvedValue([file(1, 'a.pdf', 'ready')])
    const wrapper = await factory(project('p1', 'ollama:bge-m3:vectorize'))

    await wrapper.get('[data-testid="file-1-remove"]').trigger('click')
    await flushPromises()
    expect(api.deleteProjectFile).not.toHaveBeenCalled()
    expect(document.body.textContent).toContain('original on this computer is not touched')

    ;(
      document.body.querySelector('[data-testid="confirm-dialog-confirm"]') as HTMLButtonElement
    ).click()
    await flushPromises()
    expect(api.deleteProjectFile).toHaveBeenCalledWith('p1', 1)
    expect(wrapper.find('[data-testid="file-1"]').exists()).toBe(false)
  })
})
