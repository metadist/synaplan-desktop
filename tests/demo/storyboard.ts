import { mkdirSync } from 'node:fs'
import { dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { expect, type Page } from '@playwright/test'
import type { DemoAct, DemoScenario, DemoStoryboard } from './scenarios/types'

/** Height of the caption strip under the app in the recording. */
const CAPTION_HEIGHT = 72
const TYPING_DELAY_MS = 16
const FAKE_TAURI = fileURLToPath(new URL('./fake-tauri.js', import.meta.url))

/**
 * Install the scripted Rust side before the page loads: the scenario first, then
 * the fake that reads it. Both are init scripts, so a reload keeps them.
 */
export async function installScenario(page: Page, scenario: DemoScenario): Promise<void> {
  await page.addInitScript((s: DemoScenario) => {
    ;(window as unknown as { __SYNAPLAN_DEMO__: DemoScenario }).__SYNAPLAN_DEMO__ = s
  }, scenario)
  await page.addInitScript({ path: FAKE_TAURI })
}

/** The caption strip: a subtitle bar under the app, never over it. */
async function ensureCaptionBar(page: Page): Promise<void> {
  await page.evaluate((height) => {
    if (document.getElementById('demo-caption')) {
      return
    }
    const style = document.createElement('style')
    style.textContent = `
      #app { height: calc(100% - ${height}px) !important; }
      #demo-caption {
        position: fixed; left: 0; right: 0; bottom: 0; height: ${height}px;
        display: flex; align-items: center; justify-content: center;
        padding: 0 48px; box-sizing: border-box;
        background: #0b0e13; color: #f3f5f8;
        font: 500 17px/1.35 system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif;
        text-align: center; letter-spacing: 0.005em;
        border-top: 1px solid #29313d; z-index: 99999;
        transition: opacity 0.25s ease;
      }
      #demo-caption.hidden { opacity: 0; }
    `
    document.head.appendChild(style)
    const bar = document.createElement('div')
    bar.id = 'demo-caption'
    bar.className = 'hidden'
    document.body.appendChild(bar)
  }, CAPTION_HEIGHT)
}

export async function caption(page: Page, text: string): Promise<void> {
  await ensureCaptionBar(page)
  await page.evaluate((t) => {
    const bar = document.getElementById('demo-caption')
    if (!bar) {
      return
    }
    bar.classList.add('hidden')
    setTimeout(() => {
      bar.textContent = t
      bar.classList.remove('hidden')
    }, 220)
  }, text)
  await page.waitForTimeout(450)
}

const composerInput = (page: Page) => page.locator('textarea.composer-input')
const sendButton = (page: Page) => page.locator('.composer .btn-primary')
const stopButton = (page: Page) => page.locator('.composer .btn-ghost')

/** Type the prompt like a person and send it; wait until the turn has ended. */
async function sendPrompt(page: Page, prompt: string, consent: boolean): Promise<void> {
  const input = composerInput(page)
  await input.click()
  await input.pressSequentially(prompt, { delay: TYPING_DELAY_MS })
  await page.waitForTimeout(500)
  await sendButton(page).click()
  if (consent) {
    const allow = page.locator('.consent-card .btn-primary')
    await expect(allow).toBeVisible()
    await page.waitForTimeout(2200)
    await allow.click()
  }
  await expect(stopButton(page)).toBeVisible()
  await expect(stopButton(page)).toBeHidden({ timeout: 180_000 })
  await expect(page.locator('.run-step.running')).toHaveCount(0)
  await page.waitForTimeout(900)
}

async function openPanel(page: Page, tab: 'notes' | 'files'): Promise<void> {
  await page.getByTestId(`pill-${tab}`).click()
  await expect(page.getByTestId('project-panel')).toBeVisible()
}

async function closePanel(page: Page): Promise<void> {
  const close = page.getByTestId('panel-close')
  if (await close.isVisible()) {
    await close.click()
  }
}

async function addFiles(page: Page, scenario: DemoScenario, waitReadyMs: number): Promise<void> {
  const expected = scenario.pickFiles?.length ?? 0
  await page.getByTestId('bar-add-files').click()
  await expect(page.getByTestId('project-panel')).toBeVisible()
  // Every picked file reaches "Ready" in the panel: the honest terminal state.
  await expect(page.locator('[data-testid="project-panel"] .state.ready')).toHaveCount(expected, {
    timeout: waitReadyMs + 15_000,
  })
  await page.waitForTimeout(1200)
}

async function revealArtifact(page: Page, index: number): Promise<void> {
  const cards = page.locator('.msg.assistant').last().locator('.artifact-card')
  await expect(cards.first()).toBeVisible()
  const card = cards.nth(index)
  await card.scrollIntoViewIfNeeded()
  await card.hover()
  await page.waitForTimeout(700)
  await card.locator('.artifact-reveal').click()
  const revealed = await page.evaluate(
    () =>
      (
        window as unknown as {
          __SYNAPLAN_DEMO_STATE__: { actions: Array<{ kind: string; path: string }> }
        }
      ).__SYNAPLAN_DEMO_STATE__.actions.filter((a) => a.kind === 'reveal').length,
  )
  expect(revealed).toBeGreaterThan(0)
}

/** Walk the storyboard act by act. Every act is something the person does. */
export async function playStoryboard(page: Page, board: DemoStoryboard): Promise<void> {
  const { scenario } = board
  await installScenario(page, scenario)
  await page.goto('/')
  // The shell is up when the project name is in the toolbar and the skills pill counts.
  await expect(page.locator('.chat-toolbar .title')).toHaveText(scenario.project.name)
  await expect(page.locator('.skills-pill')).toBeVisible()
  await caption(page, board.title)
  await page.waitForTimeout(2600)

  for (const act of board.acts) {
    await runAct(page, scenario, act)
  }
}

async function runAct(page: Page, scenario: DemoScenario, act: DemoAct): Promise<void> {
  switch (act.kind) {
    case 'caption':
      await caption(page, act.text)
      await page.waitForTimeout(act.holdMs ?? 2500)
      return
    case 'openPanel':
      await caption(page, act.caption)
      await openPanel(page, act.tab)
      await page.waitForTimeout(act.holdMs ?? 2500)
      return
    case 'openNote':
      await caption(page, act.caption)
      await openPanel(page, 'notes')
      await page.waitForTimeout(700)
      await page.getByTestId(`panel-note-${act.name}`).click()
      await expect(page.getByTestId('panel-note-back')).toBeVisible()
      await page.waitForTimeout(act.holdMs ?? 3000)
      return
    case 'closePanel':
      await closePanel(page)
      await page.waitForTimeout(400)
      return
    case 'addFiles':
      await caption(page, act.caption)
      await addFiles(page, scenario, act.waitReadyMs ?? 7000)
      return
    case 'send':
      await caption(page, act.caption)
      await sendPrompt(page, act.prompt, act.consent === true)
      return
    case 'reveal':
      await caption(page, act.caption)
      await revealArtifact(page, act.artifactIndex)
      return
    case 'wait':
      await page.waitForTimeout(act.ms)
      return
  }
}

/** Where the recording of one demo ends up — next to the docs so they ship together. */
export function videoTarget(id: string): string {
  const target = fileURLToPath(new URL(`../../docs/demos/videos/${id}.webm`, import.meta.url))
  mkdirSync(dirname(target), { recursive: true })
  return target
}
