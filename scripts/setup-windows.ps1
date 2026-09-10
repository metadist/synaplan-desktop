<#
.SYNOPSIS
  One-shot developer setup for Synaplan Desktop on Windows 10 22H2+ / 11.
  Ensures Rust, Node, the MSVC C++ build tools, and the WebView2 runtime are
  present, then installs JS dependencies. Re-runnable.

.NOTES
  Run in PowerShell:  ./scripts/setup-windows.ps1
  Uses winget when a tool is missing. Some installs may prompt for elevation.
  Unexpected errors always print the reason (the script never exits silently).
  A \\wsl.localhost / \\wsl$ UNC working directory is mapped to a drive letter
  so npm.cmd can run.
#>
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
    Write-Host "!! setup-windows.ps1 failed: $($Err.Exception.Message)" -ForegroundColor Red
    if ($Err.InvocationInfo) { Write-Host $Err.InvocationInfo.PositionMessage -ForegroundColor DarkYellow }
    if ($Err.ScriptStackTrace) { Write-Host $Err.ScriptStackTrace -ForegroundColor DarkGray }
  }
  exit 1
}

try {
  if (-not $PSScriptRoot) {
    throw 'This script must be run as a file (.\scripts\setup-windows.ps1), not pasted into the prompt.'
  }

  $common = Join-Path $PSScriptRoot 'windows-common.ps1'
  if (-not (Test-Path -LiteralPath $common)) {
    throw "Missing helper script: $common"
  }
  . $common

  $repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).ProviderPath
  Info "Synaplan Desktop setup (Windows)  PowerShell $($PSVersionTable.PSVersion)"
  Set-SynProjectLocation -Path $repoRoot

  $hasWinget = Test-SynCommand 'winget'

  function Invoke-SynWinget {
    param([Parameter(Mandatory = $true)][string[]] $WingetArgs)
    if (-not $hasWinget) {
      throw "winget is not available. Install the tool manually, then re-run. Attempted: winget $($WingetArgs -join ' ')"
    }
    Info "Running: winget $($WingetArgs -join ' ')"
    & winget @WingetArgs
    $code = $LASTEXITCODE
    if ($null -eq $code) { $code = 0 }
    # 0 = success, -1978335189 (0x8A15002B) = already installed - both OK.
    if ($code -notin @(0, -1978335189)) {
      throw "winget $($WingetArgs -join ' ') failed with exit code $code."
    }
  }

  # ---- Rust ----
  if (-not (Test-SynCommand 'cargo')) {
    if ($hasWinget) {
      Info 'Installing Rust (rustup) via winget...'
      Invoke-SynWinget -WingetArgs @('install', '--id', 'Rustlang.Rustup', '-e', '--accept-source-agreements', '--accept-package-agreements')
      $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
    } else {
      throw 'Rust not found. Install from https://rustup.rs and re-run. winget is not available, so the script cannot install it.'
    }
  } else {
    Info "Rust already installed: $(cargo --version)"
  }
  if (-not (Test-SynCommand 'rustup')) {
    throw 'rustup not found after the Rust check. Open a new PowerShell and re-run so PATH picks up %USERPROFILE%\.cargo\bin.'
  }
  Info 'Ensuring clippy and rustfmt...'
  & rustup component add clippy rustfmt
  if ($LASTEXITCODE -ne 0) {
    throw "rustup component add clippy rustfmt failed with exit code $LASTEXITCODE."
  }

  # ---- MSVC C++ build tools (required to link Rust on Windows) ----
  $vswhere = Get-SynVsWhere
  if (-not $vswhere) {
    if ($hasWinget) {
      Info 'Installing Visual Studio 2022 Build Tools (C++ workload)...'
      Invoke-SynWinget -WingetArgs @(
        'install', '--id', 'Microsoft.VisualStudio.2022.BuildTools', '-e',
        '--accept-source-agreements', '--accept-package-agreements',
        '--override', '--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended'
      )
    } else {
      Warn 'MSVC build tools not detected (vswhere.exe missing) and winget is not available.'
      Warn 'Install "Desktop development with C++" from the VS Build Tools, then re-run.'
    }
  } else {
    Info "Visual Studio Build Tools detected ($vswhere)."
  }

  # ---- WebView2 runtime (bundled in the installer for end users; needed to run dev) ----
  $wv2 = Get-ItemProperty 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' -ErrorAction SilentlyContinue
  if (-not $wv2) {
    $wv2 = Get-ItemProperty 'HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' -ErrorAction SilentlyContinue
  }
  if (-not $wv2) {
    if ($hasWinget) {
      Info 'Installing the Microsoft Edge WebView2 Runtime...'
      Invoke-SynWinget -WingetArgs @('install', '--id', 'Microsoft.EdgeWebView2Runtime', '-e', '--accept-source-agreements', '--accept-package-agreements')
    } else {
      Warn 'WebView2 runtime not detected. Install the Evergreen runtime from Microsoft.'
    }
  } else {
    Info "WebView2 runtime present ($($wv2.pv))."
  }

  # ---- Node ----
  $nodeStub = Get-SynCommandSource 'node'
  if (-not (Test-SynCommand 'node')) {
    if ($nodeStub -and ($nodeStub -match '\\WindowsApps\\')) {
      Warn "Node.js on PATH is the Windows Store stub ($nodeStub). Installing a real Node 22+ next."
    }
    if ($hasWinget) {
      Info 'Installing Node.js 22 LTS via winget...'
      Invoke-SynWinget -WingetArgs @('install', '--id', 'OpenJS.NodeJS.LTS', '-e', '--accept-source-agreements', '--accept-package-agreements')
      $env:Path = "$env:ProgramFiles\nodejs;$env:Path"
    } else {
      throw 'Node.js not found. Install Node 22+ from https://nodejs.org and re-run. winget is not available, so the script cannot install it.'
    }
  }
  $nodeMajor = Get-SynNodeMajor
  if ($nodeMajor -lt 22) {
    throw "Node $(Get-SynNodeVersionString) found; this project needs Node 22+."
  }
  Info "Node $(Get-SynNodeVersionString)"

  Info 'Installing JS dependencies (npm ci)...'
  Invoke-SynNpm -NpmArgs @('ci')

  Info 'Done. Start the app with:  .\start-windows.ps1'
} catch {
  Show-FailureAndExit $_
}
