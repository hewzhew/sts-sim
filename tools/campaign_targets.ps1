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

function Resolve-TargetedContinuationPlanOutcomePath {
    param(
        [object] $Request,
        [string] $DecisionOutcomeDataset,
        [long] $Seed,
        [bool] $DryRun
    )

    if ((-not $Request.PlanTargets) -or $DecisionOutcomeDataset) {
        return ""
    }

    $Path = New-CampaignScratchDecisionOutcomePath -BaseLabel "plan-targets-seed$Seed"
    if (-not $DryRun) {
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Path) | Out-Null
    }
    return $Path
}

function Resolve-TargetedContinuationDecisionOutcomePathContext {
    param(
        [object] $Request,
        [object] $RunOutputContext,
        [string] $DecisionOutcomeDataset,
        [long] $Seed,
        [bool] $DryRun
    )

    $LatestPathContext = Get-CampaignLatestDecisionOutcomePathContext
    $PlanDecisionOutcomePath = Resolve-TargetedContinuationPlanOutcomePath `
        -Request $Request `
        -DecisionOutcomeDataset $DecisionOutcomeDataset `
        -Seed $Seed `
        -DryRun $DryRun

    return [pscustomobject]@{
        BeforePath = $(if ($RunOutputContext.DecisionOutcomeBeforePath) { $RunOutputContext.DecisionOutcomeBeforePath } else { $LatestPathContext.BeforePath })
        Path = $(if ($RunOutputContext.DecisionOutcomePath) { $RunOutputContext.DecisionOutcomePath } elseif ($PlanDecisionOutcomePath) { $PlanDecisionOutcomePath } else { $LatestPathContext.Path })
        AfterPath = $(if ($RunOutputContext.DecisionOutcomeAfterPath) { $RunOutputContext.DecisionOutcomeAfterPath } else { $LatestPathContext.AfterPath })
        PlanPath = $PlanDecisionOutcomePath
        LatestPath = $LatestPathContext.Path
        LatestBeforePath = $LatestPathContext.BeforePath
        LatestAfterPath = $LatestPathContext.AfterPath
    }
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
        [string] $DecisionOutcomeAfterPath
    )

    return @(
        "dataset",
        "--inspect-report", "$RunOutputCampaignPath",
        "--inspect-checkpoint", "$RunOutputCheckpointPath",
        "--export-decision-outcome-dataset", "$DecisionOutcomeAfterPath"
    )
}

function New-TargetedContinuationEffectArgs {
    param(
        [string] $TargetDecisionOutcomePath,
        [string] $DecisionOutcomeAfterPath
    )

    return @(
        "continue",
        "--continuation-effect-before", "$TargetDecisionOutcomePath",
        "--continuation-effect-after", "$DecisionOutcomeAfterPath"
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
        [object] $OptionContext,
        [object] $RecordContext,
        [object] $ManifestContext
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
    Write-CampaignPrimaryDriverCommandRecord `
        -PrimaryDriverCommandLine (Format-CommandLine -ExePath $DriverExe -Arguments $ContinueTargetArgs) `
        -Context $RecordContext
    Write-CampaignWrapperManifest `
        -Path $RecordContext.RunManifestPath `
        -Manifest (New-TargetedContinuationWrapperManifest `
            -ExitCode $DriverExitCode `
            -Stage "initial_driver_completed" `
            -RunIdentityArgs $RunIdentityArgs `
            -OptionContext $OptionContext `
            -RecordContext $RecordContext `
            -ManifestContext $ManifestContext)
    if ($UntilMilestoneBound) {
        Invoke-CampaignUntilMilestone -AlreadySpentRounds $ContinuationRounds -RunIdentityArgs $RunIdentityArgs -OptionContext $OptionContext
        $DriverExitCode = $script:CampaignMilestoneExitCode
    }
    $ManifestStage = if ($UntilMilestoneBound) { "completed_with_milestone_loop" } else { "completed" }
    Write-CampaignWrapperManifest `
        -Path $RecordContext.RunManifestPath `
        -Manifest (New-TargetedContinuationWrapperManifest `
            -ExitCode $DriverExitCode `
            -Stage $ManifestStage `
            -RunIdentityArgs $RunIdentityArgs `
            -OptionContext $OptionContext `
            -RecordContext $RecordContext `
            -ManifestContext $ManifestContext)
    return $DriverExitCode
}

function New-TargetedContinuationWrapperManifest {
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
        -CommandKind "targeted_continuation" `
        -PrimaryDriverArgs $ManifestContext.ContinueTargetArgs `
        -PrimaryDriverCommand (Format-CommandLine -ExePath $ManifestContext.DriverExe -Arguments $ManifestContext.ContinueTargetArgs) `
        -Context $RecordContext
    $Manifest["source"] = [ordered]@{
        label = $ManifestContext.SourceLabel
        report = "$($ManifestContext.SourceCampaignPath)"
        checkpoint = "$($ManifestContext.SourceCheckpointPath)"
    }
    $Manifest["targeted_continuation"] = [ordered]@{
        limit = $ManifestContext.TargetedContinuationLimit
        candidates_per_target = $ManifestContext.TargetedContinuationCandidatesPerTarget
        decision_outcomes_before = "$($ManifestContext.TargetDecisionOutcomePath)"
        decision_outcomes_after = "$($ManifestContext.DecisionOutcomeAfterPath)"
        export_before_args = @($ManifestContext.ExportDecisionArgs)
        export_before_command = (Format-CommandLine -ExePath $ManifestContext.DriverExe -Arguments $ManifestContext.ExportDecisionArgs)
        export_after_args = @($ManifestContext.ExportDecisionAfterArgs)
        export_after_command = (Format-CommandLine -ExePath $ManifestContext.DriverExe -Arguments $ManifestContext.ExportDecisionAfterArgs)
        effect_args = @($ManifestContext.ContinuationEffectArgs)
        effect_command = (Format-CommandLine -ExePath $ManifestContext.DriverExe -Arguments $ManifestContext.ContinuationEffectArgs)
    }

    if ($ManifestContext.UntilMilestoneBound) {
        $MilestoneResumeArgs = New-MilestoneResumeDriverArgs `
            -RunIdentityArgs $RunIdentityArgs `
            -StepRounds $ManifestContext.MilestoneStepRounds `
            -OptionContext $OptionContext
        $Manifest["milestone"] = [ordered]@{
            target = $ManifestContext.UntilMilestone
            stop = $ManifestContext.ResolvedMilestoneStop
            step_rounds = $ManifestContext.MilestoneStepRounds
            max_additional_rounds = $ManifestContext.MilestoneMaxRounds
            initial_spent_rounds = $ManifestContext.ContinuationRounds
            resume_driver_args_template = @($MilestoneResumeArgs)
            resume_driver_command_template = (Format-CommandLine -ExePath $ManifestContext.DriverExe -Arguments $MilestoneResumeArgs)
        }
    }

    return $Manifest
}
