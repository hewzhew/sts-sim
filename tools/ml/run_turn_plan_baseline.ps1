<#
.SYNOPSIS
Runs the compact turn-plan ranking baseline over discovered probe JSONL files.

.EXAMPLE
.\tools\ml\run_turn_plan_baseline.ps1
Discovers turn-plan probe samples under tools\artifacts\tmp and runs source-CV.

.EXAMPLE
.\tools\ml\run_turn_plan_baseline.ps1 -Full
Prints the detailed top1/MRR and feature-weight report.

.EXAMPLE
.\tools\ml\run_turn_plan_baseline.ps1 -ShowCases 3
Prints the compact report plus three model-worse-than-ordered case comparisons.

.EXAMPLE
.\tools\ml\run_turn_plan_baseline.ps1 -CompareFeatureGroups
Prints the baseline plus opt-in experimental feature-group comparisons.
#>
param(
    [string] $ProbeRoot = "tools\artifacts\tmp",
    [int] $Epochs = 40,
    [int] $Seed = 17,
    [switch] $Full,
    [int] $ShowCases = 0,
    [ValidateSet("worse", "better", "both-bad", "all")]
    [string] $CaseKind = "worse",
    [ValidateSet("root-delta", "action-shape")]
    [string[]] $FeatureGroups = @(),
    [switch] $CompareFeatureGroups,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $ExtraArgs
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$ScriptPath = Join-Path $RepoRoot "tools\ml\combat_first_action_ranking_baseline.py"
$ReportMode = if ($Full) { "full" } else { "compact" }

$ArgsList = @(
    "$ScriptPath",
    "--discover-turn-plan-probes", "$ProbeRoot",
    "--split-mode", "source-cv",
    "--epochs", "$Epochs",
    "--seed", "$Seed",
    "--report-mode", "$ReportMode"
)

if ($ShowCases -gt 0) {
    $ArgsList += @("--show-cases", "$ShowCases", "--case-kind", "$CaseKind")
}

if ($FeatureGroups.Count -gt 0) {
    $ArgsList += "--feature-groups"
    $ArgsList += $FeatureGroups
}

if ($CompareFeatureGroups) {
    $ArgsList += "--compare-feature-groups"
}

if ($ExtraArgs) {
    $ArgsList += $ExtraArgs
}

$FeatureText = if ($FeatureGroups.Count -gt 0) { $FeatureGroups -join "," } else { "base" }
Write-Host "turn-plan baseline: root=$ProbeRoot split=source-cv epochs=$Epochs seed=$Seed report=$ReportMode cases=$ShowCases/$CaseKind features=$FeatureText compare=$CompareFeatureGroups"
python @ArgsList
