$script:CampaignPathContext = $null
$script:CampaignDir = ""
$script:ScratchCampaignDir = ""
$script:LegacyLatestModePath = ""
$script:LegacyLatestCommandPath = ""
$script:LegacyLatestManifestPath = ""
$script:LegacyLatestLogPath = ""
$script:LegacyLatestCampaignPath = ""
$script:LegacyLatestCheckpointPath = ""

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

function Convert-ToCampaignArtifactSlug {
    param(
        [string] $Value
    )

    $Slug = ($Value.Trim() -replace '[^A-Za-z0-9_.-]+', '-').Trim('-')
    if (-not $Slug) {
        return "scratch"
    }
    return $Slug
}

function New-CampaignArtifactId {
    param(
        [string] $BaseLabel
    )

    $Stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $Slug = Convert-ToCampaignArtifactSlug $BaseLabel
    $Suffix = [guid]::NewGuid().ToString("N").Substring(0, 8)
    return "$Slug-$Stamp-$Suffix"
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

function New-CampaignRunArtifact {
    param(
        [string] $BaseLabel,
        [string] $ArtifactId = ""
    )

    $Id = if ($ArtifactId) { Convert-ToCampaignArtifactSlug $ArtifactId } else { New-CampaignArtifactId -BaseLabel $BaseLabel }
    $Dir = Join-Path (Get-CampaignRunsDir) $Id
    return [pscustomobject]@{
        Kind = "run"
        Id = $Id
        Label = "run:$Id"
        Dir = $Dir
        ReportPath = Join-Path $Dir "campaign.json"
        CheckpointPath = Join-Path $Dir "checkpoint.json"
        ManifestPath = Join-Path $Dir "manifest.json"
        LogPath = Join-Path $Dir "log.txt"
        CommandPath = Join-Path $Dir "command.txt"
    }
}

function New-CampaignScratchArtifactRef {
    param(
        [string] $ArtifactId
    )

    $Id = Convert-ToCampaignArtifactSlug $ArtifactId
    return [pscustomobject]@{
        Kind = "scratch"
        Id = $Id
        Label = "scratch:$Id"
        Dir = $ScratchCampaignDir
        ReportPath = Join-Path $ScratchCampaignDir "$Id.campaign.json"
        CheckpointPath = Join-Path $ScratchCampaignDir "$Id.checkpoint.json"
        ManifestPath = Join-Path $ScratchCampaignDir "$Id.manifest.json"
        LogPath = Join-Path $ScratchCampaignDir "$Id.log"
        CommandPath = Join-Path $ScratchCampaignDir "$Id.command.txt"
    }
}

function New-CampaignScratchArtifact {
    param(
        [string] $BaseLabel
    )

    $Id = New-CampaignArtifactId -BaseLabel $BaseLabel
    return New-CampaignScratchArtifactRef -ArtifactId $Id
}

function Get-CampaignOutputBaseLabel {
    param(
        [string] $RequestKind = "",
        [string] $RunLabel,
        [long] $Seed
    )

    if ($RunLabel) {
        return $RunLabel
    }
    if ($RequestKind -eq "continue_coverage_gaps") {
        return "coverage-gap-seed$Seed"
    }
    if ($RequestKind -eq "continue_run") {
        return "continue-seed$Seed"
    }
    return "campaign-seed$Seed"
}

function Resolve-CampaignOutputArtifactContext {
    param(
        [object] $Request,
        [bool] $Scratch,
        [string] $RunLabel,
        [long] $Seed
    )

    if (-not $Request) {
        throw "Internal error: output artifact context requires CampaignEntryRequestV1."
    }
    $RequestKind = $Request.Kind
    $WritesCampaignOutput = ($Request.OutputIntent -eq "campaign_output")
    $RunOutputArtifact = $null
    $ScratchLabel = ""
    $RunOutputCampaignPath = ""
    $RunOutputCheckpointPath = ""
    $RunCommandPath = ""
    $RunManifestPath = ""
    $RunLogPath = ""

    if ($WritesCampaignOutput) {
        $OutputBaseLabel = Get-CampaignOutputBaseLabel `
            -RequestKind $RequestKind `
            -RunLabel $RunLabel `
            -Seed $Seed
        $RunOutputArtifact = if ($Scratch) {
            New-CampaignScratchArtifact -BaseLabel $OutputBaseLabel
        } else {
            New-CampaignRunArtifact -BaseLabel $OutputBaseLabel
        }
        $ScratchLabel = if ($Scratch) { $RunOutputArtifact.Id } else { "" }
        $RunOutputCampaignPath = $RunOutputArtifact.ReportPath
        $RunOutputCheckpointPath = $RunOutputArtifact.CheckpointPath
        $RunCommandPath = $RunOutputArtifact.CommandPath
        $RunManifestPath = $RunOutputArtifact.ManifestPath
        $RunLogPath = $RunOutputArtifact.LogPath
    }

    return [pscustomobject]@{
        WritesCampaignOutput = [bool] $WritesCampaignOutput
        Artifact = $RunOutputArtifact
        ScratchLabel = $ScratchLabel
        CampaignPath = $RunOutputCampaignPath
        CheckpointPath = $RunOutputCheckpointPath
        CommandPath = $RunCommandPath
        ManifestPath = $RunManifestPath
        LogPath = $RunLogPath
    }
}

function Ensure-CampaignOutputArtifactDirectory {
    param(
        [object] $OutputContext,
        [bool] $DryRun
    )

    if (-not $OutputContext.WritesCampaignOutput) {
        return
    }
    if ($DryRun) {
        return
    }
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $OutputContext.CampaignPath) | Out-Null
}

function New-CampaignLegacyLatestArtifact {
    Assert-CampaignArtifactPathsInitialized
    return [pscustomobject]@{
        Kind = "legacy_latest"
        Id = "legacy-latest"
        Label = "legacy-latest"
        Dir = $CampaignDir
        ReportPath = $LegacyLatestCampaignPath
        CheckpointPath = $LegacyLatestCheckpointPath
        ManifestPath = $LegacyLatestManifestPath
        LogPath = $LegacyLatestLogPath
        CommandPath = $LegacyLatestCommandPath
    }
}

function Read-CampaignLatestPointer {
    $PointerPath = Get-CampaignLatestPointerPath
    if (-not (Test-Path -LiteralPath $PointerPath)) {
        return $null
    }
    try {
        $Pointer = Get-Content -LiteralPath $PointerPath -Raw | ConvertFrom-Json
        if ($Pointer.schema_name -ne "CampaignLatestPointerV1") {
            return $null
        }
        if (-not $Pointer.artifact_id) {
            return $null
        }
        return $Pointer
    } catch {
        return $null
    }
}

function Write-CampaignLatestPointer {
    param(
        [object] $Artifact
    )

    if (-not $Artifact -or $Artifact.Kind -ne "run") {
        return
    }
    $PointerPath = Get-CampaignLatestPointerPath
    $Pointer = [ordered]@{
        schema_name = "CampaignLatestPointerV1"
        schema_version = 1
        updated_at = (Get-Date).ToString("o")
        artifact_id = $Artifact.Id
        report = $Artifact.ReportPath
        checkpoint = $Artifact.CheckpointPath
        manifest = $Artifact.ManifestPath
        command = $Artifact.CommandPath
        log = $Artifact.LogPath
    }
    $Pointer | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $PointerPath
}

function Read-CampaignScratchLatestPointer {
    $PointerPath = Get-CampaignScratchLatestPointerPath
    if (-not (Test-Path -LiteralPath $PointerPath)) {
        return $null
    }
    try {
        $Pointer = Get-Content -LiteralPath $PointerPath -Raw | ConvertFrom-Json
        if ($Pointer.schema_name -ne "CampaignScratchLatestPointerV1") {
            return $null
        }
        if (-not $Pointer.artifact_id) {
            return $null
        }
        return $Pointer
    } catch {
        return $null
    }
}

function Write-CampaignScratchLatestPointer {
    param(
        [object] $Artifact
    )

    if (-not $Artifact -or $Artifact.Kind -ne "scratch") {
        return
    }
    $PointerPath = Get-CampaignScratchLatestPointerPath
    $Parent = Split-Path -Parent $PointerPath
    if ($Parent) {
        New-Item -ItemType Directory -Force -Path $Parent | Out-Null
    }
    $Pointer = [ordered]@{
        schema_name = "CampaignScratchLatestPointerV1"
        schema_version = 1
        updated_at = (Get-Date).ToString("o")
        artifact_id = $Artifact.Id
        report = $Artifact.ReportPath
        checkpoint = $Artifact.CheckpointPath
        manifest = $Artifact.ManifestPath
        command = $Artifact.CommandPath
        log = $Artifact.LogPath
    }
    $Pointer | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $PointerPath
}

function Get-CampaignArtifactMode {
    param(
        [object] $Artifact
    )

    if ($Artifact -and $Artifact.ManifestPath -and (Test-Path -LiteralPath $Artifact.ManifestPath)) {
        try {
            $Manifest = Get-Content -LiteralPath $Artifact.ManifestPath -Raw | ConvertFrom-Json
            if ($Manifest.mode) {
                return ([string] $Manifest.mode).ToLowerInvariant()
            }
        } catch {
            # Keep falling back to the command text.
        }
    }
    if ($Artifact -and $Artifact.CommandPath -and (Test-Path -LiteralPath $Artifact.CommandPath)) {
        $CommandText = Get-Content -LiteralPath $Artifact.CommandPath -Raw
        if ($CommandText -match "--preset\s+('?)(quick|focused|explore|deep)\1") {
            return $Matches[2].ToLowerInvariant()
        }
    }
    if ($Artifact -and $Artifact.Kind -eq "legacy_latest") {
        return Read-LegacyLatestCampaignMode
    }
    return $null
}

function Read-LegacyLatestCampaignMode {
    Assert-CampaignArtifactPathsInitialized
    if (Test-Path -LiteralPath $LegacyLatestModePath) {
        $ModeText = (Get-Content -LiteralPath $LegacyLatestModePath -Raw).Trim().ToLowerInvariant()
        if (@("quick", "focused", "explore", "deep") -contains $ModeText) {
            return $ModeText
        }
    }
    if (Test-Path -LiteralPath $LegacyLatestCommandPath) {
        $CommandText = Get-Content -LiteralPath $LegacyLatestCommandPath -Raw
        if ($CommandText -match "--preset\s+('?)(quick|focused|explore|deep)\1") {
            return $Matches[2].ToLowerInvariant()
        }
    }
    return $null
}

function Get-LatestScratchCampaignArtifact {
    $Pointer = Read-CampaignScratchLatestPointer
    if (-not $Pointer) {
        throw "No scratch latest pointer found at $(Get-CampaignScratchLatestPointerPath). Run .\tools\campaign.ps1 -Scratch to create one, or use -From scratch:<id>."
    }

    return New-CampaignScratchArtifactRef -ArtifactId ([string] $Pointer.artifact_id)
}

function Get-CampaignArtifactRunConfig {
    param(
        [string] $CheckpointPath,
        [string] $ManifestPath
    )

    $Config = [ordered]@{
        Seed = $null
        Ascension = $null
        Class = $null
        Mode = $null
    }

    if ($CheckpointPath -and (Test-Path -LiteralPath $CheckpointPath)) {
        try {
            $Checkpoint = Get-Content -LiteralPath $CheckpointPath -Raw | ConvertFrom-Json
            if ($Checkpoint.sessions -and $Checkpoint.sessions.Count -gt 0) {
                $RunState = $Checkpoint.sessions[0].session.run_state
                if ($RunState) {
                    if ($RunState.seed -ne $null) { $Config.Seed = [long] $RunState.seed }
                    if ($RunState.ascension_level -ne $null) { $Config.Ascension = [int] $RunState.ascension_level }
                    if ($RunState.player_class) { $Config.Class = ([string] $RunState.player_class).ToLowerInvariant() }
                }
            }
        } catch {
            # Older checkpoints may not expose run_state; leave fields unset.
        }
    }

    if ($ManifestPath -and (Test-Path -LiteralPath $ManifestPath)) {
        try {
            $Manifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
            if ($Config.Seed -eq $null -and $Manifest.seed -ne $null) {
                $Config.Seed = [long] $Manifest.seed
            }
            if ($Config.Ascension -eq $null -and $Manifest.ascension -ne $null) {
                $Config.Ascension = [int] $Manifest.ascension
            }
            if (-not $Config.Class -and $Manifest.class) {
                $Config.Class = ([string] $Manifest.class).ToLowerInvariant()
            }
            if ($Manifest.mode) { $Config.Mode = ([string] $Manifest.mode).ToLowerInvariant() }
        } catch {
            # Latest artifacts can lack a manifest; existing sidecar mode fallback remains in effect.
        }
    }

    return [pscustomobject] $Config
}

function Get-CampaignSourceArtifact {
    param(
        [string] $Selector = "",
        [bool] $UseScratchLatest
    )

    Assert-CampaignArtifactPathsInitialized
    $ResolvedSelector = if ($Selector) { $Selector.Trim() } elseif ($UseScratchLatest) { "scratch:latest" } else { "latest" }

    if ($ResolvedSelector -eq "scratch:latest") {
        return Get-LatestScratchCampaignArtifact
    }

    if ($ResolvedSelector -eq "latest") {
        $Pointer = Read-CampaignLatestPointer
        if ($Pointer) {
            return New-CampaignRunArtifact -ArtifactId ([string] $Pointer.artifact_id) -BaseLabel ([string] $Pointer.artifact_id)
        }
        throw "No latest campaign pointer found at $(Get-CampaignLatestPointerPath). Run .\tools\campaign.ps1 to create one, or use -From legacy-latest to read old latest.campaign/checkpoint sidecars explicitly."
    }

    if ($ResolvedSelector -eq "legacy-latest") {
        return New-CampaignLegacyLatestArtifact
    }

    if ($ResolvedSelector -match '^run:(.+)$') {
        return New-CampaignRunArtifact -ArtifactId $Matches[1] -BaseLabel $Matches[1]
    }

    if ($ResolvedSelector -match '^scratch:(.+)$') {
        return New-CampaignScratchArtifactRef -ArtifactId $Matches[1]
    }

    throw "Unknown campaign artifact selector '$ResolvedSelector'. Use 'latest', 'legacy-latest', 'scratch:latest', 'scratch:<id>', or 'run:<id>'."
}
