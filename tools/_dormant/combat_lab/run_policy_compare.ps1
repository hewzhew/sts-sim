param(
    [string]$AuthorSpec = "data/combat_lab/specs/jaw_worm_opening.json",
    [string]$Fixture = "",
    [int]$Episodes = 20,
    [int]$Depth = 6,
    [uint64]$BaseSeed = 1,
    [string]$OutDir = "tmp/combat_lab_compare",
    [string[]]$Policies = @("heuristic", "bot", "bot_contested_takeover", "bot_no_idle_end_turn", "bot_combined")
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

$hasFixture = -not [string]::IsNullOrWhiteSpace($Fixture)
$hasAuthorSpec = -not [string]::IsNullOrWhiteSpace($AuthorSpec) -and `
    (-not $hasFixture -or $AuthorSpec -ne "data/combat_lab/specs/jaw_worm_opening.json")
if (($hasAuthorSpec -and $hasFixture) -or (-not $hasAuthorSpec -and -not $hasFixture)) {
    throw "use exactly one of -AuthorSpec or -Fixture"
}

$sourceArgs = @()
$sourceLabel = ""
if ($hasFixture) {
    $fixturePath = Resolve-RepoPath $Fixture
    $sourceArgs = @("--fixture", $fixturePath)
    $sourceLabel = $Fixture
} else {
    $authorSpecPath = Resolve-RepoPath $AuthorSpec
    $sourceArgs = @("--author-spec", $authorSpecPath)
    $sourceLabel = $AuthorSpec
}
$outRoot = Resolve-RepoPath $OutDir

New-Item -ItemType Directory -Force -Path $outRoot | Out-Null

foreach ($policy in $Policies) {
    $policyOut = Join-Path $outRoot $policy
    New-Item -ItemType Directory -Force -Path $policyOut | Out-Null

    $args = @(
        "run", "--release", "--bin", "combat_lab", "--",
        "--episodes", "$Episodes",
        "--policy", $policy,
        "--depth", "$Depth",
        "--base-seed", "$BaseSeed",
        "--out-dir", $policyOut
    )
    $args = $args[0..4] + $sourceArgs + $args[5..($args.Length - 1)]

    Write-Host "Running policy=$policy source=$sourceLabel"
    & cargo @args
    if ($LASTEXITCODE -ne 0) {
        throw "combat_lab run failed for policy=$policy source=$sourceLabel"
    }
}

$summaries = [ordered]@{}
foreach ($policy in $Policies) {
    $summaryPath = Join-Path $outRoot "$policy/summary.json"
    if (Test-Path $summaryPath) {
        $summaries[$policy] = Get-Content $summaryPath | ConvertFrom-Json
    }
}

$heuristic = $summaries["heuristic"]
$bot = $summaries["bot"]
if ($null -eq $heuristic) {
    throw "run_policy_compare requires a heuristic summary; policies=$($Policies -join ',')"
}
if ($null -eq $bot) {
    throw "run_policy_compare requires a bot summary; policies=$($Policies -join ',')"
}
$variants = @()
foreach ($policy in $Policies) {
    if ($policy -eq "heuristic" -or $policy -eq "bot") {
        continue
    }
    if ($summaries.Contains($policy)) {
        $variants += [ordered]@{
            policy = $policy
            summary = $summaries[$policy]
            delta_vs_bot = [ordered]@{
                win_rate = [double]$summaries[$policy].win_rate - [double]$bot.win_rate
                average_final_hp = [double]$summaries[$policy].average_final_hp - [double]$bot.average_final_hp
                average_damage_taken_per_episode =
                    [double]$summaries[$policy].metrics.average_damage_taken_per_episode -
                    [double]$bot.metrics.average_damage_taken_per_episode
                average_potion_uses_per_episode =
                    [double]$summaries[$policy].metrics.average_potion_uses_per_episode -
                    [double]$bot.metrics.average_potion_uses_per_episode
                bad_action_count =
                    [int]$summaries[$policy].metrics.bad_action_count -
                    [int]$bot.metrics.bad_action_count
            }
        }
    }
}

$comparison = [ordered]@{
    author_spec = if ($hasAuthorSpec) { $AuthorSpec } else { $null }
    fixture = if ($hasFixture) { $Fixture } else { $null }
    episodes = $Episodes
    base_seed = $BaseSeed
    policies = $Policies
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
    variants = $variants
}

$comparisonPath = Join-Path $outRoot "comparison.json"
$comparison | ConvertTo-Json -Depth 8 | Set-Content -Path $comparisonPath
Write-Host "Wrote comparison to $comparisonPath"
