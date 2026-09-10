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

  it('sendChat scopes the turn to a project and never sends a model itself', async () => {
    invokeMock.mockResolvedValue(undefined)
    await api.sendChat('p1', [{ role: 'user', content: 'hi' }])
    expect(invokeMock).toHaveBeenCalledWith('send_chat', {
      projectId: 'p1',
      messages: [{ role: 'user', content: 'hi' }],
    })
    await api.sendAgentChat('p1', [{ role: 'user', content: 'hi' }], true)
    expect(invokeMock).toHaveBeenCalledWith('send_agent_chat', {
      projectId: 'p1',
      messages: [{ role: 'user', content: 'hi' }],
      allowExec: true,
    })
  })

  it('onChatToken subscribes to the chat token event', async () => {
    listenMock.mockResolvedValue(() => {})
    await api.onChatToken(() => {})
    expect(listenMock).toHaveBeenCalledWith('chat://token', expect.any(Function))
  })

  it('project wrappers forward camelCase keys and never build paths', async () => {
    invokeMock.mockResolvedValue({ projects: [], activeId: '', personalId: '' })
    await api.createProject('Kitchen', 'de', 'abc')
    expect(invokeMock).toHaveBeenCalledWith('create_project', {
      name: 'Kitchen',
      dictationLanguage: 'de',
      copyModelsFrom: 'abc',
    })
    await api.updateProject('abc', { defaultAssistantId: null })
    expect(invokeMock).toHaveBeenCalledWith('update_project', {
      id: 'abc',
      patch: { defaultAssistantId: null },
    })
    await api.deleteProject('abc', true)
    expect(invokeMock).toHaveBeenCalledWith('delete_project', { id: 'abc', removeFiles: true })
    await api.setActiveProject('abc')
    expect(invokeMock).toHaveBeenCalledWith('set_active_project', { id: 'abc' })
  })

  it('chat thread wrappers scope every call to a project', async () => {
    invokeMock.mockResolvedValue([])
    await api.listChats('p1')
    expect(invokeMock).toHaveBeenCalledWith('list_chats', { projectId: 'p1' })
    await api.loadChat('p1', 'c1')
    expect(invokeMock).toHaveBeenCalledWith('load_chat', { projectId: 'p1', chatId: 'c1' })
    await api.deleteChat('p1', 'c1')
    expect(invokeMock).toHaveBeenCalledWith('delete_chat', { projectId: 'p1', chatId: 'c1' })
  })

  it('exposes exactly the eight model slots', () => {
    expect(api.MODEL_SLOTS).toEqual([
      'chat',
      'voice',
      'speak',
      'vision',
      'image',
      'video',
      'embed',
      'docs',
    ])
  })

  it('asCommandError narrows structured and unstructured errors', () => {
    expect(api.asCommandError({ code: 'network', message: 'x' })).toEqual({
      code: 'network',
      message: 'x',
    })
    expect(api.asCommandError('boom')).toEqual({ code: 'unexpected', message: 'boom' })
  })
})
