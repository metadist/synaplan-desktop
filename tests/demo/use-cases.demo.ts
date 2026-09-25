import { test, expect, type Page } from '@playwright/test'
import { contractReview } from './scenarios/contract-review'
import { immobilienExpose } from './scenarios/immobilien-expose'
import { tradeQuote } from './scenarios/trade-quote'
import type { DemoStoryboard } from './scenarios/types'
import { playStoryboard, videoTarget } from './storyboard'

/**
 * The three business use-case demos. Each test plays one storyboard in the real
 * UI (fake Rust side) and keeps its recording under docs/demos/videos/. The
 * assertions inside `playStoryboard` make this a walk, not a screenshot: every
 * turn ends in a terminal state, every file reaches Ready, the reveal happened.
 */

interface DemoState {
  outFiles: () => Array<{ name: string }>
  files: () => Array<{ state: string }>
}

async function demoState(page: Page): Promise<{ outNames: string[]; fileStates: string[] }> {
  return page.evaluate(() => {
    const state = (window as unknown as { __SYNAPLAN_DEMO_STATE__: DemoState })
      .__SYNAPLAN_DEMO_STATE__
    return {
      outNames: state.outFiles().map((f) => f.name),
      fileStates: state.files().map((f) => f.state),
    }
  })
}

async function keepRecording(page: Page, board: DemoStoryboard): Promise<void> {
  const video = page.video()
  await page.close()
  if (video) {
    await video.saveAs(videoTarget(board.scenario.id))
  }
}

test.describe('UC-D1 · Immobilienmaklerin (de)', () => {
  test.use({ locale: 'de-DE', timezoneId: 'Europe/Berlin' })

  test('Exposé, Eckdaten, Einladung und Termin aus vier Unterlagen', async ({ page }) => {
    await playStoryboard(page, immobilienExpose)
    const state = await demoState(page)
    expect(state.fileStates).toEqual(['ready', 'ready', 'ready', 'ready'])
    expect(state.outNames).toEqual(
      expect.arrayContaining([
        'Expose-Lindenstrasse-12.docx',
        'Eckdaten-Lindenstrasse-12.xlsx',
        'Einladung-Besichtigung-Lindenstrasse-12.eml',
        'Besichtigung-Lindenstrasse-12.ics',
      ]),
    )
    await expect(page.locator('.msg.assistant .artifact-card')).toHaveCount(5)
    await keepRecording(page, immobilienExpose)
  })
})

test.describe('UC-D2 · Solicitor (en)', () => {
  test.use({ locale: 'en-GB', timezoneId: 'Europe/London' })

  test('review memo, risk register, trainee deck, cover email and call', async ({ page }) => {
    await playStoryboard(page, contractReview)
    const state = await demoState(page)
    expect(state.outNames).toEqual(
      expect.arrayContaining([
        'Harbourline-MSA-Review-Memo.docx',
        'Harbourline-Risk-Register.xlsx',
        'Trainee-Briefing-Liability-IP-Payment.pptx',
        'Harbourline-cover-email.eml',
        'Harbourline-negotiation-call.ics',
      ]),
    )
    await expect(page.locator('.msg.assistant .artifact-card')).toHaveCount(5)
    await keepRecording(page, contractReview)
  })
})

test.describe('UC-D3 · Electrical contractor (en)', () => {
  test.use({ locale: 'en-GB', timezoneId: 'Europe/London' })

  test('site note and price list become a quote, email, contact and install date', async ({
    page,
  }) => {
    await playStoryboard(page, tradeQuote)
    const state = await demoState(page)
    expect(state.outNames).toEqual(
      expect.arrayContaining([
        'Okafor-kitchen-quote.xlsx',
        'Okafor-kitchen-quote.html',
        'Okafor-quote-email.eml',
        'Adaeze-Okafor.vcf',
        'Okafor-kitchen-install.ics',
      ]),
    )
    await expect(page.locator('.msg.assistant .artifact-card')).toHaveCount(6)
    await keepRecording(page, tradeQuote)
  })
})
