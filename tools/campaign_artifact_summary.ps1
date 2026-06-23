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

function Get-CampaignArtifactRawSize {
    param(
        [string] $Path
    )

    try {
        $Text = Read-CampaignArtifactText -Path $Path
        if ($null -eq $Text) {
            return $null
        }
        return [System.Text.Encoding]::UTF8.GetByteCount($Text)
    } catch {
        return $null
    }
}

function Format-CampaignArtifactStorageSize {
    param(
        [string] $Path
    )

    $CompressedBytes = (Get-Item -LiteralPath $Path).Length
    $Compressed = Format-CampaignArtifactSize -Bytes $CompressedBytes
    if ($Path.EndsWith(".gz", [System.StringComparison]::OrdinalIgnoreCase)) {
        $RawBytes = Get-CampaignArtifactRawSize -Path $Path
        if ($null -ne $RawBytes -and $RawBytes -gt 0) {
            $Raw = Format-CampaignArtifactSize -Bytes $RawBytes
            $Ratio = [Math]::Round($RawBytes / [double] $CompressedBytes, 1)
            return "$Compressed raw=$Raw ratio=${Ratio}x"
        }
    }
    return $Compressed
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
        $RequestKind = "-"
        if ($Json.request -and $Json.request.kind) {
            $RequestKind = $Json.request.kind
        }
        return "stage=$($Json.stage) kind=$($Json.command_kind) request=$RequestKind wrapper_params=$WrapperParams driver_args=$DriverArgs"
    }

    if ($Kind -eq "report") {
        $Active = Get-CampaignValueCount -Value $Json.active
        $Frozen = Get-CampaignValueCount -Value $Json.frozen
        $StateRounds = Get-CampaignValueCount -Value $Json.rounds
        $StateRef = "-"
        if ($Json.state_artifact) {
            $StateRef = $Json.state_artifact
            $StatePath = Get-CampaignStateSidecarPath -ReportPath $Path
            $StateJson = Read-CampaignJsonArtifact -Path $StatePath
            if ($StateJson) {
                $Active = Get-CampaignValueCount -Value $StateJson.active
                $Frozen = Get-CampaignValueCount -Value $StateJson.frozen
                $StateRounds = Get-CampaignValueCount -Value $StateJson.rounds
            }
        }
        $Journal = Get-CampaignValueCount -Value $Json.journal
        if ($Json.journal_event_count) {
            $Journal = $Json.journal_event_count
        }
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
        $JournalRef = "-"
        if ($Json.journal_artifact) {
            $JournalRef = $Json.journal_artifact
        }
        return "rounds=$($Json.rounds_completed) stop=$($Json.stop_reason) active=$Active frozen=$Frozen state_ref=$StateRef state_round_entries=$StateRounds journal_events=$Journal journal_ref=$JournalRef state_sessions=$StateSessions state_nodes=$StateNodes decision_sessions=$DecisionSessions route_decision_sessions=$RouteDecisionSessions pruned=$SessionsPruned"
    }

    if ($Kind -eq "state") {
        $Active = Get-CampaignValueCount -Value $Json.active
        $Frozen = Get-CampaignValueCount -Value $Json.frozen
        $Victories = Get-CampaignValueCount -Value $Json.victories
        $Abandoned = Get-CampaignValueCount -Value $Json.abandoned
        $Stuck = Get-CampaignValueCount -Value $Json.stuck
        $Rounds = Get-CampaignValueCount -Value $Json.rounds
        return "schema=$($Json.schema_name) version=$($Json.schema_version) active=$Active frozen=$Frozen victories=$Victories abandoned=$Abandoned stuck=$Stuck rounds=$Rounds discarded=$($Json.discarded_count)"
    }

    if ($Kind -eq "journal") {
        $Events = Get-CampaignValueCount -Value $Json.events
        return "schema=$($Json.schema_name) version=$($Json.schema_version) events=$Events"
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
        [pscustomobject]@{ Kind = "state"; Path = (Get-CampaignStateSidecarPath -ReportPath $ReportPath); Contract = "campaign resume state" },
        [pscustomobject]@{ Kind = "journal"; Path = (Get-CampaignJournalSidecarPath -ReportPath $ReportPath); Contract = "decision facts" },
        [pscustomobject]@{ Kind = "checkpoint"; Path = $CheckpointPath; Contract = "continuation state" },
        [pscustomobject]@{ Kind = "log"; Path = $LogPath; Contract = "optional stream log" },
        [pscustomobject]@{ Kind = "command"; Path = $CommandPath; Contract = "primary driver command" }
    )

    foreach ($Artifact in $Artifacts) {
        if (Test-Path -LiteralPath $Artifact.Path) {
            $Size = Format-CampaignArtifactStorageSize -Path $Artifact.Path
            $Shape = Get-CampaignArtifactShape -Kind $Artifact.Kind -Path $Artifact.Path
            Write-Host ("  {0,-10} {1,10} | {2,-22} | {3}" -f $Artifact.Kind, $Size, $Artifact.Contract, $Shape)
            Write-Host "    path=$($Artifact.Path)"
        } else {
            Write-Host ("  {0,-10} {1,10} | {2,-22} | missing" -f $Artifact.Kind, "-", $Artifact.Contract)
            Write-Host "    path=$($Artifact.Path)"
        }
    }
}
