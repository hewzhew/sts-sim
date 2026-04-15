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
$exeItem = Get-Item -LiteralPath $exePath
$gitShort = ""
try {
    $gitShort = (git -C $repoRoot rev-parse --short HEAD 2>$null | Select-Object -First 1).Trim()
} catch {
    $gitShort = ""
}
$profileName =
    if ($profile -and $profile.PSObject.Properties.Name -contains "activated_profile") {
        [string]$profile.activated_profile
    } else {
        ""
    }

$launchMetadata = [ordered]@{
    profile_path = (Resolve-Path -LiteralPath $ProfilePath).Path
    profile_name = $profileName
    repo_root = $repoRoot
    exe_path = $exePath
    exe_last_write_utc = $exeItem.LastWriteTimeUtc.ToString("o")
    git_short = $gitShort
}

$argList = @()
if ($profile -and $profile.PSObject.Properties.Name -contains "args" -and $null -ne $profile.args) {
    foreach ($arg in $profile.args) {
        $argList += [string]$arg
    }
}

if ($DryRun) {
    $payload = [ordered]@{
        profile_path = $launchMetadata.profile_path
        profile_name = $launchMetadata.profile_name
        repo_root = $launchMetadata.repo_root
        exe_path = $launchMetadata.exe_path
        exe_last_write_utc = $launchMetadata.exe_last_write_utc
        git_short = $launchMetadata.git_short
        args = $argList
    }
    $payload | ConvertTo-Json -Depth 4
    exit 0
}

$env:LIVE_COMM_LAUNCH_PROFILE_PATH = $launchMetadata.profile_path
$env:LIVE_COMM_LAUNCH_PROFILE_NAME = $launchMetadata.profile_name
$env:LIVE_COMM_LAUNCH_EXE_PATH = $launchMetadata.exe_path
$env:LIVE_COMM_LAUNCH_EXE_MTIME_UTC = $launchMetadata.exe_last_write_utc
$env:LIVE_COMM_LAUNCH_GIT_SHORT = $launchMetadata.git_short

Write-Host ("[live_comm launcher] profile={0} exe={1} exe_mtime_utc={2} git={3}" -f `
    ($(if ([string]::IsNullOrWhiteSpace($profileName)) { "<none>" } else { $profileName })),
    $exePath,
    $exeItem.LastWriteTimeUtc.ToString("o"),
    ($(if ([string]::IsNullOrWhiteSpace($gitShort)) { "<unknown>" } else { $gitShort })))

& $exePath @argList
exit $LASTEXITCODE
