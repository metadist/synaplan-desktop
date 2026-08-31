import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'

// Tauri expects a fixed dev port and does not want Vite to clear the screen so
// Rust build output stays visible. See docs/DEVELOPMENT.md.
// Vitest configuration lives in vitest.config.ts.
const host = process.env.TAURI_DEV_HOST

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  // Prevent Vite from obscuring Rust errors.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // Tauri sources are watched by cargo, not Vite.
      ignored: ['**/src-tauri/**'],
    },
  },
  // Env vars starting with these prefixes are exposed to the client. Only
  // dev-only flags belong here — the runtime Synaplan URL comes from pairing,
  // never from a build-time env var (see AGENTS.md).
  envPrefix: ['VITE_', 'TAURI_ENV_'],
})
