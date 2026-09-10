import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest'
import { defineComponent, h, ref } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import {
  isSafeHref,
  sanitizeMarkdownLinks,
  useMarkdownEditor,
} from '@/composables/useMarkdownEditor'

async function harness(markdown: string) {
  const changes: string[] = []
  let md!: ReturnType<typeof useMarkdownEditor>
  const Host = defineComponent({
    setup() {
      const host = ref<HTMLElement | null>(null)
      md = useMarkdownEditor(host, (m) => changes.push(m))
      return () => h('div', { ref: host })
    },
  })
  const wrapper = mount(Host, { attachTo: document.body })
  await md.create(markdown)
  await flushPromises()
  return { wrapper, md, changes }
}

describe('safe note links', () => {
  it('keeps http(s) and mailto and rejects script URLs', () => {
    expect(isSafeHref('https://example.org')).toBe(true)
    expect(isSafeHref('mailto:a@b.c')).toBe(true)
    expect(isSafeHref('#section')).toBe(true)
    expect(isSafeHref('javascript:alert(1)')).toBe(false)
    expect(isSafeHref('data:text/html,x')).toBe(false)
    expect(sanitizeMarkdownLinks('[x](javascript:alert(1))')).toBe('[x](#)')
    expect(sanitizeMarkdownLinks('[ok](https://example.org)')).toBe('[ok](https://example.org)')
  })
})

describe('useMarkdownEditor', () => {
  beforeAll(() => {
    // jsdom has no layout; ProseMirror asks for it when it moves the caret.
    const empty = { length: 0, item: () => null, [Symbol.iterator]: function* () {} }
    Range.prototype.getClientRects = () => empty as unknown as DOMRectList
    Range.prototype.getBoundingClientRect = () =>
      ({ x: 0, y: 0, width: 0, height: 0, top: 0, left: 0, right: 0, bottom: 0 }) as DOMRect
  })

  afterEach(() => {
    document.body.innerHTML = ''
  })

  it('is Markdown in, Markdown out and only reports real changes', async () => {
    const { wrapper, md, changes } = await harness('# Title\n\nBody')
    expect(md.markdown().trim()).toBe('# Title\n\nBody')
    expect(changes).toEqual([])

    md.load('# Title\n\nBody')
    expect(changes).toEqual([])

    md.load('Other file')
    expect(md.markdown().trim()).toBe('Other file')
    // Loading is not an edit of the person's making.
    expect(changes).toEqual([])
    wrapper.unmount()
  })

  it('replaces a span, glues words and lets punctuation attach', async () => {
    const { wrapper, md, changes } = await harness('Fridge')
    // The caret starts at the top of the file; a huge position means "the end".
    expect(md.selection()).toEqual({ start: 1, end: 1 })
    const caret = md.replaceRange(10_000, 10_000, 'buy')
    expect(md.markdown().trim()).toBe('Fridge buy')
    expect(caret).toBe(md.selection().start)

    md.replaceRange(caret, caret, ', please')
    expect(md.markdown().trim()).toBe('Fridge buy, please')

    md.replaceRange(1, 1, 'A\nB')
    expect(md.markdown().trim()).toBe('A B Fridge buy, please')
    expect(changes.at(-1)?.trim()).toBe('A B Fridge buy, please')
    wrapper.unmount()
  })

  it('runs the toolbar commands and links the selection', async () => {
    const { wrapper, md } = await harness('plain')
    md.run('heading2')
    expect(md.markdown().trim()).toBe('## plain')
    md.run('paragraph')
    expect(md.markdown().trim()).toBe('plain')
    md.run('bulletList')
    expect(md.markdown().trim()).toMatch(/^[-*] plain$/)

    const { wrapper: w2, md: md2 } = await harness('read this')
    md2.replaceRange(10_000, 10_000, '')
    md2.link('https://example.org')
    // remark writes a link whose text is its own URL in autolink form.
    expect(md2.markdown().trim()).toBe('read this <https://example.org>')
    wrapper.unmount()
    w2.unmount()
  })

  it('never throws when used before the editor exists', () => {
    const host = ref<HTMLElement | null>(null)
    const onChange = vi.fn()
    let md!: ReturnType<typeof useMarkdownEditor>
    const Host = defineComponent({
      setup() {
        md = useMarkdownEditor(host, onChange)
        return () => h('div')
      },
    })
    const wrapper = mount(Host)
    expect(md.markdown()).toBe('')
    expect(md.selection()).toEqual({ start: 0, end: 0 })
    expect(md.replaceRange(0, 0, 'x')).toBe(0)
    md.run('bold')
    md.link('https://example.org')
    expect(onChange).not.toHaveBeenCalled()
    wrapper.unmount()
  })
})
