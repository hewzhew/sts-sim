param(
    [string]$ProfilePath = $(Join-Path $PSScriptRoot "profile.json"),
    [switch]$DryRun,
    [switch]$SkipFreshBuild,
    [switch]$AllowAutoBuildIfStale
)

$ErrorActionPreference = "Stop"

function To-UtcIsoString {
    param(
        [datetime]$Value
    )

    return $Value.ToUniversalTime().ToString("o")
}

function Get-RepoHeadShort {
    param(
        [string]$RepoRoot
    )

    try {
        return (git -C $RepoRoot rev-parse --short HEAD 2>$null | Select-Object -First 1).Trim()
    } catch {
        return ""
    }
}

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

function Get-LiveCommSourceInputs {
    param(
        [string]$RepoRoot
    )

    $paths = New-Object System.Collections.Generic.List[object]
    $explicitFiles = @(
        (Join-Path $RepoRoot "Cargo.toml"),
        (Join-Path $RepoRoot "Cargo.lock"),
        (Join-Path $RepoRoot "build.rs"),
        (Join-Path $RepoRoot "tools\\compiled_protocol_schema.json")
    )

    foreach ($file in $explicitFiles) {
        if (Test-Path -LiteralPath $file) {
            $item = Get-Item -LiteralPath $file
            $paths.Add([pscustomobject]@{
                path = $item.FullName
                last_write_utc = $item.LastWriteTimeUtc
            })
        }
    }

    $srcRoot = Join-Path $RepoRoot "src"
    if (Test-Path -LiteralPath $srcRoot) {
        Get-ChildItem -LiteralPath $srcRoot -Recurse -File -Filter *.rs | ForEach-Object {
            $paths.Add([pscustomobject]@{
                path = $_.FullName
                last_write_utc = $_.LastWriteTimeUtc
            })
        }
    }

    if ($paths.Count -eq 0) {
        throw "Could not determine live_comm source inputs under $RepoRoot"
    }

    return $paths
}

function Get-BinaryFreshnessStatus {
    param(
        [string]$RepoRoot,
        [string]$ExePath
    )

    $exeItem = Get-Item -LiteralPath $ExePath
    $latestInput = Get-LiveCommSourceInputs -RepoRoot $RepoRoot |
        Sort-Object -Property last_write_utc -Descending |
        Select-Object -First 1

    $exeFresh = $exeItem.LastWriteTimeUtc -ge $latestInput.last_write_utc

    return [pscustomobject]@{
        exe_path = $exeItem.FullName
        exe_last_write_utc = $exeItem.LastWriteTimeUtc
        latest_input_path = [string]$latestInput.path
        latest_input_write_utc = [datetime]$latestInput.last_write_utc
        binary_is_fresh = [bool]$exeFresh
    }
}

function Get-StaleBinaryGuidance {
    param(
        [string]$RepoRoot,
        [string]$ExePath
    )

    $normalizedExePath = [System.IO.Path]::GetFullPath($ExePath)
    $releaseExe = [System.IO.Path]::GetFullPath((Join-Path $RepoRoot "target\\release\\play.exe"))
    $debugExe = [System.IO.Path]::GetFullPath((Join-Path $RepoRoot "target\\debug\\play.exe"))

    if ($normalizedExePath -eq $releaseExe) {
        return [pscustomobject]@{
            binary_kind = "release"
            auto_build_supported = $true
            build_command = "cargo build --release --bin play"
        }
    }

    if ($normalizedExePath -eq $debugExe) {
        return [pscustomobject]@{
            binary_kind = "debug"
            auto_build_supported = $true
            build_command = "cargo build --bin play"
        }
    }

    return [pscustomobject]@{
        binary_kind = "custom"
        auto_build_supported = $false
        build_command = ""
    }
}

function Resolve-FreshnessGate {
    param(
        [string]$RepoRoot,
        [string]$ExePath,
        [switch]$DryRun,
        [switch]$SkipFreshBuild,
        [switch]$AllowAutoBuildIfStale
    )

    $status = Get-BinaryFreshnessStatus -RepoRoot $RepoRoot -ExePath $ExePath
    $guidance = Get-StaleBinaryGuidance -RepoRoot $RepoRoot -ExePath $ExePath
    if ($SkipFreshBuild -or $status.binary_is_fresh) {
        return [pscustomobject]@{
            exe_path = $status.exe_path
            exe_last_write_utc = $status.exe_last_write_utc
            latest_input_path = $status.latest_input_path
            latest_input_write_utc = $status.latest_input_write_utc
            binary_is_fresh = $status.binary_is_fresh
            binary_kind = $guidance.binary_kind
            build_command = $guidance.build_command
            launch_ready = $true
            rebuilt = $false
            stale_policy = $(if ($SkipFreshBuild) { "skip_check" } else { "require_fresh" })
        }
    }

    if ($DryRun) {
        return [pscustomobject]@{
            exe_path = $status.exe_path
            exe_last_write_utc = $status.exe_last_write_utc
            latest_input_path = $status.latest_input_path
            latest_input_write_utc = $status.latest_input_write_utc
            binary_is_fresh = $status.binary_is_fresh
            binary_kind = $guidance.binary_kind
            build_command = $guidance.build_command
            launch_ready = [bool]($AllowAutoBuildIfStale -or $status.binary_is_fresh)
            rebuilt = $false
            stale_policy = $(if ($AllowAutoBuildIfStale) { "auto_build" } else { "fail_fast" })
        }
    }

    if (-not $AllowAutoBuildIfStale) {
        $message = if ($guidance.auto_build_supported -and -not [string]::IsNullOrWhiteSpace($guidance.build_command)) {
            "[live_comm launcher] stale $($guidance.binary_kind) binary detected; refusing to rebuild inside CommunicationMod handshake. Run '$($guidance.build_command)' in $RepoRoot, then relaunch the game."
        } else {
            "[live_comm launcher] stale binary detected at $ExePath; refusing to rebuild inside CommunicationMod handshake. Rebuild or replace the configured exe, then relaunch the game."
        }
        Write-Host $message
        throw $message
    }

    if ($guidance.binary_kind -eq "release") {
        Write-Host ("[live_comm launcher] stale release binary detected; rebuilding {0}" -f $ExePath)
        Push-Location $RepoRoot
        try {
            & cargo build --release --bin play
            if ($LASTEXITCODE -ne 0) {
                throw "cargo build --release --bin play failed with exit code $LASTEXITCODE"
            }
        } finally {
            Pop-Location
        }
    } elseif ($guidance.binary_kind -eq "debug") {
        Write-Host ("[live_comm launcher] stale debug binary detected; rebuilding {0}" -f $ExePath)
        Push-Location $RepoRoot
        try {
            & cargo build --bin play
            if ($LASTEXITCODE -ne 0) {
                throw "cargo build --bin play failed with exit code $LASTEXITCODE"
            }
        } finally {
            Pop-Location
        }
    } else {
        throw "Configured exe path is stale and cannot be auto-built outside repo target: $ExePath"
    }

    $refreshed = Get-BinaryFreshnessStatus -RepoRoot $RepoRoot -ExePath $ExePath
    if (-not $refreshed.binary_is_fresh) {
        throw "Binary is still stale after rebuild: $ExePath"
    }
    return [pscustomobject]@{
        exe_path = $refreshed.exe_path
        exe_last_write_utc = $refreshed.exe_last_write_utc
        latest_input_path = $refreshed.latest_input_path
        latest_input_write_utc = $refreshed.latest_input_write_utc
        binary_is_fresh = $refreshed.binary_is_fresh
        binary_kind = $guidance.binary_kind
        build_command = $guidance.build_command
        launch_ready = $true
        rebuilt = $true
        stale_policy = "auto_build"
    }
}

if (-not (Test-Path $ProfilePath)) {
    throw "live_comm profile not found: $ProfilePath"
}

$profileText = Get-Content -Raw -LiteralPath $ProfilePath
$profile = $profileText | ConvertFrom-Json
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$exePath = Resolve-PlayExePath -Profile $profile -RepoRoot $repoRoot
$gitShort = Get-RepoHeadShort -RepoRoot $repoRoot
$freshness = Resolve-FreshnessGate `
    -RepoRoot $repoRoot `
    -ExePath $exePath `
    -DryRun:$DryRun `
    -SkipFreshBuild:$SkipFreshBuild `
    -AllowAutoBuildIfStale:$AllowAutoBuildIfStale
$exeItem = Get-Item -LiteralPath $exePath
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
    repo_head_short = $gitShort
    latest_input_path = $freshness.latest_input_path
    latest_input_write_utc = (To-UtcIsoString -Value $freshness.latest_input_write_utc)
    binary_is_fresh = $freshness.binary_is_fresh
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
        repo_head_short = $launchMetadata.repo_head_short
        latest_input_path = $launchMetadata.latest_input_path
        latest_input_write_utc = $launchMetadata.latest_input_write_utc
        binary_is_fresh = $launchMetadata.binary_is_fresh
        binary_kind = $freshness.binary_kind
        stale_policy = $freshness.stale_policy
        launch_ready = $freshness.launch_ready
        rebuilt = $freshness.rebuilt
        suggested_build_cmd = $freshness.build_command
        fresh_build_skipped = [bool]$SkipFreshBuild
        auto_build_if_stale = [bool]$AllowAutoBuildIfStale
        args = $argList
    }
    $payload | ConvertTo-Json -Depth 4
    exit 0
}

$env:LIVE_COMM_LAUNCH_PROFILE_PATH = $launchMetadata.profile_path
$env:LIVE_COMM_LAUNCH_PROFILE_NAME = $launchMetadata.profile_name
$env:LIVE_COMM_LAUNCH_EXE_PATH = $launchMetadata.exe_path
$env:LIVE_COMM_LAUNCH_EXE_MTIME_UTC = $launchMetadata.exe_last_write_utc
$env:LIVE_COMM_LAUNCH_REPO_HEAD_SHORT = $launchMetadata.repo_head_short
$env:LIVE_COMM_LAUNCH_SOURCE_LATEST_PATH = $launchMetadata.latest_input_path
$env:LIVE_COMM_LAUNCH_SOURCE_LATEST_MTIME_UTC = $launchMetadata.latest_input_write_utc
$env:LIVE_COMM_LAUNCH_BINARY_IS_FRESH = $launchMetadata.binary_is_fresh.ToString().ToLowerInvariant()

Write-Host ("[live_comm launcher] profile={0} exe={1} exe_mtime_utc={2} repo_head={3} binary_is_fresh={4} stale_policy={5} rebuilt={6} latest_input={7}" -f `
    ($(if ([string]::IsNullOrWhiteSpace($profileName)) { "<none>" } else { $profileName })),
    $exePath,
    $exeItem.LastWriteTimeUtc.ToString("o"),
    ($(if ([string]::IsNullOrWhiteSpace($gitShort)) { "<unknown>" } else { $gitShort })),
    $launchMetadata.binary_is_fresh.ToString().ToLowerInvariant(),
    $freshness.stale_policy,
    $freshness.rebuilt.ToString().ToLowerInvariant(),
    $launchMetadata.latest_input_path)

& $exePath @argList
exit $LASTEXITCODE
