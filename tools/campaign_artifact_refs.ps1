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

function Resolve-CampaignMainArtifactPath {
    param(
        [string] $CompressedPath,
        [string] $LegacyPath,
        [bool] $PreferCompressed
    )

    if ($PreferCompressed) {
        return $CompressedPath
    }
    if (Test-Path -LiteralPath $CompressedPath) {
        return $CompressedPath
    }
    if (Test-Path -LiteralPath $LegacyPath) {
        return $LegacyPath
    }
    return $CompressedPath
}

function New-CampaignRunArtifact {
    param(
        [string] $BaseLabel,
        [string] $ArtifactId = ""
    )

    $Id = if ($ArtifactId) { Convert-ToCampaignArtifactSlug $ArtifactId } else { New-CampaignArtifactId -BaseLabel $BaseLabel }
    $Dir = Join-Path (Get-CampaignRunsDir) $Id
    $PreferCompressed = -not $ArtifactId
    $ReportPath = Resolve-CampaignMainArtifactPath `
        -CompressedPath (Join-Path $Dir "campaign.json.gz") `
        -LegacyPath (Join-Path $Dir "campaign.json") `
        -PreferCompressed $PreferCompressed
    $CheckpointPath = Resolve-CampaignMainArtifactPath `
        -CompressedPath (Join-Path $Dir "checkpoint.json.gz") `
        -LegacyPath (Join-Path $Dir "checkpoint.json") `
        -PreferCompressed $PreferCompressed
    return [pscustomobject]@{
        Kind = "run"
        Id = $Id
        Label = "run:$Id"
        Dir = $Dir
        ReportPath = $ReportPath
        CheckpointPath = $CheckpointPath
        ManifestPath = Join-Path $Dir "manifest.json"
        LogPath = Join-Path $Dir "log.txt"
        CommandPath = Join-Path $Dir "command.txt"
    }
}

function New-CampaignScratchArtifactRef {
    param(
        [string] $ArtifactId,
        [bool] $PreferCompressed = $false
    )

    $Id = Convert-ToCampaignArtifactSlug $ArtifactId
    $ReportPath = Resolve-CampaignMainArtifactPath `
        -CompressedPath (Join-Path $ScratchCampaignDir "$Id.campaign.json.gz") `
        -LegacyPath (Join-Path $ScratchCampaignDir "$Id.campaign.json") `
        -PreferCompressed $PreferCompressed
    $CheckpointPath = Resolve-CampaignMainArtifactPath `
        -CompressedPath (Join-Path $ScratchCampaignDir "$Id.checkpoint.json.gz") `
        -LegacyPath (Join-Path $ScratchCampaignDir "$Id.checkpoint.json") `
        -PreferCompressed $PreferCompressed
    return [pscustomobject]@{
        Kind = "scratch"
        Id = $Id
        Label = "scratch:$Id"
        Dir = $ScratchCampaignDir
        ReportPath = $ReportPath
        CheckpointPath = $CheckpointPath
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
    return New-CampaignScratchArtifactRef -ArtifactId $Id -PreferCompressed $true
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
