function Write-CoverageGapContinuationDryRunCommands {
    param(
        [bool] $PlanCoverageGaps,
        [bool] $ContinueCoverageGaps,
        [bool] $UntilMilestoneBound,
        [string] $DriverExe,
        [string[]] $CoveragePlanArgs,
        [string[]] $ContinueCoverageGapArgs,
        [object] $MilestoneContext,
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
        Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments (New-CampaignMilestoneResumeDriverArgs -MilestoneContext $MilestoneContext -StepRounds $MilestoneContext.MilestoneStepRounds))
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
        [object] $MilestoneContext,
        [object] $RecordContext,
        [object] $ManifestContext,
        [string[]] $CoverageGapMilestoneSummaryArgs
    )

    if ($PlanCoverageGaps) {
        & $DriverExe @CoveragePlanArgs | ForEach-Object { Write-Host $_ }
        return $LASTEXITCODE
    }
    if (-not $ContinueCoverageGaps) {
        return 0
    }

    & $DriverExe @ContinueCoverageGapArgs | ForEach-Object { Write-Host $_ }
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
        $DriverExitCode = Invoke-CampaignUntilMilestone `
            -MilestoneContext $MilestoneContext `
            -AlreadySpentRounds $CoverageGapInitialSpentRounds
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
