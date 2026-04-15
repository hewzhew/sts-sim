param(
    [string]$AuthorSpec = "data/combat_lab/specs/jaw_worm_opening.json",
    [int]$Episodes = 20,
    [int]$Depth = 6,
    [uint64]$BaseSeed = 1,
    [string]$OutDir = "tmp/combat_lab_compare"
)

$ErrorActionPreference = "Stop"

$repo = Split-Path -Parent $PSScriptRoot
$repo = Split-Path -Parent $repo
Set-Location $repo

function Resolve-RepoPath([string]$PathValue) {
    if ([System.IO.Path]::IsPathRooted($PathValue)) {
        return $PathValue
    }
    return (Join-Path $repo $PathValue)
}

$authorSpecPath = Resolve-RepoPath $AuthorSpec
$outRoot = Resolve-RepoPath $OutDir

New-Item -ItemType Directory -Force -Path $outRoot | Out-Null

$policies = @("heuristic", "bot")
foreach ($policy in $policies) {
    $policyOut = Join-Path $outRoot $policy
    New-Item -ItemType Directory -Force -Path $policyOut | Out-Null

    $args = @(
        "run", "--release", "--bin", "combat_lab", "--",
        "--author-spec", $authorSpecPath,
        "--episodes", "$Episodes",
        "--policy", $policy,
        "--depth", "$Depth",
        "--base-seed", "$BaseSeed",
        "--out-dir", $policyOut
    )

    Write-Host "Running policy=$policy spec=$AuthorSpec"
    & cargo @args
}

$heuristic = Get-Content (Join-Path $outRoot "heuristic/summary.json") | ConvertFrom-Json
$bot = Get-Content (Join-Path $outRoot "bot/summary.json") | ConvertFrom-Json

$comparison = [ordered]@{
    author_spec = $AuthorSpec
    episodes = $Episodes
    base_seed = $BaseSeed
    heuristic = $heuristic
    bot = $bot
    delta = [ordered]@{
        win_rate = [double]$bot.win_rate - [double]$heuristic.win_rate
        average_final_hp = [double]$bot.average_final_hp - [double]$heuristic.average_final_hp
        average_damage_taken_per_episode =
            [double]$bot.metrics.average_damage_taken_per_episode -
            [double]$heuristic.metrics.average_damage_taken_per_episode
        average_potion_uses_per_episode =
            [double]$bot.metrics.average_potion_uses_per_episode -
            [double]$heuristic.metrics.average_potion_uses_per_episode
        bad_action_count =
            [int]$bot.metrics.bad_action_count -
            [int]$heuristic.metrics.bad_action_count
    }
}

$comparisonPath = Join-Path $outRoot "comparison.json"
$comparison | ConvertTo-Json -Depth 8 | Set-Content -Path $comparisonPath
Write-Host "Wrote comparison to $comparisonPath"
