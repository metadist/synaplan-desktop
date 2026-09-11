/**
 * Markdown helpers for chat bubbles. Image sources are filtered so a
 * text-only model cannot inject script URLs; a failed or untrusted src
 * becomes a caption instead of a broken file chip.
 */

const UNSAFE_SCHEMES = ['javascript:', 'vbscript:', 'file:'] as const

export function isRenderableImageSrc(src: string): boolean {
  // Strip whitespace and control characters so `da\nta:` cannot sneak through.
  // eslint-disable-next-line no-control-regex
  const normalized = src.replace(/[\s\u0000-\u001f\u007f-\u009f]/g, '')
  if (normalized === '') {
    return false
  }
  const lower = normalized.toLowerCase()
  if (UNSAFE_SCHEMES.some((scheme) => lower.startsWith(scheme))) {
    return false
  }
  if (lower.startsWith('data:')) {
    return lower.startsWith('data:image/')
  }
  if (lower.startsWith('blob:')) {
    return true
  }
  if (normalized.startsWith('/api/v1/files/')) {
    return true
  }
  return /^https?:\/\//i.test(normalized)
}

export function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}
