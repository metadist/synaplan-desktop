import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import {
  applyResearchEvent,
  applyResearchState,
  citationsFromMarkers,
  emptyResearch,
  lastResearchSeq,
  parseResearchSse,
  researchStartBlock,
} from '@/lib/researchRun'

const fixtures = join(dirname(fileURLToPath(import.meta.url)), '../../fixtures/research-contract')

function sse(name: string): string {
  return readFileSync(join(fixtures, name), 'utf8')
}

function fold(name: string) {
  return parseResearchSse(sse(name)).reduce(
    (ui, frame) => applyResearchEvent(ui, frame),
    emptyResearch(),
  )
}

const ready = {
  question: 'Which suppliers?',
  web: true,
  files: false,
  webAvailable: true,
  filesReady: 0,
  depth: 'standard',
  thoroughBusy: false,
  standardLeft: null,
  thoroughLeft: null,
}

describe('research events', () => {
  it('replays a standard run through complete and ignores the id-less queued frame', () => {
    const frames = parseResearchSse(sse('events-standard.sse'))
    expect(frames.at(-1)?.name).toBe('queued')
    expect(frames.at(-1)?.seq).toBeNull()
    expect(lastResearchSeq(frames)).toBe(12)
    const ui = fold('events-standard.sse')
    expect(ui.status).toBe('complete')
    expect(ui.outcome?.code).toBe('complete')
    expect(ui.points[0]?.title).toContain('suppliers')
    expect(ui.found).toBe(4)
    expect(ui.read).toBe(1)
  })

  it('stops a thorough run on the plan confirmation', () => {
    const ui = fold('events-thorough-waiting.sse')
    expect(ui.status).toBe('waiting_plan')
    expect(ui.points.length).toBeGreaterThan(0)
  })

  it('ignores an event name it does not know', () => {
    const frames = parseResearchSse(sse('events-unknown.sse'))
    expect(frames.some((frame) => frame.name === 'future_widget')).toBe(true)
    const ui = frames.reduce((state, frame) => applyResearchEvent(state, frame), emptyResearch('Q'))
    expect(ui.question).toBe('Q')
    expect(ui.outcome).toBeNull()
  })

  it('rebuilds citation chips from markers when the report has none', () => {
    const chips = citationsFromMarkers('The price is 12.40. [1]\n', ['s3'])
    expect(chips).toEqual([{ n: 1, marker: '[1]', source: 's3', sentence: '' }])
  })

  it('keeps file ids from the server field name', () => {
    const ui = applyResearchState(emptyResearch(), {
      id: 'rr_1',
      status: 'complete',
      depth: 'standard',
      question: 'Q',
      sources: [
        {
          id: 's1',
          title: 'Notes',
          url: null,
          file_id: 'file-9',
          host: null,
          status: 'read',
          reason: null,
        },
      ],
      report: { markdown: 'See [1].', version: 2 },
    })
    expect(ui.sources[0]?.fileId).toBe('file-9')
    expect(ui.citations[0]?.source).toBe('s1')
    expect(ui.version).toBe(2)
  })

  it('keeps the report question when a follow-up run reports its own', () => {
    const base = { id: 'rr_2', status: 'complete', depth: 'standard', question: 'Shorter please' }
    expect(applyResearchState(emptyResearch('Which suppliers?'), base).question).toBe(
      'Which suppliers?',
    )
    expect(applyResearchState(emptyResearch(), base).question).toBe('Shorter please')
  })
})

describe('research start reasons', () => {
  it('asks for a question, a source, and stops on a used-up depth', () => {
    expect(researchStartBlock({ ...ready, question: '  ' })).toBe('research.needQuestion')
    expect(researchStartBlock({ ...ready, web: false })).toBe('research.needSource')
    expect(researchStartBlock({ ...ready, depth: 'thorough', thoroughBusy: true })).toBe(
      'research.thoroughBusy',
    )
    expect(researchStartBlock({ ...ready, depth: 'thorough', thoroughLeft: 0 })).toBe(
      'research.limit',
    )
    expect(researchStartBlock(ready)).toBeNull()
  })
})
