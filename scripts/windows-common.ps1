# Shared helpers for start-windows.ps1 and setup-windows.ps1.
# Dot-source from those scripts; do not run this file directly.

function Info([string] $m) { Write-Host "==> $m" -ForegroundColor Cyan }
function Ok([string] $m) { Write-Host " ok $m" -ForegroundColor Green }
function Fail([string] $m) { Write-Host " !! $m" -ForegroundColor Red }
function Warn([string] $m) { Write-Host " !! $m" -ForegroundColor Yellow }

function Write-SynException {
  param($Err)
  Write-Host ''
  Write-Host '!! Script failed - it did not finish.' -ForegroundColor Red
  $msg = $null
  if ($Err -and $Err.Exception) { $msg = $Err.Exception.Message }
  if (-not $msg) { $msg = [string]$Err }
  if (-not $msg) { $msg = 'Unknown error (no exception message was set).' }
  Write-Host "   Reason: $msg" -ForegroundColor Red
  if ($Err.Exception -and $Err.Exception.InnerException) {
    Write-Host "   Inner:  $($Err.Exception.InnerException.Message)" -ForegroundColor Red
  }
  if ($Err.InvocationInfo -and $Err.InvocationInfo.PositionMessage) {
    Write-Host ($Err.InvocationInfo.PositionMessage.Trim()) -ForegroundColor DarkYellow
  }
  if ($Err.ScriptStackTrace) {
    Write-Host $Err.ScriptStackTrace -ForegroundColor DarkGray
  }
  $cwd = try { (Get-Location).ProviderPath } catch { '?' }
  Write-Host "   PowerShell $($PSVersionTable.PSVersion) | cwd=$cwd" -ForegroundColor DarkGray
}

function Test-SynCommand {
  param([string] $Name)
  $cmd = Get-Command $Name -ErrorAction SilentlyContinue
  if (-not $cmd) { return $false }
  # Windows Store execution aliases look installed but only open the Store page.
  if ($cmd.Source -and ($cmd.Source -match '\\WindowsApps\\')) { return $false }
  return $true
}

function Get-SynCommandSource {
  param([string] $Name)
  $cmd = Get-Command $Name -ErrorAction SilentlyContinue
  if ($cmd) { return [string]$cmd.Source }
  return $null
}

function Get-SynNodeMajor {
  if (-not (Test-SynCommand 'node')) {
    throw 'node is not on PATH (or is the Windows Store stub).'
  }
  # Parse `node --version` (e.g. v24.15.0). Do NOT use node -p with a JS
  # string that contains quotes - PowerShell concatenates '.' into the
  # expression and Node then dies with "Unexpected token '.'".
  $raw = (& node.exe --version 2>&1 | Out-String).Trim()
  if ($LASTEXITCODE -ne 0) {
    throw "node --version failed (exit $LASTEXITCODE): $raw"
  }
  if ($raw -notmatch '^v?(\d+)\.') {
    throw "Could not parse a Node.js version from '$raw'."
  }
  return [int]$Matches[1]
}

function Get-SynNodeVersionString {
  return ((& node.exe --version 2>&1 | Out-String).Trim())
}

function Get-SynRustVersion {
  $raw = (& rustc --version 2>&1 | Out-String).Trim()
  if ($LASTEXITCODE -ne 0 -or -not $raw) {
    throw "rustc --version failed (exit $LASTEXITCODE): $raw"
  }
  $token = ($raw -split '\s+')[1]
  if (-not $token) { throw "Could not parse a Rust version from '$raw'." }
  $numeric = $token -replace '-.*$', ''
  try {
    return [version]$numeric
  } catch {
    throw "Could not parse a Rust version from '$raw'."
  }
}

function Get-SynRustHost {
  $line = & rustc -vV 2>&1 | Where-Object { $_ -match '^host:\s+\S+' } | Select-Object -First 1
  if (-not $line) { throw 'Could not read the rustc host triple (rustc -vV had no host: line).' }
  return (($line -split '\s+', 2)[1]).Trim()
}

function Find-SynNpmCmd {
  foreach ($name in @('npm.cmd', 'npm.exe', 'npm')) {
    $cmd = Get-Command $name -ErrorAction SilentlyContinue
    if ($cmd -and $cmd.Source -and ($cmd.Source -notmatch '\\WindowsApps\\')) {
      return $cmd.Source
    }
  }
  return $null
}

function Invoke-SynNpm {
  param([Parameter(Mandatory = $true)][string[]] $NpmArgs)
  $npm = Find-SynNpmCmd
  if (-not $npm) { throw 'npm not found (it ships with Node.js).' }
  $cwd = (Get-Location).ProviderPath
  if ($cwd -match '^\\\\') {
    throw "Working directory is still a UNC path ($cwd). cmd.exe / npm.cmd cannot use UNC as the current directory, so npm would silently run in C:\Windows instead."
  }
  Info "Running: $npm $($NpmArgs -join ' ')  (cwd=$cwd)"
  & $npm @NpmArgs
  $code = $LASTEXITCODE
  if ($null -eq $code) { $code = 0 }
  if ($code -ne 0) {
    throw "npm $($NpmArgs -join ' ') failed with exit code $code. Working directory: $cwd"
  }
}

function Get-SynDriveMappings {
  $rows = @()
  try {
    foreach ($m in (Get-SmbMapping -ErrorAction Stop)) {
      if ($m.LocalPath -and $m.RemotePath) {
        $rows += [pscustomobject]@{
          Local  = $m.LocalPath.TrimEnd('\')
          Remote = $m.RemotePath.TrimEnd('\')
        }
      }
    }
  } catch {
    # SmbShare cmdlets are not always present; net.exe is.
  }
  foreach ($line in (& net.exe use 2>&1)) {
    $text = [string]$line
    if ($text -match '([A-Za-z]:)\s+(\\\\[^\s]+)') {
      $local = $Matches[1]
      $remote = $Matches[2].TrimEnd('\')
      $already = $false
      foreach ($r in $rows) {
        if ($r.Local -eq $local) { $already = $true; break }
      }
      if (-not $already) {
        $rows += [pscustomobject]@{ Local = $local; Remote = $remote }
      }
    }
  }
  return $rows
}

function Get-SynExistingUncMapping {
  param([string] $UncPath)
  $best = $null
  $bestLen = -1
  foreach ($m in (Get-SynDriveMappings)) {
    if ($UncPath.StartsWith($m.Remote, [StringComparison]::OrdinalIgnoreCase)) {
      if ($m.Remote.Length -gt $bestLen) {
        $best = $m
        $bestLen = $m.Remote.Length
      }
    }
  }
  if (-not $best) { return $null }
  $suffix = $UncPath.Substring($best.Remote.Length).TrimStart('\')
  if ($suffix) { return "$($best.Local)\$suffix" }
  return $best.Local
}

function Get-SynFreeDriveLetter {
  $used = @{}
  Get-PSDrive -PSProvider FileSystem | ForEach-Object { $used[$_.Name] = $true }
  foreach ($m in (Get-SynDriveMappings)) {
    if ($m.Local -match '^([A-Za-z]):') { $used[$Matches[1]] = $true }
  }
  # Prefer S (Synaplan), then Z down to D. Never A/B/C.
  $order = @([char]'S') + [char[]](90..68)
  foreach ($c in $order) {
    $n = [string]$c
    if ($used.ContainsKey($n)) { continue }
    if (Test-Path -LiteralPath "${n}:\") { continue }
    return $n
  }
  throw 'No free drive letter left to map the WSL/UNC project path. Disconnect an unused mapped drive (net use X: /delete) and retry.'
}

function Convert-SynLinkTargetToUnc {
  param([string] $Target)
  if (-not $Target) { return $null }
  $t = $Target.Trim()
  # Directory symlink targets often look like UNC\wsl$\Distro\path, not \\wsl$\...
  if ($t -match '^(?:\?\?\\)?UNC\\(.+)$') { $t = '\\' + $Matches[1] }
  if ($t -notmatch '^\\\\') { return $null }
  return ($t -replace '^\\\\wsl\$\\', '\\wsl.localhost\')
}

function Get-SynUncFromReparse {
  param([string] $Path)
  $current = $Path
  $suffixParts = @()
  while ($current) {
    $item = Get-Item -LiteralPath $current -Force -ErrorAction SilentlyContinue
    $raw = $null
    if ($item -and $item.Target) {
      $raw = $item.Target
      if ($raw -is [array]) { $raw = $raw[0] }
      $raw = [string]$raw
    }
    $target = Convert-SynLinkTargetToUnc -Target $raw
    if ($target) {
      if ($suffixParts.Count -gt 0) {
        return ($target.TrimEnd('\') + '\' + ($suffixParts -join '\'))
      }
      return $target
    }
    $root = [System.IO.Path]::GetPathRoot($current)
    if (-not $root -or ($current.TrimEnd('\') -eq $root.TrimEnd('\'))) { break }
    $suffixParts = @((Split-Path $current -Leaf)) + $suffixParts
    $parent = Split-Path $current -Parent
    if (-not $parent -or $parent -eq $current) { break }
    $current = $parent
  }
  return $null
}

function Convert-SynUncToDrivePath {
  param([string] $Path)
  if (-not $Path) {
    throw 'Project path is empty. Run this script as a file (.\start-windows.ps1), not by pasting it into the prompt.'
  }
  $Path = $Path -replace '^Microsoft\.PowerShell\.Core\\FileSystem::', ''
  try {
    $Path = [System.IO.Path]::GetFullPath($Path)
  } catch {
    throw "Could not normalize project path '$Path': $($_.Exception.Message)"
  }
  $fromLink = Get-SynUncFromReparse -Path $Path
  if ($fromLink) {
    Info "C: path is a symlink/junction to WSL ($fromLink)"
    $Path = $fromLink
  }
  $Path = $Path -replace '^\\\\wsl\$\\', '\\wsl.localhost\'
  if ($Path -match '^[A-Za-z]:\\') { return $Path }
  if ($Path -notmatch '^\\\\') { return $Path }

  $existing = Get-SynExistingUncMapping -UncPath $Path
  if ($existing) {
    Info "Reusing mapped drive $existing  (from $Path)"
    return $existing
  }

  if ($Path -notmatch '^\\\\([^\\]+)\\([^\\]+)(?:\\(.*))?$') {
    throw "Cannot parse UNC project path '$Path'. npm and cargo cannot use a UNC current directory. Clone the repo to a Windows path such as C:\src\synaplan-desktop."
  }
  $share = "\\$($Matches[1])\$($Matches[2])"
  $rest = $Matches[3]

  $letter = Get-SynFreeDriveLetter
  Info "UNC path detected. Mapping $share -> ${letter}: so npm/cmd/cargo can run."
  Info "  $Path"

  $netOut = & net.exe use "${letter}:" $share /persistent:no 2>&1
  $code = $LASTEXITCODE
  if ($code -ne 0) {
    $retry = Get-SynExistingUncMapping -UncPath $Path
    if ($retry) { return $retry }
    $detail = ($netOut | Out-String).Trim()
    throw "Could not map '$share' to ${letter}: (exit $code). $detail npm and cargo cannot use a UNC working directory (cmd.exe would silently switch to C:\Windows). Map the share yourself (net use ${letter}: $share) or clone the repo to C:\src\synaplan-desktop."
  }

  $mapped = if ($rest) { "${letter}:\$rest" } else { "${letter}:\" }
  if (-not (Test-Path -LiteralPath $mapped)) {
    throw "Mapped $share to ${letter}: but '$mapped' does not exist."
  }
  Ok "Project directory $mapped"
  return $mapped
}

function Test-SynWslBackedPath {
  param([string] $Path)
  if (Get-SynUncFromReparse -Path $Path) { return $true }
  if ($Path -match '^\\\\wsl(\.localhost|\$)\\') { return $true }
  $drive = $null
  if ($Path -match '^([A-Za-z]):') { $drive = "$($Matches[1]):" }
  if (-not $drive) { return $false }
  foreach ($m in (Get-SynDriveMappings)) {
    if ($m.Local -eq $drive -and $m.Remote -match '^\\\\wsl(\.localhost|\$)\\') { return $true }
  }
  return $false
}

function Enable-SynWslDevWorkaround {
  $env:CHOKIDAR_USEPOLLING = '1'
  $env:CHOKIDAR_INTERVAL = '300'
  $env:SYNAPLAN_VITE_POLL = '1'
  $env:WATCHPACK_POLLING = 'true'
  Warn 'WSL filesystem detected. Using polling file watchers because Windows fs.watch reports EISDIR on \\wsl.localhost / mapped WSL drives.'

  # Cargo incremental compilation lock files fail on the WSL 9p/network drive
  # with "Incorrect function" (os error -2147024895). Keep artifacts on NTFS.
  $localTarget = Join-Path $env:LOCALAPPDATA 'Synaplan\Desktop\cargo-target'
  try {
    New-Item -ItemType Directory -Force -Path $localTarget | Out-Null
    $env:CARGO_TARGET_DIR = $localTarget
    Warn "Cargo target dir is $localTarget (WSL drives cannot host incremental lock files)."
  } catch {
    $env:CARGO_INCREMENTAL = '0'
    Warn "Could not create $localTarget ($($_.Exception.Message)). Set CARGO_INCREMENTAL=0 so cargo does not lock on the WSL drive."
  }
}

function Set-SynProjectLocation {
  param([string] $Path)
  $resolved = Convert-SynUncToDrivePath -Path $Path
  Set-Location -LiteralPath $resolved
  $cwd = (Get-Location).ProviderPath
  if ($cwd -match '^\\\\') {
    throw "Working directory is still UNC ($cwd). cmd.exe / npm.cmd cannot run here. The drive-letter mapping failed."
  }
  if ((Test-SynWslBackedPath $Path) -or (Test-SynWslBackedPath $cwd)) {
    Enable-SynWslDevWorkaround
  }
}

function Get-SynProgramFilesX86 {
  # $env:ProgramFiles(x86) is often missing when powershell.exe is launched
  # from WSL (the parentheses are dropped by the interop env). Do not rely on it.
  $path = [Environment]::GetFolderPath('ProgramFilesX86')
  if ($path -and (Test-Path -LiteralPath $path)) { return $path }
  $path = [Environment]::GetEnvironmentVariable('ProgramFiles(x86)')
  if ($path -and (Test-Path -LiteralPath $path)) { return $path }
  if (Test-Path -LiteralPath 'C:\Program Files (x86)') { return 'C:\Program Files (x86)' }
  return $null
}

function Get-SynVsWhere {
  $pf = [Environment]::GetFolderPath('ProgramFiles')
  foreach ($root in @((Get-SynProgramFilesX86), $pf)) {
    if (-not $root) { continue }
    $candidate = Join-Path $root 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (Test-Path -LiteralPath $candidate) { return $candidate }
  }
  return $null
}

function Get-SynMsvcInstallPath {
  $vswhere = Get-SynVsWhere
  if ($vswhere) {
    $install = ((& $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath | Out-String).Trim())
    if ($install) { return $install }
  }
  $pf86 = Get-SynProgramFilesX86
  if (-not $pf86) { return $null }
  foreach ($year in @('2022', '2026', '2019', '2017')) {
    foreach ($edition in @('BuildTools', 'Community', 'Professional', 'Enterprise')) {
      $msvcRoot = Join-Path $pf86 "Microsoft Visual Studio\$year\$edition\VC\Tools\MSVC"
      if (-not (Test-Path -LiteralPath $msvcRoot)) { continue }
      $link = Get-ChildItem -LiteralPath $msvcRoot -Directory -ErrorAction SilentlyContinue |
        ForEach-Object { Join-Path $_.FullName 'bin\Hostx64\x64\link.exe' } |
        Where-Object { Test-Path -LiteralPath $_ } |
        Select-Object -First 1
      if ($link) {
        return (Join-Path $pf86 "Microsoft Visual Studio\$year\$edition")
      }
    }
  }
  return $null
}
