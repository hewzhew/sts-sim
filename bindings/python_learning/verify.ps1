param(
    [string]$Python = "python",
    [string]$MaturinPython = "python",
    [switch]$InstallTarget
)

$ErrorActionPreference = "Stop"

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
$targetInstallLog = Join-Path $runRoot "target-install.log"

New-Item -ItemType Directory -Path $wheelRoot -Force | Out-Null

$pythonPath = (& $Python -c "import sys; print(sys.executable)").Trim()
if ($LASTEXITCODE -ne 0 -or -not $pythonPath) {
    throw "failed to resolve target Python executable"
}
$pythonRuntimeRoot = (& $pythonPath -c "import sys; print(sys.base_prefix)").Trim()
if ($LASTEXITCODE -ne 0 -or -not $pythonRuntimeRoot -or -not (Test-Path -LiteralPath $pythonRuntimeRoot -PathType Container)) {
    throw "failed to resolve target Python runtime root"
}

$env:PYO3_PYTHON = $pythonPath
$savedErrorPreference = $ErrorActionPreference
$ErrorActionPreference = "Continue"
& $MaturinPython -m maturin build `
    --manifest-path (Join-Path $bridgeRoot "Cargo.toml") `
    --release `
    --interpreter $pythonPath `
    --out $wheelRoot *> $buildLog
$buildExit = $LASTEXITCODE
$ErrorActionPreference = $savedErrorPreference
if ($buildExit -ne 0) {
    Get-Content -LiteralPath $buildLog -Tail 80
    throw "Maturin wheel build failed; full log: $buildLog"
}

$savedPath = [Environment]::GetEnvironmentVariable("PATH", "Process")
$env:PATH = "$pythonRuntimeRoot;$savedPath"
try {
    $ErrorActionPreference = "Continue"
    & cargo test `
        --manifest-path (Join-Path $bridgeRoot "Cargo.toml") `
        --release `
        --lib *> $rustTestLog
    $rustTestExit = $LASTEXITCODE
}
finally {
    $ErrorActionPreference = $savedErrorPreference
    [Environment]::SetEnvironmentVariable("PATH", $savedPath, "Process")
}
if ($rustTestExit -ne 0) {
    Get-Content -LiteralPath $rustTestLog -Tail 80
    throw "Rust learning bridge contract tests failed; full log: $rustTestLog"
}

$wheel = Get-ChildItem -LiteralPath $wheelRoot -Filter "*.whl" | Select-Object -First 1
if (-not $wheel) {
    throw "Maturin completed without producing a wheel"
}

$ErrorActionPreference = "Continue"
& $pythonPath -m venv --system-site-packages $venvRoot *>> $buildLog
$venvExit = $LASTEXITCODE
$ErrorActionPreference = $savedErrorPreference
if ($venvExit -ne 0) {
    throw "failed to create isolated Python environment: $venvRoot"
}
$venvPython = Join-Path $venvRoot "Scripts\python.exe"

$ErrorActionPreference = "Continue"
& $venvPython -m pip install --disable-pip-version-check --no-deps $wheel.FullName *> $smokeLog
$installExit = $LASTEXITCODE
$ErrorActionPreference = $savedErrorPreference
if ($installExit -ne 0) {
    Get-Content -LiteralPath $smokeLog -Tail 80
    throw "wheel installation failed; full log: $smokeLog"
}

$ErrorActionPreference = "Continue"
& $venvPython (Join-Path $bridgeRoot "tests\smoke.py") *>> $smokeLog
$smokeExit = $LASTEXITCODE
$ErrorActionPreference = $savedErrorPreference
if ($smokeExit -ne 0) {
    Get-Content -LiteralPath $smokeLog -Tail 80
    throw "Python learning bridge smoke failed; full log: $smokeLog"
}

$savedPythonPath = [Environment]::GetEnvironmentVariable("PYTHONPATH", "Process")
$env:PYTHONPATH = (Resolve-Path -LiteralPath (Join-Path $repositoryRoot "learning\src")).Path
$ErrorActionPreference = "Continue"
& $venvPython -m unittest discover `
    -s (Join-Path $repositoryRoot "learning\tests") `
    -v *> $learningTestLog
$learningTestExit = $LASTEXITCODE
$ErrorActionPreference = $savedErrorPreference
[Environment]::SetEnvironmentVariable("PYTHONPATH", $savedPythonPath, "Process")
if ($learningTestExit -ne 0) {
    Get-Content -LiteralPath $learningTestLog -Tail 80
    throw "Python learning caller tests failed; full log: $learningTestLog"
}

if ($InstallTarget) {
    $ErrorActionPreference = "Continue"
    & $pythonPath -m pip install `
        --disable-pip-version-check `
        --force-reinstall `
        --no-deps `
        $wheel.FullName *> $targetInstallLog
    $targetInstallExit = $LASTEXITCODE
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
Write-Output "rust_tests=passed"
Write-Output "isolated_caller_tests=passed_optional_dependencies_may_skip"
if ($InstallTarget) {
    Write-Output "target_install=refreshed"
}
Write-Output ("artifact_root=" + $runRoot)
