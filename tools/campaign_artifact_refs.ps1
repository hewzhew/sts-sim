function Convert-CampaignDriverArtifactRef {
    param(
        [object] $Artifact
    )

    if (-not $Artifact) {
        throw "Internal error: empty artifact resolver response."
    }

    $Kind = ([string] $Artifact.kind).ToLowerInvariant()
    return [pscustomobject]@{
        Kind = $Kind
        Id = [string] $Artifact.id
        Label = [string] $Artifact.label
        Dir = [string] $Artifact.dir
        ReportPath = [string] $Artifact.report_path
        StatePath = [string] $Artifact.state_path
        JournalPath = [string] $Artifact.journal_path
        CheckpointPath = [string] $Artifact.checkpoint_path
        ManifestPath = [string] $Artifact.manifest_path
        LogPath = [string] $Artifact.log_path
        CommandPath = [string] $Artifact.command_path
    }
}

function New-CampaignOutputArtifactViaDriver {
    param(
        [string] $BaseLabel,
        [bool] $Scratch,
        [string] $DriverExe
    )

    if (-not $DriverExe) {
        throw "Internal error: Rust campaign artifact allocator requires DriverExe."
    }

    $Kind = if ($Scratch) { "scratch" } else { "run" }
    $Args = @(
        "artifact",
        "allocate",
        "--kind", $Kind,
        "--label", "$BaseLabel",
        "--stamp", (Get-Date -Format "yyyyMMdd-HHmmss"),
        "--suffix", ([guid]::NewGuid().ToString("N").Substring(0, 8)),
        "--campaign-dir", "$script:CampaignDir",
        "--json"
    )
    $Json = & $DriverExe @Args
    if ($LASTEXITCODE -ne 0) {
        throw "Rust campaign artifact allocator failed with exit code $LASTEXITCODE."
    }
    try {
        return Convert-CampaignDriverArtifactRef -Artifact ($Json | ConvertFrom-Json)
    } catch {
        throw "Rust campaign artifact allocator returned invalid JSON: $_"
    }
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

function Get-CampaignSidecarPath {
    param(
        [string] $ReportPath,
        [string] $Sidecar
    )

    if (-not $ReportPath) {
        return ""
    }

    $Directory = [System.IO.Path]::GetDirectoryName($ReportPath)
    $Name = [System.IO.Path]::GetFileName($ReportPath)
    if ($Name.EndsWith(".campaign.json.gz", [System.StringComparison]::OrdinalIgnoreCase)) {
        $SidecarName = $Name.Substring(0, $Name.Length - ".campaign.json.gz".Length) + ".$Sidecar.json.gz"
    } elseif ($Name.EndsWith(".campaign.json", [System.StringComparison]::OrdinalIgnoreCase)) {
        $SidecarName = $Name.Substring(0, $Name.Length - ".campaign.json".Length) + ".$Sidecar.json"
    } elseif ($Name.EndsWith(".json.gz", [System.StringComparison]::OrdinalIgnoreCase)) {
        $SidecarName = $Name.Substring(0, $Name.Length - ".json.gz".Length) + ".$Sidecar.json.gz"
    } elseif ($Name.EndsWith(".json", [System.StringComparison]::OrdinalIgnoreCase)) {
        $SidecarName = $Name.Substring(0, $Name.Length - ".json".Length) + ".$Sidecar.json"
    } else {
        $SidecarName = "$Name.$Sidecar.json.gz"
    }
    return Join-Path $Directory $SidecarName
}

function Get-CampaignJournalSidecarPath {
    param(
        [string] $ReportPath
    )

    return Get-CampaignSidecarPath -ReportPath $ReportPath -Sidecar "journal"
}

function Get-CampaignStateSidecarPath {
    param(
        [string] $ReportPath
    )

    return Get-CampaignSidecarPath -ReportPath $ReportPath -Sidecar "state"
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
        [long] $Seed,
        [string] $DriverExe
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
        $RunOutputArtifact = New-CampaignOutputArtifactViaDriver `
            -BaseLabel $OutputBaseLabel `
            -Scratch ([bool] $Scratch) `
            -DriverExe $DriverExe
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
