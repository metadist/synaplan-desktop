#!/usr/bin/env bash
# One-shot developer setup for Synaplan Desktop on Linux.
# Installs the Tauri build prerequisites, the Rust toolchain, and JS deps.
# Re-runnable: it skips anything already present.
set -euo pipefail

cd "$(cd "$(dirname "$0")/.." && pwd)"

info() { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m!!\033[0m %s\n' "$*"; }

# ---- System build dependencies ----
PKGS_APT="libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf libxdo-dev libssl-dev libdbus-1-dev pkg-config build-essential curl wget file"
PKGS_DNF="webkit2gtk4.1-devel libappindicator-gtk3-devel librsvg2-devel patchelf libxdo-devel openssl-devel dbus-devel gcc gcc-c++ make curl wget file"
PKGS_PACMAN="webkit2gtk-4.1 libappindicator-gtk3 librsvg patchelf xdotool openssl dbus base-devel curl wget file"

if command -v apt-get >/dev/null 2>&1; then
  info "Installing build dependencies with apt…"
  sudo apt-get update
  # shellcheck disable=SC2086
  sudo apt-get install -y $PKGS_APT
elif command -v dnf >/dev/null 2>&1; then
  info "Installing build dependencies with dnf…"
  # shellcheck disable=SC2086
  sudo dnf install -y $PKGS_DNF
elif command -v pacman >/dev/null 2>&1; then
  info "Installing build dependencies with pacman…"
  # shellcheck disable=SC2086
  sudo pacman -S --needed --noconfirm $PKGS_PACMAN
else
  warn "Unknown package manager. Install the Tauri Linux prerequisites manually:"
  warn "  https://v2.tauri.app/start/prerequisites/"
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

# ---- Node ----
if ! command -v node >/dev/null 2>&1; then
  warn "Node.js not found. Install Node 22+ (https://nodejs.org) and re-run."
  exit 1
fi
NODE_MAJOR="$(node -p 'process.versions.node.split(".")[0]')"
if [ "$NODE_MAJOR" -lt 22 ]; then
  warn "Node $(node --version) found; this project needs Node 22+."
fi

info "Installing JS dependencies (npm ci)…"
npm ci

info "Done. Start the app with:  npm run tauri dev"
