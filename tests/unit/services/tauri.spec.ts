import { beforeEach, describe, expect, it, vi } from 'vitest'

const invokeMock = vi.fn()
const listenMock = vi.fn()

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: unknown) => invokeMock(cmd, args),
}))
vi.mock('@tauri-apps/api/event', () => ({
  listen: (event: string, cb: unknown) => listenMock(event, cb),
}))

import * as api from '@/services/tauri'

describe('tauri service wrappers', () => {
  beforeEach(() => {
    invokeMock.mockReset()
    listenMock.mockReset()
  })

  it('pair forwards camelCase argument keys the Rust side expects', async () => {
    invokeMock.mockResolvedValue({ paired: true })
    await api.pair('https://web.synaplan.com', 'ABCD1234', "Jan's laptop")
    expect(invokeMock).toHaveBeenCalledWith('pair', {
      baseUrl: 'https://web.synaplan.com',
      code: 'ABCD1234',
      deviceName: "Jan's laptop",
    })
  })

  it('validateBaseUrl passes the url argument', async () => {
    invokeMock.mockResolvedValue('https://web.synaplan.com')
    await api.validateBaseUrl('web.synaplan.com')
    expect(invokeMock).toHaveBeenCalledWith('validate_base_url', { url: 'web.synaplan.com' })
  })

  it('sendChat passes messages and a null model', async () => {
    invokeMock.mockResolvedValue(undefined)
    await api.sendChat([{ role: 'user', content: 'hi' }], null)
    expect(invokeMock).toHaveBeenCalledWith('send_chat', {
      messages: [{ role: 'user', content: 'hi' }],
      model: null,
    })
  })

  it('onChatToken subscribes to the chat token event', async () => {
    listenMock.mockResolvedValue(() => {})
    await api.onChatToken(() => {})
    expect(listenMock).toHaveBeenCalledWith('chat://token', expect.any(Function))
  })

  it('asCommandError narrows structured and unstructured errors', () => {
    expect(api.asCommandError({ code: 'network', message: 'x' })).toEqual({
      code: 'network',
      message: 'x',
    })
    expect(api.asCommandError('boom')).toEqual({ code: 'unexpected', message: 'boom' })
  })
})
