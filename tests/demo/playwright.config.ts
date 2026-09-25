import { defineConfig } from '@playwright/test'

/**
 * Recorded use-case demos (docs/demos). Not part of `make ci-local`: this drives
 * the real Vue app in Chromium against the scripted Rust side in fake-tauri.js
 * and writes one video per use case into docs/demos/videos/.
 *
 *   npm run demo:record            # all three
 *   npm run demo:record -- --grep D1
 */
export default defineConfig({
  testDir: '.',
  testMatch: /.*\.demo\.ts$/,
  fullyParallel: false,
  workers: 1,
  retries: 0,
  timeout: 600_000,
  reporter: [['list']],
  outputDir: '../../node_modules/.cache/demo-results',
  use: {
    baseURL: 'http://localhost:1420',
    viewport: { width: 1280, height: 800 },
    deviceScaleFactor: 1,
    colorScheme: 'dark',
    video: { mode: 'on', size: { width: 1280, height: 800 } },
    trace: 'off',
    actionTimeout: 20_000,
  },
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:1420',
    reuseExistingServer: true,
    timeout: 60_000,
    cwd: '../..',
  },
})
