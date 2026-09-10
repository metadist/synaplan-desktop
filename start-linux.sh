#!/usr/bin/env bash
# Start Synaplan Desktop in development mode on Linux.
#
# Checks the toolchain first (Node 22+, Rust stable, the Tauri system libraries,
# JS dependencies) and names exactly what is missing — with the command that
# fixes it — instead of failing deep inside a build. `./start-linux.sh --check`
# runs only the checks. Any other arguments are passed on to `npm run tauri dev`.
set -euo pipefail

cd "$(cd "$(dirname "$0")" && pwd)"

info() { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
ok() { printf '\033[1;32m ok\033[0m %s\n' "$*"; }
fail() { printf '\033[1;31m !!\033[0m %s\n' "$*"; }

MIN_NODE=22
MIN_RUST="1.77"
SETUP="bash scripts/setup-linux.sh"
problems=0
check_only=0
if [ "${1:-}" = "--check" ]; then
  check_only=1
  shift
fi

# ---- Node + npm ----
if command -v node >/dev/null 2>&1; then
  node_major="$(node -p 'process.versions.node.split(".")[0]')"
  if [ "$node_major" -ge "$MIN_NODE" ]; then
    ok "Node $(node --version)"
  else
    fail "Node $(node --version) is too old; this project needs Node ${MIN_NODE}+."
    problems=$((problems + 1))
  fi
else
  fail "Node.js not found. Install Node ${MIN_NODE}+ (https://nodejs.org) or run: $SETUP"
  problems=$((problems + 1))
fi
if command -v npm >/dev/null 2>&1; then
  ok "npm $(npm --version)"
else
  fail "npm not found (it ships with Node.js)."
  problems=$((problems + 1))
fi

# ---- Rust ----
if command -v cargo >/dev/null 2>&1 && command -v rustc >/dev/null 2>&1; then
  rust_version="$(rustc --version | awk '{print $2}')"
  if [ "$(printf '%s\n%s\n' "$MIN_RUST" "$rust_version" | sort -V | head -n1)" = "$MIN_RUST" ]; then
    ok "Rust $rust_version"
  else
    fail "Rust $rust_version is too old; this project needs ${MIN_RUST}+. Run: rustup update"
    problems=$((problems + 1))
  fi
else
  fail "Rust toolchain (cargo/rustc) not found. Run: $SETUP   (or https://rustup.rs)"
  problems=$((problems + 1))
fi

# ---- C toolchain + Tauri system libraries ----
if command -v cc >/dev/null 2>&1 || command -v gcc >/dev/null 2>&1 || command -v clang >/dev/null 2>&1; then
  ok "C compiler"
else
  fail "No C compiler (build-essential / gcc / clang). Run: $SETUP"
  problems=$((problems + 1))
fi
if command -v pkg-config >/dev/null 2>&1; then
  ok "pkg-config"
  # Library → what installs it (Debian/Ubuntu names; the setup script maps dnf/pacman).
  while IFS='|' read -r lib pkg; do
    if pkg-config --exists "$lib"; then
      ok "$lib $(pkg-config --modversion "$lib")"
    else
      fail "Missing system library '$lib' ($pkg). Run: $SETUP"
      problems=$((problems + 1))
    fi
  done <<'EOF'
webkit2gtk-4.1|libwebkit2gtk-4.1-dev
gtk+-3.0|libgtk-3-dev
ayatana-appindicator3-0.1|libayatana-appindicator3-dev
librsvg-2.0|librsvg2-dev
openssl|libssl-dev
dbus-1|libdbus-1-dev
EOF
else
  fail "pkg-config not found, so the Tauri system libraries cannot be checked. Run: $SETUP"
  problems=$((problems + 1))
fi

# ---- Project dependencies ----
if [ -d node_modules ] && [ -x node_modules/.bin/tauri ]; then
  ok "JS dependencies (node_modules)"
elif [ "$problems" -eq 0 ]; then
  info "JS dependencies missing — installing with npm ci…"
  npm ci
else
  fail "JS dependencies missing (npm ci) — skipped until the toolchain is complete."
  problems=$((problems + 1))
fi

if [ "$problems" -ne 0 ]; then
  echo
  fail "$problems problem(s) found. Fix them (usually: $SETUP) and start again."
  exit 1
fi

if [ "$check_only" -eq 1 ]; then
  ok "Toolchain complete."
  exit 0
fi

# ---- Start ----
if [ -z "${DISPLAY:-}" ] && [ -z "${WAYLAND_DISPLAY:-}" ]; then
  fail "No display (DISPLAY / WAYLAND_DISPLAY unset). The app needs a desktop session."
  exit 1
fi

info "Starting Synaplan Desktop (Vite on http://localhost:1420 + the Tauri window)…"
exec npm run tauri dev "$@"
