<#
.SYNOPSIS
Generates turn-plan tactical episodes from combat benchmark manifests.

.EXAMPLE
.\tools\ml\run_tactical_trace_batch.ps1
Finds tools\artifacts\tmp\ml_capture_seed*\benchmark.json, writes tactical
labs and enriched CombatTacticalEpisodeV1 JSONL files under
tools\artifacts\tmp\current_tactical_batch.

.EXAMPLE
.\tools\ml\run_tactical_trace_batch.ps1 -RunBaseline
Generates episodes, then runs the compact ranking baseline over the generated
episode JSONL files.
#>
param(
    [string] $BenchmarkRoot = "tools\artifacts\tmp",
    [string[]] $BenchmarkPath = @(),
    [string] $OutputRoot = "tools\artifacts\tmp\current_tactical_batch",
    [string] $BenchmarkDirectoryPattern = "ml_capture_seed*",
    [int] $GuidanceLabMaxCases = 100,
    [int] $SearchMaxNodes = 1000,
    [int] $ProbeMaxNodes = 256,
    [switch] $Build,
    [ValidateSet("fast-run", "release-final", "release", "dev-opt", "debug")]
    [string] $BuildProfile = "fast-run",
    [switch] $RunBaseline,
    [ValidateSet("source", "group", "source-cv")]
    [string] $SplitMode = "source-cv",
    [ValidateSet("root-delta", "action-shape", "target-detail", "enemy-slot-context", "tactical-summary", "action-facts")]
    [string[]] $FeatureGroups = @("tactical-summary", "action-facts"),
    [int] $Epochs = 10,
    [ValidateSet("binary", "pairwise-utility", "decomposed-utility")]
    [string] $TrainingMode = "decomposed-utility",
    [switch] $CompareFeatureGroups,
    [switch] $CleanOutput
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$DriverExe = Join-Path $RepoRoot "target\$BuildProfile\combat_search_v2_driver.exe"
$Extractor = Join-Path $RepoRoot "tools\ml\combat_tactical_trace_extract.py"
$Baseline = Join-Path $RepoRoot "tools\ml\combat_first_action_ranking_baseline.py"
$OutputDir = Join-Path $RepoRoot $OutputRoot

if ($Build -or -not (Test-Path -LiteralPath $DriverExe)) {
    Write-Host "building combat_search_v2_driver profile=$BuildProfile"
    cargo build --profile $BuildProfile --bin combat_search_v2_driver
}

if ($CleanOutput -and (Test-Path -LiteralPath $OutputDir)) {
    Remove-Item -LiteralPath $OutputDir -Recurse -Force
}
New-Item -ItemType Directory -Force $OutputDir | Out-Null

if ($BenchmarkPath.Count -eq 0) {
    $BenchmarkPath = Get-ChildItem -LiteralPath (Join-Path $RepoRoot $BenchmarkRoot) -Directory -Filter $BenchmarkDirectoryPattern |
        ForEach-Object { Join-Path $_.FullName "benchmark.json" } |
        Where-Object { Test-Path -LiteralPath $_ } |
        Sort-Object
}

if ($BenchmarkPath.Count -eq 0) {
    throw "No benchmark.json files found under $BenchmarkRoot matching $BenchmarkDirectoryPattern."
}

Write-Host "tactical trace batch: benchmarks=$($BenchmarkPath.Count) output=$OutputRoot max_cases=$GuidanceLabMaxCases max_nodes=$SearchMaxNodes probe_nodes=$ProbeMaxNodes"

$EpisodeFiles = @()
foreach ($Benchmark in $BenchmarkPath) {
    $BenchmarkFullPath = if ([System.IO.Path]::IsPathRooted($Benchmark)) {
        $Benchmark
    } else {
        Join-Path $RepoRoot $Benchmark
    }
    $Name = Split-Path (Split-Path $BenchmarkFullPath -Parent) -Leaf
    $LabPath = Join-Path $OutputDir "$Name.turn_plan_lab.json"
    $EpisodePath = Join-Path $OutputDir "$Name.enriched_tactical_episode.jsonl"

    Write-Host "[$Name] turn-plan lab"
    & $DriverExe `
        --benchmark-spec $BenchmarkFullPath `
        --turn-plan-guidance-lab `
        --guidance-lab-max-cases $GuidanceLabMaxCases `
        --max-nodes $SearchMaxNodes `
        --probe-max-nodes $ProbeMaxNodes `
        --output $LabPath

    Write-Host "[$Name] tactical episode"
    python $Extractor $LabPath --out-jsonl $EpisodePath --summary-only
    $EpisodeFiles += $EpisodePath
}

if ($RunBaseline) {
    $ArgsList = @(
        "$Baseline",
        $EpisodeFiles,
        "--split-mode", "$SplitMode",
        "--epochs", "$Epochs",
        "--training-mode", "$TrainingMode",
        "--report-mode", "compact"
    )
    if ($FeatureGroups.Count -gt 0) {
        $ArgsList += "--feature-groups"
        $ArgsList += $FeatureGroups
    }
    if ($CompareFeatureGroups) {
        $ArgsList += "--compare-feature-groups"
    }
    Write-Host "ranking baseline: files=$($EpisodeFiles.Count) split=$SplitMode training=$TrainingMode features=$($FeatureGroups -join ',') epochs=$Epochs"
    python @ArgsList
}
