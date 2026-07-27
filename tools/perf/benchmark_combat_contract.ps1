<#
.SYNOPSIS
Benchmarks the canonical replay-verified combat contract.

.DESCRIPTION
Builds the narrow profiling target once, warms it up, and runs repeatable
batches using exactly the same workload as the WPR profiler. Every iteration
must return identical search counters and witness identity; timing-only fields
are deliberately excluded from that contract check.

.EXAMPLE
.\tools\perf\benchmark_combat_contract.ps1

.EXAMPLE
.\tools\perf\benchmark_combat_contract.ps1 -SkipBuild -Batches 5 -IterationsPerBatch 12
#>
[CmdletBinding(PositionalBinding = $false)]
param(
    [string] $Case = ".oracle-lab\cases\seed022-a2f32-collector-full-hp.combat.json",
    [ValidateRange(1, 20)]
    [int] $Batches = 3,
    [ValidateRange(1, 100)]
    [int] $IterationsPerBatch = 8,
    [ValidateRange(0, 20)]
    [int] $WarmupIterations = 1,
    [switch] $SkipBuild,
    [switch] $AsJson
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "combat_contract_workload.ps1")

function Get-ContractIdentity($Report) {
    return [ordered]@{
        status = $Report.status
        counters = $Report.counters
        witness = $Report.witness
    }
}

function Invoke-CombatContract(
    [string] $Executable,
    [object[]] $Arguments
) {
    $StartInfo = [Diagnostics.ProcessStartInfo]::new()
    $StartInfo.FileName = $Executable
    $StartInfo.UseShellExecute = $false
    $StartInfo.CreateNoWindow = $true
    $StartInfo.RedirectStandardOutput = $true
    $StartInfo.RedirectStandardError = $true
    foreach ($Argument in $Arguments) {
        $StartInfo.ArgumentList.Add([string] $Argument)
    }

    $Process = [Diagnostics.Process]::new()
    $Process.StartInfo = $StartInfo
    $Stopwatch = [Diagnostics.Stopwatch]::StartNew()
    if (-not $Process.Start()) {
        throw "failed to start combat contract"
    }
    $StandardOutput = $Process.StandardOutput.ReadToEndAsync()
    $StandardError = $Process.StandardError.ReadToEndAsync()
    $Process.WaitForExit()
    $Stopwatch.Stop()
    $Raw = $StandardOutput.GetAwaiter().GetResult().Trim()
    $ErrorText = $StandardError.GetAwaiter().GetResult().Trim()
    if ($Process.ExitCode -ne 0) {
        throw "combat contract failed with exit code $($Process.ExitCode)`n$ErrorText`n$Raw"
    }
    try {
        $Report = $Raw | ConvertFrom-Json -Depth 100
    }
    catch {
        throw "combat contract did not return valid JSON`n$Raw"
    }
    return [pscustomobject]@{
        report = $Report
        elapsed_milliseconds = $Stopwatch.Elapsed.TotalMilliseconds
    }
}

$RepoRoot = [IO.Path]::GetFullPath(
    (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))
)
$CasePath = if ([IO.Path]::IsPathRooted($Case)) {
    [IO.Path]::GetFullPath($Case)
} else {
    [IO.Path]::GetFullPath((Join-Path $RepoRoot $Case))
}
if (-not (Test-Path -LiteralPath $CasePath -PathType Leaf)) {
    throw "combat case is missing at '$CasePath'"
}

Push-Location $RepoRoot
try {
    if (-not $SkipBuild) {
        & cargo build --locked --profile profiling -p sts_combat_contract --bin combat_contract
        if ($LASTEXITCODE -ne 0) {
            throw "combat contract profiling build failed"
        }
    }

    $Executable = Join-Path $RepoRoot "target\profiling\combat_contract.exe"
    if (-not (Test-Path -LiteralPath $Executable -PathType Leaf)) {
        throw "combat contract is missing at '$Executable'; rerun without -SkipBuild"
    }
    $WorkloadArguments = Get-StsCombatContractWorkloadArguments $CasePath

    $ExpectedIdentity = $null
    $ExpectedIdentityJson = $null
    $LastReport = $null
    for ($Index = 0; $Index -lt $WarmupIterations; $Index++) {
        $Run = Invoke-CombatContract $Executable $WorkloadArguments
        $LastReport = $Run.report
        $ExpectedIdentity = Get-ContractIdentity $LastReport
        $ExpectedIdentityJson = $ExpectedIdentity | ConvertTo-Json -Depth 100 -Compress
    }

    $BatchMilliseconds = [Collections.Generic.List[double]]::new()
    $SearchMilliseconds = [Collections.Generic.List[double]]::new()
    for ($Batch = 1; $Batch -le $Batches; $Batch++) {
        $BatchElapsedMilliseconds = 0.0
        for ($Iteration = 1; $Iteration -le $IterationsPerBatch; $Iteration++) {
            $Run = Invoke-CombatContract $Executable $WorkloadArguments
            $BatchElapsedMilliseconds += $Run.elapsed_milliseconds
            $LastReport = $Run.report
            $SearchMilliseconds.Add($LastReport.search_elapsed_ns / 1000000.0)
            $Identity = Get-ContractIdentity $LastReport
            $IdentityJson = $Identity | ConvertTo-Json -Depth 100 -Compress
            if ($null -eq $ExpectedIdentityJson) {
                $ExpectedIdentity = $Identity
                $ExpectedIdentityJson = $IdentityJson
            } elseif ($IdentityJson -cne $ExpectedIdentityJson) {
                throw "search identity changed in batch $Batch iteration $Iteration"
            }
        }
        $BatchMilliseconds.Add($BatchElapsedMilliseconds)
    }

    $Sorted = @($BatchMilliseconds | Sort-Object)
    $Middle = [int][Math]::Floor($Sorted.Count / 2)
    $MedianBatchMilliseconds = if (($Sorted.Count % 2) -eq 1) {
        $Sorted[$Middle]
    } else {
        ($Sorted[$Middle - 1] + $Sorted[$Middle]) / 2.0
    }
    $SortedSearch = @($SearchMilliseconds | Sort-Object)
    $SearchMiddle = [int][Math]::Floor($SortedSearch.Count / 2)
    $MedianSearchMilliseconds = if (($SortedSearch.Count % 2) -eq 1) {
        $SortedSearch[$SearchMiddle]
    } else {
        ($SortedSearch[$SearchMiddle - 1] + $SortedSearch[$SearchMiddle]) / 2.0
    }
    $GitCommit = (& git rev-parse HEAD).Trim()
    $GitDirty = -not [string]::IsNullOrWhiteSpace((& git status --porcelain) -join "`n")
    $Result = [ordered]@{
        schema_name = "CombatContractBenchmarkV1"
        schema_version = 1
        git_commit = $GitCommit
        git_dirty = $GitDirty
        case = $CasePath
        batches = $Batches
        iterations_per_batch = $IterationsPerBatch
        warmup_iterations = $WarmupIterations
        batch_milliseconds = @($BatchMilliseconds)
        median_batch_milliseconds = $MedianBatchMilliseconds
        median_iteration_milliseconds = $MedianBatchMilliseconds / $IterationsPerBatch
        median_search_milliseconds = $MedianSearchMilliseconds
        identity = $ExpectedIdentity
    }

    if ($AsJson) {
        $Result | ConvertTo-Json -Depth 100
    } else {
        [pscustomobject]@{
            commit = $GitCommit.Substring(0, [Math]::Min(8, $GitCommit.Length))
            dirty = $GitDirty
            batches = "$Batches x $IterationsPerBatch"
            batch_ms = (@($BatchMilliseconds | ForEach-Object { [Math]::Round($_, 2) }) -join ", ")
            median_process_ms = [Math]::Round($Result.median_iteration_milliseconds, 2)
            median_search_ms = [Math]::Round($Result.median_search_milliseconds, 2)
            generation_work = $ExpectedIdentity.counters.generation_work
            transitions = $ExpectedIdentity.counters.applied_action_transitions
            exact_nodes = $ExpectedIdentity.counters.exact_nodes
            witness = "$($ExpectedIdentity.witness.final_hp) HP / $($ExpectedIdentity.witness.actions) actions"
        } | Format-List
    }
}
finally {
    Pop-Location
}
