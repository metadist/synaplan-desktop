#!/usr/bin/env bash
# One-shot developer setup for Synaplan Desktop on macOS (13 Ventura+).
# Ensures the Xcode Command Line Tools, Rust, and JS deps are present.
# Re-runnable: it skips anything already installed.
set -euo pipefail

cd "$(cd "$(dirname "$0")/.." && pwd)"

info() { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m!!\033[0m %s\n' "$*"; }

# ---- Xcode Command Line Tools (provides the C toolchain + WebKit headers) ----
if ! xcode-select -p >/dev/null 2>&1; then
  info "Installing the Xcode Command Line Tools (a GUI prompt will appear)…"
  xcode-select --install || true
  warn "Finish the Command Line Tools install, then re-run this script."
  exit 1
else
  info "Xcode Command Line Tools present."
fi

# ---- Rust ----
if ! command -v cargo >/dev/null 2>&1; then
  info "Installing the Rust toolchain via rustup…"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
else
  info "Rust already installed: $(cargo --version)"
fi
rustup component add clippy rustfmt >/dev/null 2>&1 || true
# Both architectures for a universal release build (Sprint B6).
rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null 2>&1 || true

# ---- Node ----
if ! command -v node >/dev/null 2>&1; then
  if command -v brew >/dev/null 2>&1; then
    info "Installing Node 22 via Homebrew…"
    brew install node@22 || brew install node
  else
    warn "Node.js not found and Homebrew is unavailable."
    warn "Install Node 22+ from https://nodejs.org and re-run."
    exit 1
  fi
fi
NODE_MAJOR="$(node -p 'process.versions.node.split(".")[0]')"
if [ "$NODE_MAJOR" -lt 22 ]; then
  warn "Node $(node --version) found; this project needs Node 22+."
fi

info "Installing JS dependencies (npm ci)…"
npm ci

info "Done. Start the app with:  npm run tauri dev"
