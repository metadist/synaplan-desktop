#!/usr/bin/env bash
# Start Synaplan Desktop in development mode on macOS (13 Ventura+).
#
# Double-click in Finder or run from Terminal. Checks the toolchain first
# (Xcode Command Line Tools, Node 22+, Rust stable, JS dependencies) and names
# exactly what is missing — with the command that fixes it — instead of failing
# deep inside a build. `./start-macos.command --check` runs only the checks. Any
# other arguments are passed on to `npm run tauri dev`.
set -euo pipefail

cd "$(cd "$(dirname "$0")" && pwd)"

info() { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
ok() { printf '\033[1;32m ok\033[0m %s\n' "$*"; }
fail() { printf '\033[1;31m !!\033[0m %s\n' "$*"; }

MIN_NODE=22
MIN_RUST="1.77"
SETUP="bash scripts/setup-macos.sh"
problems=0
check_only=0
if [ "${1:-}" = "--check" ]; then
  check_only=1
  shift
fi

# A Finder double-click starts with a minimal PATH; pick up the usual installs.
for extra in "$HOME/.cargo/bin" /opt/homebrew/bin /usr/local/bin; do
  case ":$PATH:" in *":$extra:"*) ;; *) [ -d "$extra" ] && PATH="$extra:$PATH" ;; esac
done
if [ -s "$HOME/.nvm/nvm.sh" ] && ! command -v node >/dev/null 2>&1; then
  # shellcheck disable=SC1091
  . "$HOME/.nvm/nvm.sh" >/dev/null 2>&1 || true
fi
export PATH

# ---- Xcode Command Line Tools (C toolchain + WebKit headers) ----
if xcode-select -p >/dev/null 2>&1 && command -v clang >/dev/null 2>&1; then
  ok "Xcode Command Line Tools ($(xcode-select -p))"
else
  fail "Xcode Command Line Tools not installed. Run: xcode-select --install   (or $SETUP)"
  problems=$((problems + 1))
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
  fail "Node.js not found. Install Node ${MIN_NODE}+ (brew install node@22 / https://nodejs.org) or run: $SETUP"
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
  if [ "$(printf '%s\n%s\n' "$MIN_RUST" "$rust_version" | sort -t. -k1,1n -k2,2n -k3,3n | head -n1)" = "$MIN_RUST" ]; then
    ok "Rust $rust_version"
  else
    fail "Rust $rust_version is too old; this project needs ${MIN_RUST}+. Run: rustup update"
    problems=$((problems + 1))
  fi
  host="$(rustc -vV | awk '/^host:/ {print $2}')"
  case "$host" in
    *-apple-darwin) ok "Rust host $host" ;;
    *) fail "Rust host is '$host', expected an apple-darwin toolchain."; problems=$((problems + 1)) ;;
  esac
else
  fail "Rust toolchain (cargo/rustc) not found. Run: $SETUP   (or https://rustup.rs)"
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
  # Keep the Terminal window readable when launched from Finder.
  [ -t 0 ] && read -r -p "Press Return to close." _
  exit 1
fi

if [ "$check_only" -eq 1 ]; then
  ok "Toolchain complete."
  exit 0
fi

# ---- Start ----
info "Starting Synaplan Desktop (Vite on http://localhost:1420 + the Tauri window)…"
exec npm run tauri dev "$@"
