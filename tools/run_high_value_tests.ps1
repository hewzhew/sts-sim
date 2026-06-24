param(
    [switch]$IncludeParity,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot

$commands = @(
    "cargo test --quiet"
)

Write-Host "High-value correctness suite" -ForegroundColor Cyan
Write-Host "Repo: $repoRoot"
Write-Host ""
Write-Host "Commands:"
foreach ($command in $commands) {
    Write-Host "  $command"
}

if ($IncludeParity) {
    Write-Host ""
    Write-Host "IncludeParity is currently compatibility-only: live_comm parity drivers are retired until the adapter is rebuilt." -ForegroundColor DarkYellow
}

if ($DryRun) {
    return
}

Push-Location $repoRoot
try {
    foreach ($command in $commands) {
        Write-Host ""
        Write-Host "==> $command" -ForegroundColor Yellow
        Invoke-Expression $command
        if ($LASTEXITCODE -ne 0) {
            throw "Command failed: $command"
        }
    }
}
finally {
    Pop-Location
}
