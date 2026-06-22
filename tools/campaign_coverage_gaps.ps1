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
        [string] $SourceCheckpointPath
    )

    $Args = @(
        "dataset",
        "--inspect-report", "$SourceCampaignPath",
        "--inspect-checkpoint", "$SourceCheckpointPath",
        "--plan-coverage-gap-continuation",
        "--coverage-gap-limit", "$CoverageGapLimit",
        "--coverage-gap-candidates-per-decision", "$CoverageGapCandidatesPerDecision"
    )
    $Args += $CoverageGapFilterArgs
    return $Args
}

function New-CoverageGapContinueDriverArgs {
    param(
        [string] $SourceCampaignPath,
        [string] $SourceCheckpointPath,
        [string] $RunOutputCampaignPath,
        [string] $RunOutputCheckpointPath,
        [string[]] $RoundBudgetArgs,
        [string] $DriverExecution
    )

    $Args = @(
        "continue",
        "--preset", "$Mode",
        "--seed", "$Seed",
        "--ascension", "$Ascension",
        "--class", "$Class"
    )
    if (@(0, 10, 15, 17, 20) -contains $Ascension) {
        $Args += @("--ascension-domain", "a$Ascension")
    }
    $Args += @(
        "--resume", "$SourceCampaignPath",
        "--resume-checkpoint", "$SourceCheckpointPath",
        "--execute-coverage-gap-continuation",
        "--coverage-gap-limit", "$CoverageGapLimit",
        "--coverage-gap-candidates-per-decision", "$CoverageGapCandidatesPerDecision",
        "--coverage-gap-budget-intent", "$CoverageGapIntent",
        "--coverage-gap-execution-mode", "$DriverExecution",
        "--out", "$RunOutputCampaignPath",
        "--checkpoint-out", "$RunOutputCheckpointPath"
    )
    $Args += $CoverageGapFilterArgs
    $Args += $RoundBudgetArgs
    return Add-CampaignSharedDriverOptions `
        -Arguments $Args `
        -IncludeActiveLineageDiversity $false `
        -IncludeBossRelicAxes $false `
        -IncludeAutoCaptureCombat $true
}

function New-CoverageGapMilestoneSummaryArgs {
    $Args = @(
        "inspect",
        "--inspect-report", "$RunOutputCampaignPath",
        "--inspect-coverage-gap-milestone-summary",
        "--coverage-gap-milestone-target", "$UntilMilestone"
    )
    $Args += $CoverageGapResultFilterArgs
    if (Test-Path -LiteralPath $RunOutputCheckpointPath) {
        $Args += @("--inspect-checkpoint", "$RunOutputCheckpointPath")
    }
    return $Args
}

function Write-CoverageGapContinuationDryRunCommands {
    param(
        [bool] $PlanCoverageGaps,
        [bool] $ContinueCoverageGaps,
        [bool] $UntilMilestoneBound,
        [string] $DriverExe,
        [string[]] $CoveragePlanArgs,
        [string[]] $ContinueCoverageGapArgs,
        [int] $MilestoneStepRounds
    )

    if ($PlanCoverageGaps) {
        Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments $CoveragePlanArgs)
    }
    if ($ContinueCoverageGaps) {
        Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments $ContinueCoverageGapArgs)
    }
    if ($UntilMilestoneBound) {
        Write-Host "milestone-loop-command-template:"
        Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments (New-MilestoneResumeDriverArgs -StepRounds $MilestoneStepRounds))
        if ($ContinueCoverageGaps) {
            Write-Host "milestone-summary-command:"
            Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments (New-CoverageGapMilestoneSummaryArgs))
        }
    }
}

function Invoke-CoverageGapMilestoneSummary {
    param(
        [string] $Target
    )

    if (-not (Test-Path -LiteralPath $RunOutputCampaignPath)) {
        Write-Host "coverage-gap-milestone-summary=skipped missing-report=$RunOutputCampaignPath"
        return 0
    }

    $SummaryArgs = @(
        "inspect",
        "--inspect-report", "$RunOutputCampaignPath",
        "--inspect-coverage-gap-milestone-summary",
        "--coverage-gap-milestone-target", "$Target"
    )
    $SummaryArgs += $CoverageGapResultFilterArgs
    if (Test-Path -LiteralPath $RunOutputCheckpointPath) {
        $SummaryArgs += @("--inspect-checkpoint", "$RunOutputCheckpointPath")
    }
    Write-Host "coverage-gap-milestone-summary:"
    & $DriverExe @SummaryArgs | ForEach-Object { Write-Host $_ }
    return $LASTEXITCODE
}

function New-CoverageGapWrapperManifest {
    param(
        [int] $ExitCode,
        [string] $Stage
    )

    $Manifest = New-CampaignWrapperManifestBase `
        -ExitCode $ExitCode `
        -Stage $Stage `
        -CommandKind "coverage_gap_continuation" `
        -PrimaryDriverArgs $ContinueCoverageGapArgs `
        -PrimaryDriverCommand (Format-CommandLine -ExePath $DriverExe -Arguments $ContinueCoverageGapArgs)
    $Manifest["source"] = [ordered]@{
        label = $SourceLabel
        report = "$SourceCampaignPath"
        checkpoint = "$SourceCheckpointPath"
    }
    $Manifest["coverage_gap"] = [ordered]@{
        limit = $CoverageGapLimit
        candidates_per_decision = $CoverageGapCandidatesPerDecision
        intent = $CoverageGapIntent
        execution = $CoverageGapExecutionLabel
        seed_execution = $CoverageGapDriverExecution
        filter = $CoverageGapFilterLabel
        result_filter = $CoverageGapResultFilterLabel
    }

    if ($UntilMilestoneBound) {
        $MilestoneResumeArgs = New-MilestoneResumeDriverArgs -StepRounds $MilestoneStepRounds
        $MilestoneSummaryArgs = New-CoverageGapMilestoneSummaryArgs
        $Manifest["milestone"] = [ordered]@{
            target = $UntilMilestone
            stop = $ResolvedMilestoneStop
            step_rounds = $MilestoneStepRounds
            max_additional_rounds = $MilestoneMaxRounds
            initial_spent_rounds = $CoverageGapInitialSpentRounds
            resume_driver_args_template = @($MilestoneResumeArgs)
            resume_driver_command_template = (Format-CommandLine -ExePath $DriverExe -Arguments $MilestoneResumeArgs)
            summary_driver_args = @($MilestoneSummaryArgs)
            summary_driver_command = (Format-CommandLine -ExePath $DriverExe -Arguments $MilestoneSummaryArgs)
        }
    }

    return $Manifest
}
