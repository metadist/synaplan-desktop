import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'

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

/**
 * Stream one chat turn inside a project. The Rust side puts the project's Chat
 * model in the request and refuses with `chat_model_unset` when none is set —
 * the webview never picks a model per turn.
 */
export function sendChat(
  projectId: string,
  messages: ChatMessage[],
  assistantId: number | null = null,
): Promise<void> {
  return invoke<void>('send_chat', { projectId, messages, assistantId })
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
  assistantId: number | null = null,
): Promise<void> {
  return invoke<void>('send_agent_chat', { projectId, messages, allowExec, assistantId })
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

// ---- Model catalog ----------------------------------------------------------

/** One selectable model as the workspace advertises it. */
export interface CatalogEntry {
  /** Catalog key `service:providerId:tag`; a bare provider id only in the flat-list fallback. */
  id: string
  providerId: string
  service: string
  name: string
  available: boolean
  unavailableReason: string | null
}

/** Where a slot's entries came from. */
export type SlotSource = 'catalog' | 'audio_models' | 'flat_models' | 'none'

export interface ModelCatalog {
  slots: Record<ModelSlot, CatalogEntry[]>
  sources: Record<ModelSlot, SlotSource>
  /** The workspace does not offer the model catalog yet. */
  catalogMissing: boolean
}

export interface ModelCatalogResult {
  catalog: ModelCatalog
  /** The project's pre-catalog Chat pick was upgraded and saved; reload the project. */
  rebound: boolean
}

export function getModelCatalog(projectId: string): Promise<ModelCatalogResult> {
  return invoke<ModelCatalogResult>('get_model_catalog', { projectId })
}

// ---- Assistants (recipes on the workspace) ----------------------------------

/** Catalog keys a published recipe names; `null` means "workspace default". */
export interface AssistantModels {
  chat: string | null
  vision: string | null
  vectorize: string | null
}

/** Reader view of an Assistant. `models` is absent until the workspace sends it. */
export interface Assistant {
  id: number
  name: string
  description: string | null
  icon: string | null
  status: string | null
  models: AssistantModels | null
}

/**
 * The Assistants this key may run. Rejects with `assistants_disabled` when the
 * workspace has them turned off — that is a state to name, not an empty list.
 */
export function listAssistants(): Promise<Assistant[]> {
  return invoke<Assistant[]>('list_assistants')
}

// ---- Dictation (audio bytes go to Rust; the key never comes back) ------------

export interface DictationSession {
  sessionId: string
}

/**
 * Open a live dictation session with the project's Dictation model and
 * language. Rejects with `voice_model_unset` when the project has none —
 * the caller must not open the microphone then.
 */
export function dictationStart(projectId: string, prompt: string): Promise<DictationSession> {
  return invoke<DictationSession>('dictation_start', { projectId, prompt })
}

/** Append 16 kHz mono 16-bit PCM; `commit` marks the end of a phrase. */
export function dictationChunk(sessionId: string, pcm: Uint8Array, commit: boolean): Promise<void> {
  return invoke<void>('dictation_chunk', { sessionId, pcm: Array.from(pcm), commit })
}

/** The text recognised so far in a live session. */
export function dictationPoll(sessionId: string): Promise<string> {
  return invoke<string>('dictation_poll', { sessionId })
}

/** Transcribe what is still pending and return the whole live text. */
export function dictationCommit(sessionId: string): Promise<string> {
  return invoke<string>('dictation_commit', { sessionId })
}

export function dictationClose(sessionId: string): Promise<void> {
  return invoke<void>('dictation_close', { sessionId })
}

/** One-shot transcription of a whole take (e.g. `audio/webm`). */
export function dictationTranscribe(
  projectId: string,
  prompt: string,
  audio: Uint8Array,
  mime: string,
): Promise<string> {
  return invoke<string>('dictation_transcribe', {
    projectId,
    prompt,
    audio: Array.from(audio),
    mime,
  })
}

// ---- Out folder (what skills produced for the project) -----------------------

export interface OutFile {
  name: string
  size: number
  modifiedAt: string
  /** Platform-native path, for "Show in folder" only. */
  path: string
}

/** Files in the project's `out/` folder, newest first. */
export function listOutFiles(projectId: string): Promise<OutFile[]> {
  return invoke<OutFile[]>('list_out_files', { projectId })
}

// ---- Knowledge folder (files sent to Synaplan) ------------------------------

/** Plain-language lifecycle of a file in the project's knowledge folder. */
export type KnowledgeState = 'sent' | 'reading' | 'indexing' | 'ready' | 'stale' | 'failed'

export interface KnowledgeFile {
  id: number
  name: string
  size: number
  state: KnowledgeState
  /** Plain reason when `state` is `failed`, if the workspace gave one. */
  detail: string | null
  uploadedAt: string
}

export function listProjectFiles(projectId: string): Promise<KnowledgeFile[]> {
  return invoke<KnowledgeFile[]>('list_project_files', { projectId })
}

/**
 * Send a local file (a path the OS handed us) into the project's knowledge
 * folder. Rust reads the file, checks it lies in an allowed folder and attaches
 * the project's index/documents models; the key never enters JS.
 */
export function uploadProjectFile(projectId: string, path: string): Promise<KnowledgeFile> {
  return invoke<KnowledgeFile>('upload_project_file', { projectId, path })
}

export function deleteProjectFile(projectId: string, fileId: number): Promise<void> {
  return invoke<void>('delete_project_file', { projectId, fileId })
}

/** OS drag-and-drop over the app window, as Tauri reports it. */
export type FileDropEvent =
  | { type: 'enter'; paths: string[] }
  | { type: 'over' }
  | { type: 'drop'; paths: string[] }
  | { type: 'leave' }

export function onFileDrop(cb: (event: FileDropEvent) => void): Promise<UnlistenFn> {
  return getCurrentWebview().onDragDropEvent((event) => cb(event.payload))
}

// ---- Notes ------------------------------------------------------------------

/** A Markdown note on this computer, addressed by its file name only. */
export interface NoteSummary {
  name: string
  title: string
  updatedAt: string
  size: number
}

export interface Note extends Omit<NoteSummary, 'size'> {
  content: string
  /** Platform-native path — reveal it, never build on it in JS. */
  path: string
}

export function listNotes(projectId: string, query = ''): Promise<NoteSummary[]> {
  return invoke<NoteSummary[]>('list_notes', { projectId, query })
}

export function createNote(projectId: string): Promise<Note> {
  return invoke<Note>('create_note', { projectId })
}

export function readNote(projectId: string, name: string): Promise<Note> {
  return invoke<Note>('read_note', { projectId, name })
}

export function writeNote(projectId: string, name: string, content: string): Promise<NoteSummary> {
  return invoke<NoteSummary>('write_note', { projectId, name, content })
}

export function deleteNote(projectId: string, name: string): Promise<void> {
  return invoke<void>('delete_note', { projectId, name })
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
