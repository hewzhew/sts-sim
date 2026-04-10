param(
    [string]$ProfilePath = $(Join-Path $PSScriptRoot "profile.json"),
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

function Resolve-PlayExePath {
    param(
        [object]$Profile,
        [string]$RepoRoot
    )

    $candidates = @()
    if ($Profile -and $Profile.PSObject.Properties.Name -contains "exe_path") {
        $configured = [string]$Profile.exe_path
        if (-not [string]::IsNullOrWhiteSpace($configured)) {
            $candidates += $configured
        }
    }

    $candidates += (Join-Path $RepoRoot "target\release\play.exe")
    $candidates += (Join-Path $RepoRoot "target\debug\play.exe")

    foreach ($candidate in $candidates) {
        if (-not [string]::IsNullOrWhiteSpace($candidate) -and (Test-Path $candidate)) {
            return (Resolve-Path $candidate).Path
        }
    }

    throw "Could not find play.exe. Checked: $($candidates -join ', ')"
}

if (-not (Test-Path $ProfilePath)) {
    throw "live_comm profile not found: $ProfilePath"
}

$profileText = Get-Content -Raw -LiteralPath $ProfilePath
$profile = $profileText | ConvertFrom-Json
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$exePath = Resolve-PlayExePath -Profile $profile -RepoRoot $repoRoot

$argList = @()
if ($profile -and $profile.PSObject.Properties.Name -contains "args" -and $null -ne $profile.args) {
    foreach ($arg in $profile.args) {
        $argList += [string]$arg
    }
}

if ($DryRun) {
    $payload = [ordered]@{
        profile_path = (Resolve-Path -LiteralPath $ProfilePath).Path
        repo_root = $repoRoot
        exe_path = $exePath
        args = $argList
    }
    $payload | ConvertTo-Json -Depth 4
    exit 0
}

& $exePath @argList
exit $LASTEXITCODE
