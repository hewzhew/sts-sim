function Assert-CoverageGapPresetCompatible {
    param(
        [string] $Preset,
        [string] $Name,
        [string] $Actual,
        [string] $Expected
    )

    if ($Actual -and $Actual -ne $Expected) {
        throw "$Preset conflicts with -$Name $Actual; expected $Expected."
    }
}

function New-CoverageGapFilterArgs {
    param(
        [string] $Bucket,
        [string] $EventId,
        [string] $Lane,
        [string] $OriginSource,
        [string] $Progress
    )

    $Args = @()
    if ($Bucket) {
        $Args += @("--coverage-gap-bucket", "$Bucket")
    }
    if ($EventId) {
        $Args += @("--coverage-gap-event-id", "$EventId")
    }
    if ($Lane) {
        $Args += @("--coverage-gap-lane", "$Lane")
    }
    if ($OriginSource) {
        $Args += @("--coverage-gap-origin-source", "$OriginSource")
    }
    if ($Progress) {
        $Args += @("--coverage-gap-progress", "$Progress")
    }
    return $Args
}

function Format-CoverageGapFilterLabel {
    param(
        [string] $Bucket,
        [string] $EventId,
        [string] $Lane,
        [string] $OriginSource,
        [string] $Progress
    )

    $Parts = @()
    if ($Bucket) {
        $Parts += "bucket=$Bucket"
    }
    if ($EventId) {
        $Parts += "event_id=$EventId"
    }
    if ($Lane) {
        $Parts += "lane=$Lane"
    }
    if ($OriginSource) {
        $Parts += "origin_source=$OriginSource"
    }
    if ($Progress) {
        $Parts += "progress=$Progress"
    }
    if ($Parts.Count -eq 0) {
        return "-"
    }
    return $Parts -join " "
}

function Resolve-CoverageGapFilterContext {
    param(
        [bool] $Route,
        [bool] $RouteMissing,
        [bool] $EventBoundary,
        [bool] $EventBoundaryMissing,
        [string] $Bucket,
        [string] $EventId,
        [string] $Lane,
        [string] $OriginSource,
        [string] $Progress
    )

    $RoutePreset = $Route -or $RouteMissing
    $EventBoundaryPreset = $EventBoundary -or $EventBoundaryMissing
    if ($RoutePreset -and $EventBoundaryPreset) {
        throw "Choose a route coverage-gap preset or an event-boundary coverage-gap preset, not both."
    }
    if ($RoutePreset) {
        $PresetName = if ($RouteMissing) { "-CoverageGapRouteMissing" } else { "-CoverageGapRoute" }
        Assert-CoverageGapPresetCompatible -Preset $PresetName -Name "CoverageGapBucket" -Actual $Bucket -Expected "route"
        Assert-CoverageGapPresetCompatible -Preset $PresetName -Name "CoverageGapOriginSource" -Actual $OriginSource -Expected "map_decision_packet"
        if (-not $Bucket) {
            $Bucket = "route"
        }
        if (-not $OriginSource) {
            $OriginSource = "map_decision_packet"
        }
    }
    if ($RouteMissing) {
        Assert-CoverageGapPresetCompatible -Preset "-CoverageGapRouteMissing" -Name "CoverageGapProgress" -Actual $Progress -Expected "missing"
        if (-not $Progress) {
            $Progress = "missing"
        }
    }
    if ($EventBoundaryPreset) {
        $PresetName = if ($EventBoundaryMissing) { "-CoverageGapEventBoundaryMissing" } else { "-CoverageGapEventBoundary" }
        Assert-CoverageGapPresetCompatible -Preset $PresetName -Name "CoverageGapBucket" -Actual $Bucket -Expected "event"
        Assert-CoverageGapPresetCompatible -Preset $PresetName -Name "CoverageGapOriginSource" -Actual $OriginSource -Expected "event_boundary_packet"
        if (-not $Bucket) {
            $Bucket = "event"
        }
        if (-not $OriginSource) {
            $OriginSource = "event_boundary_packet"
        }
    }
    if ($EventBoundaryMissing) {
        Assert-CoverageGapPresetCompatible -Preset "-CoverageGapEventBoundaryMissing" -Name "CoverageGapProgress" -Actual $Progress -Expected "missing"
        if (-not $Progress) {
            $Progress = "missing"
        }
    }

    $FilterArgs = @(New-CoverageGapFilterArgs `
        -Bucket $Bucket `
        -EventId $EventId `
        -Lane $Lane `
        -OriginSource $OriginSource `
        -Progress $Progress)
    $FilterLabel = Format-CoverageGapFilterLabel `
        -Bucket $Bucket `
        -EventId $EventId `
        -Lane $Lane `
        -OriginSource $OriginSource `
        -Progress $Progress
    $ResultFilterArgs = @(New-CoverageGapFilterArgs `
        -Bucket $Bucket `
        -EventId $EventId `
        -Lane $Lane `
        -OriginSource $OriginSource `
        -Progress "")
    $ResultFilterLabel = Format-CoverageGapFilterLabel `
        -Bucket $Bucket `
        -EventId $EventId `
        -Lane $Lane `
        -OriginSource $OriginSource `
        -Progress ""

    return [pscustomobject]@{
        Bucket = $Bucket
        EventId = $EventId
        Lane = $Lane
        OriginSource = $OriginSource
        Progress = $Progress
        FilterArgs = @($FilterArgs)
        FilterLabel = $FilterLabel
        ResultFilterArgs = @($ResultFilterArgs)
        ResultFilterLabel = $ResultFilterLabel
    }
}

function Resolve-CoverageGapExecutionContext {
    param(
        [string] $Execution,
        [bool] $UntilMilestoneBound,
        [bool] $ContinueCoverageGaps,
        [bool] $HasExplicitRoundBudget,
        [string] $Intent,
        [int] $ContinuationRounds
    )

    if ($Execution -eq "milestone" -and -not $UntilMilestoneBound) {
        throw "-CoverageGapExecution milestone requires -UntilMilestone."
    }

    $DriverExecution = $Execution
    $Label = $Execution
    if ($Execution -eq "auto") {
        if ($UntilMilestoneBound -and $ContinueCoverageGaps) {
            $Label = "milestone_continuation"
            $DriverExecution = "target_only"
        } elseif ($HasExplicitRoundBudget) {
            $Label = "advance_rounds"
            $DriverExecution = "advance_rounds"
        } elseif ($Intent -eq "gap_closure") {
            $Label = "target_only"
            $DriverExecution = "target_only"
        } else {
            $Label = "advance_rounds"
            $DriverExecution = "advance_rounds"
        }
    } elseif ($Execution -eq "milestone") {
        $Label = "milestone_continuation"
        $DriverExecution = "target_only"
    }

    $InitialSpentRounds = $ContinuationRounds
    if ($DriverExecution -eq "target_only") {
        $InitialSpentRounds = 0
    }

    return [pscustomobject]@{
        Label = $Label
        DriverExecution = $DriverExecution
        InitialSpentRounds = $InitialSpentRounds
    }
}

function New-CoverageGapPlanDriverArgs {
    param(
        [string] $SourceCampaignPath,
        [string] $SourceCheckpointPath,
        [int] $CoverageGapLimit,
        [int] $CoverageGapCandidatesPerDecision,
        [string[]] $CoverageGapFilterArgs
    )

    $Args = @(
        "campaign",
        "coverage",
        "plan",
        "--inspect-report", "$SourceCampaignPath",
        "--inspect-checkpoint", "$SourceCheckpointPath",
        "--coverage-gap-limit", "$CoverageGapLimit",
        "--coverage-gap-candidates-per-decision", "$CoverageGapCandidatesPerDecision"
    )
    $Args += @($CoverageGapFilterArgs | Where-Object { $_ })
    return $Args
}

function New-CoverageGapContinueDriverArgs {
    param(
        [string[]] $RunIdentityArgs,
        [string] $SourceCampaignPath,
        [string] $SourceCheckpointPath,
        [string] $RunOutputCampaignPath,
        [string] $RunOutputCheckpointPath,
        [string[]] $RoundBudgetArgs,
        [string] $DriverExecution,
        [int] $CoverageGapLimit,
        [int] $CoverageGapCandidatesPerDecision,
        [string] $CoverageGapIntent,
        [string[]] $CoverageGapFilterArgs,
        [object] $OptionContext
    )

    $Args = @("campaign", "coverage", "execute")
    if ($RunIdentityArgs.Count -ge 2 -and $RunIdentityArgs[0] -eq "campaign" -and $RunIdentityArgs[1] -eq "run") {
        $Args += @($RunIdentityArgs | Select-Object -Skip 2)
    } else {
        $Args += @($RunIdentityArgs | Select-Object -Skip 1)
    }
    $Args += @(
        "--resume", "$SourceCampaignPath",
        "--resume-checkpoint", "$SourceCheckpointPath",
        "--coverage-gap-limit", "$CoverageGapLimit",
        "--coverage-gap-candidates-per-decision", "$CoverageGapCandidatesPerDecision",
        "--coverage-gap-budget-intent", "$CoverageGapIntent",
        "--coverage-gap-execution-mode", "$DriverExecution",
        "--out", "$RunOutputCampaignPath",
        "--checkpoint-out", "$RunOutputCheckpointPath"
    )
    $Args += @($CoverageGapFilterArgs | Where-Object { $_ })
    $Args += $RoundBudgetArgs
    return Add-CampaignSharedDriverOptions `
        -Arguments $Args `
        -IncludeActiveLineageDiversity $false `
        -IncludeBossRelicAxes $false `
        -IncludeAutoCaptureCombat $true `
        -OptionContext $OptionContext
}
