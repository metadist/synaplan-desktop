/**
 * The JSON contract between a recorded demo and `fake-tauri.js`. Everything is
 * plain data so it can be handed to the page with `addInitScript`; the fake
 * side reads it as `window.__SYNAPLAN_DEMO__`.
 */

export type DemoLanguage = 'de' | 'en'

export interface DemoModels {
  chat: string
  voice: string
  speak: string
  vision: string
  image: string
  video: string
  embed: string
  docs: string
  chatLegacyProviderId: null
}

export interface DemoProject {
  id: string
  slug: string
  name: string
  kind: 'personal' | 'project'
  enabledSkills: string[]
  webSearch: boolean
  models: DemoModels
  /** Platform-native paths, shown or revealed only. */
  projectDir: string
  notesDir: string
  outDir: string
}

export interface DemoNote {
  name: string
  title: string
  content: string
  updatedAt?: string
}

export interface DemoFile {
  name: string
  size: number
  path?: string
}

/** A path the native file dialog "returns" when the person clicks Add files. */
export interface DemoPick {
  path: string
  size: number
}

/** One tool step in the run feed: start copy, then end copy, optionally a produced file. */
export interface StepEvent {
  kind: 'step'
  tool: 'list_files' | 'read_file' | 'write_file' | 'run_program'
  start: string
  end: string
  ok?: boolean
  artifact?: string
  size?: number
  /** How long the step "runs" before it ends. */
  ms: number
}

export interface TextEvent {
  kind: 'text'
  text: string
  wordMs?: number
}

export interface PauseEvent {
  kind: 'pause'
  ms: number
}

export type TurnEvent = StepEvent | TextEvent | PauseEvent

export interface TurnScript {
  /** Delay before the first event, the model "thinking". */
  thinkMs?: number
  events: TurnEvent[]
}

export interface DoctorTool {
  id: string
  name: string
  found: boolean
  path: string | null
  version: string | null
  hint: string
}

export interface DemoScenario {
  id: string
  language: DemoLanguage
  apiBaseUrl: string
  deviceName?: string
  keyBackend?: string
  lastCheckinUnix?: number
  paths: {
    sep: '/' | '\\'
    projectsDir: string
    outboxDir: string
    skillsDir: string
    configDir: string
  }
  personal: DemoProject
  project: DemoProject
  /** Skills enabled on this computer (the project narrows them with enabledSkills). */
  skills: string[]
  defaultModels?: Partial<Omit<DemoModels, 'chatLegacyProviderId'>>
  executionConsent: boolean
  studioTiles?: string[]
  assistants?: unknown[]
  notes?: DemoNote[]
  files?: DemoFile[]
  pickFiles?: DemoPick[]
  pickFolder?: string | null
  uploadMs?: number
  wordMs?: number
  doctor?: DoctorTool[]
  /** Consumed in order: the n-th send gets the n-th script. */
  turns: TurnScript[]
}

/** What the spec drives, step by step; also the storyboard the docs describe. */
export interface DemoStoryboard {
  scenario: DemoScenario
  /** The caption shown at the bottom of the recording before the first action. */
  title: string
  /** Each act is one thing the person does, with the caption shown while it happens. */
  acts: DemoAct[]
}

export type DemoAct =
  | { kind: 'caption'; text: string; holdMs?: number }
  | { kind: 'openPanel'; tab: 'notes' | 'files'; caption: string; holdMs?: number }
  | { kind: 'openNote'; name: string; caption: string; holdMs?: number }
  | { kind: 'closePanel' }
  | { kind: 'addFiles'; caption: string; waitReadyMs?: number }
  | { kind: 'send'; prompt: string; caption: string; consent?: boolean }
  | { kind: 'reveal'; artifactIndex: number; caption: string }
  | { kind: 'wait'; ms: number }
