/** Research events and the card state built from them. No network. */

export interface ResearchFrame {
  seq: number | null
  name: string
  data: Record<string, unknown>
}

export interface ResearchSource {
  id: string
  title: string
  url: string | null
  fileId: string | null
  host: string | null
  status: string
  reason: string | null
}

export interface ResearchCitation {
  n: number
  marker: string
  source: string
  sentence: string
}

export interface ResearchPoint {
  id: string
  title: string
  status: string
}

export interface ResearchQuestion {
  id: string
  text: string
  options: string[]
  allowText: boolean
}

export interface ResearchOutcome {
  code: string
  text: string
  params: Record<string, string | number>
}

export type ResearchDetail =
  | { kind: 'search'; query: string; scope: string; results: number }
  | { kind: 'closed'; title: string; reason: string }

export interface ResearchUi {
  runId: string
  status: string
  depth: string
  question: string
  points: ResearchPoint[]
  questionCard: ResearchQuestion | null
  disclosures: string[]
  estimateMinutes: number | null
  estimateCost: number | null
  queuedSince: number | null
  details: ResearchDetail[]
  found: number
  read: number
  closed: number
  now: string
  markdown: string
  citations: ResearchCitation[]
  sources: ResearchSource[]
  sourcesOrder: string[]
  check: { statements: number; supported: number; removed: number } | null
  outcome: ResearchOutcome | null
  version: number
}

export function emptyResearch(question = '', depth = 'standard'): ResearchUi {
  return {
    runId: '',
    status: '',
    depth,
    question,
    points: [],
    questionCard: null,
    disclosures: [],
    estimateMinutes: null,
    estimateCost: null,
    queuedSince: null,
    details: [] as ResearchDetail[],
    found: 0,
    read: 0,
    closed: 0,
    now: '',
    markdown: '',
    citations: [],
    sources: [],
    sourcesOrder: [],
    check: null,
    outcome: null,
    version: 1,
  }
}

export function parseResearchSse(text: string): ResearchFrame[] {
  const frames: ResearchFrame[] = []
  const blocks = text.replace(/\r\n/g, '\n').split('\n\n')
  for (const block of blocks) {
    if (!block.trim()) {
      continue
    }
    let seq: number | null = null
    let name = ''
    let data = ''
    for (const line of block.split('\n')) {
      if (line.startsWith('id:')) {
        const parsed = Number(line.slice(3).trim())
        seq = Number.isFinite(parsed) ? parsed : null
      } else if (line.startsWith('event:')) {
        name = line.slice(6).trim()
      } else if (line.startsWith('data:')) {
        data += line.slice(5).trim()
      }
    }
    if (!name) {
      continue
    }
    let parsed: Record<string, unknown> = {}
    try {
      const value = JSON.parse(data) as unknown
      if (value && typeof value === 'object' && !Array.isArray(value)) {
        parsed = value as Record<string, unknown>
      }
    } catch {
      parsed = {}
    }
    frames.push({ seq, name, data: parsed })
  }
  return frames
}

export function lastResearchSeq(frames: ResearchFrame[]): number | null {
  let last: number | null = null
  for (const frame of frames) {
    if (frame.seq !== null) {
      last = frame.seq
    }
  }
  return last
}

function asString(value: unknown): string {
  return typeof value === 'string' ? value : ''
}

function asNumber(value: unknown): number | null {
  return typeof value === 'number' ? value : null
}

function idOf(value: unknown): string | null {
  if (typeof value === 'string' && value) {
    return value
  }
  if (typeof value === 'number') {
    return String(value)
  }
  return null
}

export function citationsFromMarkers(markdown: string, order: string[]): ResearchCitation[] {
  const found: ResearchCitation[] = []
  const pattern = /\[(\d+)\]/g
  let match = pattern.exec(markdown)
  let n = 1
  while (match) {
    const marker = Number(match[1])
    found.push({
      n,
      marker: `[${marker}]`,
      source: order[marker - 1] ?? '',
      sentence: '',
    })
    n += 1
    match = pattern.exec(markdown)
  }
  return found
}

function outcomeOf(data: Record<string, unknown>): ResearchOutcome | null {
  const raw = data.outcome
  if (!raw || typeof raw !== 'object') {
    return null
  }
  const row = raw as Record<string, unknown>
  const params: Record<string, string | number> = {}
  if (row.params && typeof row.params === 'object') {
    for (const [key, value] of Object.entries(row.params as Record<string, unknown>)) {
      if (typeof value === 'string' || typeof value === 'number') {
        params[key] = value
      }
    }
  }
  return { code: asString(row.code), text: asString(row.text), params }
}

export function applyResearchEvent(ui: ResearchUi, frame: ResearchFrame): ResearchUi {
  const next: ResearchUi = {
    ...ui,
    points: ui.points.map((point) => ({ ...point })),
    sources: ui.sources.map((source) => ({ ...source })),
    details: [...ui.details],
    citations: [...ui.citations],
    disclosures: [...ui.disclosures],
  }
  const data = frame.data
  switch (frame.name) {
    case 'run':
      next.runId = asString(data.id) || next.runId
      next.status = asString(data.status) || next.status
      next.depth = asString(data.depth) || next.depth
      break
    case 'queued':
      next.queuedSince = asNumber(data.since_seconds)
      break
    case 'question':
      next.status = 'waiting_answer'
      next.questionCard = {
        id: asString(data.id),
        text: asString(data.text),
        options: Array.isArray(data.options)
          ? data.options.filter((item) => typeof item === 'string')
          : [],
        allowText: data.allow_text === true,
      }
      break
    case 'plan': {
      const points = Array.isArray(data.points) ? data.points : []
      next.points = points
        .filter((item) => item && typeof item === 'object')
        .map((item) => {
          const row = item as Record<string, unknown>
          return { id: asString(row.id), title: asString(row.title), status: 'open' }
        })
      next.disclosures = Array.isArray(data.disclosures)
        ? data.disclosures.filter((item) => typeof item === 'string')
        : []
      const estimate = data.estimate
      if (estimate && typeof estimate === 'object') {
        const row = estimate as Record<string, unknown>
        next.estimateMinutes = asNumber(row.minutes)
        next.estimateCost = asNumber(row.cost)
      }
      break
    }
    case 'plan_waiting':
      next.status = 'waiting_plan'
      break
    case 'search':
      next.details.push({
        kind: 'search',
        query: asString(data.query),
        scope: asString(data.scope) || 'web',
        results: asNumber(data.results) ?? 0,
      })
      next.found += asNumber(data.results) ?? 0
      break
    case 'source': {
      const source: ResearchSource = {
        id: asString(data.id),
        title: asString(data.title),
        url: typeof data.url === 'string' ? data.url : null,
        fileId: idOf(data.file_id),
        host: typeof data.host === 'string' ? data.host : null,
        status: asString(data.status),
        reason: typeof data.reason === 'string' ? data.reason : null,
      }
      const existing = next.sources.findIndex((item) => item.id === source.id)
      if (existing >= 0) {
        next.sources[existing] = source
      } else {
        next.sources.push(source)
      }
      if (source.status === 'read') {
        next.read += 1
        next.now = source.title
      }
      if (source.status === 'skipped' || source.reason) {
        next.closed += 1
        next.details.push({
          kind: 'closed',
          title: source.host || source.title,
          reason: source.reason ?? '',
        })
      }
      break
    }
    case 'point': {
      const id = asString(data.id)
      const status = asString(data.status)
      const point = next.points.find((item) => item.id === id)
      if (point) {
        point.status = status
      }
      break
    }
    case 'writing':
      next.status = 'writing'
      next.now = asString(data.section)
      break
    case 'checking':
      next.status = 'checking'
      break
    case 'check':
      next.check = {
        statements: asNumber(data.statements) ?? 0,
        supported: asNumber(data.supported) ?? 0,
        removed: asNumber(data.removed) ?? 0,
      }
      break
    case 'complete':
    case 'stopped':
    case 'failed':
      next.status = frame.name === 'complete' ? 'complete' : frame.name
      next.outcome = outcomeOf(data)
      next.queuedSince = null
      next.questionCard = null
      break
    default:
      break
  }
  return next
}

export function applyResearchState(
  ui: ResearchUi,
  state: {
    id: string
    status: string
    depth: string
    question: string
    report?: {
      markdown: string
      version?: number
      citations?: ResearchCitation[]
      sources_order?: string[]
    } | null
    sources?: Array<{
      id: string
      title: string
      url?: string | null
      fileId?: string | null
      file_id?: string | null
      host?: string | null
      status?: string
      reason?: string | null
    }>
    outcome?: ResearchOutcome | null
    check?: { statements: number; supported: number; removed: number } | null
  },
): ResearchUi {
  const next = {
    ...ui,
    runId: state.id,
    status: state.status,
    depth: state.depth,
    question: state.question,
  }
  if (state.sources) {
    next.sources = state.sources.map((row) => ({
      id: row.id,
      title: row.title,
      url: row.url ?? null,
      fileId: row.fileId ?? row.file_id ?? null,
      host: row.host ?? null,
      status: row.status ?? '',
      reason: row.reason ?? null,
    }))
  }
  if (state.report) {
    next.markdown = state.report.markdown
    next.version = state.report.version ?? 1
    next.sourcesOrder =
      state.report.sources_order ?? state.sources?.map((source) => source.id) ?? []
    next.citations =
      state.report.citations && state.report.citations.length > 0
        ? state.report.citations
        : citationsFromMarkers(state.report.markdown, next.sourcesOrder)
  }
  next.outcome = state.outcome ?? next.outcome
  next.check = state.check ?? next.check
  return next
}

/** A reason key, or null when Start research can run. */
export function researchStartBlock(input: {
  question: string
  web: boolean
  files: boolean
  webAvailable: boolean
  filesReady: number
  depth: string
  thoroughBusy: boolean
  standardLeft: number | null
  thoroughLeft: number | null
}): string | null {
  if (!input.question.trim()) {
    return 'research.needQuestion'
  }
  const webOn = input.web && input.webAvailable
  const filesOn = input.files && input.filesReady > 0
  if (!webOn && !filesOn) {
    return 'research.needSource'
  }
  if (input.depth === 'thorough' && input.thoroughBusy) {
    return 'research.thoroughBusy'
  }
  if (input.depth === 'thorough' && input.thoroughLeft === 0) {
    return 'research.limit'
  }
  if (input.depth !== 'thorough' && input.standardLeft === 0) {
    return 'research.limit'
  }
  return null
}
