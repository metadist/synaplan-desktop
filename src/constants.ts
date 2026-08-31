// Public documentation site. Not the Synaplan instance URL (that comes from
// pairing) — this is the fixed docs host, safe to hardcode.
export const DOCS_BASE = 'https://docs.synaplan.com'

export const DOCS = {
  overview: `${DOCS_BASE}/desktop`,
  skills: `${DOCS_BASE}/desktop-skills`,
  folders: `${DOCS_BASE}/desktop-folders`,
  tools: `${DOCS_BASE}/desktop-tools`,
} as const
