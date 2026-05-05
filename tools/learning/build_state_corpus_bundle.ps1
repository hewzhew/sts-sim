param(
    [string]$OutDir = "tmp/state_corpus_bundle",
    [int]$Depth = 4,
    [int]$TrainPct = 80,
    [int]$ValPct = 10,
    [int]$LimitPerRaw = 0,
    [int]$PreserveTriggerNegativeRows = 0,
    [string[]]$IncludeBuckets = @(),
    [string[]]$ExcludeBuckets = @(),
    [string[]]$RunIds = @(),
    [switch]$IncludeLiveRaw,
    [switch]$TrainAuxBaseline,
    [string[]]$AuxTargets = @("needs_exact_trigger_target", "regime")
)

$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$tmpRoot = Join-Path $repoRoot "tmp"
$combatCasesDir = Join-Path $repoRoot "tests\combat_cases"
$outDirAbs = if ([System.IO.Path]::IsPathRooted($OutDir)) { $OutDir } else { Join-Path $repoRoot $OutDir }

New-Item -ItemType Directory -Force -Path $outDirAbs | Out-Null

$fixtureDirs = New-Object System.Collections.Generic.List[string]
$fixtureFiles = New-Object System.Collections.Generic.List[string]

$fixturePatterns = @(
    (Join-Path $tmpRoot "decision_corpus_*\fixtures\*")
)

foreach ($pattern in $fixturePatterns) {
    Get-ChildItem -Path $pattern -Directory -ErrorAction SilentlyContinue |
        ForEach-Object {
            if (-not $fixtureDirs.Contains($_.FullName)) {
                $fixtureDirs.Add($_.FullName)
            }
        }
}

Get-ChildItem -Path (Join-Path $tmpRoot "live_comm_disagreement_fixtures*") -Directory -ErrorAction SilentlyContinue |
    ForEach-Object {
        Get-ChildItem -Path $_.FullName -Filter *.fixture.json -File -ErrorAction SilentlyContinue |
            ForEach-Object {
                if (-not $fixtureFiles.Contains($_.FullName)) {
                    $fixtureFiles.Add($_.FullName)
                }
            }
    }

if ($fixtureDirs.Count -eq 0 -and $fixtureFiles.Count -eq 0) {
    throw "No fixture directories found under tmp/. Expected decision_corpus_* or live_comm_disagreement_fixtures* outputs."
}

$stateCorpusPath = Join-Path $outDirAbs "state_corpus.jsonl"
$stateSummaryPath = Join-Path $outDirAbs "state_corpus_summary.json"
$splitOutDir = Join-Path $outDirAbs "split"
$auxPreflightPath = Join-Path $splitOutDir "aux_training_preflight.json"

$buildArgs = @(
    "run", "--bin", "sts_dev_tool", "--",
    "combat", "build-state-corpus",
    "--combat-case-dirs", $combatCasesDir,
    "--depth", "$Depth",
    "--out", $stateCorpusPath,
    "--summary-out", $stateSummaryPath
)

foreach ($fixtureDir in $fixtureDirs) {
    $buildArgs += @("--fixture-dirs", $fixtureDir)
}

if ($IncludeLiveRaw -or $RunIds.Count -gt 0) {
    foreach ($runId in $RunIds) {
        $buildArgs += @("--run-ids", $runId)
    }
    if ($LimitPerRaw -gt 0) {
        $buildArgs += @("--limit-per-raw", "$LimitPerRaw")
    }
}

foreach ($bucket in $IncludeBuckets) {
    if ($PreserveTriggerNegativeRows -le 0) {
        $buildArgs += @("--include-buckets", $bucket)
    }
}
foreach ($bucket in $ExcludeBuckets) {
    $buildArgs += @("--exclude-buckets", $bucket)
}

Write-Host "Building state corpus into $outDirAbs"
Write-Host "Fixture dirs: $($fixtureDirs.Count)"
Write-Host "Fixture files: $($fixtureFiles.Count)"
if ($PreserveTriggerNegativeRows -gt 0 -and $IncludeBuckets.Count -gt 0) {
    Write-Host "Preserve-trigger-negatives mode: build step keeps include buckets broad; split step will preserve up to $PreserveTriggerNegativeRows trigger-negative rows."
}

foreach ($fixtureFile in $fixtureFiles) {
    $buildArgs += @("--fixtures", $fixtureFile)
}

& cargo @buildArgs
if ($LASTEXITCODE -ne 0) {
    throw "build-state-corpus failed with exit code $LASTEXITCODE"
}

$splitArgs = @(
    "run", "--bin", "sts_dev_tool", "--",
    "combat", "split-state-corpus",
    "--input", $stateCorpusPath,
    "--out-dir", $splitOutDir,
    "--train-pct", "$TrainPct",
    "--val-pct", "$ValPct"
)

foreach ($bucket in $IncludeBuckets) {
    $splitArgs += @("--include-buckets", $bucket)
}
foreach ($bucket in $ExcludeBuckets) {
    $splitArgs += @("--exclude-buckets", $bucket)
}
if ($PreserveTriggerNegativeRows -gt 0) {
    $splitArgs += @("--preserve-trigger-negative-rows", "$PreserveTriggerNegativeRows")
}

Write-Host "Splitting state corpus into $splitOutDir"
& cargo @splitArgs
if ($LASTEXITCODE -ne 0) {
    throw "split-state-corpus failed with exit code $LASTEXITCODE"
}

$trainPath = Join-Path $splitOutDir "train.jsonl"
$trainRows = @()
if (Test-Path $trainPath) {
    $trainRows = Get-Content -Path $trainPath |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
        ForEach-Object { $_ | ConvertFrom-Json }
}

$triggerPositive = @($trainRows | Where-Object { $_.needs_exact_trigger_target }).Count
$triggerNegative = @($trainRows).Count - $triggerPositive
$regimeLabels = @(
    $trainRows |
        ForEach-Object { [string]($_.regime) } |
        Sort-Object -Unique
)
$triggerSupported = ($triggerPositive -gt 0 -and $triggerNegative -gt 0)
$regimeSupported = ($regimeLabels.Count -ge 2)
$requestedTargets = @($AuxTargets | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | ForEach-Object { $_.Trim() } | Select-Object -Unique)
if ($requestedTargets.Count -eq 0) {
    throw "AuxTargets cannot be empty."
}
$supportedRequestedTargets = New-Object System.Collections.Generic.List[string]
$skippedRequestedTargets = [ordered]@{}

if ($requestedTargets -contains "needs_exact_trigger_target" -or $requestedTargets -contains "trigger") {
    if ($triggerSupported) {
        $supportedRequestedTargets.Add("needs_exact_trigger_target")
    } else {
        $skippedRequestedTargets["needs_exact_trigger_target"] = "single_class_train"
    }
}

if ($requestedTargets -contains "regime") {
    if ($regimeSupported) {
        $supportedRequestedTargets.Add("regime")
    } else {
        $skippedRequestedTargets["regime"] = "single_class_train"
    }
}

if ($supportedRequestedTargets.Count -eq 0) {
    Write-Host "No requested auxiliary targets are currently trainable in this split."
}

$auxPreflight = [ordered]@{
    split_dir = $splitOutDir
    requested_targets = $requestedTargets
    supported_requested_targets = @($supportedRequestedTargets)
    skipped_requested_targets = $skippedRequestedTargets
    train_rows = @($trainRows).Count
    needs_exact_trigger_target = [ordered]@{
        positive_rows = $triggerPositive
        negative_rows = $triggerNegative
        supported = $triggerSupported
        reason = if ($triggerSupported) { $null } else { "single_class_train" }
    }
    regime = [ordered]@{
        distinct_label_count = $regimeLabels.Count
        labels = $regimeLabels
        supported = $regimeSupported
        reason = if ($regimeSupported) { $null } else { "single_class_train" }
    }
}

$auxPreflight | ConvertTo-Json -Depth 6 | Set-Content -Path $auxPreflightPath -Encoding UTF8

Write-Host "Aux training preflight:"
Write-Host "  trigger positives=$triggerPositive negatives=$triggerNegative supported=$triggerSupported"
Write-Host "  regime labels=$($regimeLabels.Count) [$($regimeLabels -join ', ')] supported=$regimeSupported"
Write-Host "  requested targets=[$($requestedTargets -join ', ')] supported=[$(@($supportedRequestedTargets) -join ', ')]"

if ($TrainAuxBaseline) {
    if (($requestedTargets -contains "needs_exact_trigger_target" -or $requestedTargets -contains "trigger") -and -not $triggerSupported) {
        Write-Host "  trigger target unsupported in train split; trainer will skip it."
    }
    if (($requestedTargets -contains "regime") -and -not $regimeSupported) {
        Write-Host "  regime target unsupported in train split; trainer will skip it."
    }

    if ($supportedRequestedTargets.Count -eq 0) {
        Write-Host "Skipping auxiliary baseline: no requested targets are supported in train split."
    } else {
        $pythonCmd = Join-Path $repoRoot ".venv-rl\Scripts\python.exe"
        if (-not (Test-Path $pythonCmd)) {
            $pythonCmd = "python"
        }
        Write-Host "Training auxiliary baseline on split corpus"
        $trainerArgs = @(
            (Join-Path $repoRoot "tools\learning\train_state_corpus_aux_baseline.py"),
            "--split-dir", $splitOutDir,
            "--targets", ($requestedTargets -join ",")
        )
        & $pythonCmd @trainerArgs
        if ($LASTEXITCODE -ne 0) {
            throw "train_state_corpus_aux_baseline.py failed with exit code $LASTEXITCODE"
        }
    }
}

Write-Host ""
Write-Host "Bundle ready:"
Write-Host "  state corpus: $stateCorpusPath"
Write-Host "  state summary: $stateSummaryPath"
Write-Host "  split dir: $splitOutDir"
Write-Host "  aux preflight: $auxPreflightPath"
