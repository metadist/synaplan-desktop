import { readFileSync, readdirSync, statSync } from 'node:fs'
import { dirname, join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

/**
 * The webview never talks to the workspace itself: every request goes through
 * a Tauri command so the API key stays in the OS secret store. A `fetch` to
 * `/v1/audio` (or any auth header) in `src/` would mean the key crossed the seam.
 */
const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', 'src')

function walk(dir: string, out: string[] = []): string[] {
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry)
    if (statSync(path).isDirectory()) {
      walk(path, out)
    } else if (/\.(ts|vue)$/.test(entry)) {
      out.push(path)
    }
  }
  return out
}

const FORBIDDEN: Array<[string, RegExp]> = [
  ['fetch to the audio routes', /fetch\([^)]*\/v1\/audio/],
  ['any fetch or XHR to the workspace', /\b(fetch\(|new XMLHttpRequest\()/],
  ['an API key header', /x-api-key|Authorization['"]?\s*[:=]/i],
  ['a raw key', /\bsk_[A-Za-z0-9]{8,}/],
]

describe('webview never calls the workspace directly', () => {
  it('has no fetch/XHR, auth header, or key in src/', () => {
    const hits: string[] = []
    for (const file of walk(ROOT)) {
      const text = readFileSync(file, 'utf8')
      for (const [what, pattern] of FORBIDDEN) {
        if (pattern.test(text)) {
          hits.push(`${relative(ROOT, file)}: ${what}`)
        }
      }
    }
    expect(hits).toEqual([])
  })
})
