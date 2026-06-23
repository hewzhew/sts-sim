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

function Get-LatestScratchCampaignArtifact {
    $Pointer = Read-CampaignScratchLatestPointer
    if (-not $Pointer) {
        throw "No scratch latest pointer found at $(Get-CampaignScratchLatestPointerPath). Run .\tools\campaign.ps1 -Scratch to create one, or use -From scratch:<id>."
    }

    return New-CampaignScratchArtifactRef -ArtifactId ([string] $Pointer.artifact_id)
}
