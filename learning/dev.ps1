param(
    [Parameter(Mandatory = $true, Position = 0)]
    [ValidateSet("configure", "doctor", "test", "verify", "check-bridge", "refresh-bridge")]
    [string]$Command,
    [string]$Python,
    [string]$MaturinPython = "python"
)

$ErrorActionPreference = "Stop"

$learningRoot = (Resolve-Path -LiteralPath $PSScriptRoot).Path
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $learningRoot "..")).Path
$sourceRoot = Join-Path $learningRoot "src"
$testRoot = Join-Path $learningRoot "tests"
$hostRoot = Join-Path $repositoryRoot ".oracle-lab\hosts"
$pythonFile = Join-Path $hostRoot "learning-python.txt"
$reportRoot = Join-Path $repositoryRoot ".oracle-lab\reports"

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

function Invoke-Doctor([string]$PythonPath) {
    $doctor = @'
import pathlib
import sys

if sys.version_info[:2] != (3, 12):
    raise SystemExit(f"expected Python 3.12, got {sys.version.split()[0]}")

import numpy
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
            & $PythonPath -m unittest discover -s $testRoot -v *> $log
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
    $ran = Get-Content -LiteralPath $log | Where-Object { $_ -match "^Ran [0-9]+ tests? in " } | Select-Object -Last 1
    $result = Get-Content -LiteralPath $log | Where-Object { $_ -match "^OK$" } | Select-Object -Last 1
    if (-not $ran -or -not $result) {
        throw "learning tests completed without an unskipped OK summary; full log: $log"
    }
    Write-Output $ran
    Write-Output "learning_tests=passed"
    Write-Output ("log=" + $log)
}

switch ($Command) {
    "configure" {
        $pythonPath = Resolve-PythonExecutable $Python
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
        & (Join-Path $repositoryRoot "bindings\python_learning\verify.ps1") `
            -Python $pythonPath `
            -MaturinPython $MaturinPython `
            -InstallTarget `
            -SkipRustTests
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
