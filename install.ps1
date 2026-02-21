# Merlin installer — Windows (PowerShell)
# Usage:
#   irm https://github.com/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest/download/install.ps1 | iex
#
# Environment variables:
#   $env:MERLIN_VERSION      Pin a release tag, e.g. "v1.2.0" (default: latest)
#   $env:MERLIN_INSTALL_DIR  Where to put the binary (default: $HOME\.merlin\bin)
#   $env:MERLIN_NO_VERIFY    Set to "1" to skip SHA-256 verification

[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$Repo    = "Arunachalamkalimuthu/merlin-ai-code-review"
$Binary  = "merlin"
$Asset   = "merlin-windows-amd64.exe"

function Write-Step([string]$Msg) {
    Write-Host "[merlin] $Msg" -ForegroundColor Green
}
function Write-Warning2([string]$Msg) {
    Write-Host "[merlin] WARNING: $Msg" -ForegroundColor Yellow
}
function Fail([string]$Msg) {
    Write-Host "[merlin] ERROR: $Msg" -ForegroundColor Red
    exit 1
}

# ── Resolve version ─────────────────────────────────────────────────────────────

$Version = $env:MERLIN_VERSION
if (-not $Version -or $Version -eq "latest") {
    Write-Step "Fetching latest release tag..."
    try {
        $Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
        $Version = $Release.tag_name
    } catch {
        Fail "Could not fetch latest release: $_"
    }
}
if (-not $Version) { Fail "Could not determine release version." }

Write-Step "Installing Merlin $Version ($Asset)..."

$BaseUrl   = "https://github.com/$Repo/releases/download/$Version"
$AssetUrl  = "$BaseUrl/$Asset"
$Sha256Url = "$BaseUrl/$Asset.sha256"

# ── Download ────────────────────────────────────────────────────────────────────

$TmpDir = [System.IO.Path]::GetTempPath() + [System.IO.Path]::GetRandomFileName()
New-Item -ItemType Directory -Path $TmpDir -Force | Out-Null

$AssetPath  = Join-Path $TmpDir $Asset
$Sha256Path = Join-Path $TmpDir "$Asset.sha256"

Write-Step "Downloading $AssetUrl"
try {
    Invoke-WebRequest -Uri $AssetUrl -OutFile $AssetPath -UseBasicParsing
} catch {
    Remove-Item -Recurse -Force $TmpDir -ErrorAction SilentlyContinue
    Fail "Download failed: $_"
}

# ── Verify checksum ─────────────────────────────────────────────────────────────

$NoVerify = $env:MERLIN_NO_VERIFY
if ($NoVerify -ne "1") {
    try {
        Write-Step "Downloading checksum..."
        Invoke-WebRequest -Uri $Sha256Url -OutFile $Sha256Path -UseBasicParsing

        $ExpectedLine = Get-Content $Sha256Path -Raw
        $Expected = $ExpectedLine.Trim().Split(' ')[0].ToLower()

        Write-Step "Verifying checksum..."
        $Actual = (Get-FileHash -Path $AssetPath -Algorithm SHA256).Hash.ToLower()

        if ($Actual -ne $Expected) {
            Remove-Item -Recurse -Force $TmpDir -ErrorAction SilentlyContinue
            Fail "Checksum mismatch!`n  Expected: $Expected`n  Got:      $Actual`n`nThe download may be corrupt. Try again."
        }
        Write-Step "Checksum OK."
    } catch {
        Write-Warning2 "Could not verify checksum: $_ — continuing without verification."
    }
}

# ── Install ─────────────────────────────────────────────────────────────────────

$InstallDir = $env:MERLIN_INSTALL_DIR
if (-not $InstallDir) {
    $InstallDir = Join-Path $HOME ".merlin\bin"
}

New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null

$Dest = Join-Path $InstallDir "$Binary.exe"
Move-Item -Path $AssetPath -Destination $Dest -Force

Write-Step "Installed → $Dest"

# ── Add to PATH ─────────────────────────────────────────────────────────────────

$UserPath = [System.Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [System.Environment]::SetEnvironmentVariable(
        "PATH",
        "$InstallDir;$UserPath",
        "User"
    )
    Write-Step "Added $InstallDir to your user PATH."
    Write-Step "Restart your terminal for the PATH change to take effect."
} else {
    # Also update the current session
    $env:PATH = "$InstallDir;$env:PATH"
}

# ── Cleanup ─────────────────────────────────────────────────────────────────────

Remove-Item -Recurse -Force $TmpDir -ErrorAction SilentlyContinue

# ── Done ─────────────────────────────────────────────────────────────────────────

Write-Step "Done! Run: merlin --help"
