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

.EXAMPLE
.\tools\ml\run_turn_plan_baseline.ps1 -CompareTargetModes
Prints selected-label vs equivalent-outcome target comparisons.

.EXAMPLE
.\tools\ml\run_turn_plan_baseline.ps1 -CompareTrainingModes
Prints binary-label vs pairwise-utility training comparisons.

.EXAMPLE
.\tools\ml\run_turn_plan_baseline.ps1 -ShowTrainingCases 3
Prints compact binary vs decomposed-utility disagreement case comparisons.
#>
param(
    [string] $ProbeRoot = "tools\artifacts\tmp",
    [int] $Epochs = 40,
    [int] $Seed = 17,
    [switch] $Full,
    [int] $ShowCases = 0,
    [ValidateSet("worse", "better", "both-bad", "all")]
    [string] $CaseKind = "worse",
    [ValidateSet("root-delta", "action-shape", "target-detail")]
    [string[]] $FeatureGroups = @(),
    [ValidateSet("selected", "equivalent-hp-outcome")]
    [string] $TargetMode = "selected",
    [ValidateSet("binary", "pairwise-utility", "decomposed-utility")]
    [string] $TrainingMode = "binary",
    [switch] $CompareFeatureGroups,
    [switch] $CompareTargetModes,
    [switch] $CompareTrainingModes,
    [int] $ShowTrainingCases = 0,
    [ValidateSet("better", "worse", "both-bad", "disagree", "all")]
    [string] $TrainingCaseKind = "all",
    [ValidateSet("binary", "pairwise-utility", "decomposed-utility")]
    [string] $ReferenceTrainingMode = "binary",
    [ValidateSet("binary", "pairwise-utility", "decomposed-utility")]
    [string] $CandidateTrainingMode = "decomposed-utility",
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
    "--target-mode", "$TargetMode",
    "--training-mode", "$TrainingMode",
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

if ($CompareTargetModes) {
    $ArgsList += "--compare-target-modes"
}

if ($CompareTrainingModes) {
    $ArgsList += "--compare-training-modes"
}

if ($ShowTrainingCases -gt 0) {
    $ArgsList += @(
        "--show-training-cases", "$ShowTrainingCases",
        "--training-case-kind", "$TrainingCaseKind",
        "--reference-training-mode", "$ReferenceTrainingMode",
        "--candidate-training-mode", "$CandidateTrainingMode"
    )
}

if ($ExtraArgs) {
    $ArgsList += $ExtraArgs
}

$FeatureText = if ($FeatureGroups.Count -gt 0) { $FeatureGroups -join "," } else { "base" }
Write-Host "turn-plan baseline: root=$ProbeRoot split=source-cv epochs=$Epochs seed=$Seed target=$TargetMode training=$TrainingMode report=$ReportMode cases=$ShowCases/$CaseKind training_cases=$ShowTrainingCases/$TrainingCaseKind features=$FeatureText compare_features=$CompareFeatureGroups compare_targets=$CompareTargetModes compare_training=$CompareTrainingModes"
python @ArgsList
