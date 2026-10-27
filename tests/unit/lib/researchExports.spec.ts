import { describe, expect, it } from 'vitest'
import { exportBlock, exportInstruction } from '@/lib/researchExports'

describe('research exports', () => {
  it('names the report path and the version in the instruction', () => {
    const text = exportInstruction('docx', '/Projects/out/suppliers.md', 3)
    expect(text).toContain('/Projects/out/suppliers.md')
    expect(text).toContain('version 3')
  })

  it('disables an export when the skill is missing or blocked', () => {
    expect(exportBlock(undefined)).toBe('research.exportBlocked')
    expect(exportBlock({ enabled: false, blocked: false, blockedReason: null })).toBe(
      'research.exportBlocked',
    )
    expect(
      exportBlock({ enabled: true, blocked: true, blockedReason: 'Install the office program.' }),
    ).toBe('Install the office program.')
    expect(exportBlock({ enabled: true, blocked: false, blockedReason: null })).toBeNull()
  })
})
