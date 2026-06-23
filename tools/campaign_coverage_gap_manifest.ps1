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
        $MilestoneResumeArgs = New-CampaignMilestoneResumeDriverArgs `
            -MilestoneContext $ManifestContext.MilestoneContext `
            -StepRounds $ManifestContext.MilestoneStepRounds
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
