<#
.SYNOPSIS
Benchmarks a small identity-locked panel of distinct exact combats.

.DESCRIPTION
Runs light, long-horizon, large-state, and replay-verified witness cases with
one shared work budget. The checked-in panel locks deterministic search
identity while timing remains observational. Source and artifact identity are
validated by the same build receipt as the single-case benchmark and WPR tool.
#>
[CmdletBinding(PositionalBinding = $false)]
param(
    [string] $Panel = "tools\perf\combat_performance_panel.json",
    [ValidateRange(1, 10)]
    [int] $Batches = 2,
    [ValidateRange(1, 20)]
    [int] $IterationsPerCase = 3,
    [ValidateRange(0, 10)]
    [int] $WarmupIterations = 1,
    [switch] $SkipBuild,
    [switch] $AsJson
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "combat_contract_build_receipt.ps1")

function Get-Median([double[]] $Values) {
    if ($Values.Count -eq 0) {
        return $null
    }
    $Sorted = @($Values | Sort-Object)
    $Middle = [int] [Math]::Floor($Sorted.Count / 2)
    if (($Sorted.Count % 2) -eq 1) {
        return $Sorted[$Middle]
    }
    return ($Sorted[$Middle - 1] + $Sorted[$Middle]) / 2.0
}

function Invoke-CombatPanelCase(
    [string] $Executable,
    [string] $CasePath
) {
    $Arguments = @(
        "--case", $CasePath,
        "--max-nodes", "20000",
        "--max-selections", "20000",
        "--wall-ms", "5000",
        "--max-potions-used", "2",
        "--improve-incumbent",
        "--typed-plan-guide",
        "--performance-only"
    )
    $StartInfo = [Diagnostics.ProcessStartInfo]::new()
    $StartInfo.FileName = $Executable
    $StartInfo.UseShellExecute = $false
    $StartInfo.CreateNoWindow = $true
    $StartInfo.RedirectStandardOutput = $true
    $StartInfo.RedirectStandardError = $true
    foreach ($Argument in $Arguments) {
        $StartInfo.ArgumentList.Add($Argument)
    }

    $Process = [Diagnostics.Process]::new()
    $Process.StartInfo = $StartInfo
    $Stopwatch = [Diagnostics.Stopwatch]::StartNew()
    if (-not $Process.Start()) {
        throw "failed to start combat panel case '$CasePath'"
    }
    $StandardOutput = $Process.StandardOutput.ReadToEndAsync()
    $StandardError = $Process.StandardError.ReadToEndAsync()
    $Process.WaitForExit()
    $Stopwatch.Stop()
    $Raw = $StandardOutput.GetAwaiter().GetResult().Trim()
    $ErrorText = $StandardError.GetAwaiter().GetResult().Trim()
    if ($Process.ExitCode -ne 0) {
        throw "combat panel case failed with exit code $($Process.ExitCode)`n$ErrorText`n$Raw"
    }
    try {
        $Report = $Raw | ConvertFrom-Json -Depth 100
    }
    catch {
        throw "combat panel case did not return valid JSON`n$Raw"
    }
    return [pscustomobject]@{
        report = $Report
        process_milliseconds = $Stopwatch.Elapsed.TotalMilliseconds
    }
}

function Get-CombatPanelIdentity($Report) {
    return [ordered]@{
        status = $Report.status
        generation_work = $Report.counters.generation_work
        applied_action_transitions = $Report.counters.applied_action_transitions
        exact_nodes = $Report.counters.exact_nodes
        completed_turn_options = $Report.counters.completed_turn_options
        unique_successor_states = $Report.counters.unique_successor_states
        duplicate_exact_successors = $Report.counters.duplicate_exact_successors
        terminal_win_options = $Report.counters.terminal_win_options
        witness = if ($null -eq $Report.witness) {
            $null
        }
        else {
            [ordered]@{
                final_hp = $Report.witness.final_hp
                actions = $Report.witness.actions
            }
        }
    }
}

function Assert-CombatPanelIdentity($Case, $Report) {
    $Expected = $Case.expected | ConvertTo-Json -Depth 20 -Compress
    $Actual = Get-CombatPanelIdentity $Report | ConvertTo-Json -Depth 20 -Compress
    if ($Actual -cne $Expected) {
        throw "combat panel identity changed for '$($Case.name)'`nexpected: $Expected`nactual:   $Actual"
    }
}

$RepoRoot = [IO.Path]::GetFullPath(
    (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))
)
$PanelPath = if ([IO.Path]::IsPathRooted($Panel)) {
    [IO.Path]::GetFullPath($Panel)
}
else {
    [IO.Path]::GetFullPath((Join-Path $RepoRoot $Panel))
}
if (-not (Test-Path -LiteralPath $PanelPath -PathType Leaf)) {
    throw "combat performance panel is missing at '$PanelPath'"
}
$PanelDefinition = Get-Content -LiteralPath $PanelPath -Raw | ConvertFrom-Json
if ($PanelDefinition.schema_name -ne "StsCombatPerformancePanelV1" -or
    [int] $PanelDefinition.schema_version -ne 1) {
    throw "unsupported combat performance panel at '$PanelPath'"
}

Push-Location $RepoRoot
try {
    if (-not $SkipBuild) {
        & cargo build --locked --profile profiling -p sts_combat_contract --bin combat_contract
        if ($LASTEXITCODE -ne 0) {
            throw "combat panel profiling build failed"
        }
    }
    $Executable = Join-Path $RepoRoot "target\profiling\combat_contract.exe"
    $BuildReceipt = if ($SkipBuild) {
        Assert-StsCombatContractBuildReceipt $RepoRoot $Executable
    }
    else {
        Write-StsCombatContractBuildReceipt $RepoRoot $Executable
    }

    $Cases = @($PanelDefinition.cases | ForEach-Object {
            $Definition = $_
            $CasePath = if ([IO.Path]::IsPathRooted([string] $Definition.path)) {
                [IO.Path]::GetFullPath([string] $Definition.path)
            }
            else {
                [IO.Path]::GetFullPath((Join-Path $RepoRoot ([string] $Definition.path)))
            }
            if (-not (Test-Path -LiteralPath $CasePath -PathType Leaf)) {
                throw "combat panel case is missing at '$CasePath'"
            }
            [pscustomobject]@{
                definition = $Definition
                path = $CasePath
                process_ms = [Collections.Generic.List[double]]::new()
                search_ms = [Collections.Generic.List[double]]::new()
                simulation_ns = [Collections.Generic.List[double]]::new()
                identity_ns = [Collections.Generic.List[double]]::new()
                key_build_ns = [Collections.Generic.List[double]]::new()
                publish_ns = [Collections.Generic.List[double]]::new()
            }
        })

    for ($Warmup = 0; $Warmup -lt $WarmupIterations; $Warmup++) {
        foreach ($Case in $Cases) {
            $Run = Invoke-CombatPanelCase $Executable $Case.path
            Assert-CombatPanelIdentity $Case.definition $Run.report
        }
    }
    for ($Batch = 0; $Batch -lt $Batches; $Batch++) {
        for ($Iteration = 0; $Iteration -lt $IterationsPerCase; $Iteration++) {
            foreach ($Case in $Cases) {
                $Run = Invoke-CombatPanelCase $Executable $Case.path
                Assert-CombatPanelIdentity $Case.definition $Run.report
                $Case.process_ms.Add($Run.process_milliseconds)
                $Case.search_ms.Add($Run.report.search_elapsed_ns / 1000000.0)
                $Case.simulation_ns.Add($Run.report.ns_per_applied_transition.simulation)
                $Case.identity_ns.Add($Run.report.ns_per_applied_transition.identity)
                $Case.key_build_ns.Add($Run.report.ns_per_applied_transition.key_build)
                $Case.publish_ns.Add($Run.report.ns_per_applied_transition.publish)
            }
        }
    }

    $Rows = @($Cases | ForEach-Object {
            [pscustomobject]@{
                case = $_.definition.name
                process_ms = [math]::Round((Get-Median $_.process_ms), 2)
                search_ms = [math]::Round((Get-Median $_.search_ms), 2)
                simulation_ns = [math]::Round((Get-Median $_.simulation_ns), 1)
                identity_ns = [math]::Round((Get-Median $_.identity_ns), 1)
                key_build_ns = [math]::Round((Get-Median $_.key_build_ns), 1)
                publish_ns = [math]::Round((Get-Median $_.publish_ns), 1)
                transitions = [int] $_.definition.expected.applied_action_transitions
                exact_nodes = [int] $_.definition.expected.exact_nodes
                witness_hp = if ($null -eq $_.definition.expected.witness) {
                    $null
                }
                else {
                    $_.definition.expected.witness.final_hp
                }
            }
        })
    $Result = [ordered]@{
        schema_name = "CombatPerformancePanelBenchmarkV1"
        schema_version = 1
        git_commit = (& git rev-parse HEAD).Trim()
        git_dirty = -not [string]::IsNullOrWhiteSpace((& git status --porcelain) -join "`n")
        build_source_fingerprint = $BuildReceipt.source_fingerprint
        executable_sha256 = $BuildReceipt.executable_sha256
        batches = $Batches
        iterations_per_case = $IterationsPerCase
        warmup_iterations = $WarmupIterations
        cases = $Rows
    }
    if ($AsJson) {
        $Result | ConvertTo-Json -Depth 20
    }
    else {
        Write-Host "build source: $($BuildReceipt.source_fingerprint.Substring(0, 12))"
        Write-Host "executable:   $($BuildReceipt.executable_sha256.Substring(0, 12))"
        $Rows | Format-Table -AutoSize
    }
}
finally {
    Pop-Location
}
