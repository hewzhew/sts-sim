function Set-CampaignArtifactResolverDriver {
    param(
        [string] $DriverExe
    )

    if (-not $DriverExe) {
        throw "Internal error: campaign artifact resolver requires DriverExe."
    }
    $script:CampaignArtifactResolverDriverExe = $DriverExe
}

function Get-CampaignManifestField {
    param(
        [object] $Manifest,
        [string] $Name
    )

    if (-not $Manifest -or -not $Name) {
        return $null
    }
    $Direct = $Manifest.PSObject.Properties[$Name]
    if ($Direct -and $Direct.Value -ne $null) {
        return $Direct.Value
    }
    if ($Manifest.PSObject.Properties["compatibility"]) {
        $Compatibility = $Manifest.compatibility
        if ($Compatibility) {
            $CompatField = $Compatibility.PSObject.Properties[$Name]
            if ($CompatField -and $CompatField.Value -ne $null) {
                return $CompatField.Value
            }
        }
    }
    if ($Manifest.PSObject.Properties["payload"]) {
        $Payload = $Manifest.payload
        if ($Payload) {
            $PayloadField = $Payload.PSObject.Properties[$Name]
            if ($PayloadField -and $PayloadField.Value -ne $null) {
                return $PayloadField.Value
            }
        }
    }
    return $null
}

function Get-CampaignSourceArtifactViaDriver {
    param(
        [string] $Selector
    )

    if (-not $script:CampaignArtifactResolverDriverExe) {
        throw "Internal error: Rust campaign artifact resolver was not configured."
    }

    $Args = @(
        "artifact",
        "resolve",
        "$Selector",
        "--campaign-dir", "$script:CampaignDir",
        "--json"
    )
    $Json = & $script:CampaignArtifactResolverDriverExe @Args
    if ($LASTEXITCODE -ne 0) {
        throw "Rust campaign artifact resolver failed with exit code $LASTEXITCODE for selector '$Selector'."
    }
    try {
        return Convert-CampaignDriverArtifactRef -Artifact ($Json | ConvertFrom-Json)
    } catch {
        throw "Rust campaign artifact resolver returned invalid JSON for selector '$Selector': $_"
    }
}

function Get-CampaignArtifactMode {
    param(
        [object] $Artifact
    )

    if ($Artifact -and $Artifact.ManifestPath -and (Test-Path -LiteralPath $Artifact.ManifestPath)) {
        try {
            $Manifest = Get-Content -LiteralPath $Artifact.ManifestPath -Raw | ConvertFrom-Json
            $Mode = Get-CampaignManifestField -Manifest $Manifest -Name "mode"
            if ($Mode) {
                return ([string] $Mode).ToLowerInvariant()
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
            $Checkpoint = Read-CampaignJsonArtifact -Path $CheckpointPath
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
            $Seed = Get-CampaignManifestField -Manifest $Manifest -Name "seed"
            $ManifestAscension = Get-CampaignManifestField -Manifest $Manifest -Name "ascension"
            $ManifestClass = Get-CampaignManifestField -Manifest $Manifest -Name "class"
            if (-not $ManifestClass) {
                $ManifestClass = Get-CampaignManifestField -Manifest $Manifest -Name "player_class"
            }
            $ManifestMode = Get-CampaignManifestField -Manifest $Manifest -Name "mode"
            if ($Config.Seed -eq $null -and $Seed -ne $null) {
                $Config.Seed = [long] $Seed
            }
            if ($Config.Ascension -eq $null -and $ManifestAscension -ne $null) {
                $Config.Ascension = [int] $ManifestAscension
            }
            if (-not $Config.Class -and $ManifestClass) {
                $Config.Class = ([string] $ManifestClass).ToLowerInvariant()
            }
            if ($ManifestMode) { $Config.Mode = ([string] $ManifestMode).ToLowerInvariant() }
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

    if ($ResolvedSelector -eq "legacy-latest") {
        return New-CampaignLegacyLatestArtifact
    }

    return Get-CampaignSourceArtifactViaDriver -Selector $ResolvedSelector
}
