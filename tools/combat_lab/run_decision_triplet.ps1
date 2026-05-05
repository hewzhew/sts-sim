param(
    [string[]]$Specs = @(
        "data/combat_lab/specs/spot_weakness_attack_intent_window.json",
        "data/combat_lab/specs/survival_override_guardrail.json",
        "data/combat_lab/specs/power_through_not_on_lagavulin_debuff_turn.json"
    ),
    [int]$Episodes = 20,
    [int]$Depth = 6,
    [uint64]$BaseSeed = 1,
    [string]$OutDir = "tmp/combat_lab_decision_triplet",
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

$outRoot = Resolve-RepoPath $OutDir
New-Item -ItemType Directory -Force -Path $outRoot | Out-Null

$rows = @()
foreach ($spec in $Specs) {
    $specPath = Resolve-RepoPath $spec
    $specName = [System.IO.Path]::GetFileNameWithoutExtension($specPath)
    $specOut = Join-Path $outRoot $specName
    New-Item -ItemType Directory -Force -Path $specOut | Out-Null

    & (Join-Path $repo "tools/combat_lab/run_policy_compare.ps1") `
        -AuthorSpec $specPath `
        -Episodes $Episodes `
        -Depth $Depth `
        -BaseSeed $BaseSeed `
        -OutDir $specOut `
        -Policies $Policies

    $comparison = Get-Content (Join-Path $specOut "comparison.json") | ConvertFrom-Json
    $variantIndex = @{}
    foreach ($variant in $comparison.variants) {
        $variantIndex[$variant.policy] = $variant
    }

    $rows += [pscustomobject]@{
        spec = $specName
        heuristic_win_rate = [double]$comparison.heuristic.win_rate
        bot_win_rate = [double]$comparison.bot.win_rate
        bot_avg_hp = [double]$comparison.bot.average_final_hp
        bot_avg_dmg_taken = [double]$comparison.bot.metrics.average_damage_taken_per_episode
        bot_bad_actions = [int]$comparison.bot.metrics.bad_action_count
        contested_delta_win_rate = if ($variantIndex.ContainsKey("bot_contested_takeover")) { [double]$variantIndex["bot_contested_takeover"].delta_vs_bot.win_rate } else { $null }
        contested_delta_avg_hp = if ($variantIndex.ContainsKey("bot_contested_takeover")) { [double]$variantIndex["bot_contested_takeover"].delta_vs_bot.average_final_hp } else { $null }
        no_idle_delta_win_rate = if ($variantIndex.ContainsKey("bot_no_idle_end_turn")) { [double]$variantIndex["bot_no_idle_end_turn"].delta_vs_bot.win_rate } else { $null }
        no_idle_delta_avg_hp = if ($variantIndex.ContainsKey("bot_no_idle_end_turn")) { [double]$variantIndex["bot_no_idle_end_turn"].delta_vs_bot.average_final_hp } else { $null }
        combined_delta_win_rate = if ($variantIndex.ContainsKey("bot_combined")) { [double]$variantIndex["bot_combined"].delta_vs_bot.win_rate } else { $null }
        combined_delta_avg_hp = if ($variantIndex.ContainsKey("bot_combined")) { [double]$variantIndex["bot_combined"].delta_vs_bot.average_final_hp } else { $null }
        combined_delta_bad_actions = if ($variantIndex.ContainsKey("bot_combined")) { [int]$variantIndex["bot_combined"].delta_vs_bot.bad_action_count } else { $null }
    }
}

$summary = [ordered]@{
    specs = $Specs
    episodes = $Episodes
    depth = $Depth
    base_seed = $BaseSeed
    policies = $Policies
    rows = $rows
}

$summaryPath = Join-Path $outRoot "decision_triplet_summary.json"
$summary | ConvertTo-Json -Depth 8 | Set-Content -Path $summaryPath
$rows | Format-Table -AutoSize
Write-Host "Wrote decision triplet summary to $summaryPath"
