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

/**
 * Stream one chat turn inside a project. The Rust side puts the project's Chat
 * model in the request and refuses with `chat_model_unset` when none is set —
 * the webview never picks a model per turn.
 */
export function sendChat(projectId: string, messages: ChatMessage[]): Promise<void> {
  return invoke<void>('send_chat', { projectId, messages })
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
  source: string
  license: string | null
  compatibilityWarning: boolean
  allowUnattended: boolean
  blocked: boolean
  blockedReason: string | null
  version: string | null
  url: string | null
  sha: string | null
  needsPython: boolean
  needsNode: boolean
  needsLibreoffice: boolean
  pythonImports: string[]
}

export interface InstallPreview {
  name: string
  description: string
  license: string | null
  files: string[]
  compatibilityWarning: boolean
  source: string
  needsPython: boolean
  needsNode: boolean
  needsLibreoffice: boolean
  pythonImports: string[]
  cachePath: string | null
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

export function setSkillUnattended(name: string, allow: boolean): Promise<Skill[]> {
  return invoke<Skill[]>('set_skill_unattended', { name, allow })
}

export function previewSkillFolder(folder: string): Promise<InstallPreview> {
  return invoke<InstallPreview>('preview_skill_folder', { folder })
}

export function previewSkillZip(zipPath: string): Promise<InstallPreview> {
  return invoke<InstallPreview>('preview_skill_zip', { zipPath })
}

export function previewSkillUrl(url: string): Promise<InstallPreview> {
  return invoke<InstallPreview>('preview_skill_url', { url })
}

export function installSkillFromFolder(folder: string): Promise<Skill[]> {
  return invoke<Skill[]>('install_skill_from_folder', { folder })
}

export function installSkillFromZip(zipPath: string): Promise<Skill[]> {
  return invoke<Skill[]>('install_skill_from_zip', { zipPath })
}

export function installSkillFromUrl(url: string, cachePath: string | null): Promise<Skill[]> {
  return invoke<Skill[]>('install_skill_from_url', { url, cachePath })
}

export function removeSkill(name: string): Promise<Skill[]> {
  return invoke<Skill[]>('remove_skill', { name })
}

export function skillsDir(): Promise<string> {
  return invoke<string>('skills_dir')
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

/** Run one agentic (skill-enabled) turn inside a project. Emits agent://* events. */
export function sendAgentChat(
  projectId: string,
  messages: ChatMessage[],
  allowExec: boolean,
): Promise<void> {
  return invoke<void>('send_agent_chat', { projectId, messages, allowExec })
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

export interface PollStatus {
  running: boolean
  lastCheckinUnix: number | null
  nextCallAt: number | null
  jobsWaiting: number
  lastError: string | null
  plaintextBlocked: boolean
}

export function getPollStatus(): Promise<PollStatus> {
  return invoke<PollStatus>('get_poll_status')
}

export function getLastChatModel(): Promise<string | null> {
  return invoke<string | null>('get_last_chat_model')
}

export function setLastChatModel(model: string): Promise<void> {
  return invoke<void>('set_last_chat_model', { model })
}

export function getStudioTiles(): Promise<string[]> {
  return invoke<string[]>('get_studio_tiles')
}

export function setStudioTiles(tiles: string[]): Promise<string[]> {
  return invoke<string[]>('set_studio_tiles', { tiles })
}

export function getAutostart(): Promise<boolean> {
  return invoke<boolean>('get_autostart')
}

export function setAutostart(enabled: boolean): Promise<boolean> {
  return invoke<boolean>('set_autostart', { enabled })
}

export function onPollStatus(cb: (status: PollStatus) => void): Promise<UnlistenFn> {
  return listen<PollStatus>('poll://status', (event) => cb(event.payload))
}

// ---- Projects ---------------------------------------------------------------

/** The eight per-project model slots (UI ids; never the server capability names). */
export const MODEL_SLOTS = [
  'chat',
  'voice',
  'speak',
  'vision',
  'image',
  'video',
  'embed',
  'docs',
] as const
export type ModelSlot = (typeof MODEL_SLOTS)[number]

/** This project's models. Each value is a catalog key (`service:providerId:tag`) or ''. */
export interface ProjectModels {
  chat: string
  voice: string
  speak: string
  vision: string
  image: string
  video: string
  embed: string
  docs: string
  /** Set while `chat` still holds a pre-catalog bare provider id. */
  chatLegacyProviderId: string | null
}

export type ProjectKind = 'personal' | 'project'

export interface Project {
  id: string
  slug: string
  name: string
  kind: ProjectKind
  createdAt: string
  updatedAt: string
  dictationLanguage: string
  defaultAssistantId: number | null
  assistantIds: number[]
  enabledSkills: string[]
  models: ProjectModels
  /** Synaplan knowledge-folder group key, always `DESKTOP:{id}`. */
  knowledgeFolder: string
  /** Platform-native paths; display or reveal them, never build on them in JS. */
  projectDir: string
  notesDir: string
  outDir: string
}

export interface ProjectsState {
  projects: Project[]
  activeId: string
  personalId: string
}

/** Partial update. Omit a key to leave it alone; `defaultAssistantId: null` clears it. */
export interface ProjectPatch {
  name?: string
  dictationLanguage?: string
  defaultAssistantId?: number | null
  assistantIds?: number[]
  enabledSkills?: string[]
  models?: ProjectModels
}

export function listProjects(): Promise<ProjectsState> {
  return invoke<ProjectsState>('list_projects')
}

export function getProject(id: string): Promise<Project> {
  return invoke<Project>('get_project', { id })
}

export function getActiveProject(): Promise<Project> {
  return invoke<Project>('get_active_project')
}

export function createProject(
  name: string,
  dictationLanguage: string,
  copyModelsFrom: string | null,
): Promise<Project> {
  return invoke<Project>('create_project', { name, dictationLanguage, copyModelsFrom })
}

export function updateProject(id: string, patch: ProjectPatch): Promise<Project> {
  return invoke<Project>('update_project', { id, patch })
}

export function deleteProject(id: string, removeFiles: boolean): Promise<ProjectsState> {
  return invoke<ProjectsState>('delete_project', { id, removeFiles })
}

export function setActiveProject(id: string): Promise<ProjectsState> {
  return invoke<ProjectsState>('set_active_project', { id })
}

// ---- Chats (per project) ----------------------------------------------------

export interface StoredChatMessage {
  role: 'user' | 'assistant'
  content: string
  /** Chat model in force for an assistant reply; '' for user messages. */
  model: string
  createdAt: string
}

export interface ChatThread {
  id: string
  projectId: string
  title: string
  createdAt: string
  updatedAt: string
  assistantId: number | null
  messages: StoredChatMessage[]
}

export interface ChatSummary {
  id: string
  projectId: string
  title: string
  createdAt: string
  updatedAt: string
  messageCount: number
  assistantId: number | null
}

export function listChats(projectId: string): Promise<ChatSummary[]> {
  return invoke<ChatSummary[]>('list_chats', { projectId })
}

/** A fresh, unsaved thread with a Rust-minted id. */
export function newChat(projectId: string): Promise<ChatThread> {
  return invoke<ChatThread>('new_chat', { projectId })
}

export function loadChat(projectId: string, chatId: string): Promise<ChatThread> {
  return invoke<ChatThread>('load_chat', { projectId, chatId })
}

export function saveChat(thread: ChatThread): Promise<void> {
  return invoke<void>('save_chat', { thread })
}

export function deleteChat(projectId: string, chatId: string): Promise<void> {
  return invoke<void>('delete_chat', { projectId, chatId })
}

/** Narrow an unknown thrown value into a {@link CommandError}. */
export function asCommandError(err: unknown): CommandError {
  if (err && typeof err === 'object' && 'code' in err && 'message' in err) {
    return err as CommandError
  }
  return { code: 'unexpected', message: String(err) }
}
