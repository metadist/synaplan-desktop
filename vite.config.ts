import { execFileSync } from 'node:child_process'
import { fileURLToPath, URL } from 'node:url'
import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

// Tauri expects a fixed dev port and does not want Vite to clear the screen so
// Rust build output stays visible. See docs/DEVELOPMENT.md.
// Vitest configuration lives in vitest.config.ts.
const host = process.env.TAURI_DEV_HOST

function windowsCwdIsWslShare(): boolean {
  if (process.platform !== 'win32') {
    return false
  }
  const cwd = process.cwd()
  if (/^[/\\]{2}wsl/i.test(cwd)) {
    return true
  }
  const drive = /^[A-Za-z]:/.exec(cwd)?.[0]
  if (!drive) {
    return false
  }
  try {
    const out = execFileSync('net.exe', ['use', drive], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    })
    return /\\\\wsl(?:\.localhost|\$)\\/i.test(out)
  } catch {
    return false
  }
}

// Windows fs.watch on a \\wsl.localhost / mapped WSL drive reports files as
// directories (EISDIR) and Vite dies. Polling is slower but works.
const usePolling =
  process.env.SYNAPLAN_VITE_POLL === '1' ||
  process.env.CHOKIDAR_USEPOLLING === '1' ||
  process.env.CHOKIDAR_USEPOLLING === 'true' ||
  windowsCwdIsWslShare()

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
      usePolling,
      ...(usePolling ? { interval: 300 } : {}),
    },
  },
  // Env vars starting with these prefixes are exposed to the client. Only
  // dev-only flags belong here — the runtime Synaplan URL comes from pairing,
  // never from a build-time env var (see AGENTS.md).
  envPrefix: ['VITE_', 'TAURI_ENV_'],
})
