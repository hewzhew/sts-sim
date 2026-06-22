function New-TargetedContinuationExportBeforeArgs {
    param(
        [string] $SourceCampaignPath,
        [string] $SourceCheckpointPath,
        [string] $TargetDecisionOutcomePath
    )

    return @(
        "dataset",
        "--inspect-report", "$SourceCampaignPath",
        "--inspect-checkpoint", "$SourceCheckpointPath",
        "--export-decision-outcome-dataset", "$TargetDecisionOutcomePath"
    )
}

function New-TargetedContinuationPlanDriverArgs {
    param(
        [string] $TargetDecisionOutcomePath
    )

    return @("continue", "--plan-targeted-continuation", "$TargetDecisionOutcomePath")
}

function New-TargetedContinuationExportAfterArgs {
    param(
        [string] $RunOutputCampaignPath,
        [string] $RunOutputCheckpointPath,
        [string] $LatestDecisionOutcomeAfterPath
    )

    return @(
        "dataset",
        "--inspect-report", "$RunOutputCampaignPath",
        "--inspect-checkpoint", "$RunOutputCheckpointPath",
        "--export-decision-outcome-dataset", "$LatestDecisionOutcomeAfterPath"
    )
}

function New-TargetedContinuationEffectArgs {
    param(
        [string] $TargetDecisionOutcomePath,
        [string] $LatestDecisionOutcomeAfterPath
    )

    return @(
        "continue",
        "--continuation-effect-before", "$TargetDecisionOutcomePath",
        "--continuation-effect-after", "$LatestDecisionOutcomeAfterPath"
    )
}

function New-TargetedContinuationContinueDriverArgs {
    param(
        [string[]] $RunIdentityArgs,
        [string] $SourceCampaignPath,
        [string] $SourceCheckpointPath,
        [string] $RunOutputCampaignPath,
        [string] $RunOutputCheckpointPath,
        [string] $TargetDecisionOutcomePath,
        [string[]] $RoundBudgetArgs,
        [object] $OptionContext
    )

    $Args = @($RunIdentityArgs)
    $Args[0] = "continue"
    $Args += @(
        "--resume", "$SourceCampaignPath",
        "--resume-checkpoint", "$SourceCheckpointPath",
        "--execute-targeted-continuation", "$TargetDecisionOutcomePath",
        "--targeted-continuation-limit", "$TargetedContinuationLimit",
        "--targeted-continuation-candidates-per-target", "$TargetedContinuationCandidatesPerTarget",
        "--out", "$RunOutputCampaignPath",
        "--checkpoint-out", "$RunOutputCheckpointPath"
    )
    $Args += $RoundBudgetArgs
    return Add-CampaignSharedDriverOptions `
        -Arguments $Args `
        -IncludeActiveLineageDiversity $false `
        -IncludeBossRelicAxes $false `
        -IncludeAutoCaptureCombat $true `
        -OptionContext $OptionContext
}

function Write-TargetedContinuationDryRunCommands {
    param(
        [bool] $PlanTargets,
        [bool] $ContinueTargets,
        [string] $DriverExe,
        [string[]] $ExportDecisionArgs,
        [string[]] $PlanTargetArgs,
        [string[]] $ContinueTargetArgs,
        [string[]] $ExportDecisionAfterArgs,
        [string[]] $ContinuationEffectArgs
    )

    if ($PlanTargets -or $ContinueTargets) {
        Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments $ExportDecisionArgs)
    }
    if ($PlanTargets) {
        Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments $PlanTargetArgs)
    }
    if ($ContinueTargets) {
        Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments $ContinueTargetArgs)
        Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments $ExportDecisionAfterArgs)
        Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments $ContinuationEffectArgs)
    }
}

function Invoke-TargetedContinuationCommands {
    param(
        [bool] $PlanTargets,
        [bool] $ContinueTargets,
        [string] $DriverExe,
        [string[]] $ExportDecisionArgs,
        [string[]] $PlanTargetArgs,
        [string[]] $ContinueTargetArgs,
        [string[]] $ExportDecisionAfterArgs,
        [string[]] $ContinuationEffectArgs,
        [bool] $UntilMilestoneBound,
        [int] $ContinuationRounds,
        [string[]] $RunIdentityArgs,
        [object] $OptionContext
    )

    if ($PlanTargets -or $ContinueTargets) {
        & $DriverExe @ExportDecisionArgs
        if ($LASTEXITCODE -ne 0) {
            return $LASTEXITCODE
        }
    }
    if ($PlanTargets) {
        & $DriverExe @PlanTargetArgs
        return $LASTEXITCODE
    }
    if (-not $ContinueTargets) {
        return 0
    }

    & $DriverExe @ContinueTargetArgs
    $DriverExitCode = $LASTEXITCODE
    if ($DriverExitCode -ne 0) {
        return $DriverExitCode
    }

    & $DriverExe @ExportDecisionAfterArgs
    if ($LASTEXITCODE -ne 0) {
        return $LASTEXITCODE
    }
    & $DriverExe @ContinuationEffectArgs
    if ($LASTEXITCODE -ne 0) {
        return $LASTEXITCODE
    }
    Write-CampaignPrimaryDriverCommandRecord -PrimaryDriverCommandLine (Format-CommandLine -ExePath $DriverExe -Arguments $ContinueTargetArgs)
    if ($UntilMilestoneBound) {
        Invoke-CampaignUntilMilestone -AlreadySpentRounds $ContinuationRounds -RunIdentityArgs $RunIdentityArgs -OptionContext $OptionContext
        $DriverExitCode = $script:CampaignMilestoneExitCode
    }
    return $DriverExitCode
}
