function Get-CampaignSourceContext {
    param(
        [bool] $ReadsCampaignSource,
        [bool] $Last,
        [string] $From,
        [bool] $UseScratchLatest
    )

    if (-not ($ReadsCampaignSource -or $Last)) {
        return [pscustomobject]@{
            Artifact = $null
            RunConfig = $null
        }
    }

    $Artifact = Get-CampaignSourceArtifact -Selector $From -UseScratchLatest $UseScratchLatest
    $RunConfig = Get-CampaignArtifactRunConfig `
        -CheckpointPath $Artifact.CheckpointPath `
        -ManifestPath $Artifact.ManifestPath

    return [pscustomobject]@{
        Artifact = $Artifact
        RunConfig = $RunConfig
    }
}

function Resolve-CampaignMode {
    param(
        [string] $Mode,
        [bool] $ModeBound,
        [bool] $IsContinuationFamily,
        [bool] $ContinueCampaign,
        [object] $SourceArtifact
    )

    if ($ModeBound) {
        return $Mode
    }

    if ($IsContinuationFamily) {
        $SavedMode = Get-CampaignArtifactMode -Artifact $SourceArtifact
        if ($SavedMode) {
            return $SavedMode
        }
        return "focused"
    }

    if ($ContinueCampaign) {
        $SavedMode = Get-CampaignArtifactMode -Artifact $SourceArtifact
        if ($SavedMode) {
            return $SavedMode
        }
        return "deep"
    }

    return $Mode
}

function Resolve-CampaignSeed {
    param(
        [long] $Seed,
        [bool] $ReadsCampaignSource,
        [bool] $Last,
        [object] $SourceArtifact,
        [object] $SourceRunConfig
    )

    if (($ReadsCampaignSource -or $Last) -and $Seed -le 0 -and $SourceRunConfig -and $SourceRunConfig.Seed -ne $null) {
        return [long] $SourceRunConfig.Seed
    }
    if ($Last -and $Seed -le 0) {
        throw "No reusable campaign seed found in source artifact '$($SourceArtifact.Label)'. Use -Seed or a source with checkpoint run_state."
    }
    if ($Seed -le 0) {
        return (Get-Random -Minimum 1 -Maximum 2147483647)
    }
    return $Seed
}

function Resolve-CampaignRunIdentity {
    param(
        [int] $Ascension,
        [string] $Class,
        [string] $Domain,
        [bool] $AscensionBound,
        [bool] $ClassBound,
        [bool] $DomainBound,
        [bool] $Last,
        [bool] $Inspect,
        [bool] $ReadsCampaignSource,
        [object] $SourceRunConfig
    )

    if ($DomainBound) {
        $DomainAscension = [int] $Domain.Substring(1)
        if ($AscensionBound -and $Ascension -ne $DomainAscension) {
            throw "-Domain $Domain conflicts with -Ascension $Ascension."
        }
        $Ascension = $DomainAscension
        $AscensionBound = $true
    }

    if ($Last -or $Inspect -or $ReadsCampaignSource) {
        if (-not $AscensionBound) {
            if ($SourceRunConfig -and $SourceRunConfig.Ascension -ne $null) {
                $Ascension = [int] $SourceRunConfig.Ascension
            } else {
                $SavedConfig = Read-LatestCheckpointRunConfig
                if ($SavedConfig -and $SavedConfig.ascension_level -ne $null) {
                    $Ascension = [int] $SavedConfig.ascension_level
                }
            }
        }
        if (-not $ClassBound) {
            if ($SourceRunConfig -and $SourceRunConfig.Class) {
                $Class = ([string] $SourceRunConfig.Class).ToLowerInvariant()
            } else {
                $SavedConfig = Read-LatestCheckpointRunConfig
                if ($SavedConfig -and $SavedConfig.player_class) {
                    $Class = ([string] $SavedConfig.player_class).ToLowerInvariant()
                }
            }
        }
    }

    return [pscustomobject]@{
        Ascension = $Ascension
        Class = $Class
        AscensionBound = $AscensionBound
        ClassBound = $ClassBound
    }
}
