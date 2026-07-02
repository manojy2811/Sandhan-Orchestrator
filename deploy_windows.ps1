# Aether ACP Agent Wrapper Windows Deployment Script

Write-Host "Checking Rust Installation..." -ForegroundColor Cyan
if ((Get-Command "cargo" -ErrorAction SilentlyContinue) -eq $null) {
    Write-Host "Warning: Rust is not installed on this system." -ForegroundColor Yellow
    Write-Host "Please install Rustup from https://rustup.rs/ before running this project." -ForegroundColor Yellow
} else {
    Write-Host "Rust toolchain detected. Compiling Aether ACP Agent wrapper..." -ForegroundColor Green
    cargo build --release
    Write-Host "Compilation complete. Binary located at target\release\acp-agent-wrapper.exe" -ForegroundColor Green
}

Write-Host "`nSetting up environment settings..." -ForegroundColor Cyan
$WorkspaceDir = Join-Path $PSScriptRoot "workspace"
if (-not (Test-Path $WorkspaceDir)) {
    New-Item -ItemType Directory -Path $WorkspaceDir | Out-Null
    Write-Host "Created isolated workspace directory at: $WorkspaceDir" -ForegroundColor Green
}

Write-Host "`nTo run the ACP agent in your editor, add the execution command path:" -ForegroundColor Cyan
Write-Host "Command: target\release\acp-agent-wrapper.exe" -ForegroundColor Green
