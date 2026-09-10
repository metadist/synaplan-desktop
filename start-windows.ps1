<#
.SYNOPSIS
  Start Synaplan Desktop in development mode on Windows 10 22H2+ / 11.

.DESCRIPTION
  Checks the toolchain first (Node 22+, Rust stable with the MSVC host, the
  Visual Studio C++ build tools, the WebView2 runtime, JS dependencies) and
  names exactly what is missing — with the command that fixes it — instead of
  failing deep inside a build. Extra arguments are passed on to `npm run tauri dev`.

.NOTES
  Run in PowerShell from the repository root:
    ./start-windows.ps1            # check, then start
    ./start-windows.ps1 -Check     # checks only
  If scripts are blocked by the execution policy:
    powershell -ExecutionPolicy Bypass -File .\start-windows.ps1
#>
[CmdletBinding()]
param(
  # Only run the toolchain checks; do not start the app.
  [switch] $Check,
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]] $TauriArgs
)

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot

function Info($m) { Write-Host "==> $m" -ForegroundColor Cyan }
function Ok($m) { Write-Host " ok $m" -ForegroundColor Green }
function Fail($m) { Write-Host " !! $m" -ForegroundColor Red }
function Have($name) { return [bool](Get-Command $name -ErrorAction SilentlyContinue) }

$MinNode = 22
$MinRust = [version]'1.77'
$Setup = './scripts/setup-windows.ps1'
$problems = 0

# A fresh PowerShell may not see tools installed a moment ago.
foreach ($extra in @("$env:USERPROFILE\.cargo\bin", "$env:ProgramFiles\nodejs")) {
  if ((Test-Path $extra) -and (($env:Path -split ';') -notcontains $extra)) { $env:Path = "$extra;$env:Path" }
}

# ---- Node + npm ----
if (Have 'node') {
  $nodeMajor = [int](node -p 'process.versions.node.split(".")[0]')
  if ($nodeMajor -ge $MinNode) { Ok "Node $(node --version)" }
  else { Fail "Node $(node --version) is too old; this project needs Node $MinNode+."; $problems++ }
} else {
  Fail "Node.js not found. Install Node $MinNode+ (https://nodejs.org) or run: $Setup"; $problems++
}
if (Have 'npm') { Ok "npm $(npm --version)" } else { Fail 'npm not found (it ships with Node.js).'; $problems++ }

# ---- Rust (MSVC host) ----
if ((Have 'cargo') -and (Have 'rustc')) {
  $rustVersion = [version](((rustc --version) -split ' ')[1] -replace '-.*$', '')
  if ($rustVersion -ge $MinRust) { Ok "Rust $rustVersion" }
  else { Fail "Rust $rustVersion is too old; this project needs $MinRust+. Run: rustup update"; $problems++ }
  $rustHost = ((rustc -vV | Select-String '^host:') -split '\s+')[1]
  if ($rustHost -like '*-pc-windows-msvc') { Ok "Rust host $rustHost" }
  else { Fail "Rust host is '$rustHost'; Tauri on Windows needs the MSVC toolchain. Run: rustup default stable-msvc"; $problems++ }
} else {
  Fail "Rust toolchain (cargo/rustc) not found. Run: $Setup   (or https://rustup.rs)"; $problems++
}

# ---- MSVC C++ build tools (linker for Rust) ----
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$msvc = $null
if (Test-Path $vswhere) {
  $msvc = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
}
if ($msvc) { Ok "MSVC C++ build tools ($msvc)" }
elseif (Have 'link.exe') { Ok 'MSVC linker (link.exe) on PATH' }
else { Fail "MSVC C++ build tools not detected (""Desktop development with C++""). Run: $Setup"; $problems++ }

# ---- WebView2 runtime ----
$wv2Keys = @(
  'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
  'HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
)
$wv2 = $wv2Keys | ForEach-Object { Get-ItemProperty $_ -ErrorAction SilentlyContinue } | Select-Object -First 1
if ($wv2) { Ok "WebView2 runtime $($wv2.pv)" }
else { Fail "WebView2 runtime not detected. Run: $Setup   (or install the Evergreen runtime from Microsoft)"; $problems++ }

# ---- Project dependencies ----
if ((Test-Path 'node_modules') -and (Test-Path 'node_modules\.bin\tauri.cmd')) {
  Ok 'JS dependencies (node_modules)'
} elseif ($problems -eq 0) {
  Info 'JS dependencies missing - installing with npm ci...'
  npm ci
  if ($LASTEXITCODE -ne 0) { Fail 'npm ci failed.'; exit 1 }
} else {
  Fail 'JS dependencies missing (npm ci) - skipped until the toolchain is complete.'; $problems++
}

if ($problems -ne 0) {
  Write-Host ''
  Fail "$problems problem(s) found. Fix them (usually: $Setup) and start again."
  exit 1
}

if ($Check) { Ok 'Toolchain complete.'; exit 0 }

# ---- Start ----
Info 'Starting Synaplan Desktop (Vite on http://localhost:1420 + the Tauri window)...'
npm run tauri dev @TauriArgs
exit $LASTEXITCODE
