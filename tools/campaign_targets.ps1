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
        [string] $SourceCampaignPath,
        [string] $SourceCheckpointPath,
        [string] $RunOutputCampaignPath,
        [string] $RunOutputCheckpointPath,
        [string] $TargetDecisionOutcomePath,
        [string[]] $RoundBudgetArgs
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
        -IncludeAutoCaptureCombat $true
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
