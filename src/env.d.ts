/// <reference types="vite/client" />

interface ImportMetaEnv {
  /**
   * Dev-only convenience: pre-fills the Synaplan address field on the pairing
   * screen so a developer does not retype their dev instance URL each run. This
   * is NOT the runtime configuration source — the real address is always the
   * one the user pairs against and is stored per install. See AGENTS.md.
   */
  readonly VITE_SYNAPLAN_DEV_URL?: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}

declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  const component: DefineComponent<Record<string, unknown>, Record<string, unknown>, unknown>
  export default component
}
