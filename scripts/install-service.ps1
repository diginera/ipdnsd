#Requires -RunAsAdministrator
<#
.SYNOPSIS
    Install ipdnsd as a Windows service

.DESCRIPTION
    This script installs ipdnsd as a Windows service that starts automatically on boot.

.EXAMPLE
    .\install-service.ps1
#>

$ServiceName = "ipdnsd"
$DisplayName = "IP to DNS Updater"
$Description = "Monitors IP addresses and updates DNS records automatically"

# Find the binary
$BinaryPath = $null

if (Test-Path ".\target\release\ipdnsd.exe") {
    $BinaryPath = (Resolve-Path ".\target\release\ipdnsd.exe").Path
}
elseif (Test-Path ".\ipdnsd.exe") {
    $BinaryPath = (Resolve-Path ".\ipdnsd.exe").Path
}
else {
    $InPath = Get-Command ipdnsd.exe -ErrorAction SilentlyContinue
    if ($InPath) {
        $BinaryPath = $InPath.Source
    }
}

if (-not $BinaryPath) {
    Write-Error "Error: ipdnsd.exe not found"
    Write-Host "Please build with 'cargo build --release' or ensure ipdnsd.exe is in PATH"
    exit 1
}

Write-Host "Using binary: $BinaryPath"

# Check if service already exists
$existingService = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue

if ($existingService) {
    Write-Host "Service already exists. Stopping and removing..."
    Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
    sc.exe delete $ServiceName | Out-Null
    Start-Sleep -Seconds 2
}

# Create the service
Write-Host "Creating service..."
$binPathWithArgs = "`"$BinaryPath`" daemon"

sc.exe create $ServiceName binPath= $binPathWithArgs start= auto DisplayName= $DisplayName | Out-Null

if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to create service"
    exit 1
}

# Set description
sc.exe description $ServiceName $Description | Out-Null

# Configure service recovery (restart on failure)
sc.exe failure $ServiceName reset= 86400 actions= restart/5000/restart/10000/restart/30000 | Out-Null

# Start the service
Write-Host "Starting service..."
Start-Service -Name $ServiceName

$service = Get-Service -Name $ServiceName
Write-Host ""
Write-Host "Service installed successfully!" -ForegroundColor Green
Write-Host "Status: $($service.Status)"
Write-Host ""
Write-Host "Useful commands:"
Write-Host "  Get-Service $ServiceName           - Check service status"
Write-Host "  Stop-Service $ServiceName          - Stop the service"
Write-Host "  Start-Service $ServiceName         - Start the service"
Write-Host "  Restart-Service $ServiceName       - Restart the service"
Write-Host "  Get-EventLog -LogName Application -Source $ServiceName  - View logs"
Write-Host ""
Write-Host "Before starting, make sure you have:"
Write-Host "  1. Created a config file (run 'ipdnsd config' for location)"
Write-Host "  2. Stored your API credentials with 'ipdnsd set-key <provider>'"
