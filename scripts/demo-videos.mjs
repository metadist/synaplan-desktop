#!/usr/bin/env node
// Convert the recorded use-case demos (docs/demos/videos/*.webm, written by
// `npm run demo:record`) into H.264 .mp4 files that play in every browser,
// Teams, PowerPoint and on phones. Needs `ffmpeg` on the PATH; without it the
// .webm files stay as they are and this script says so.
//
// Usage: npm run demo:videos        (or: node scripts/demo-videos.mjs)

import { execFileSync, spawnSync } from 'node:child_process'
import { readdirSync, statSync, unlinkSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = dirname(dirname(fileURLToPath(import.meta.url)))
const dir = join(root, 'docs', 'demos', 'videos')

function hasFfmpeg() {
  const probe = spawnSync('ffmpeg', ['-version'], { stdio: 'ignore' })
  return probe.status === 0
}

if (!hasFfmpeg()) {
  console.error('ffmpeg not found on PATH — keeping the .webm recordings as they are.')
  console.error('Install ffmpeg (brew install ffmpeg / apt install ffmpeg / winget install ffmpeg).')
  process.exit(1)
}

const sources = readdirSync(dir).filter((name) => name.endsWith('.webm'))
if (sources.length === 0) {
  console.log(`No .webm recordings in ${dir}. Run: npm run demo:record`)
  process.exit(0)
}

for (const name of sources) {
  const input = join(dir, name)
  const output = join(dir, name.replace(/\.webm$/, '.mp4'))
  console.log(`${name} → ${output.split('/').pop()}`)
  execFileSync(
    'ffmpeg',
    [
      '-y',
      '-loglevel',
      'error',
      '-i',
      input,
      '-c:v',
      'libx264',
      '-preset',
      'slow',
      '-crf',
      '24',
      '-pix_fmt',
      'yuv420p',
      '-movflags',
      '+faststart',
      '-an',
      output,
    ],
    { stdio: 'inherit' },
  )
  const mb = (statSync(output).size / (1024 * 1024)).toFixed(1)
  console.log(`  ${mb} MB`)
  unlinkSync(input)
}
