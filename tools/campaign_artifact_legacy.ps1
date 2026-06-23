function New-CampaignLegacyLatestArtifact {
    Assert-CampaignArtifactPathsInitialized
    return [pscustomobject]@{
        Kind = "legacy_latest"
        Id = "legacy-latest"
        Label = "legacy-latest"
        Dir = $CampaignDir
        ReportPath = $LegacyLatestCampaignPath
        StatePath = Get-CampaignStateSidecarPath -ReportPath $LegacyLatestCampaignPath
        JournalPath = Get-CampaignJournalSidecarPath -ReportPath $LegacyLatestCampaignPath
        CheckpointPath = $LegacyLatestCheckpointPath
        ManifestPath = $LegacyLatestManifestPath
        LogPath = $LegacyLatestLogPath
        CommandPath = $LegacyLatestCommandPath
    }
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
