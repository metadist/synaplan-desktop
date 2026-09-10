import { onUnmounted, ref, type Ref } from 'vue'
import { Editor, defaultValueCtx, editorViewCtx, rootCtx, serializerCtx } from '@milkdown/kit/core'
import { commonmark } from '@milkdown/kit/preset/commonmark'
import {
  createCodeBlockCommand,
  toggleEmphasisCommand,
  toggleInlineCodeCommand,
  toggleLinkCommand,
  toggleStrongCommand,
  turnIntoTextCommand,
  wrapInBulletListCommand,
  wrapInHeadingCommand,
  wrapInOrderedListCommand,
} from '@milkdown/kit/preset/commonmark'
import { gfm } from '@milkdown/kit/preset/gfm'
import { history } from '@milkdown/kit/plugin/history'
import { clipboard } from '@milkdown/kit/plugin/clipboard'
import { $prose, callCommand, replaceAll } from '@milkdown/kit/utils'
import { Plugin, PluginKey, Selection, TextSelection } from '@milkdown/kit/prose/state'

/** What the toolbar can ask for; each maps to one Milkdown command. */
export type EditorAction =
  | 'paragraph'
  | 'heading1'
  | 'heading2'
  | 'heading3'
  | 'bold'
  | 'italic'
  | 'bulletList'
  | 'orderedList'
  | 'inlineCode'
  | 'codeBlock'

export interface EditorRange {
  /** ProseMirror document positions, not Markdown offsets. */
  start: number
  end: number
}

/**
 * One Milkdown instance for one Markdown file. Markdown in, Markdown out —
 * the file stays the only storage format. `onChange` fires synchronously on
 * every document change with the serialised Markdown, so the note manager's
 * dirty flag and autosave see the same thing the person sees.
 *
 * Positions handed out by `selection()` are ProseMirror positions and only
 * meaningful for `replaceRange()` on the same instance — dictation uses them
 * to swap one take's interim span, never to address the Markdown string.
 */
export function useMarkdownEditor(host: Ref<HTMLElement | null>, onChange: (md: string) => void) {
  const ready = ref(false)
  let editor: Editor | null = null
  /** The last Markdown we handed out; used to ignore our own echo. */
  let lastMarkdown = ''
  /** True while a file is being loaded — that is not an edit of the person's. */
  let loading = false

  const changePlugin = $prose(
    (ctx) =>
      new Plugin({
        key: new PluginKey('synaplan-change'),
        view: () => ({
          update: (view, prevState) => {
            if (loading || view.state.doc.eq(prevState.doc)) {
              return
            }
            const md = ctx.get(serializerCtx)(view.state.doc)
            lastMarkdown = md
            onChange(md)
          },
        }),
      }),
  )

  async function create(markdown: string): Promise<void> {
    if (!host.value || editor) {
      return
    }
    lastMarkdown = markdown
    editor = await Editor.make()
      .config((ctx) => {
        ctx.set(rootCtx, host.value as HTMLElement)
        ctx.set(defaultValueCtx, markdown)
      })
      .use(commonmark)
      .use(gfm)
      .use(history)
      .use(clipboard)
      .use(changePlugin)
      .create()
    ready.value = true
  }

  async function destroy(): Promise<void> {
    if (editor) {
      const e = editor
      editor = null
      ready.value = false
      await e.destroy()
    }
  }

  /** Load another file's Markdown (or an outside change to this one). */
  function load(markdown: string): void {
    if (!editor || markdown === lastMarkdown) {
      return
    }
    lastMarkdown = markdown
    loading = true
    try {
      editor.action(replaceAll(markdown))
    } finally {
      loading = false
    }
  }

  function markdown(): string {
    return editor
      ? editor.action((ctx) => ctx.get(serializerCtx)(ctx.get(editorViewCtx).state.doc))
      : ''
  }

  function selection(): EditorRange {
    if (!editor) {
      return { start: 0, end: 0 }
    }
    return editor.action((ctx) => {
      const { from, to } = ctx.get(editorViewCtx).state.selection
      return { start: from, end: to }
    })
  }

  /**
   * Replace `[start, end)` with plain text and put the caret after it. A space
   * is added on either side where the text would otherwise run into a word
   * (punctuation is allowed to attach directly).
   * Returns the new caret position (the end of the inserted span).
   */
  function replaceRange(start: number, end: number, text: string): number {
    if (!editor) {
      return 0
    }
    return editor.action((ctx) => {
      const view = ctx.get(editorViewCtx)
      // Clamp to the first/last position that is inside a text block, so an
      // out-of-range position lands at the end of the last paragraph instead
      // of opening a new one after it.
      const first = Selection.atStart(view.state.doc).from
      const last = Selection.atEnd(view.state.doc).to
      const from = Math.max(first, Math.min(start, last))
      const to = Math.max(from, Math.min(end, last))
      const flat = text.replace(/\s*\n\s*/g, ' ')
      const doc = view.state.doc
      const before = from > first ? doc.textBetween(from - 1, from, '\n') : ''
      const after = to < last ? doc.textBetween(to, to + 1, '\n') : ''
      const isWordChar = (c: string) => c !== '' && !/[\s.,;:!?)\]}]/.test(c)
      const lead = flat !== '' && isWordChar(before) && !/^[.,;:!?)\]}]/.test(flat) ? ' ' : ''
      const tail = flat !== '' && isWordChar(after) && !/[\s(\[{]$/.test(flat) ? ' ' : ''
      const inserted = lead + flat + tail
      let tr = view.state.tr.insertText(inserted, from, to)
      const caret = from + inserted.length
      tr = tr.setSelection(TextSelection.create(tr.doc, caret))
      view.dispatch(tr)
      return caret
    })
  }

  /** Insert at the current selection (replacing it). */
  function insertAtCaret(text: string): number {
    const sel = selection()
    return replaceRange(sel.start, sel.end, text)
  }

  function focus(): void {
    editor?.action((ctx) => ctx.get(editorViewCtx).focus())
  }

  function run(action: EditorAction): void {
    if (!editor) {
      return
    }
    switch (action) {
      case 'paragraph':
        editor.action(callCommand(turnIntoTextCommand.key))
        break
      case 'heading1':
        editor.action(callCommand(wrapInHeadingCommand.key, 1))
        break
      case 'heading2':
        editor.action(callCommand(wrapInHeadingCommand.key, 2))
        break
      case 'heading3':
        editor.action(callCommand(wrapInHeadingCommand.key, 3))
        break
      case 'bold':
        editor.action(callCommand(toggleStrongCommand.key))
        break
      case 'italic':
        editor.action(callCommand(toggleEmphasisCommand.key))
        break
      case 'bulletList':
        editor.action(callCommand(wrapInBulletListCommand.key))
        break
      case 'orderedList':
        editor.action(callCommand(wrapInOrderedListCommand.key))
        break
      case 'inlineCode':
        editor.action(callCommand(toggleInlineCodeCommand.key))
        break
      case 'codeBlock':
        editor.action(callCommand(createCodeBlockCommand.key))
        break
    }
    focus()
  }

  /** Link the selection to `href`; with nothing selected, insert the URL as a link. */
  function link(href: string): void {
    if (!editor) {
      return
    }
    const sel = selection()
    if (sel.start === sel.end) {
      // Nothing selected: the URL itself becomes the link text (with glue).
      const caret = replaceRange(sel.start, sel.end, href)
      editor.action((ctx) => {
        const view = ctx.get(editorViewCtx)
        const span = view.state.doc.textBetween(sel.start, caret, '\n')
        const at = sel.start + Math.max(0, span.indexOf(href))
        view.dispatch(
          view.state.tr.setSelection(TextSelection.create(view.state.doc, at, at + href.length)),
        )
      })
    }
    editor.action(callCommand(toggleLinkCommand.key, { href }))
    focus()
  }

  onUnmounted(() => {
    void destroy()
  })

  return {
    ready,
    create,
    destroy,
    load,
    markdown,
    selection,
    replaceRange,
    insertAtCaret,
    focus,
    run,
    link,
  }
}
