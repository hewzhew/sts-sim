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
        [string[]] $RunIdentityArgs,
        [string] $SourceCampaignPath,
        [string] $SourceCheckpointPath,
        [string] $RunOutputCampaignPath,
        [string] $RunOutputCheckpointPath,
        [string[]] $RoundBudgetArgs,
        [string] $DriverExecution,
        [object] $OptionContext
    )

    $Args = @($RunIdentityArgs)
    $Args[0] = "continue"
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
        -IncludeAutoCaptureCombat $true `
        -OptionContext $OptionContext
}

function Write-CoverageGapContinuationDryRunCommands {
    param(
        [bool] $PlanCoverageGaps,
        [bool] $ContinueCoverageGaps,
        [bool] $UntilMilestoneBound,
        [string] $DriverExe,
        [string[]] $CoveragePlanArgs,
        [string[]] $ContinueCoverageGapArgs,
        [string[]] $RunIdentityArgs,
        [int] $MilestoneStepRounds,
        [object] $OptionContext,
        [string[]] $CoverageGapMilestoneSummaryArgs
    )

    if ($PlanCoverageGaps) {
        Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments $CoveragePlanArgs)
    }
    if ($ContinueCoverageGaps) {
        Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments $ContinueCoverageGapArgs)
    }
    if ($UntilMilestoneBound) {
        Write-Host "milestone-loop-command-template:"
        Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments (New-MilestoneResumeDriverArgs -RunIdentityArgs $RunIdentityArgs -StepRounds $MilestoneStepRounds -OptionContext $OptionContext))
        if ($ContinueCoverageGaps) {
            Write-Host "milestone-summary-command:"
            Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments $CoverageGapMilestoneSummaryArgs)
        }
    }
}

function Invoke-CoverageGapMilestoneSummary {
    param(
        [string] $RunOutputCampaignPath,
        [string] $DriverExe,
        [string[]] $CoverageGapMilestoneSummaryArgs
    )

    if (-not (Test-Path -LiteralPath $RunOutputCampaignPath)) {
        Write-Host "coverage-gap-milestone-summary=skipped missing-report=$RunOutputCampaignPath"
        return 0
    }

    Write-Host "coverage-gap-milestone-summary:"
    & $DriverExe @CoverageGapMilestoneSummaryArgs | ForEach-Object { Write-Host $_ }
    return $LASTEXITCODE
}

function Invoke-CoverageGapContinuationCommands {
    param(
        [bool] $PlanCoverageGaps,
        [bool] $ContinueCoverageGaps,
        [string] $DriverExe,
        [string[]] $CoveragePlanArgs,
        [string[]] $ContinueCoverageGapArgs,
        [bool] $UntilMilestoneBound,
        [int] $CoverageGapInitialSpentRounds,
        [string[]] $RunIdentityArgs,
        [object] $OptionContext,
        [object] $RecordContext,
        [object] $ManifestContext,
        [string[]] $CoverageGapMilestoneSummaryArgs
    )

    if ($PlanCoverageGaps) {
        & $DriverExe @CoveragePlanArgs
        return $LASTEXITCODE
    }
    if (-not $ContinueCoverageGaps) {
        return 0
    }

    & $DriverExe @ContinueCoverageGapArgs
    $DriverExitCode = $LASTEXITCODE
    if ($DriverExitCode -ne 0) {
        return $DriverExitCode
    }

    Write-CampaignPrimaryDriverCommandRecord `
        -PrimaryDriverCommandLine (Format-CommandLine -ExePath $DriverExe -Arguments $ContinueCoverageGapArgs) `
        -Context $RecordContext
    Write-CampaignWrapperManifest `
        -Path $RecordContext.RunManifestPath `
        -Manifest (New-CoverageGapWrapperManifest `
            -ExitCode $DriverExitCode `
            -Stage "initial_driver_completed" `
            -RunIdentityArgs $RunIdentityArgs `
            -OptionContext $OptionContext `
            -RecordContext $RecordContext `
            -ManifestContext $ManifestContext)
    if ($UntilMilestoneBound) {
        Invoke-CampaignUntilMilestone -AlreadySpentRounds $CoverageGapInitialSpentRounds -RunIdentityArgs $RunIdentityArgs -OptionContext $OptionContext
        $DriverExitCode = $script:CampaignMilestoneExitCode
        if ($DriverExitCode -eq 0) {
            $DriverExitCode = Invoke-CoverageGapMilestoneSummary `
                -RunOutputCampaignPath $RecordContext.RunOutputCampaignPath `
                -DriverExe $DriverExe `
                -CoverageGapMilestoneSummaryArgs $CoverageGapMilestoneSummaryArgs
        }
    }
    $ManifestStage = if ($UntilMilestoneBound) { "completed_with_milestone_loop" } else { "completed" }
    Write-CampaignWrapperManifest `
        -Path $RecordContext.RunManifestPath `
        -Manifest (New-CoverageGapWrapperManifest `
            -ExitCode $DriverExitCode `
            -Stage $ManifestStage `
            -RunIdentityArgs $RunIdentityArgs `
            -OptionContext $OptionContext `
            -RecordContext $RecordContext `
            -ManifestContext $ManifestContext)
    return $DriverExitCode
}

function New-CoverageGapWrapperManifest {
    param(
        [int] $ExitCode,
        [string] $Stage,
        [string[]] $RunIdentityArgs,
        [object] $OptionContext,
        [object] $RecordContext,
        [object] $ManifestContext
    )

    $Manifest = New-CampaignWrapperManifestBase `
        -ExitCode $ExitCode `
        -Stage $Stage `
        -CommandKind "coverage_gap_continuation" `
        -PrimaryDriverArgs $ManifestContext.ContinueCoverageGapArgs `
        -PrimaryDriverCommand (Format-CommandLine -ExePath $ManifestContext.DriverExe -Arguments $ManifestContext.ContinueCoverageGapArgs) `
        -Context $RecordContext
    $Manifest["source"] = [ordered]@{
        label = $ManifestContext.SourceLabel
        report = "$($ManifestContext.SourceCampaignPath)"
        checkpoint = "$($ManifestContext.SourceCheckpointPath)"
    }
    $Manifest["coverage_gap"] = [ordered]@{
        limit = $ManifestContext.CoverageGapLimit
        candidates_per_decision = $ManifestContext.CoverageGapCandidatesPerDecision
        intent = $ManifestContext.CoverageGapIntent
        execution = $ManifestContext.CoverageGapExecutionLabel
        seed_execution = $ManifestContext.CoverageGapDriverExecution
        filter = $ManifestContext.CoverageGapFilterLabel
        result_filter = $ManifestContext.CoverageGapResultFilterLabel
    }

    if ($ManifestContext.UntilMilestoneBound) {
        $MilestoneResumeArgs = New-MilestoneResumeDriverArgs `
            -RunIdentityArgs $RunIdentityArgs `
            -StepRounds $ManifestContext.MilestoneStepRounds `
            -OptionContext $OptionContext
        $MilestoneSummaryArgs = @($ManifestContext.CoverageGapMilestoneSummaryArgs)
        $Manifest["milestone"] = [ordered]@{
            target = $ManifestContext.UntilMilestone
            stop = $ManifestContext.ResolvedMilestoneStop
            step_rounds = $ManifestContext.MilestoneStepRounds
            max_additional_rounds = $ManifestContext.MilestoneMaxRounds
            initial_spent_rounds = $ManifestContext.CoverageGapInitialSpentRounds
            resume_driver_args_template = @($MilestoneResumeArgs)
            resume_driver_command_template = (Format-CommandLine -ExePath $ManifestContext.DriverExe -Arguments $MilestoneResumeArgs)
            summary_driver_args = @($MilestoneSummaryArgs)
            summary_driver_command = (Format-CommandLine -ExePath $ManifestContext.DriverExe -Arguments $MilestoneSummaryArgs)
        }
    }

    return $Manifest
}
