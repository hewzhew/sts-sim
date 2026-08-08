param(
    [string]$Python = "python",
    [string]$MaturinPython = "python",
    [switch]$InstallTarget,
    [switch]$Fast,
    [switch]$SkipRustTests
)

$ErrorActionPreference = "Stop"

function Resolve-PytestToolPython(
    [string]$PythonPath,
    [string]$Requirement,
    [string]$CacheRoot,
    [string]$Log
) {
    $cacheTag = (& $PythonPath -c "import platform, sys; print(f'{sys.implementation.cache_tag}-{platform.machine().lower()}')").Trim()
    if ($LASTEXITCODE -ne 0 -or -not $cacheTag) {
        throw "failed to resolve the pytest tool cache tag"
    }
    $toolRoot = Join-Path $CacheRoot $cacheTag
    $toolPython = Join-Path $toolRoot "Scripts\python.exe"
    if (-not (Test-Path -LiteralPath $toolPython -PathType Leaf)) {
        New-Item -ItemType Directory -Path $CacheRoot -Force | Out-Null
        & $PythonPath -m venv --system-site-packages $toolRoot *> $Log
        if ($LASTEXITCODE -ne 0) {
            throw "failed to create cached pytest tool environment: $toolRoot"
        }
    }
    $requirementCheck = @'
import importlib.metadata
import sys

from packaging.requirements import Requirement

requirement = Requirement(sys.argv[1])
installed = importlib.metadata.version(requirement.name)
if installed not in requirement.specifier:
    raise SystemExit(f"{requirement.name} {installed} does not satisfy {requirement.specifier}")
'@
    & $toolPython -c $requirementCheck $Requirement *> $Log
    if ($LASTEXITCODE -ne 0) {
        & $toolPython -m pip install `
            --disable-pip-version-check `
            $Requirement *> $Log
        if ($LASTEXITCODE -ne 0) {
            Get-Content -LiteralPath $Log -Tail 80
            throw "failed to prepare cached pytest tool environment"
        }
    }
    return (Resolve-Path -LiteralPath $toolPython).Path
}

$bridgeRoot = (Resolve-Path -LiteralPath $PSScriptRoot).Path
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $bridgeRoot "..\..")).Path
$runId = (Get-Date -Format "yyyyMMdd-HHmmss") + "-" + [Guid]::NewGuid().ToString("N").Substring(0, 8)
$runRoot = Join-Path $repositoryRoot ".oracle-lab\python-learning-bridge\$runId"
$wheelRoot = Join-Path $runRoot "wheels"
$venvRoot = Join-Path $runRoot "venv"
$buildLog = Join-Path $runRoot "build.log"
$rustTestLog = Join-Path $runRoot "rust-tests.log"
$smokeLog = Join-Path $runRoot "smoke.log"
$learningTestLog = Join-Path $runRoot "learning-tests.log"
$pytestToolLog = Join-Path $runRoot "pytest-tool.log"
$targetInstallLog = Join-Path $runRoot "target-install.log"
$rustTestTarget = Join-Path $repositoryRoot ".oracle-lab\target\python-learning-rust-tests"
$totalWatch = [Diagnostics.Stopwatch]::StartNew()
$phaseWatch = [Diagnostics.Stopwatch]::new()
$rustTestSeconds = 0.0
$pytestToolSeconds = 0.0
$targetInstallSeconds = 0.0

New-Item -ItemType Directory -Path $wheelRoot -Force | Out-Null

$pythonPath = (& $Python -c "import sys; print(sys.executable)").Trim()
if ($LASTEXITCODE -ne 0 -or -not $pythonPath) {
    throw "failed to resolve target Python executable"
}
$pythonRuntimeRoot = (& $pythonPath -c "import sys; print(sys.base_prefix)").Trim()
if ($LASTEXITCODE -ne 0 -or -not $pythonRuntimeRoot -or -not (Test-Path -LiteralPath $pythonRuntimeRoot -PathType Container)) {
    throw "failed to resolve target Python runtime root"
}

$learningProject = Join-Path $repositoryRoot "learning\pyproject.toml"
$testRequirement = (& $pythonPath -c @'
import pathlib
import sys
import tomllib

project = tomllib.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
requirements = project["project"]["optional-dependencies"]["test"]
if len(requirements) != 1 or not requirements[0].startswith("pytest"):
    raise SystemExit("learning test extra must contain exactly one pytest requirement")
print(requirements[0])
'@ $learningProject).Trim()
if ($LASTEXITCODE -ne 0 -or -not $testRequirement) {
    throw "failed to resolve the learning pytest requirement"
}
$phaseWatch.Restart()
$pytestToolPython = Resolve-PytestToolPython `
    -PythonPath $pythonPath `
    -Requirement $testRequirement `
    -CacheRoot (Join-Path $repositoryRoot ".oracle-lab\python-learning-pytest") `
    -Log $pytestToolLog
$phaseWatch.Stop()
$pytestToolSeconds = $phaseWatch.Elapsed.TotalSeconds

$env:PYO3_PYTHON = $pythonPath
$savedErrorPreference = $ErrorActionPreference
$ErrorActionPreference = "Continue"
$phaseWatch.Restart()
$maturinArgs = @(
    "-m", "maturin", "build",
    "--manifest-path", (Join-Path $bridgeRoot "Cargo.toml"),
    "--interpreter", $pythonPath,
    "--out", $wheelRoot
)
if (-not $Fast) {
    $maturinArgs += "--release"
}
& $MaturinPython @maturinArgs *> $buildLog
$buildExit = $LASTEXITCODE
$phaseWatch.Stop()
$buildSeconds = $phaseWatch.Elapsed.TotalSeconds
$ErrorActionPreference = $savedErrorPreference
if ($buildExit -ne 0) {
    Get-Content -LiteralPath $buildLog -Tail 80
    throw "Maturin wheel build failed; full log: $buildLog"
}

if (-not $Fast -and -not $SkipRustTests) {
    $savedPath = [Environment]::GetEnvironmentVariable("PATH", "Process")
    $env:PATH = "$pythonRuntimeRoot;$savedPath"
    try {
        $ErrorActionPreference = "Continue"
        $phaseWatch.Restart()
        & cargo test `
            --manifest-path (Join-Path $bridgeRoot "Cargo.toml") `
            --target-dir $rustTestTarget `
            --release `
            --lib *> $rustTestLog
        $rustTestExit = $LASTEXITCODE
        $phaseWatch.Stop()
        $rustTestSeconds = $phaseWatch.Elapsed.TotalSeconds
    }
    finally {
        $ErrorActionPreference = $savedErrorPreference
        [Environment]::SetEnvironmentVariable("PATH", $savedPath, "Process")
    }
    if ($rustTestExit -ne 0) {
        Get-Content -LiteralPath $rustTestLog -Tail 80
        throw "Rust learning bridge contract tests failed; full log: $rustTestLog"
    }
}

$wheel = Get-ChildItem -LiteralPath $wheelRoot -Filter "*.whl" | Select-Object -First 1
if (-not $wheel) {
    throw "Maturin completed without producing a wheel"
}

$ErrorActionPreference = "Continue"
$phaseWatch.Restart()
& $pythonPath -m venv --system-site-packages $venvRoot *>> $buildLog
$venvExit = $LASTEXITCODE
$phaseWatch.Stop()
$venvSeconds = $phaseWatch.Elapsed.TotalSeconds
$ErrorActionPreference = $savedErrorPreference
if ($venvExit -ne 0) {
    throw "failed to create isolated Python environment: $venvRoot"
}
$venvPython = Join-Path $venvRoot "Scripts\python.exe"

$ErrorActionPreference = "Continue"
$phaseWatch.Restart()
& $venvPython -m pip install --disable-pip-version-check --no-deps $wheel.FullName *> $smokeLog
$installExit = $LASTEXITCODE
$phaseWatch.Stop()
$isolatedInstallSeconds = $phaseWatch.Elapsed.TotalSeconds
$ErrorActionPreference = $savedErrorPreference
if ($installExit -ne 0) {
    Get-Content -LiteralPath $smokeLog -Tail 80
    throw "isolated wheel installation failed; full log: $smokeLog"
}

$ErrorActionPreference = "Continue"
$phaseWatch.Restart()
& $venvPython (Join-Path $bridgeRoot "tests\smoke.py") *>> $smokeLog
$smokeExit = $LASTEXITCODE
$phaseWatch.Stop()
$smokeSeconds = $phaseWatch.Elapsed.TotalSeconds
$ErrorActionPreference = $savedErrorPreference
if ($smokeExit -ne 0) {
    Get-Content -LiteralPath $smokeLog -Tail 80
    throw "Python learning bridge smoke failed; full log: $smokeLog"
}

$savedPythonPath = [Environment]::GetEnvironmentVariable("PYTHONPATH", "Process")
$learningSource = (Resolve-Path -LiteralPath (Join-Path $repositoryRoot "learning\src")).Path
$venvSite = (& $venvPython -c "import sysconfig; print(sysconfig.get_path('purelib'))").Trim()
if ($LASTEXITCODE -ne 0 -or -not $venvSite) {
    throw "failed to resolve isolated wheel site-packages"
}
$env:PYTHONPATH = "$venvSite;$learningSource;$repositoryRoot"
$ErrorActionPreference = "Continue"
$phaseWatch.Restart()
$bridgeProbe = @'
import pathlib
import sys

import sts_learning_bridge

bridge_path = pathlib.Path(sts_learning_bridge.__file__).resolve()
venv_root = pathlib.Path(sys.argv[1]).resolve()
if venv_root not in bridge_path.parents:
    raise SystemExit(f"caller imported bridge outside isolated venv: {bridge_path}")
print(f"isolated_bridge={bridge_path}")
'@
& $pytestToolPython -c $bridgeProbe $venvRoot *> $learningTestLog
$learningTestExit = $LASTEXITCODE
if ($learningTestExit -eq 0) {
    & $pytestToolPython -m pytest (Join-Path $repositoryRoot "learning\tests") -q *>> $learningTestLog
    $learningTestExit = $LASTEXITCODE
}
$phaseWatch.Stop()
$callerTestSeconds = $phaseWatch.Elapsed.TotalSeconds
$ErrorActionPreference = $savedErrorPreference
[Environment]::SetEnvironmentVariable("PYTHONPATH", $savedPythonPath, "Process")
if ($learningTestExit -ne 0) {
    Get-Content -LiteralPath $learningTestLog -Tail 80
    throw "Python learning caller tests failed; full log: $learningTestLog"
}

if ($InstallTarget) {
    $ErrorActionPreference = "Continue"
    $phaseWatch.Restart()
    & $pythonPath -m pip install `
        --disable-pip-version-check `
        --force-reinstall `
        --no-deps `
        $wheel.FullName *> $targetInstallLog
    $targetInstallExit = $LASTEXITCODE
    $phaseWatch.Stop()
    $targetInstallSeconds = $phaseWatch.Elapsed.TotalSeconds
    $ErrorActionPreference = $savedErrorPreference
    if ($targetInstallExit -ne 0) {
        Get-Content -LiteralPath $targetInstallLog -Tail 80
        throw "target wheel refresh failed; full log: $targetInstallLog"
    }
}

$summary = Get-Content -LiteralPath $smokeLog |
    Where-Object { $_ -match "^python_learning_bridge_smoke " } |
    Select-Object -Last 1
if (-not $summary) {
    throw "smoke completed without its compact summary; full log: $smokeLog"
}

Write-Output $summary
Write-Output ("python=" + $pythonPath)
Write-Output ("wheel=" + $wheel.Name)
if ($Fast) {
    Write-Output "build_profile=dev"
    Write-Output "rust_tests=deferred_to_release_verify"
}
else {
    Write-Output "build_profile=release"
    if ($SkipRustTests) {
        Write-Output "rust_tests=deferred_to_release_verify"
    }
    else {
        Write-Output "rust_tests=passed"
    }
}
Write-Output "isolated_caller_tests=passed_optional_dependencies_may_skip"
if ($InstallTarget) {
    Write-Output "target_install=refreshed"
}
$totalWatch.Stop()
Write-Output (
    "timing_seconds=" +
    "build:" + [math]::Round($buildSeconds, 3) + "," +
    "rust_tests:" + [math]::Round($rustTestSeconds, 3) + "," +
    "pytest_tool:" + [math]::Round($pytestToolSeconds, 3) + "," +
    "venv:" + [math]::Round($venvSeconds, 3) + "," +
    "isolated_install:" + [math]::Round($isolatedInstallSeconds, 3) + "," +
    "smoke:" + [math]::Round($smokeSeconds, 3) + "," +
    "caller_tests:" + [math]::Round($callerTestSeconds, 3) + "," +
    "target_install:" + [math]::Round($targetInstallSeconds, 3) + "," +
    "total:" + [math]::Round($totalWatch.Elapsed.TotalSeconds, 3)
)
Write-Output ("artifact_root=" + $runRoot)
