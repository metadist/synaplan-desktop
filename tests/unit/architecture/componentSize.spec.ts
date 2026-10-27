// @vitest-environment node
import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'
import { afterAll, describe, expect, it } from 'vitest'

/**
 * Vue components stay at or under 600 lines. Offenders that already existed
 * when this check arrived are frozen in `componentSizeLedger.json`. A listed
 * file may shrink or disappear, never grow, and a new file over the limit fails.
 *
 * Shrunk or split one? Regenerate with UPDATE_COMPONENT_SIZE=1 and review the
 * diff — removals and smaller numbers only.
 */

const MAX_COMPONENT_LINES = 600
const REPO_ROOT = fileURLToPath(new URL('../../..', import.meta.url))
const LEDGER_PATH = fileURLToPath(new URL('./componentSizeLedger.json', import.meta.url))

interface LedgerFile {
  recorded?: string
  components?: Record<string, number>
}

export function lineCount(text: string): number {
  if (text.length === 0) {
    return 0
  }
  const parts = text.split('\n')
  return text.endsWith('\n') ? parts.length - 1 : parts.length
}

export function componentLines(root: string): Record<string, number> {
  const counts: Record<string, number> = {}
  walk(join(root, 'src'))
  return counts

  function walk(dir: string): void {
    for (const name of readdirSync(dir)) {
      const path = join(dir, name)
      if (statSync(path).isDirectory()) {
        walk(path)
        continue
      }
      if (!name.endsWith('.vue')) {
        continue
      }
      const lines = lineCount(readFileSync(path, 'utf8'))
      if (lines > MAX_COMPONENT_LINES) {
        counts[relative(root, path)] = lines
      }
    }
  }
}

function readLedger(): Record<string, number> {
  const file = JSON.parse(readFileSync(LEDGER_PATH, 'utf8')) as LedgerFile
  return file.components ?? {}
}

if (process.env.UPDATE_COMPONENT_SIZE === '1') {
  const previous = readLedger()
  const measured = componentLines(REPO_ROOT)
  const components = Object.fromEntries(
    Object.keys(previous)
      .filter((key) => key in measured)
      .sort()
      .map((key) => [key, Math.min(measured[key], previous[key])]),
  )
  const payload = {
    _comment:
      'Component size ledger (tests/unit/architecture/componentSize.spec.ts). Frozen offenders only; entries may shrink or disappear, never grow or be added. Regenerate with UPDATE_COMPONENT_SIZE=1 and review the diff.',
    recorded: '2026-10-27',
    maxLines: MAX_COMPONENT_LINES,
    components,
  }
  writeFileSync(LEDGER_PATH, `${JSON.stringify(payload, null, 2)}\n`)
}

const measured = componentLines(REPO_ROOT)
const ledger = readLedger()

describe('component size', () => {
  it('keeps every component at or under 600 lines unless the ledger already lists it', () => {
    const unlisted = Object.entries(measured).filter(([key]) => !(key in ledger))

    expect(
      unlisted.map(([key, lines]) => `${key} = ${lines}`),
      `New component(s) over ${MAX_COMPONENT_LINES} lines. Split them. The ledger only freezes what was already over the limit.`,
    ).toEqual([])
  })

  it('lets a listed component only shrink', () => {
    const grown = Object.entries(ledger).filter(([key, frozen]) => (measured[key] ?? 0) > frozen)

    expect(
      grown.map(([key, frozen]) => `${key}: ${frozen} → ${measured[key]}`),
      'A frozen component grew. Put the new code in a file of its own.',
    ).toEqual([])
  })

  it('keeps no stale entry on the ledger', () => {
    const stale = Object.entries(ledger).filter(([key]) => !(key in measured))

    expect(
      stale.map(([key, frozen]) => `${key} (frozen at ${frozen})`),
      'These files are at or under the limit, or gone. Regenerate with UPDATE_COMPONENT_SIZE=1 and review the diff.',
    ).toEqual([])
  })

  describe('on a scratch tree', () => {
    const root = mkdtempSync(join(tmpdir(), 'component-size-'))
    afterAll(() => rmSync(root, { recursive: true, force: true }))

    it('flags a component of 601 lines and not one of 600', () => {
      writeVue(root, 'src/Ok.vue', MAX_COMPONENT_LINES)
      writeVue(root, 'src/Big.vue', MAX_COMPONENT_LINES + 1)

      expect(componentLines(root)).toEqual({ 'src/Big.vue': MAX_COMPONENT_LINES + 1 })
    })
  })
})

function writeVue(root: string, file: string, lines: number): void {
  const path = join(root, file)
  mkdirSync(dirname(path), { recursive: true })
  const body = Array.from({ length: lines }, (_, index) => `<!-- ${index} -->`).join('\n')
  writeFileSync(path, body.endsWith('\n') ? body : `${body}\n`)
}
