import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

/** The paired/unpaired status reported by the Rust side. */
export interface Status {
  paired: boolean
  apiBaseUrl: string | null
  deviceId: number | null
  keyBackend: string
  keyIsPlaintext: boolean
}

/** The serialised error shape every command rejects with. */
export interface CommandError {
  code: string
  message: string
}

export interface ChatMessage {
  role: 'user' | 'assistant'
  content: string
}

export interface StreamError {
  code: string
  message: string
}

export function getStatus(): Promise<Status> {
  return invoke<Status>('get_status')
}

export function defaultDeviceName(): Promise<string> {
  return invoke<string>('default_device_name')
}

export function validateBaseUrl(url: string): Promise<string> {
  return invoke<string>('validate_base_url', { url })
}

export function pair(baseUrl: string, code: string, deviceName: string): Promise<Status> {
  return invoke<Status>('pair', { baseUrl, code, deviceName })
}

export function pairWithKey(baseUrl: string, key: string): Promise<Status> {
  return invoke<Status>('pair_with_key', { baseUrl, key })
}

export function signOut(): Promise<void> {
  return invoke<void>('sign_out')
}

export function listModels(): Promise<string[]> {
  return invoke<string[]>('list_models')
}

export function sendChat(messages: ChatMessage[], model: string | null): Promise<void> {
  return invoke<void>('send_chat', { messages, model })
}

export function onChatToken(cb: (token: string) => void): Promise<UnlistenFn> {
  return listen<string>('chat://token', (event) => cb(event.payload))
}

export function onChatDone(cb: () => void): Promise<UnlistenFn> {
  return listen<null>('chat://done', () => cb())
}

export function onChatError(cb: (error: StreamError) => void): Promise<UnlistenFn> {
  return listen<StreamError>('chat://error', (event) => cb(event.payload))
}

/** Narrow an unknown thrown value into a {@link CommandError}. */
export function asCommandError(err: unknown): CommandError {
  if (err && typeof err === 'object' && 'code' in err && 'message' in err) {
    return err as CommandError
  }
  return { code: 'unexpected', message: String(err) }
}
