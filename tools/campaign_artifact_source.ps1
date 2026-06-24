function Set-CampaignArtifactResolverDriver {
    param(
        [string] $DriverExe
    )

    if (-not $DriverExe) {
        throw "Internal error: campaign artifact resolver requires DriverExe."
    }
    $script:CampaignArtifactResolverDriverExe = $DriverExe
}

function New-CampaignRunConfigObject {
    param(
        [object] $Seed = $null,
        [object] $Ascension = $null,
        [string] $Class = "",
        [string] $Mode = ""
    )

    return [pscustomobject]@{
        Seed = if ($Seed -ne $null) { [long] $Seed } else { $null }
        Ascension = if ($Ascension -ne $null) { [int] $Ascension } else { $null }
        Class = if ($Class) { ([string] $Class).ToLowerInvariant() } else { $null }
        Mode = if ($Mode) { ([string] $Mode).ToLowerInvariant() } else { $null }
    }
}

function Convert-CampaignDriverRunConfig {
    param(
        [object] $RunConfig
    )

    if (-not $RunConfig) {
        return New-CampaignRunConfigObject
    }
    return New-CampaignRunConfigObject `
        -Seed $RunConfig.seed `
        -Ascension $RunConfig.ascension `
        -Class ([string] $RunConfig.class) `
        -Mode ([string] $RunConfig.mode)
}

function Convert-CampaignDriverSourceInfo {
    param(
        [object] $Info
    )

    if (-not $Info -or -not $Info.artifact) {
        throw "Internal error: empty artifact source-info response."
    }
    return [pscustomobject]@{
        Artifact = Convert-CampaignDriverArtifactRef -Artifact $Info.artifact
        RunConfig = Convert-CampaignDriverRunConfig -RunConfig $Info.run_config
        Progress = [pscustomobject]@{
            RoundsCompleted = if ($Info.progress -and $Info.progress.rounds_completed -ne $null) { [int] $Info.progress.rounds_completed } else { $null }
            StopReason = if ($Info.progress -and $Info.progress.stop_reason) { [string] $Info.progress.stop_reason } else { $null }
        }
    }
}

function Get-CampaignSourceArtifactInfoViaDriver {
    param(
        [string] $Selector
    )

    if (-not $script:CampaignArtifactResolverDriverExe) {
        throw "Internal error: Rust campaign artifact resolver was not configured."
    }

    $Args = @(
        "artifact",
        "source-info",
        "$Selector",
        "--campaign-dir", "$script:CampaignDir",
        "--json"
    )
    $Json = & $script:CampaignArtifactResolverDriverExe @Args
    if ($LASTEXITCODE -ne 0) {
        throw "Rust campaign artifact source-info failed with exit code $LASTEXITCODE for selector '$Selector'."
    }
    try {
        return Convert-CampaignDriverSourceInfo -Info ($Json | ConvertFrom-Json)
    } catch {
        throw "Rust campaign artifact source-info returned invalid JSON for selector '$Selector': $_"
    }
}

function Get-CampaignLegacyLatestRunConfig {
    param(
        [object] $Artifact
    )

    $Mode = Read-LegacyLatestCampaignMode
    $Seed = $null
    $Ascension = $null
    $Class = $null

    if ($Artifact -and $Artifact.CheckpointPath -and (Test-Path -LiteralPath $Artifact.CheckpointPath)) {
        try {
            $Checkpoint = Read-CampaignJsonArtifact -Path $Artifact.CheckpointPath
            if ($Checkpoint.sessions -and $Checkpoint.sessions.Count -gt 0) {
                $RunState = $Checkpoint.sessions[0].session.run_state
                if ($RunState) {
                    if ($RunState.seed -ne $null) { $Seed = [long] $RunState.seed }
                    if ($RunState.ascension_level -ne $null) { $Ascension = [int] $RunState.ascension_level }
                    if ($RunState.player_class) { $Class = ([string] $RunState.player_class).ToLowerInvariant() }
                }
            }
        } catch {
            # Legacy sidecars are best-effort archaeology.
        }
    }

    return New-CampaignRunConfigObject -Seed $Seed -Ascension $Ascension -Class $Class -Mode $Mode
}

function Get-CampaignSourceArtifactInfo {
    param(
        [string] $Selector = "",
        [bool] $UseScratchLatest
    )

    Assert-CampaignArtifactPathsInitialized
    $ResolvedSelector = if ($Selector) { $Selector.Trim() } elseif ($UseScratchLatest) { "scratch:latest" } else { "latest" }

    if ($ResolvedSelector -eq "legacy-latest") {
        $Artifact = New-CampaignLegacyLatestArtifact
    return [pscustomobject]@{
        Artifact = $Artifact
        RunConfig = Get-CampaignLegacyLatestRunConfig -Artifact $Artifact
        Progress = [pscustomobject]@{
            RoundsCompleted = $null
            StopReason = $null
        }
    }
}

    return Get-CampaignSourceArtifactInfoViaDriver -Selector $ResolvedSelector
}
