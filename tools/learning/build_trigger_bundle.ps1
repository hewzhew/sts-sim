param(
    [string]$OutDir = "tmp/state_corpus_bundle_trigger",
    [int]$Depth = 4,
    [int]$TrainPct = 80,
    [int]$ValPct = 10,
    [int]$LimitPerRaw = 0,
    [int]$PreserveTriggerNegativeRows = 8,
    [string[]]$RunIds = @(),
    [switch]$IncludeLiveRaw,
    [string[]]$ExcludeBuckets = @()
)

$bundleParams = @{
    OutDir = $OutDir
    Depth = $Depth
    TrainPct = $TrainPct
    ValPct = $ValPct
    LimitPerRaw = $LimitPerRaw
    PreserveTriggerNegativeRows = $PreserveTriggerNegativeRows
    TrainAuxBaseline = $true
    AuxTargets = @("needs_exact_trigger_target")
    IncludeBuckets = @("elite", "setup_window")
    ExcludeBuckets = $ExcludeBuckets
    RunIds = $RunIds
}
if ($IncludeLiveRaw) {
    $bundleParams["IncludeLiveRaw"] = $true
}

& (Join-Path $PSScriptRoot "build_state_corpus_bundle.ps1") @bundleParams
exit $LASTEXITCODE
