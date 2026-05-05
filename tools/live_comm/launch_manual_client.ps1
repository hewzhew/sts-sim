param()

$ErrorActionPreference = "Stop"

$scriptPath = Join-Path $PSScriptRoot "manual_client.py"
if (-not (Test-Path -LiteralPath $scriptPath)) {
    throw "manual client script not found: $scriptPath"
}

$python = $null
foreach ($candidate in @("py", "python")) {
    try {
        $cmd = Get-Command $candidate -ErrorAction Stop
        $python = $cmd.Source
        break
    } catch {
    }
}

if ($null -eq $python) {
    throw "Could not find a Python interpreter (`py` or `python`) on PATH."
}

$args = @()
if ((Split-Path -Leaf $python) -ieq "py.exe" -or (Split-Path -Leaf $python) -ieq "py") {
    $args += "-3"
}
$args += $scriptPath

Write-Host "[manual scenario launcher] python=$python script=$scriptPath"
& $python @args
exit $LASTEXITCODE
