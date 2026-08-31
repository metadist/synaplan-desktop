#!/usr/bin/env node
// A tiny offline stand-in for a Synaplan instance so the app can be run and
// demoed without a live server (DC3). It accepts pairing, lists a mock model,
// streams a short Anthropic-style SSE chat turn, and 403s on /admin.
//
// Usage: node tests/mock-server/server.mjs  (PORT env, default 8788)
// Then pair the app against http://localhost:8788 with any code.

import { createServer } from 'node:http'

const PORT = Number(process.env.PORT ?? 8788)

function send(res, status, headers, body) {
  res.writeHead(status, headers)
  res.end(body)
}

function json(res, status, obj) {
  send(res, status, { 'content-type': 'application/json' }, JSON.stringify(obj))
}

async function readBody(req) {
  const chunks = []
  for await (const chunk of req) chunks.push(chunk)
  return Buffer.concat(chunks).toString('utf8')
}

const SSE_EVENTS = [
  {
    event: 'message_start',
    data: {
      type: 'message_start',
      message: { id: 'msg_mock', role: 'assistant', model: 'mock-model', content: [] },
    },
  },
  {
    event: 'content_block_start',
    data: { type: 'content_block_start', index: 0, content_block: { type: 'text', text: '' } },
  },
  {
    event: 'content_block_delta',
    data: { type: 'content_block_delta', index: 0, delta: { type: 'text_delta', text: 'PONG' } },
  },
  {
    event: 'content_block_delta',
    data: {
      type: 'content_block_delta',
      index: 0,
      delta: { type: 'text_delta', text: ' from the mock server.' },
    },
  },
  { event: 'content_block_stop', data: { type: 'content_block_stop', index: 0 } },
  { event: 'message_stop', data: { type: 'message_stop' } },
]

const server = createServer(async (req, res) => {
  const url = new URL(req.url ?? '/', `http://localhost:${PORT}`)
  const path = url.pathname

  if (path.startsWith('/admin')) {
    return json(res, 403, { error: 'forbidden' })
  }

  if (req.method === 'POST' && path === '/api/v1/desktop/pair') {
    await readBody(req)
    return json(res, 201, {
      success: true,
      deviceId: 1,
      key: 'sk_test_mock_key_0000000000000000000000000000',
      apiBaseUrl: `http://localhost:${PORT}`,
    })
  }

  if (req.method === 'GET' && path === '/v1/models') {
    return json(res, 200, { object: 'list', data: [{ id: 'mock-model', object: 'model' }] })
  }

  if (req.method === 'POST' && path === '/v1/messages') {
    await readBody(req)
    res.writeHead(200, {
      'content-type': 'text/event-stream',
      'cache-control': 'no-cache',
      connection: 'keep-alive',
    })
    let i = 0
    const timer = setInterval(() => {
      if (i >= SSE_EVENTS.length) {
        clearInterval(timer)
        res.end()
        return
      }
      const { event, data } = SSE_EVENTS[i++]
      res.write(`event: ${event}\ndata: ${JSON.stringify(data)}\n\n`)
    }, 120)
    req.on('close', () => clearInterval(timer))
    return
  }

  return json(res, 404, { error: 'not found' })
})

server.listen(PORT, () => {
  console.log(`Synaplan mock server on http://localhost:${PORT}`)
  console.log('Pair the app against this URL with any pairing code.')
})
