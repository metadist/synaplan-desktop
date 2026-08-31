#!/usr/bin/env bash
# C12 — "No shell is ever constructed." Fails if any client source constructs a
# shell command (sh -c / bash -c / cmd /c / powershell -Command / osascript -e).
# Tools take {program, args[]} only; a quoting bug must never become command
# injection. This guard runs in CI on every PR.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Directories that contain shippable client source.
SEARCH_DIRS=("src" "src-tauri/src" "src-tauri/synaplan-core/src")

# Forbidden shell-construction patterns.
PATTERNS=(
  'sh -c'
  'bash -c'
  'zsh -c'
  'cmd /c'
  'cmd.exe /c'
  'powershell -Command'
  'powershell.exe -Command'
  'pwsh -Command'
  'osascript -e'
)

found=0
for dir in "${SEARCH_DIRS[@]}"; do
  [ -d "$dir" ] || continue
  for pat in "${PATTERNS[@]}"; do
    if grep -RnF --include='*.rs' --include='*.ts' --include='*.vue' -- "$pat" "$dir"; then
      echo "ERROR: forbidden shell construction '$pat' found in $dir (C12)." >&2
      found=1
    fi
  done
done

if [ "$found" -ne 0 ]; then
  echo "No-shell guard failed. Tools must take {program, args[]}, never a shell string." >&2
  exit 1
fi

echo "No-shell guard passed: no shell is constructed in client sources."
