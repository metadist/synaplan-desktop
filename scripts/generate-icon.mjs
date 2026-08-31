#!/usr/bin/env node
// Generates a placeholder 1024x1024 app icon (brand-blue rounded square with a
// white chat bubble) at src-tauri/icons/source.png. Run `npx tauri icon
// src-tauri/icons/source.png` afterwards to produce the platform icon set.
// Real branding replaces this before GA (Sprint B6).
import { deflateSync } from 'node:zlib'
import { writeFileSync, mkdirSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const SIZE = 1024
const buf = Buffer.alloc(SIZE * SIZE * 4) // RGBA

function px(x, y, r, g, b, a) {
  if (x < 0 || y < 0 || x >= SIZE || y >= SIZE) return
  const i = (y * SIZE + x) * 4
  buf[i] = r
  buf[i + 1] = g
  buf[i + 2] = b
  buf[i + 3] = a
}

function insideRounded(x, y, x0, y0, x1, y1, radius) {
  if (x < x0 || x > x1 || y < y0 || y > y1) return false
  const cx = Math.min(Math.max(x, x0 + radius), x1 - radius)
  const cy = Math.min(Math.max(y, y0 + radius), y1 - radius)
  const dx = x - cx
  const dy = y - cy
  return dx * dx + dy * dy <= radius * radius
}

// Brand blue rounded square.
const BG = [47, 107, 255]
const margin = 48
const radius = 190
for (let y = 0; y < SIZE; y++) {
  for (let x = 0; x < SIZE; x++) {
    if (insideRounded(x, y, margin, margin, SIZE - margin, SIZE - margin, radius)) {
      px(x, y, BG[0], BG[1], BG[2], 255)
    }
  }
}

// White chat bubble.
const bx0 = 300
const by0 = 330
const bx1 = 724
const by1 = 610
const bRad = 70
for (let y = by0; y <= by1; y++) {
  for (let x = bx0; x <= bx1; x++) {
    if (insideRounded(x, y, bx0, by0, bx1, by1, bRad)) {
      px(x, y, 255, 255, 255, 255)
    }
  }
}
// Bubble tail (small triangle bottom-left).
for (let y = by1 - 10; y < by1 + 90; y++) {
  const w = by1 + 90 - y
  for (let x = 360; x < 360 + w; x++) {
    px(x, y, 255, 255, 255, 255)
  }
}

// Encode PNG (color type 6 = RGBA, 8-bit).
function crc32(bytes) {
  let c = ~0
  for (let i = 0; i < bytes.length; i++) {
    c ^= bytes[i]
    for (let k = 0; k < 8; k++) c = (c >>> 1) ^ (0xedb88320 & -(c & 1))
  }
  return ~c >>> 0
}

function chunk(type, data) {
  const len = Buffer.alloc(4)
  len.writeUInt32BE(data.length, 0)
  const typeBuf = Buffer.from(type, 'ascii')
  const body = Buffer.concat([typeBuf, data])
  const crc = Buffer.alloc(4)
  crc.writeUInt32BE(crc32(body), 0)
  return Buffer.concat([len, body, crc])
}

const ihdr = Buffer.alloc(13)
ihdr.writeUInt32BE(SIZE, 0)
ihdr.writeUInt32BE(SIZE, 4)
ihdr[8] = 8 // bit depth
ihdr[9] = 6 // color type RGBA
ihdr[10] = 0
ihdr[11] = 0
ihdr[12] = 0

// Raw scanlines with filter byte 0 per row.
const raw = Buffer.alloc(SIZE * (SIZE * 4 + 1))
for (let y = 0; y < SIZE; y++) {
  raw[y * (SIZE * 4 + 1)] = 0
  buf.copy(raw, y * (SIZE * 4 + 1) + 1, y * SIZE * 4, (y + 1) * SIZE * 4)
}

const png = Buffer.concat([
  Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
  chunk('IHDR', ihdr),
  chunk('IDAT', deflateSync(raw, { level: 9 })),
  chunk('IEND', Buffer.alloc(0)),
])

const out = resolve(
  dirname(fileURLToPath(import.meta.url)),
  '..',
  'src-tauri',
  'icons',
  'source.png',
)
mkdirSync(dirname(out), { recursive: true })
writeFileSync(out, png)
console.log(`Wrote ${out} (${png.length} bytes)`)
