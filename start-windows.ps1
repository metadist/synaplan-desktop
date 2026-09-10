<#
.SYNOPSIS
  Start Synaplan Desktop in development mode on Windows 10 22H2+ / 11.

.DESCRIPTION
  Checks the toolchain first (Node 22+, Rust stable with the MSVC host, the
  Visual Studio C++ build tools, the WebView2 runtime, JS dependencies) and
  names exactly what is missing - with the command that fixes it - instead of
  failing deep inside a build. Extra arguments are passed on to `npm run tauri dev`.

  Unexpected errors always print the reason (the script never exits silently).
  A \\wsl.localhost / \\wsl$ UNC working directory is mapped to a drive letter
  so npm.cmd and cargo can run (cmd.exe cannot use UNC as the current directory).

.NOTES
  Run in PowerShell from the repository root:
    ./start-windows.ps1            # check, then start
    ./start-windows.ps1 -Check     # checks only
  If scripts are blocked by the execution policy:
    powershell -ExecutionPolicy Bypass -File .\start-windows.ps1
  From Explorer on a WSL path (\\wsl.localhost\...):
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
if (Test-Path variable:/PSNativeCommandUseErrorActionPreference) {
  $PSNativeCommandUseErrorActionPreference = $false
}

function Show-FailureAndExit {
  param($Err)
  if (Get-Command Write-SynException -ErrorAction SilentlyContinue) {
    Write-SynException $Err
  } else {
    Write-Host ''
    Write-Host "!! start-windows.ps1 failed: $($Err.Exception.Message)" -ForegroundColor Red
    if ($Err.InvocationInfo) { Write-Host $Err.InvocationInfo.PositionMessage -ForegroundColor DarkYellow }
    if ($Err.ScriptStackTrace) { Write-Host $Err.ScriptStackTrace -ForegroundColor DarkGray }
  }
  exit 1
}

try {
  if (-not $PSScriptRoot) {
    throw 'This script must be run as a file (.\start-windows.ps1 or powershell -ExecutionPolicy Bypass -File .\start-windows.ps1), not pasted into the prompt.'
  }

  $common = Join-Path $PSScriptRoot 'scripts\windows-common.ps1'
  if (-not (Test-Path -LiteralPath $common)) {
    throw "Missing helper script: $common"
  }
  . $common

  Info "Synaplan Desktop (Windows)  PowerShell $($PSVersionTable.PSVersion)"
  Set-SynProjectLocation -Path $PSScriptRoot

  $MinNode = 22
  $MinRust = [version]'1.77'
  $Setup = '.\scripts\setup-windows.ps1'
  $problems = 0

  # A fresh PowerShell may not see tools installed a moment ago.
  foreach ($extra in @("$env:USERPROFILE\.cargo\bin", "$env:ProgramFiles\nodejs")) {
    if ((Test-Path $extra) -and (($env:Path -split ';') -notcontains $extra)) {
      $env:Path = "$extra;$env:Path"
    }
  }

  # ---- Node + npm ----
  $nodeStub = Get-SynCommandSource 'node'
  if (Test-SynCommand 'node') {
    $nodeMajor = Get-SynNodeMajor
    $nodeVer = Get-SynNodeVersionString
    if ($nodeMajor -ge $MinNode) { Ok "Node $nodeVer" }
    else { Fail "Node $nodeVer is too old; this project needs Node ${MinNode}+."; $problems++ }
  } elseif ($nodeStub -and ($nodeStub -match '\\WindowsApps\\')) {
    Fail "Node.js is the Windows Store stub ($nodeStub), not a real install. Install Node ${MinNode}+ from https://nodejs.org or run: $Setup"
    $problems++
  } else {
    Fail "Node.js not found. Install Node ${MinNode}+ (https://nodejs.org) or run: $Setup"
    $problems++
  }

  $npmCmd = Find-SynNpmCmd
  if ($npmCmd) {
    $npmVer = (& $npmCmd --version 2>&1 | Out-String).Trim()
    Ok "npm $npmVer"
  } else {
    Fail 'npm not found (it ships with Node.js).'
    $problems++
  }

  # ---- Rust (MSVC host) ----
  if ((Test-SynCommand 'cargo') -and (Test-SynCommand 'rustc')) {
    $rustVersion = Get-SynRustVersion
    if ($rustVersion -ge $MinRust) { Ok "Rust $rustVersion" }
    else { Fail "Rust $rustVersion is too old; this project needs ${MinRust}+. Run: rustup update"; $problems++ }
    $rustHost = Get-SynRustHost
    if ($rustHost -like '*-pc-windows-msvc') { Ok "Rust host $rustHost" }
    else { Fail "Rust host is '$rustHost'; Tauri on Windows needs the MSVC toolchain. Run: rustup default stable-msvc"; $problems++ }
  } else {
    Fail "Rust toolchain (cargo/rustc) not found. Run: $Setup   (or https://rustup.rs)"
    $problems++
  }

  # ---- MSVC C++ build tools (linker for Rust) ----
  # Do not use $env:ProgramFiles(x86) - it is empty when PowerShell is started
  # from WSL. Do not redirect vswhere stderr with 2>$null - on Windows PowerShell
  # that becomes a terminating NativeCommandError and the script dies with no reason.
  $msvc = Get-SynMsvcInstallPath
  if ($msvc) { Ok "MSVC C++ build tools ($msvc)" }
  elseif (Test-SynCommand 'link.exe') { Ok 'MSVC linker (link.exe) on PATH' }
  else {
    $vswhere = Get-SynVsWhere
    if (-not $vswhere) {
      Fail "MSVC C++ build tools not detected: vswhere.exe was not found under Program Files and link.exe is not on PATH."
    } else {
      Fail "MSVC C++ build tools not detected: vswhere ($vswhere) found no C++ workload, and link.exe is not on PATH."
    }
    Fail "Tauri on Windows cannot link Rust without those tools. Run: $Setup"
    $problems++
  }

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
    Invoke-SynNpm -NpmArgs @('ci')
  } else {
    Fail 'JS dependencies missing (npm ci) - skipped until the toolchain is complete.'
    $problems++
  }

  if ($problems -ne 0) {
    Write-Host ''
    Fail "$problems problem(s) found. Fix them (usually: $Setup) and start again."
    exit 1
  }

  if ($Check) { Ok 'Toolchain complete.'; exit 0 }

  Info 'Starting Synaplan Desktop (Vite on http://localhost:1420 + the Tauri window)...'
  $tauri = @('run', 'tauri', 'dev')
  if ($TauriArgs) { $tauri += $TauriArgs }
  Invoke-SynNpm -NpmArgs $tauri
  exit 0
} catch {
  Show-FailureAndExit $_
}
