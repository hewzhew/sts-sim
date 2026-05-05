param(
    [string]$SpecDir = "data/combat_lab/specs",
    [int]$Episodes = 20,
    [int]$Depth = 6,
    [uint64]$BaseSeed = 1,
    [string]$OutDir = "tmp/combat_lab_batch"
)

$ErrorActionPreference = "Stop"

$repo = Split-Path -Parent $PSScriptRoot
$repo = Split-Path -Parent $repo
Set-Location $repo

$specRoot = Join-Path $repo $SpecDir
$outRoot = Join-Path $repo $OutDir
New-Item -ItemType Directory -Force -Path $outRoot | Out-Null

$rows = @()
Get-ChildItem -Path $specRoot -Filter *.json | ForEach-Object {
    $spec = $_.FullName
    $specName = $_.BaseName
    $specOut = Join-Path $outRoot $specName
    New-Item -ItemType Directory -Force -Path $specOut | Out-Null

    & powershell -ExecutionPolicy Bypass -File (Join-Path $repo "tools/combat_lab/run_policy_compare.ps1") `
        -AuthorSpec $spec `
        -Episodes $Episodes `
        -Depth $Depth `
        -BaseSeed $BaseSeed `
        -OutDir $specOut

    $comparison = Get-Content (Join-Path $specOut "comparison.json") | ConvertFrom-Json
    $rows += [pscustomobject]@{
        spec = $specName
        heuristic_win_rate = [double]$comparison.heuristic.win_rate
        bot_win_rate = [double]$comparison.bot.win_rate
        delta_win_rate = [double]$comparison.delta.win_rate
        heuristic_avg_hp = [double]$comparison.heuristic.average_final_hp
        bot_avg_hp = [double]$comparison.bot.average_final_hp
        delta_avg_hp = [double]$comparison.delta.average_final_hp
        heuristic_bad_actions = [int]$comparison.heuristic.metrics.bad_action_count
        bot_bad_actions = [int]$comparison.bot.metrics.bad_action_count
        delta_bad_actions = [int]$comparison.delta.bad_action_count
    }
}

$summaryPath = Join-Path $outRoot "batch_summary.json"
$rows | ConvertTo-Json -Depth 6 | Set-Content -Path $summaryPath
$rows | Format-Table -AutoSize
Write-Host "Wrote batch summary to $summaryPath"
