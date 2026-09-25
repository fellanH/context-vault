# Install the native context-vault binary on Windows — no Node, no npm.
#
#   irm https://raw.githubusercontent.com/fellanH/context-vault/main/install.ps1 | iex
#
# Env overrides:
#   $env:CONTEXT_VAULT_VERSION      release tag to install, e.g. v4.0.0 (default: latest)
#   $env:CONTEXT_VAULT_INSTALL_DIR  install directory (default: %LOCALAPPDATA%\context-vault\bin)

$ErrorActionPreference = 'Stop'

$Repo = 'fellanH/context-vault'
$Target = 'x86_64-pc-windows-msvc'
$InstallDir = if ($env:CONTEXT_VAULT_INSTALL_DIR) { $env:CONTEXT_VAULT_INSTALL_DIR } else { "$env:LOCALAPPDATA\context-vault\bin" }

function Resolve-Version {
    if ($env:CONTEXT_VAULT_VERSION) { return $env:CONTEXT_VAULT_VERSION }
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
    return $release.tag_name
}

$Version = Resolve-Version
if (-not $Version) { throw "could not resolve latest release version" }

$BaseUrl = "https://github.com/$Repo/releases/download/$Version"
$ArchiveName = "context-vault-$Target.zip"
$Workdir = Join-Path $env:TEMP "context-vault-install-$([guid]::NewGuid())"
New-Item -ItemType Directory -Force -Path $Workdir | Out-Null

try {
    Write-Host "Downloading context-vault $Version ($Target)..."
    $ArchivePath = Join-Path $Workdir $ArchiveName
    Invoke-WebRequest -Uri "$BaseUrl/$ArchiveName" -OutFile $ArchivePath
    Invoke-WebRequest -Uri "$BaseUrl/SHA256SUMS" -OutFile (Join-Path $Workdir 'SHA256SUMS')

    $Sums = Get-Content (Join-Path $Workdir 'SHA256SUMS')
    $ExpectedLine = $Sums | Where-Object { $_ -match [regex]::Escape($ArchiveName) }
    if (-not $ExpectedLine) { throw "no checksum entry found for $ArchiveName" }
    $Expected = ($ExpectedLine -split '\s+')[0]
    $Actual = (Get-FileHash -Path $ArchivePath -Algorithm SHA256).Hash.ToLower()
    if ($Expected -ne $Actual) {
        throw "checksum mismatch for $ArchiveName (expected $Expected, got $Actual)"
    }

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Expand-Archive -Path $ArchivePath -DestinationPath $InstallDir -Force

    Write-Host "Installed context-vault $Version to $InstallDir\context-vault.exe"

    $UserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if ($UserPath -notlike "*$InstallDir*") {
        [Environment]::SetEnvironmentVariable('Path', "$UserPath;$InstallDir", 'User')
        Write-Host ""
        Write-Host "Added $InstallDir to your user PATH. Restart your terminal to pick it up."
    }

    Write-Host ""
    Write-Host "Next: run 'context-vault setup' to register the MCP server with your AI tools."
}
finally {
    Remove-Item -Recurse -Force $Workdir -ErrorAction SilentlyContinue
}
