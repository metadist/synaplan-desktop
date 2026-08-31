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

/** A model advertised by the instance, with its provider. */
export interface ModelInfo {
  id: string
  provider: string
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

export function listModels(): Promise<ModelInfo[]> {
  return invoke<ModelInfo[]>('list_models')
}

export function sendChat(messages: ChatMessage[], model: string | null): Promise<void> {
  return invoke<void>('send_chat', { messages, model })
}

export function cancelChat(): Promise<void> {
  return invoke<void>('cancel_chat')
}

/** Open an http(s) URL in the user's default browser. */
export function openUrl(url: string): Promise<void> {
  return invoke<void>('open_url', { url })
}

/** Reveal a local folder/file in the OS file manager. */
export function revealPath(path: string): Promise<void> {
  return invoke<void>('reveal_path', { path })
}

export interface FilesystemPolicy {
  read: string[]
  outbox: string
  deny: string[]
  maxFileBytes: number
}

export interface Skill {
  name: string
  description: string
  dir: string
  bundled: boolean
  enabled: boolean
}

export function getFilesystemPolicy(): Promise<FilesystemPolicy> {
  return invoke<FilesystemPolicy>('get_filesystem_policy')
}

export function addReadFolder(path: string): Promise<FilesystemPolicy> {
  return invoke<FilesystemPolicy>('add_read_folder', { path })
}

export function removeReadFolder(path: string): Promise<FilesystemPolicy> {
  return invoke<FilesystemPolicy>('remove_read_folder', { path })
}

export function listSkills(): Promise<Skill[]> {
  return invoke<Skill[]>('list_skills')
}

export function setSkillEnabled(name: string, enabled: boolean): Promise<Skill[]> {
  return invoke<Skill[]>('set_skill_enabled', { name, enabled })
}

export interface Tool {
  id: string
  name: string
  found: boolean
  path: string | null
  version: string | null
  hint: string
}

export function runDoctor(): Promise<Tool[]> {
  return invoke<Tool[]>('run_doctor')
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

/** Has the user allowed this install to run installed skills? */
export function getExecutionConsent(): Promise<boolean> {
  return invoke<boolean>('get_execution_consent')
}

export function setExecutionConsent(): Promise<void> {
  return invoke<void>('set_execution_consent')
}

/** Run one agentic (skill-enabled) turn. Emits agent://* events. */
export function sendAgentChat(
  messages: ChatMessage[],
  model: string | null,
  allowExec: boolean,
): Promise<void> {
  return invoke<void>('send_agent_chat', { messages, model, allowExec })
}

/** One step in the run activity feed. */
export interface AgentToolEvent {
  phase: 'start' | 'end'
  name: string
  summary: string
  ok: boolean
  artifact: string | null
}

export function onAgentText(cb: (text: string) => void): Promise<UnlistenFn> {
  return listen<string>('agent://text', (event) => cb(event.payload))
}

export function onAgentTool(cb: (event: AgentToolEvent) => void): Promise<UnlistenFn> {
  return listen<AgentToolEvent>('agent://tool', (event) => cb(event.payload))
}

export function onAgentDone(cb: () => void): Promise<UnlistenFn> {
  return listen<null>('agent://done', () => cb())
}

export function onAgentError(cb: (error: StreamError) => void): Promise<UnlistenFn> {
  return listen<StreamError>('agent://error', (event) => cb(event.payload))
}

/** Narrow an unknown thrown value into a {@link CommandError}. */
export function asCommandError(err: unknown): CommandError {
  if (err && typeof err === 'object' && 'code' in err && 'message' in err) {
    return err as CommandError
  }
  return { code: 'unexpected', message: String(err) }
}
