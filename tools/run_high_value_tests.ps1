param(
    [switch]$IncludeParity,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot

$commands = @(
    "cargo test --test protocol_truth_samples",
    "cargo test --test state_sync_strictness",
    "cargo test --test guardian_threshold_behavior",
    "cargo test --test stasis_behavior"
)

if ($IncludeParity) {
    $commands += "cargo test --test live_comm_replay_driver"
}

Write-Host "High-value correctness suite" -ForegroundColor Cyan
Write-Host "Repo: $repoRoot"
Write-Host ""
Write-Host "Commands:"
foreach ($command in $commands) {
    Write-Host "  $command"
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
