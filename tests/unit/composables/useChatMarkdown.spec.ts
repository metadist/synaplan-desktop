import { describe, expect, it } from 'vitest'
import { escapeHtml, isRenderableImageSrc } from '@/composables/useChatMarkdown'

describe('isRenderableImageSrc', () => {
  it('accepts http(s), data-image, blob, and workspace file paths', () => {
    expect(isRenderableImageSrc('https://images.pexels.com/cat.jpg')).toBe(true)
    expect(isRenderableImageSrc('http://localhost:8000/api/v1/files/12')).toBe(true)
    expect(isRenderableImageSrc('/api/v1/files/uploads/cat.png')).toBe(true)
    expect(isRenderableImageSrc('data:image/png;base64,aaa')).toBe(true)
    expect(isRenderableImageSrc('blob:http://localhost/abc')).toBe(true)
  })

  it('rejects scripts, non-image data URIs, and bare filenames', () => {
    expect(isRenderableImageSrc('javascript:alert(1)')).toBe(false)
    expect(isRenderableImageSrc('data:text/html,hi')).toBe(false)
    expect(isRenderableImageSrc('file:///etc/passwd')).toBe(false)
    expect(isRenderableImageSrc('cat.jpg')).toBe(false)
    expect(isRenderableImageSrc('')).toBe(false)
  })
})

describe('escapeHtml', () => {
  it('escapes markup in alt text and URLs', () => {
    expect(escapeHtml('<img src=x onerror=alert(1)>')).toBe('&lt;img src=x onerror=alert(1)&gt;')
  })
})
