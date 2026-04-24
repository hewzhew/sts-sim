param(
    [string]$OutDir = "tmp/aux_bundle_suite",
    [int]$Depth = 4,
    [int]$TrainPct = 80,
    [int]$ValPct = 10,
    [int]$LimitPerRaw = 0,
    [int]$PreserveTriggerNegativeRows = 8,
    [string[]]$RunIds = @(),
    [switch]$IncludeLiveRaw
)

$ErrorActionPreference = "Stop"

function Read-JsonFile {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    if (-not (Test-Path $Path)) {
        throw "Missing JSON artifact: $Path"
    }

    return Get-Content -Path $Path -Raw | ConvertFrom-Json
}

function Build-CommonParams {
    param(
        [string]$BundleOutDir
    )

    $params = @{
        OutDir = $BundleOutDir
        Depth = $Depth
        TrainPct = $TrainPct
        ValPct = $ValPct
        LimitPerRaw = $LimitPerRaw
        PreserveTriggerNegativeRows = $PreserveTriggerNegativeRows
        RunIds = $RunIds
    }
    if ($IncludeLiveRaw) {
        $params["IncludeLiveRaw"] = $true
    }
    return $params
}

function Summarize-BundleResult {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [Parameter(Mandatory = $true)]
        [string]$BundleDir
    )

    $splitDir = Join-Path $BundleDir "split"
    $preflight = Read-JsonFile (Join-Path $splitDir "aux_training_preflight.json")
    $splitSummary = Read-JsonFile (Join-Path $splitDir "split_summary.json")
    $metricsPath = Join-Path $splitDir "state_corpus_aux_baseline_metrics.json"
    $metrics = if (Test-Path $metricsPath) { Read-JsonFile $metricsPath } else { $null }

    $summary = [ordered]@{
        name = $Name
        bundle_dir = $BundleDir
        split_dir = $splitDir
        requested_targets = @($preflight.requested_targets)
        supported_requested_targets = @($preflight.supported_requested_targets)
        skipped_requested_targets = $preflight.skipped_requested_targets
        split_counts = $splitSummary.split_counts
        split_group_counts = $splitSummary.split_group_counts
        split_trigger_label_counts = $splitSummary.split_trigger_label_counts
        include_bucket_filters = @($splitSummary.include_bucket_filters)
        exclude_bucket_filters = @($splitSummary.exclude_bucket_filters)
        preserve_trigger_negative_rows = $splitSummary.preserve_trigger_negative_rows
        preserved_trigger_negative_count = $splitSummary.preserved_trigger_negative_count
        trigger_coverage_adjustments = @($splitSummary.trigger_coverage_adjustments)
        preflight = [ordered]@{
            trigger = $preflight.needs_exact_trigger_target
            regime = $preflight.regime
        }
        metrics_path = if ($metrics) { $metricsPath } else { $null }
    }

    if ($metrics) {
        $summary["metrics"] = [ordered]@{
            feature_count = $metrics.feature_count
            train_rows = $metrics.train_rows
            val_rows = $metrics.val_rows
            test_rows = $metrics.test_rows
            requested_targets = @($metrics.requested_targets)
            supported_targets = @($metrics.supported_targets)
            skipped_targets = $metrics.skipped_targets
            train = $metrics.train
            val = $metrics.val
            test = $metrics.test
        }
    }

    return $summary
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$outDirAbs = if ([System.IO.Path]::IsPathRooted($OutDir)) { $OutDir } else { Join-Path $repoRoot $OutDir }
$triggerDir = Join-Path $outDirAbs "trigger"
$regimeDir = Join-Path $outDirAbs "regime"

New-Item -ItemType Directory -Force -Path $outDirAbs | Out-Null

Write-Host "Evaluating trigger auxiliary bundle into $triggerDir"
$triggerParams = Build-CommonParams -BundleOutDir $triggerDir
& (Join-Path $PSScriptRoot "build_trigger_bundle.ps1") @triggerParams
if ($LASTEXITCODE -ne 0) {
    throw "build_trigger_bundle.ps1 failed with exit code $LASTEXITCODE"
}

Write-Host "Evaluating regime auxiliary bundle into $regimeDir"
$regimeParams = Build-CommonParams -BundleOutDir $regimeDir
& (Join-Path $PSScriptRoot "build_regime_bundle.ps1") @regimeParams
if ($LASTEXITCODE -ne 0) {
    throw "build_regime_bundle.ps1 failed with exit code $LASTEXITCODE"
}

$triggerSummary = Summarize-BundleResult -Name "trigger" -BundleDir $triggerDir
$regimeSummary = Summarize-BundleResult -Name "regime" -BundleDir $regimeDir

$suiteSummary = [ordered]@{
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    out_dir = $outDirAbs
    params = [ordered]@{
        depth = $Depth
        train_pct = $TrainPct
        val_pct = $ValPct
        limit_per_raw = $LimitPerRaw
        preserve_trigger_negative_rows = $PreserveTriggerNegativeRows
        run_ids = $RunIds
        include_live_raw = [bool]$IncludeLiveRaw
    }
    bundles = [ordered]@{
        trigger = $triggerSummary
        regime = $regimeSummary
    }
    scorecard = [ordered]@{
        trigger = [ordered]@{
            test_balanced_accuracy = $triggerSummary.metrics.test.trigger.balanced_accuracy
            test_accuracy = $triggerSummary.metrics.test.trigger.accuracy
            test_rows = $triggerSummary.metrics.test.trigger.rows
            supported = $triggerSummary.metrics.supported_targets -contains "needs_exact_trigger_target"
        }
        regime = [ordered]@{
            test_balanced_accuracy = $regimeSummary.metrics.test.regime.balanced_accuracy
            test_accuracy = $regimeSummary.metrics.test.regime.accuracy
            test_rows = $regimeSummary.metrics.test.regime.rows
            supported = $regimeSummary.metrics.supported_targets -contains "regime"
        }
    }
}

$summaryPath = Join-Path $outDirAbs "aux_bundle_suite_summary.json"
$suiteSummary | ConvertTo-Json -Depth 8 | Set-Content -Path $summaryPath -Encoding UTF8

Write-Host ""
Write-Host "Aux bundle suite ready:"
Write-Host "  summary: $summaryPath"
Write-Host "  trigger test balanced_accuracy: $($suiteSummary.scorecard.trigger.test_balanced_accuracy)"
Write-Host "  regime test balanced_accuracy: $($suiteSummary.scorecard.regime.test_balanced_accuracy)"
