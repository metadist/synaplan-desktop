import { beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent, h, nextTick, ref } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'

vi.mock('@/services/tauri', () => ({
  listNotes: vi.fn(),
  readNote: vi.fn(),
  writeNote: vi.fn(),
  createNote: vi.fn(),
  deleteNote: vi.fn(),
}))

import * as api from '@/services/tauri'
import { composeNote, useNotes } from '@/composables/useNotes'

function note(name: string, content = '', project = 'p1') {
  return {
    name,
    title: name,
    content,
    updatedAt: '2026-09-10T00:00:00Z',
    path: `/tmp/${project}/${name}`,
  }
}

function harness() {
  const projectId = ref('p1')
  let notes!: ReturnType<typeof useNotes>
  const Host = defineComponent({
    setup() {
      notes = useNotes(projectId)
      return () => h('div')
    },
  })
  const wrapper = mount(Host)
  return { wrapper, projectId, notes }
}

describe('useNotes', () => {
  beforeEach(() => {
    vi.mocked(api.listNotes).mockReset().mockResolvedValue([])
    vi.mocked(api.readNote).mockReset()
    vi.mocked(api.writeNote).mockReset().mockResolvedValue({
      name: 'a.md',
      title: 'a',
      updatedAt: '2026-09-10T00:00:00Z',
      size: 1,
    })
    vi.mocked(api.createNote).mockReset()
    vi.mocked(api.deleteNote).mockReset().mockResolvedValue(undefined)
  })

  it('does not open the next note when the current draft failed to save', async () => {
    vi.mocked(api.writeNote).mockRejectedValue({ code: 'project_io', message: 'disk full' })
    vi.mocked(api.readNote).mockResolvedValue(note('b.md', 'other'))
    const { wrapper, notes } = harness()
    notes.current.value = note('a.md', 'saved')
    notes.draft.value = 'dirty'
    await notes.open('b.md')
    await flushPromises()
    expect(api.readNote).not.toHaveBeenCalled()
    expect(notes.current.value?.name).toBe('a.md')
    expect(notes.draft.value).toBe('dirty')
    wrapper.unmount()
  })

  it('writes a dirty draft to the previous project on switch', async () => {
    const { wrapper, projectId, notes } = harness()
    notes.current.value = note('a.md', 'old')
    notes.draft.value = 'new text'
    projectId.value = 'p2'
    await nextTick()
    await flushPromises()
    expect(api.writeNote).toHaveBeenCalledWith('p1', 'a.md', 'new text')
    expect(notes.current.value).toBeNull()
    wrapper.unmount()
  })

  it('ignores a stale search result after the query changes', async () => {
    let resolveFirst: ((value: never[]) => void) | undefined
    vi.mocked(api.listNotes)
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveFirst = resolve as (value: never[]) => void
          }),
      )
      .mockResolvedValueOnce([])
    const { wrapper, notes } = harness()
    notes.query.value = 'alpha'
    await nextTick()
    notes.query.value = 'beta'
    await flushPromises()
    resolveFirst?.([])
    await flushPromises()
    expect(api.listNotes).toHaveBeenLastCalledWith('p1', 'beta')
    wrapper.unmount()
  })

  it('keep saves text as a new note without opening it', async () => {
    vi.mocked(api.createNote).mockResolvedValue(note('2026-09-10-1200.md'))
    vi.mocked(api.writeNote).mockResolvedValue({
      name: '2026-09-10-1200.md',
      title: 'Volcanoes',
      updatedAt: '2026-09-10T12:00:00Z',
      size: 40,
    })
    const { wrapper, notes } = harness()
    const saved = await notes.keep(
      'Volcanoes\nThey are mountains that erupt.',
      'Kept from “Homework”',
    )
    expect(api.writeNote).toHaveBeenCalledWith(
      'p1',
      '2026-09-10-1200.md',
      '# Volcanoes\n\nThey are mountains that erupt.\n\n---\n\nKept from “Homework”',
    )
    expect(saved?.title).toBe('Volcanoes')
    expect(notes.current.value).toBeNull()
    // The list is refreshed so the pill count moves.
    expect(api.listNotes).toHaveBeenCalledWith('p1', '')

    expect(await notes.keep('   ')).toBeNull()
    expect(api.createNote).toHaveBeenCalledTimes(1)
    wrapper.unmount()
  })
})

describe('composeNote', () => {
  it('turns a short first line into the heading and keeps the rest', () => {
    expect(composeNote('Buy milk')).toBe('# Buy milk')
    expect(composeNote('  Plan  \n\n- eggs\n- milk\n')).toBe('# Plan\n\n- eggs\n- milk')
    expect(composeNote('**Bold** _title_ `x`\nbody')).toBe('# Bold title x\n\nbody')
    expect(composeNote('- first item\nsecond')).toBe('# first item\n\nsecond')
    expect(composeNote('## Sub heading\ntext')).toBe('# Sub heading\n\ntext')
  })

  it('leaves an existing heading or a long first line alone', () => {
    expect(composeNote('# Already\n\nbody')).toBe('# Already\n\nbody')
    const long = 'x'.repeat(120)
    expect(composeNote(`${long}\nmore`)).toBe(`${long}\nmore`)
  })

  it('appends the footer under a rule', () => {
    expect(composeNote('Hi', 'Kept from “Homework” on 10 Sep 2026')).toBe(
      '# Hi\n\n---\n\nKept from “Homework” on 10 Sep 2026',
    )
  })
})
