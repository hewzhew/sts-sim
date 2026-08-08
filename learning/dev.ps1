param(
    [Parameter(Mandatory = $true, Position = 0)]
    [ValidateSet("configure", "doctor", "test", "verify", "check-bridge", "refresh-bridge", "train-combat", "evaluate-combat", "evaluate-combat-potions", "evaluate-run", "evaluate-run-potions", "collect-run-roots", "train-run")]
    [string]$Command,
    [string]$Python,
    [string]$MaturinPython = "python",
    [string]$Artifact,
    [string]$Behavior,
    [string]$Output,
    [int]$Roots,
    [int]$Replicates = 8,
    [int]$Updates,
    [long]$ModelSeed = 0,
    [long]$BehaviorSeedBase = 1000,
    [int]$Slots = 4,
    [int]$Attempts = 8,
    [int]$MaxBatchSteps = 4096,
    [long]$BehaviorSeed = 10000,
    [long]$HeldOutSeedStart = 1000000,
    [int]$Generations = 1,
    [int]$AttemptsPerUpdate = 8,
    [long]$TrainingSeedStart = 0,
    [int]$EvaluationAttempts = 16,
    [int]$EvaluationMaxBatchSteps = 4096,
    [long]$EvaluationBehaviorSeed = 100000,
    [int]$WallMs = 60000,
    [int]$MinFloor = 2,
    [int]$MinUsablePotions = 1,
    [int]$MaxArtifactBytes = 16777216,
    [string]$RequiredPotionId,
    [Nullable[int]]$RequiredPotionSlot,
    [ValidateSet("raw-return", "leave-one-out", "matched-floor", "matched-floor-context", "matched-episode-floor-context")]
    [string]$AdvantageMode = "raw-return",
    [ValidateSet("independent-cohorts", "episode-root-retries")]
    [string]$SamplingMode = "independent-cohorts",
    [Nullable[int]]$EpisodeRootAttempts,
    [ValidateSet("all", "strategic")]
    [string]$DecisionScope = "all",
    [ValidateSet("all", "never", "root-slots")]
    [string]$PotionLane = "all",
    [ValidateSet("trained", "all", "never")]
    [string]$RunPotionLane = "trained",
    [int[]]$PotionSlots = @(),
    [switch]$DistinctEncounters,
    [ValidateSet("dev", "release")]
    [string]$BridgeProfile = "release"
)

$ErrorActionPreference = "Stop"

$learningRoot = (Resolve-Path -LiteralPath $PSScriptRoot).Path
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $learningRoot "..")).Path
$sourceRoot = Join-Path $learningRoot "src"
$testRoot = Join-Path $learningRoot "tests"
$hostRoot = Join-Path $repositoryRoot ".oracle-lab\hosts"
$pythonFile = Join-Path $hostRoot "learning-python.txt"
$reportRoot = Join-Path $repositoryRoot ".oracle-lab\reports"
$potionArguments = @("--potion-lane", $PotionLane)
foreach ($potionSlot in $PotionSlots) {
    $potionArguments += @("--potion-slot", $potionSlot)
}

function Resolve-PythonExecutable([string]$Candidate) {
    if (-not $Candidate) {
        throw "missing -Python <python-3.12-with-numpy-torch-and-sts_learning_bridge>"
    }
    if (-not (Test-Path -LiteralPath $Candidate -PathType Leaf)) {
        throw "Python executable does not exist: $Candidate"
    }
    return (Resolve-Path -LiteralPath $Candidate).Path
}

function Get-ConfiguredPython {
    if (-not (Test-Path -LiteralPath $pythonFile -PathType Leaf)) {
        throw "learning Python is not configured; run: .\learning\dev.ps1 configure -Python <python.exe>"
    }
    return Resolve-PythonExecutable ((Get-Content -LiteralPath $pythonFile -Raw).Trim())
}

function Invoke-WithLearningPath([scriptblock]$Body) {
    $savedPythonPath = [Environment]::GetEnvironmentVariable("PYTHONPATH", "Process")
    $env:PYTHONPATH = "$sourceRoot;$repositoryRoot"
    try {
        & $Body
    }
    finally {
        [Environment]::SetEnvironmentVariable("PYTHONPATH", $savedPythonPath, "Process")
    }
}

function Install-TestDependencies([string]$PythonPath) {
    $projectFile = Join-Path $learningRoot "pyproject.toml"
    $requirements = @(& $PythonPath -c @'
import pathlib
import sys
import tomllib

project = tomllib.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
requirements = project["project"]["optional-dependencies"]["test"]
if not requirements:
    raise SystemExit("sts-learning test extra is empty")
for requirement in requirements:
    print(requirement)
'@ $projectFile)
    if ($LASTEXITCODE -ne 0 -or $requirements.Count -eq 0) {
        throw "failed to resolve sts-learning test dependencies"
    }
    & $PythonPath -m pip install `
        --disable-pip-version-check `
        @requirements
    if ($LASTEXITCODE -ne 0) {
        throw "failed to install sts-learning test dependencies"
    }
}

function Invoke-Doctor([string]$PythonPath) {
    $doctor = @'
import pathlib
import sys

if sys.version_info[:2] != (3, 12):
    raise SystemExit(f"expected Python 3.12, got {sys.version.split()[0]}")

import numpy
try:
    import pytest
except ModuleNotFoundError as error:
    raise SystemExit(
        "pytest is missing; rerun .\\learning\\dev.ps1 configure "
        "-Python <python.exe>"
    ) from error
import torch
import sts_learning
import sts_learning_bridge
from sts_learning_bridge import CombatLearningBatchEnv, LearningBatchEnv

source_root = pathlib.Path(sys.argv[1]).resolve()
package_root = pathlib.Path(sts_learning.__file__).resolve()
if source_root not in package_root.parents:
    raise SystemExit(f"sts_learning came from unexpected path: {package_root}")

required_bridge_methods = (
    "from_combat_root_artifact_bytes",
        "merge_combat_root_artifact_bytes",
        "supported_potion_ids",
        "combat_root_artifact_bytes",
    "combat_group",
    "combat_root_contexts",
)
missing = [
    name
    for name in required_bridge_methods
    if not callable(getattr(LearningBatchEnv, name, None))
]
if missing:
    raise SystemExit(
        "installed bridge is stale; missing LearningBatchEnv methods: "
        + ", ".join(missing)
        + "; run: .\\learning\\dev.ps1 refresh-bridge [-Python <python.exe>]"
    )

required_combat_group_methods = ("capture_recovery_root",)
missing = [
    name
    for name in required_combat_group_methods
    if not callable(getattr(CombatLearningBatchEnv, name, None))
]
if missing:
    raise SystemExit(
        "installed bridge is stale; missing CombatLearningBatchEnv methods: "
        + ", ".join(missing)
        + "; run: .\\learning\\dev.ps1 refresh-bridge [-Python <python.exe>]"
    )

print(f"python={sys.executable}")
print(f"numpy={numpy.__version__}")
print(f"pytest={pytest.__version__}")
print(f"torch={torch.__version__}")
print(f"bridge={sts_learning_bridge.__file__}")
print(f"learning={package_root}")
'@
    Invoke-WithLearningPath {
        & $PythonPath -c $doctor $sourceRoot
        if ($LASTEXITCODE -ne 0) {
            throw "learning runtime doctor failed"
        }
    }
}

function Invoke-LearningTests([string]$PythonPath) {
    Invoke-Doctor $PythonPath
    New-Item -ItemType Directory -Path $reportRoot -Force | Out-Null
    $runId = (Get-Date -Format "yyyyMMdd-HHmmss") + "-" + [Guid]::NewGuid().ToString("N").Substring(0, 8)
    $log = Join-Path $reportRoot "learning-tests-$runId.log"
    $savedErrorPreference = $ErrorActionPreference
    Push-Location $repositoryRoot
    try {
        $ErrorActionPreference = "Continue"
        Invoke-WithLearningPath {
            & $PythonPath -m pytest $testRoot -q *> $log
            $script:testExit = $LASTEXITCODE
        }
    }
    finally {
        Pop-Location
        $ErrorActionPreference = $savedErrorPreference
    }
    if ($script:testExit -ne 0) {
        Get-Content -LiteralPath $log -Tail 80
        throw "learning tests failed; full log: $log"
    }
    $summary = Get-Content -LiteralPath $log | Where-Object { $_.Trim() } | Select-Object -Last 1
    Write-Output $summary
    Write-Output "learning_tests=passed"
    Write-Output ("log=" + $log)
}

switch ($Command) {
    "configure" {
        $pythonPath = Resolve-PythonExecutable $Python
        Install-TestDependencies $pythonPath
        Invoke-Doctor $pythonPath
        New-Item -ItemType Directory -Path $hostRoot -Force | Out-Null
        Set-Content -LiteralPath $pythonFile -Value $pythonPath -NoNewline
        Write-Output ("configured=" + $pythonPath)
    }
    "doctor" {
        Invoke-Doctor (Get-ConfiguredPython)
    }
    "test" {
        Invoke-LearningTests (Get-ConfiguredPython)
    }
    "train-combat" {
        $pythonPath = Get-ConfiguredPython
        $warmStartArguments = @()
        if ($Behavior) {
            $warmStartArguments = @("--warm-start-behavior", $Behavior)
        }
        Invoke-Doctor $pythonPath
        Invoke-WithLearningPath {
            & $pythonPath -m sts_learning.train_combat `
                --artifact $Artifact `
                --output $Output `
                --roots $Roots `
                --replicates $Replicates `
                --updates $Updates `
                --model-seed $ModelSeed `
                --behavior-seed-base $BehaviorSeedBase `
                @warmStartArguments `
                @potionArguments
            if ($LASTEXITCODE -ne 0) {
                throw "combat training command failed"
            }
        }
    }
    "evaluate-combat" {
        $pythonPath = Get-ConfiguredPython
        Invoke-Doctor $pythonPath
        Invoke-WithLearningPath {
            & $pythonPath -m sts_learning.evaluate_combat `
                --artifact $Artifact `
                --behavior $Behavior `
                --output $Output `
                --roots $Roots `
                --replicates $Replicates `
                --behavior-seed-base $BehaviorSeedBase `
                @potionArguments
            if ($LASTEXITCODE -ne 0) {
                throw "combat evaluation command failed"
            }
        }
    }
    "evaluate-combat-potions" {
        $pythonPath = Get-ConfiguredPython
        Invoke-Doctor $pythonPath
        Invoke-WithLearningPath {
            & $pythonPath -m sts_learning.evaluate_combat_potions `
                --artifact $Artifact `
                --behavior $Behavior `
                --output $Output `
                --roots $Roots `
                --replicates $Replicates `
                --behavior-seed-base $BehaviorSeedBase
            if ($LASTEXITCODE -ne 0) {
                throw "combat potion sweep command failed"
            }
        }
    }
    "evaluate-run" {
        $pythonPath = Get-ConfiguredPython
        Invoke-Doctor $pythonPath
        Invoke-WithLearningPath {
            & $pythonPath -m sts_learning.evaluate_run `
                --behavior $Behavior `
                --output $Output `
                --slots 1 `
                --attempts $Attempts `
                --max-batch-steps $MaxBatchSteps `
                --behavior-seed $BehaviorSeed `
                --held-out-seed-start $HeldOutSeedStart `
                --potion-lane $RunPotionLane
            if ($LASTEXITCODE -ne 0) {
                throw "run evaluation command failed"
            }
        }
    }
    "evaluate-run-potions" {
        $pythonPath = Get-ConfiguredPython
        Invoke-Doctor $pythonPath
        Invoke-WithLearningPath {
            & $pythonPath -m sts_learning.evaluate_run_potions `
                --behavior $Behavior `
                --output $Output `
                --attempts $Attempts `
                --max-batch-steps $MaxBatchSteps `
                --behavior-seed $BehaviorSeed `
                --held-out-seed-start $HeldOutSeedStart
            if ($LASTEXITCODE -ne 0) {
                throw "run potion comparison command failed"
            }
        }
    }
    "collect-run-roots" {
        $pythonPath = Get-ConfiguredPython
        $requiredPotionArguments = @()
        if ($RequiredPotionId -or $null -ne $RequiredPotionSlot) {
            if (-not $RequiredPotionId -or $null -eq $RequiredPotionSlot) {
                throw "collect-run-roots requires both -RequiredPotionId and -RequiredPotionSlot"
            }
            $requiredPotionArguments = @(
                "--required-potion-id", $RequiredPotionId,
                "--required-potion-slot", $RequiredPotionSlot
            )
        }
        if ($DistinctEncounters) {
            $requiredPotionArguments += @("--distinct-encounters")
        }
        Invoke-Doctor $pythonPath
        Invoke-WithLearningPath {
            & $pythonPath -m sts_learning.collect_run_combat_roots `
                --behavior $Behavior `
                --output $Output `
                --roots $Roots `
                --max-batch-steps $MaxBatchSteps `
                --wall-ms $WallMs `
                --behavior-seed $BehaviorSeed `
                --training-seed-start $TrainingSeedStart `
                --min-floor $MinFloor `
                --min-usable-potions $MinUsablePotions `
                --max-artifact-bytes $MaxArtifactBytes `
                --potion-lane $RunPotionLane `
                @requiredPotionArguments
            if ($LASTEXITCODE -ne 0) {
                throw "run combat-root collection failed"
            }
        }
    }
    "train-run" {
        $pythonPath = Get-ConfiguredPython
        $episodeRootArguments = @()
        if ($null -ne $EpisodeRootAttempts) {
            $episodeRootArguments = @(
                "--episode-root-attempts",
                $EpisodeRootAttempts
            )
        }
        Invoke-Doctor $pythonPath
        Invoke-WithLearningPath {
            & $pythonPath -m sts_learning.train_run `
                --warm-start-behavior $Behavior `
                --output $Output `
                --slots $Slots `
                --generations $Generations `
                --attempts-per-update $AttemptsPerUpdate `
                --max-batch-steps $MaxBatchSteps `
                --model-seed $ModelSeed `
                --behavior-seed $BehaviorSeed `
                --training-seed-start $TrainingSeedStart `
                --evaluation-attempts $EvaluationAttempts `
                --evaluation-max-batch-steps $EvaluationMaxBatchSteps `
                --evaluation-behavior-seed $EvaluationBehaviorSeed `
                --held-out-seed-start $HeldOutSeedStart `
                --advantage-mode $AdvantageMode `
                --decision-scope $DecisionScope `
                --sampling-mode $SamplingMode `
                @episodeRootArguments `
                --potion-lane $RunPotionLane
            if ($LASTEXITCODE -ne 0) {
                throw "run training command failed"
            }
        }
    }
    "verify" {
        $pythonPath = Get-ConfiguredPython
        Invoke-LearningTests $pythonPath
        & (Join-Path $repositoryRoot "bindings\python_learning\verify.ps1") `
            -Python $pythonPath `
            -MaturinPython $MaturinPython
        if ($LASTEXITCODE -ne 0) {
            throw "isolated bridge verification failed"
        }
    }
    "check-bridge" {
        $pythonPath = if ($Python) {
            Resolve-PythonExecutable $Python
        }
        else {
            Get-ConfiguredPython
        }
        & (Join-Path $repositoryRoot "bindings\python_learning\verify.ps1") `
            -Python $pythonPath `
            -MaturinPython $MaturinPython `
            -Fast
        if ($LASTEXITCODE -ne 0) {
            throw "isolated dev-profile bridge check failed"
        }
    }
    "refresh-bridge" {
        $pythonPath = if ($Python) {
            Resolve-PythonExecutable $Python
        }
        else {
            Get-ConfiguredPython
        }
        if ($Python) {
            Install-TestDependencies $pythonPath
        }
        & (Join-Path $repositoryRoot "bindings\python_learning\verify.ps1") `
            -Python $pythonPath `
            -MaturinPython $MaturinPython `
            -InstallTarget `
            -SkipRustTests `
            -Fast:($BridgeProfile -eq "dev")
        if ($LASTEXITCODE -ne 0) {
            throw "learning bridge refresh failed"
        }
        Invoke-Doctor $pythonPath
        if ($Python) {
            New-Item -ItemType Directory -Path $hostRoot -Force | Out-Null
            Set-Content -LiteralPath $pythonFile -Value $pythonPath -NoNewline
            Write-Output ("configured=" + $pythonPath)
        }
    }
}
