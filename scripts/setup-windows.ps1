<#
.SYNOPSIS
  One-shot developer setup for Synaplan Desktop on Windows 10 22H2+ / 11.
  Ensures Rust, Node, the MSVC C++ build tools, and the WebView2 runtime are
  present, then installs JS dependencies. Re-runnable.

.NOTES
  Run in PowerShell:  ./scripts/setup-windows.ps1
  Uses winget when a tool is missing. Some installs may prompt for elevation.
#>
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')

function Info($m) { Write-Host "==> $m" -ForegroundColor Cyan }
function Warn($m) { Write-Host "!! $m" -ForegroundColor Yellow }

function Have($name) { return [bool](Get-Command $name -ErrorAction SilentlyContinue) }

$hasWinget = Have 'winget'

# ---- Rust ----
if (-not (Have 'cargo')) {
  if ($hasWinget) {
    Info 'Installing Rust (rustup) via winget...'
    winget install --id Rustlang.Rustup -e --accept-source-agreements --accept-package-agreements
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
  } else {
    Warn 'Rust not found. Install from https://rustup.rs and re-run.'
    exit 1
  }
} else {
  Info "Rust already installed: $(cargo --version)"
}
rustup component add clippy rustfmt | Out-Null

# ---- MSVC C++ build tools (required to link Rust on Windows) ----
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (-not (Test-Path $vswhere)) {
  if ($hasWinget) {
    Info 'Installing Visual Studio 2022 Build Tools (C++ workload)...'
    winget install --id Microsoft.VisualStudio.2022.BuildTools -e --accept-source-agreements --accept-package-agreements `
      --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
  } else {
    Warn 'MSVC build tools not detected. Install "Desktop development with C++" from the VS Build Tools.'
  }
} else {
  Info 'Visual Studio Build Tools detected.'
}

# ---- WebView2 runtime (bundled in the installer for end users; needed to run dev) ----
$wv2 = Get-ItemProperty 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' -ErrorAction SilentlyContinue
if (-not $wv2) {
  if ($hasWinget) {
    Info 'Installing the Microsoft Edge WebView2 Runtime...'
    winget install --id Microsoft.EdgeWebView2Runtime -e --accept-source-agreements --accept-package-agreements
  } else {
    Warn 'WebView2 runtime not detected. Install the Evergreen runtime from Microsoft.'
  }
} else {
  Info 'WebView2 runtime present.'
}

# ---- Node ----
if (-not (Have 'node')) {
  if ($hasWinget) {
    Info 'Installing Node.js 22 LTS via winget...'
    winget install --id OpenJS.NodeJS.LTS -e --accept-source-agreements --accept-package-agreements
    $env:Path = "$env:ProgramFiles\nodejs;$env:Path"
  } else {
    Warn 'Node.js not found. Install Node 22+ from https://nodejs.org and re-run.'
    exit 1
  }
}
$nodeMajor = (node -p 'process.versions.node.split(".")[0]')
if ([int]$nodeMajor -lt 22) { Warn "Node $(node --version) found; this project needs Node 22+." }

Info 'Installing JS dependencies (npm ci)...'
npm ci

Info 'Done. Start the app with:  npm run tauri dev'
