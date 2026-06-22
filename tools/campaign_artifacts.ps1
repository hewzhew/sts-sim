$LatestSeedPath = Join-Path $CampaignDir "latest.seed.txt"
$LatestAscensionPath = Join-Path $CampaignDir "latest.ascension.txt"
$LatestClassPath = Join-Path $CampaignDir "latest.class.txt"
$LatestModePath = Join-Path $CampaignDir "latest.mode.txt"
$LatestCommandPath = Join-Path $CampaignDir "latest.command.txt"
$LatestManifestPath = Join-Path $CampaignDir "latest.manifest.json"
$LatestLogPath = Join-Path $CampaignDir "latest.log"
$LatestCampaignPath = Join-Path $CampaignDir "latest.campaign.json"
$LatestCheckpointPath = Join-Path $CampaignDir "latest.checkpoint.json"
$LatestDecisionOutcomePath = Join-Path $CampaignDir "latest.decision_outcomes.jsonl"
$LatestDecisionOutcomeBeforePath = Join-Path $CampaignDir "latest.decision_outcomes.before.jsonl"
$LatestDecisionOutcomeAfterPath = Join-Path $CampaignDir "latest.decision_outcomes.after.jsonl"

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
    return (Join-Path $CampaignDir "runs")
}

function Get-CampaignLatestPointerPath {
    return (Join-Path $CampaignDir "latest.json")
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

function New-CampaignScratchArtifact {
    param(
        [string] $BaseLabel
    )

    $Id = New-CampaignArtifactId -BaseLabel $BaseLabel
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

function New-CampaignLegacyLatestArtifact {
    return [pscustomobject]@{
        Kind = "legacy_latest"
        Id = "legacy-latest"
        Label = "legacy-latest"
        Dir = $CampaignDir
        ReportPath = $LatestCampaignPath
        CheckpointPath = $LatestCheckpointPath
        ManifestPath = $LatestManifestPath
        LogPath = $LatestLogPath
        CommandPath = $LatestCommandPath
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
        return Read-LatestCampaignMode
    }
    return $null
}

function Read-LatestCheckpointRunConfig {
    if (-not (Test-Path -LiteralPath $LatestCheckpointPath)) {
        return $null
    }
    try {
        $Checkpoint = Get-Content -LiteralPath $LatestCheckpointPath -Raw | ConvertFrom-Json
        if ($Checkpoint.sessions -and $Checkpoint.sessions.Count -gt 0) {
            return $Checkpoint.sessions[0].session.run_state
        }
    } catch {
        return $null
    }
    return $null
}

function Read-LatestCampaignMode {
    if (Test-Path -LiteralPath $LatestModePath) {
        $ModeText = (Get-Content -LiteralPath $LatestModePath -Raw).Trim().ToLowerInvariant()
        if (@("quick", "focused", "explore", "deep") -contains $ModeText) {
            return $ModeText
        }
    }
    if (Test-Path -LiteralPath $LatestCommandPath) {
        $CommandText = Get-Content -LiteralPath $LatestCommandPath -Raw
        if ($CommandText -match "--preset\s+('?)(quick|focused|explore|deep)\1") {
            return $Matches[2].ToLowerInvariant()
        }
    }
    return $null
}

function Get-LatestScratchCampaignArtifact {
    $ScratchReport = Get-ChildItem -LiteralPath $ScratchCampaignDir -Filter "*.campaign.json" -File -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $ScratchReport) {
        throw "No scratch campaign report found under $ScratchCampaignDir."
    }

    $ScratchCheckpointPath = $ScratchReport.FullName -replace '\.campaign\.json$', '.checkpoint.json'
    if (-not (Test-Path -LiteralPath $ScratchCheckpointPath)) {
        throw "Latest scratch report has no matching checkpoint: $ScratchCheckpointPath"
    }

    return [pscustomobject]@{
        ReportPath = $ScratchReport.FullName
        CheckpointPath = $ScratchCheckpointPath
        ManifestPath = $ScratchReport.FullName -replace '\.campaign\.json$', '.manifest.json'
        LogPath = $ScratchReport.FullName -replace '\.campaign\.json$', '.log'
        CommandPath = $ScratchReport.FullName -replace '\.campaign\.json$', '.command.txt'
        Label = $ScratchReport.BaseName -replace '\.campaign$', ''
    }
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

    $ResolvedSelector = if ($Selector) { $Selector.Trim() } elseif ($UseScratchLatest) { "scratch:latest" } else { "latest" }

    if ($ResolvedSelector -eq "scratch:latest") {
        $ScratchArtifact = Get-LatestScratchCampaignArtifact
        return [pscustomobject]@{
            Kind = "scratch"
            Id = $ScratchArtifact.Label
            ReportPath = $ScratchArtifact.ReportPath
            CheckpointPath = $ScratchArtifact.CheckpointPath
            ManifestPath = $ScratchArtifact.ManifestPath
            LogPath = $ScratchArtifact.LogPath
            CommandPath = $ScratchArtifact.CommandPath
            Label = "scratch:$($ScratchArtifact.Label)"
        }
    }

    if ($ResolvedSelector -eq "latest") {
        $Pointer = Read-CampaignLatestPointer
        if ($Pointer) {
            return New-CampaignRunArtifact -ArtifactId ([string] $Pointer.artifact_id) -BaseLabel ([string] $Pointer.artifact_id)
        }
        return New-CampaignLegacyLatestArtifact
    }

    if ($ResolvedSelector -eq "legacy-latest") {
        return New-CampaignLegacyLatestArtifact
    }

    if ($ResolvedSelector -match '^run:(.+)$') {
        return New-CampaignRunArtifact -ArtifactId $Matches[1] -BaseLabel $Matches[1]
    }

    throw "Unknown campaign artifact selector '$ResolvedSelector'. Use 'latest', 'legacy-latest', 'scratch:latest', or 'run:<id>'."
}

function Get-CampaignArtifactShortLabel {
    param(
        [object] $Artifact
    )

    if (-not $Artifact) {
        return "-"
    }
    return $Artifact.Label
}

function Format-CampaignArtifactSize {
    param(
        [long] $Bytes
    )

    if ($Bytes -ge 1048576) {
        return "{0:n1} MiB" -f ($Bytes / 1048576.0)
    }
    if ($Bytes -ge 1024) {
        return "{0:n1} KiB" -f ($Bytes / 1024.0)
    }
    return "$Bytes B"
}

function Get-CampaignValueCount {
    param(
        [object] $Value
    )

    if ($null -eq $Value) {
        return 0
    }
    if ($Value -is [System.Array]) {
        return $Value.Count
    }
    if ($Value -is [System.Collections.ICollection]) {
        return $Value.Count
    }
    return 1
}

function Get-CampaignJsonTopFields {
    param(
        [object] $Json,
        [int] $Limit = 10
    )

    if ($null -eq $Json) {
        return "-"
    }
    $Names = @($Json.PSObject.Properties.Name)
    if ($Names.Count -eq 0) {
        return "-"
    }
    $Shown = @($Names | Select-Object -First $Limit)
    $Suffix = if ($Names.Count -gt $Limit) { ", ..." } else { "" }
    return ($Shown -join ", ") + $Suffix
}

function Read-CampaignJsonArtifact {
    param(
        [string] $Path
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        return $null
    }
    try {
        return Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
    } catch {
        return $null
    }
}

function Get-CampaignArtifactShape {
    param(
        [string] $Kind,
        [string] $Path
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        return "missing"
    }

    if ($Kind -eq "log") {
        return "text log"
    }
    if ($Kind -eq "command") {
        return "primary_driver_command"
    }

    $Json = Read-CampaignJsonArtifact -Path $Path
    if ($null -eq $Json) {
        return "unreadable_json"
    }

    if ($Kind -eq "manifest") {
        $WrapperParams = 0
        if ($Json.wrapper_invocation -and $Json.wrapper_invocation.bound_parameters) {
            $WrapperParams = @($Json.wrapper_invocation.bound_parameters.PSObject.Properties).Count
        }
        $DriverArgs = Get-CampaignValueCount -Value $Json.primary_driver.args
        return "stage=$($Json.stage) kind=$($Json.command_kind) wrapper_params=$WrapperParams driver_args=$DriverArgs"
    }

    if ($Kind -eq "report") {
        $Active = Get-CampaignValueCount -Value $Json.active
        $Frozen = Get-CampaignValueCount -Value $Json.frozen
        $Journal = Get-CampaignValueCount -Value $Json.journal
        $Rounds = Get-CampaignValueCount -Value $Json.rounds
        $StateSessions = 0
        $StateNodes = 0
        $DecisionSessions = 0
        $RouteDecisionSessions = 0
        $SessionsPruned = 0
        if ($Json.state_store) {
            $StateSessions = $Json.state_store.sessions
            $StateNodes = $Json.state_store.nodes
            $DecisionSessions = $Json.state_store.decision_coordinate_sessions
            $RouteDecisionSessions = $Json.state_store.route_decision_coordinate_sessions
            $SessionsPruned = $Json.state_store.sessions_pruned
        }
        return "rounds=$($Json.rounds_completed) stop=$($Json.stop_reason) active=$Active frozen=$Frozen journal=$Journal round_entries=$Rounds state_sessions=$StateSessions state_nodes=$StateNodes decision_sessions=$DecisionSessions route_decision_sessions=$RouteDecisionSessions pruned=$SessionsPruned"
    }

    if ($Kind -eq "checkpoint") {
        $Nodes = Get-CampaignValueCount -Value $Json.nodes
        $Sessions = Get-CampaignValueCount -Value $Json.sessions
        $AnchorPaths = Get-CampaignValueCount -Value $Json.decision_parent_anchor_commands
        $PreludeCommands = 0
        if ($Json.run_prelude -and $Json.run_prelude.commands) {
            $PreludeCommands = Get-CampaignValueCount -Value $Json.run_prelude.commands
        }
        $ApproxSessionBytes = "-"
        if ($Sessions -gt 0) {
            $CheckpointBytes = (Get-Item -LiteralPath $Path).Length
            $ApproxSessionBytes = Format-CampaignArtifactSize -Bytes ([long]($CheckpointBytes / $Sessions))
        }
        return "rounds=$($Json.rounds_completed) nodes=$Nodes sessions=$Sessions anchor_paths=$AnchorPaths approx_bytes_per_session=$ApproxSessionBytes prelude_commands=$PreludeCommands"
    }

    return "json_fields=$(Get-CampaignJsonTopFields -Json $Json -Limit 6)"
}

function Write-CampaignArtifactSummary {
    param(
        [string] $SourceLabel,
        [string] $ReportPath,
        [string] $CheckpointPath,
        [string] $ManifestPath,
        [string] $LogPath,
        [string] $CommandPath
    )

    Write-Host "CampaignArtifactContractV1 source=$SourceLabel"
    $Artifacts = @(
        [pscustomobject]@{ Kind = "manifest"; Path = $ManifestPath; Contract = "run provenance" },
        [pscustomobject]@{ Kind = "report"; Path = $ReportPath; Contract = "campaign summary" },
        [pscustomobject]@{ Kind = "checkpoint"; Path = $CheckpointPath; Contract = "continuation state" },
        [pscustomobject]@{ Kind = "log"; Path = $LogPath; Contract = "optional stream log" },
        [pscustomobject]@{ Kind = "command"; Path = $CommandPath; Contract = "primary driver command" }
    )

    foreach ($Artifact in $Artifacts) {
        if (Test-Path -LiteralPath $Artifact.Path) {
            $Item = Get-Item -LiteralPath $Artifact.Path
            $Size = Format-CampaignArtifactSize -Bytes $Item.Length
            $Shape = Get-CampaignArtifactShape -Kind $Artifact.Kind -Path $Artifact.Path
            Write-Host ("  {0,-10} {1,10} | {2,-22} | {3}" -f $Artifact.Kind, $Size, $Artifact.Contract, $Shape)
            Write-Host "    path=$($Artifact.Path)"
        } else {
            Write-Host ("  {0,-10} {1,10} | {2,-22} | missing" -f $Artifact.Kind, "-", $Artifact.Contract)
            Write-Host "    path=$($Artifact.Path)"
        }
    }
}
