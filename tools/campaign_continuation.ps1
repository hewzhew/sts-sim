function Invoke-CampaignContinuationEntry {
    param(
        [object] $Context
    )

    $InspectScratchLatest = [bool] $Context.InspectScratchLatest
    $PlanTargets = [bool] $Context.PlanTargets
    $ContinueTargets = [bool] $Context.ContinueTargets
    $PlanCoverageGaps = [bool] $Context.PlanCoverageGaps
    $ContinueCoverageGaps = [bool] $Context.ContinueCoverageGaps
    $CampaignSourceArtifact = $Context.CampaignSourceArtifact
    $DecisionOutcomeDataset = $Context.DecisionOutcomeDataset
    $LatestDecisionOutcomeBeforePath = $Context.LatestDecisionOutcomeBeforePath
    $LatestDecisionOutcomePath = $Context.LatestDecisionOutcomePath
    $LatestDecisionOutcomeAfterPath = $Context.LatestDecisionOutcomeAfterPath
    $RunOutputCampaignPath = $Context.RunOutputCampaignPath
    $RunOutputCheckpointPath = $Context.RunOutputCheckpointPath
    $UntilMilestoneBound = [bool] $Context.UntilMilestoneBound
    $MilestoneStepRounds = $Context.MilestoneStepRounds
    $RoundsBound = [bool] $Context.RoundsBound
    $Rounds = $Context.Rounds
    $UntilRoundBound = [bool] $Context.UntilRoundBound
    $UntilRound = $Context.UntilRound
    $MaxRoundsBound = [bool] $Context.MaxRoundsBound
    $MaxRounds = $Context.MaxRounds
    $CoverageGapExecution = $Context.CoverageGapExecution
    $CoverageGapIntent = $Context.CoverageGapIntent
    $CampaignRunIdentityArgs = @($Context.CampaignRunIdentityArgs)
    $CampaignSharedDriverOptionContext = $Context.CampaignSharedDriverOptionContext
    $Seed = $Context.Seed
    $Ascension = $Context.Ascension
    $Class = $Context.Class
    $BuildProfile = $Context.BuildProfile
    $DriverExe = $Context.DriverExe
    $NeedsBuild = [bool] $Context.NeedsBuild
    $Scratch = [bool] $Context.Scratch
    $ScratchLabel = $Context.ScratchLabel
    $TargetedContinuationLimit = $Context.TargetedContinuationLimit
    $TargetedContinuationCandidatesPerTarget = $Context.TargetedContinuationCandidatesPerTarget
    $UntilMilestone = $Context.UntilMilestone
    $MilestoneMaxRounds = $Context.MilestoneMaxRounds
    $ResolvedMilestoneStop = $Context.ResolvedMilestoneStop
    $CoverageGapLimit = $Context.CoverageGapLimit
    $CoverageGapCandidatesPerDecision = $Context.CoverageGapCandidatesPerDecision
    $CoverageGapFilterLabel = $Context.CoverageGapFilterLabel
    $CoverageGapResultFilterLabel = $Context.CoverageGapResultFilterLabel
    $DryRun = [bool] $Context.DryRun
    $BuildArgs = @($Context.BuildArgs)
    $RepoRoot = $Context.RepoRoot

    if ($InspectScratchLatest -and ($PlanTargets -or $ContinueTargets)) {
        throw "-InspectScratchLatest is not supported for targeted continuation yet; use inspect or coverage-gap continuation."
    }
    $ContinuationSource = $CampaignSourceArtifact
    if (-not $ContinuationSource) {
        throw "Internal error: campaign continuation did not resolve a source artifact."
    }
    $SourceCampaignPath = $ContinuationSource.ReportPath
    $SourceCheckpointPath = $ContinuationSource.CheckpointPath
    $SourceLabel = $ContinuationSource.Label

    if (-not (Test-Path $SourceCampaignPath)) {
        throw "No previous campaign report found at $SourceCampaignPath. Run .\tools\campaign.ps1 first."
    }
    if (-not (Test-Path $SourceCheckpointPath)) {
        throw "No previous campaign checkpoint found at $SourceCheckpointPath. Run .\tools\campaign.ps1 first."
    }

    $TargetDecisionOutcomePath = if ($DecisionOutcomeDataset) {
        $DecisionOutcomeDataset
    } elseif ($ContinueTargets) {
        $LatestDecisionOutcomeBeforePath
    } else {
        $LatestDecisionOutcomePath
    }
    $CoveragePlanArgs = New-CoverageGapPlanDriverArgs `
        -SourceCampaignPath $SourceCampaignPath `
        -SourceCheckpointPath $SourceCheckpointPath
    $ExportDecisionArgs = New-TargetedContinuationExportBeforeArgs `
        -SourceCampaignPath $SourceCampaignPath `
        -SourceCheckpointPath $SourceCheckpointPath `
        -TargetDecisionOutcomePath $TargetDecisionOutcomePath
    $PlanTargetArgs = New-TargetedContinuationPlanDriverArgs `
        -TargetDecisionOutcomePath $TargetDecisionOutcomePath
    $ExportDecisionAfterArgs = New-TargetedContinuationExportAfterArgs `
        -RunOutputCampaignPath $RunOutputCampaignPath `
        -RunOutputCheckpointPath $RunOutputCheckpointPath `
        -LatestDecisionOutcomeAfterPath $LatestDecisionOutcomeAfterPath
    $ContinuationEffectArgs = New-TargetedContinuationEffectArgs `
        -TargetDecisionOutcomePath $TargetDecisionOutcomePath `
        -LatestDecisionOutcomeAfterPath $LatestDecisionOutcomeAfterPath

    $ResumeReport = Get-Content -LiteralPath $SourceCampaignPath -Raw | ConvertFrom-Json
    $ResumeRoundsCompleted = [int] $ResumeReport.rounds_completed
    $ContinuationRoundBudget = Resolve-CampaignAdditionalRoundBudget `
        -ResumeRoundsCompleted $ResumeRoundsCompleted `
        -UntilMilestoneBound $UntilMilestoneBound `
        -MilestoneStepRounds $MilestoneStepRounds `
        -RoundsBound $RoundsBound `
        -Rounds $Rounds `
        -UntilRoundBound $UntilRoundBound `
        -UntilRound $UntilRound `
        -MaxRoundsBound $MaxRoundsBound `
        -MaxRounds $MaxRounds `
        -MaxRoundsDriverFlag "--max-rounds"
    $ContinuationRounds = $ContinuationRoundBudget.AdditionalRounds
    $ContinuationRoundBudgetArgs = @($ContinuationRoundBudget.Args)
    $TargetRounds = $ContinuationRoundBudget.TargetRounds
    $ContinuationRoundSource = $ContinuationRoundBudget.Source
    $CoverageGapExecutionContext = Resolve-CoverageGapExecutionContext `
        -Execution $CoverageGapExecution `
        -UntilMilestoneBound $UntilMilestoneBound `
        -ContinueCoverageGaps ([bool] $ContinueCoverageGaps) `
        -HasExplicitRoundBudget ($RoundsBound -or $UntilRoundBound -or $MaxRoundsBound) `
        -Intent $CoverageGapIntent `
        -ContinuationRounds $ContinuationRounds
    $CoverageGapExecutionLabel = $CoverageGapExecutionContext.Label
    $CoverageGapDriverExecution = $CoverageGapExecutionContext.DriverExecution
    $CoverageGapInitialSpentRounds = $CoverageGapExecutionContext.InitialSpentRounds

    $ContinueTargetArgs = New-TargetedContinuationContinueDriverArgs `
        -RunIdentityArgs $CampaignRunIdentityArgs `
        -SourceCampaignPath $SourceCampaignPath `
        -SourceCheckpointPath $SourceCheckpointPath `
        -RunOutputCampaignPath $RunOutputCampaignPath `
        -RunOutputCheckpointPath $RunOutputCheckpointPath `
        -TargetDecisionOutcomePath $TargetDecisionOutcomePath `
        -RoundBudgetArgs $ContinuationRoundBudgetArgs `
        -OptionContext $CampaignSharedDriverOptionContext
    $ContinueCoverageGapArgs = New-CoverageGapContinueDriverArgs `
        -RunIdentityArgs $CampaignRunIdentityArgs `
        -SourceCampaignPath $SourceCampaignPath `
        -SourceCheckpointPath $SourceCheckpointPath `
        -RunOutputCampaignPath $RunOutputCampaignPath `
        -RunOutputCheckpointPath $RunOutputCheckpointPath `
        -RoundBudgetArgs $ContinuationRoundBudgetArgs `
        -DriverExecution $CoverageGapDriverExecution `
        -OptionContext $CampaignSharedDriverOptionContext

    $ContinuationPreflightContext = New-CampaignContinuationPreflightContext `
        -PlanTargets ([bool] $PlanTargets) `
        -ContinueTargets ([bool] $ContinueTargets) `
        -PlanCoverageGaps ([bool] $PlanCoverageGaps) `
        -ContinueCoverageGaps ([bool] $ContinueCoverageGaps) `
        -Seed $Seed `
        -Ascension $Ascension `
        -Class $Class `
        -BuildProfile $BuildProfile `
        -DriverExe $DriverExe `
        -NeedsBuild ([bool] $NeedsBuild) `
        -SourceLabel $SourceLabel `
        -SourceCampaignPath $SourceCampaignPath `
        -SourceCheckpointPath $SourceCheckpointPath `
        -Scratch ([bool] $Scratch) `
        -ScratchLabel $ScratchLabel `
        -RunOutputCampaignPath $RunOutputCampaignPath `
        -RunOutputCheckpointPath $RunOutputCheckpointPath `
        -TargetDecisionOutcomePath $TargetDecisionOutcomePath `
        -LatestDecisionOutcomeAfterPath $LatestDecisionOutcomeAfterPath `
        -TargetedContinuationLimit $TargetedContinuationLimit `
        -TargetedContinuationCandidatesPerTarget $TargetedContinuationCandidatesPerTarget `
        -ResumeRoundsCompleted $ResumeRoundsCompleted `
        -TargetRounds $TargetRounds `
        -ContinuationRoundSource $ContinuationRoundSource `
        -ContinuationRounds $ContinuationRounds `
        -UntilMilestoneBound $UntilMilestoneBound `
        -UntilMilestone $UntilMilestone `
        -MilestoneStepRounds $MilestoneStepRounds `
        -MilestoneMaxRounds $MilestoneMaxRounds `
        -ResolvedMilestoneStop $ResolvedMilestoneStop `
        -CoverageGapLimit $CoverageGapLimit `
        -CoverageGapCandidatesPerDecision $CoverageGapCandidatesPerDecision `
        -CoverageGapIntent $CoverageGapIntent `
        -CoverageGapExecutionLabel $CoverageGapExecutionLabel `
        -CoverageGapDriverExecution $CoverageGapDriverExecution `
        -CoverageGapFilterLabel $CoverageGapFilterLabel `
        -CoverageGapInitialSpentRounds $CoverageGapInitialSpentRounds `
        -CoverageGapResultFilterLabel $CoverageGapResultFilterLabel
    Write-CampaignContinuationPreflight -Context $ContinuationPreflightContext

    if ($DryRun) {
        if ($NeedsBuild) {
            Write-CampaignBuildCommandPreview -BuildArgs $BuildArgs
        }
        Write-TargetedContinuationDryRunCommands `
            -PlanTargets ([bool] $PlanTargets) `
            -ContinueTargets ([bool] $ContinueTargets) `
            -DriverExe $DriverExe `
            -ExportDecisionArgs $ExportDecisionArgs `
            -PlanTargetArgs $PlanTargetArgs `
            -ContinueTargetArgs $ContinueTargetArgs `
            -ExportDecisionAfterArgs $ExportDecisionAfterArgs `
            -ContinuationEffectArgs $ContinuationEffectArgs
        Write-CoverageGapContinuationDryRunCommands `
            -PlanCoverageGaps ([bool] $PlanCoverageGaps) `
            -ContinueCoverageGaps ([bool] $ContinueCoverageGaps) `
            -UntilMilestoneBound $UntilMilestoneBound `
            -DriverExe $DriverExe `
            -CoveragePlanArgs $CoveragePlanArgs `
            -ContinueCoverageGapArgs $ContinueCoverageGapArgs `
            -RunIdentityArgs $CampaignRunIdentityArgs `
            -MilestoneStepRounds $MilestoneStepRounds `
            -OptionContext $CampaignSharedDriverOptionContext
        return 0
    }

    if (($ContinueTargets -or $ContinueCoverageGaps) -and $ContinuationRounds -eq 0) {
        Write-Host "already-at-target-rounds=yes; nothing to run"
        return 0
    }

    Push-Location $RepoRoot
    try {
        if ($NeedsBuild) {
            & cargo @BuildArgs
            if ($LASTEXITCODE -ne 0) {
                return $LASTEXITCODE
            }
        }
        if ($PlanTargets -or $ContinueTargets) {
            return Invoke-TargetedContinuationCommands `
                -PlanTargets ([bool] $PlanTargets) `
                -ContinueTargets ([bool] $ContinueTargets) `
                -DriverExe $DriverExe `
                -ExportDecisionArgs $ExportDecisionArgs `
                -PlanTargetArgs $PlanTargetArgs `
                -ContinueTargetArgs $ContinueTargetArgs `
                -ExportDecisionAfterArgs $ExportDecisionAfterArgs `
                -ContinuationEffectArgs $ContinuationEffectArgs `
                -UntilMilestoneBound $UntilMilestoneBound `
                -ContinuationRounds $ContinuationRounds `
                -RunIdentityArgs $CampaignRunIdentityArgs `
                -OptionContext $CampaignSharedDriverOptionContext
        }
        if ($PlanCoverageGaps -or $ContinueCoverageGaps) {
            return Invoke-CoverageGapContinuationCommands `
                -PlanCoverageGaps ([bool] $PlanCoverageGaps) `
                -ContinueCoverageGaps ([bool] $ContinueCoverageGaps) `
                -DriverExe $DriverExe `
                -CoveragePlanArgs $CoveragePlanArgs `
                -ContinueCoverageGapArgs $ContinueCoverageGapArgs `
                -UntilMilestoneBound $UntilMilestoneBound `
                -CoverageGapInitialSpentRounds $CoverageGapInitialSpentRounds `
                -RunIdentityArgs $CampaignRunIdentityArgs `
                -OptionContext $CampaignSharedDriverOptionContext
        }
        return 0
    } finally {
        Pop-Location
    }
}
