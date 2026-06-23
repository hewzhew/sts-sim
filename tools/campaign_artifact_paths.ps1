function New-CampaignPathContext {
    param(
        [string] $RepoRoot
    )

    $CampaignDir = Join-Path $RepoRoot "tools\artifacts\campaigns"
    return [pscustomobject]@{
        RepoRoot = $RepoRoot
        CampaignDir = $CampaignDir
        ScratchCampaignDir = Join-Path $CampaignDir "scratch"
    }
}

function Initialize-CampaignArtifactPaths {
    param(
        [object] $PathContext
    )

    if (-not $PathContext -or -not $PathContext.CampaignDir -or -not $PathContext.ScratchCampaignDir) {
        throw "Internal error: campaign artifact path initialization requires CampaignPathContext."
    }

    $script:CampaignPathContext = $PathContext
    $script:CampaignDir = $PathContext.CampaignDir
    $script:ScratchCampaignDir = $PathContext.ScratchCampaignDir
    $LegacySidecarPaths = New-CampaignLegacyLatestSidecarPathContext -CampaignDir $script:CampaignDir
    $script:LegacyLatestModePath = $LegacySidecarPaths.ModePath
    $script:LegacyLatestCommandPath = $LegacySidecarPaths.CommandPath
    $script:LegacyLatestManifestPath = $LegacySidecarPaths.ManifestPath
    $script:LegacyLatestLogPath = $LegacySidecarPaths.LogPath
    $script:LegacyLatestCampaignPath = $LegacySidecarPaths.CampaignPath
    $script:LegacyLatestCheckpointPath = $LegacySidecarPaths.CheckpointPath
}

function Assert-CampaignArtifactPathsInitialized {
    if (-not $script:CampaignPathContext) {
        throw "Internal error: campaign artifact paths are not initialized."
    }
}

function New-CampaignLegacyLatestSidecarPathContext {
    param(
        [string] $CampaignDir
    )

    return [pscustomobject]@{
        ModePath = Join-Path $CampaignDir "latest.mode.txt"
        CommandPath = Join-Path $CampaignDir "latest.command.txt"
        ManifestPath = Join-Path $CampaignDir "latest.manifest.json"
        LogPath = Join-Path $CampaignDir "latest.log"
        CampaignPath = Join-Path $CampaignDir "latest.campaign.json"
        CheckpointPath = Join-Path $CampaignDir "latest.checkpoint.json"
    }
}

function Get-CampaignRunsDir {
    Assert-CampaignArtifactPathsInitialized
    return (Join-Path $CampaignDir "runs")
}

function Get-CampaignLatestPointerPath {
    Assert-CampaignArtifactPathsInitialized
    return (Join-Path $CampaignDir "latest.json")
}

function Get-CampaignScratchLatestPointerPath {
    Assert-CampaignArtifactPathsInitialized
    return (Join-Path $ScratchCampaignDir "latest.json")
}
