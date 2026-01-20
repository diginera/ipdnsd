<#
.SYNOPSIS
    ipdnsd installer script for Windows

.DESCRIPTION
    Downloads and installs the latest release of ipdnsd.

    Usage:
    irm https://raw.githubusercontent.com/diginera/ipdnsd/main/scripts/install.ps1 | iex

    Or:
    Invoke-WebRequest -Uri https://raw.githubusercontent.com/diginera/ipdnsd/main/scripts/install.ps1 -OutFile install.ps1; .\install.ps1

.NOTES
    This script:
    1. Downloads the latest Windows release
    2. Installs it to %LOCALAPPDATA%\ipdnsd
    3. Adds it to your PATH
    4. Creates a default config file
#>

$ErrorActionPreference = "Stop"

$Repo = "diginera/ipdnsd"
$BinaryName = "ipdnsd"

function Write-Info {
    param([string]$Message)
    Write-Host "[INFO] " -ForegroundColor Blue -NoNewline
    Write-Host $Message
}

function Write-Success {
    param([string]$Message)
    Write-Host "[OK] " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

function Write-Warn {
    param([string]$Message)
    Write-Host "[WARN] " -ForegroundColor Yellow -NoNewline
    Write-Host $Message
}

function Write-Error {
    param([string]$Message)
    Write-Host "[ERROR] " -ForegroundColor Red -NoNewline
    Write-Host $Message
    exit 1
}

function Get-LatestVersion {
    try {
        $releases = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -UseBasicParsing
        return $releases.tag_name
    }
    catch {
        Write-Error "Could not fetch latest version: $_"
    }
}

function Get-Architecture {
    $arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    switch ($arch) {
        "X64" { return "amd64" }
        "Arm64" { return "arm64" }
        default { Write-Error "Unsupported architecture: $arch" }
    }
}

function Install-Ipdnsd {
    Write-Host ""
    Write-Info "Installing ipdnsd - IP to DNS Updater"
    Write-Host ""

    # Get latest version
    $version = Get-LatestVersion
    $arch = Get-Architecture

    Write-Info "Latest version: $version"
    Write-Info "Architecture: $arch"

    # Setup install directory
    $installDir = Join-Path $env:LOCALAPPDATA "ipdnsd"
    $binPath = Join-Path $installDir "$BinaryName.exe"

    if (-not (Test-Path $installDir)) {
        New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    }

    # Download binary
    $assetName = "$BinaryName-windows-$arch.exe"
    $downloadUrl = "https://github.com/$Repo/releases/download/$version/$assetName"

    Write-Info "Downloading from: $downloadUrl"

    try {
        Invoke-WebRequest -Uri $downloadUrl -OutFile $binPath -UseBasicParsing
    }
    catch {
        Write-Error "Download failed: $_"
    }

    Write-Success "Downloaded to $binPath"

    # Add to PATH if not already there
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$installDir*") {
        Write-Info "Adding $installDir to user PATH..."
        $newPath = "$userPath;$installDir"
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        $env:Path = "$env:Path;$installDir"
        Write-Success "Added to PATH"
    }
    else {
        Write-Info "$installDir already in PATH"
    }

    # Create config directory and default config
    $configDir = "C:\ProgramData\ipdnsd"
    $configFile = Join-Path $configDir "config.toml"

    if (-not (Test-Path $configDir)) {
        New-Item -ItemType Directory -Path $configDir -Force | Out-Null
    }

    if (-not (Test-Path $configFile)) {
        $configContent = @"
# ipdnsd Configuration
# See https://github.com/diginera/ipdnsd for documentation

[daemon]
interval_seconds = 300  # Check every 5 minutes
log_level = "info"

# Example DNS entry - update with your domain
# Uncomment and modify the following:

# [[dns_entries]]
# provider = "godaddy"
# domain = "example.com"
# record_name = "@"
# record_type = "A"
# ip_source = "external"
"@
        Set-Content -Path $configFile -Value $configContent
        Write-Success "Created config file at $configFile"
    }
    else {
        Write-Info "Config file already exists at $configFile"
    }

    # Print next steps
    Write-Host ""
    Write-Host "==============================================" -ForegroundColor Cyan
    Write-Host "  ipdnsd installed successfully!" -ForegroundColor Cyan
    Write-Host "==============================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Next steps:"
    Write-Host ""
    Write-Host "1. " -NoNewline; Write-Host "Restart your terminal" -ForegroundColor Yellow
    Write-Host "   (Required for PATH changes to take effect)"
    Write-Host ""
    Write-Host "2. Store your DNS provider API credentials:"
    Write-Host "   ipdnsd set-key godaddy" -ForegroundColor Gray
    Write-Host ""
    Write-Host "3. Edit your config file:"
    Write-Host "   $configFile" -ForegroundColor Gray
    Write-Host ""
    Write-Host "4. Test your configuration:"
    Write-Host "   ipdnsd check" -ForegroundColor Gray
    Write-Host ""
    Write-Host "5. Run the daemon:"
    Write-Host "   ipdnsd daemon" -ForegroundColor Gray
    Write-Host ""
    Write-Host "6. (Optional) Install as a Windows service (Run as Administrator):"
    Write-Host "   ipdnsd install" -ForegroundColor Gray
    Write-Host ""
    Write-Host "For more information: https://github.com/$Repo"
    Write-Host ""
}

# Run installer
Install-Ipdnsd
