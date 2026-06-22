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
