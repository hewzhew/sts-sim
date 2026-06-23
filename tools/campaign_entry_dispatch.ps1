function Invoke-CampaignEntryDispatch {
    param(
        [object] $Context
    )

    switch ($Context.CampaignRequest.Kind) {
        { @("plan_coverage_gaps", "continue_coverage_gaps") -contains $_ } {
            $ContinuationEntryContext = New-CampaignContinuationEntryContext `
                -CampaignRequest $Context.CampaignRequest `
                -WrapperScript $Context.WrapperScript `
                -Mode $Context.Mode `
                -RunOutputContext $Context.RunOutputContext `
                -BoundParameterContext $Context.BoundParameterContext `
                -CampaignSourceArtifact $Context.CampaignSourceArtifact `
                -InspectScratchLatest ([bool] $Context.InspectScratchLatest) `
                -CoverageGapExecution $Context.CoverageGapExecution `
                -CoverageGapIntent $Context.CoverageGapIntent `
                -CoverageGapFilterLabel $Context.CoverageGapFilterContext.FilterLabel `
                -CoverageGapFilterArgs $Context.CoverageGapFilterContext.FilterArgs `
                -CoverageGapResultFilterArgs $Context.CoverageGapFilterContext.ResultFilterArgs `
                -CoverageGapResultFilterLabel $Context.CoverageGapFilterContext.ResultFilterLabel `
                -CampaignRunIdentityArgs $Context.CampaignRunIdentityArgs `
                -CampaignSharedDriverOptionContext $Context.CampaignSharedDriverOptionContext `
                -Seed $Context.Seed `
                -Ascension $Context.Ascension `
                -Class $Context.Class `
                -BuildContext $Context.BuildContext `
                -NeedsBuild ([bool] $Context.NeedsBuild) `
                -Scratch ([bool] $Context.Scratch) `
                -RunRoundContext $Context.RunRoundContext `
                -CoverageGapLimit $Context.CoverageGapLimit `
                -CoverageGapCandidatesPerDecision $Context.CoverageGapCandidatesPerDecision `
                -DryRun ([bool] $Context.DryRun) `
                -RepoRoot $Context.RepoRoot
            return Invoke-CampaignContinuationEntry -Context $ContinuationEntryContext
        }

        "inspect" {
            $InspectOptionContext = New-CampaignInspectOptionContext `
                -BoundParameters $Context.BoundParameterContext.CampaignBoundParameters `
                -InspectState ([bool] $Context.InspectState) `
                -InspectShopEvidence ([bool] $Context.InspectShopEvidence) `
                -InspectShopChallenge ([bool] $Context.InspectShopChallenge) `
                -InspectCardRewardEvidence ([bool] $Context.InspectCardRewardEvidence) `
                -InspectDecisionObservations ([bool] $Context.InspectDecisionObservations) `
                -InspectJournal ([bool] $Context.InspectJournal) `
                -InspectLineageDecisions ([bool] $Context.InspectLineageDecisions) `
                -InspectCampfireEvidence ([bool] $Context.InspectCampfireEvidence) `
                -InspectDeckMutation ([bool] $Context.InspectDeckMutation) `
                -InspectRouteEvidence ([bool] $Context.InspectRouteEvidence) `
                -InspectLastAutoCombat ([bool] $Context.InspectLastAutoCombat) `
                -InspectCombatLab ([bool] $Context.InspectCombatLab) `
                -InspectFinalBossCombat ([bool] $Context.InspectFinalBossCombat) `
                -InspectCoverageGapMilestoneSummary ([bool] $Context.InspectCoverageGapMilestoneSummary) `
                -InspectCoverageGapTargetState ([bool] $Context.InspectCoverageGapTargetState) `
                -ExportLearningDataset $Context.ExportLearningDataset `
                -BranchExamples $Context.BranchExamples `
                -ChallengeMaxPlans $Context.ChallengeMaxPlans `
                -ChallengeDepth $Context.ChallengeDepth `
                -ChallengeMaxBranches $Context.ChallengeMaxBranches `
                -SearchWallMs $Context.SearchWallMs `
                -SearchMaxNodes $Context.SearchMaxNodes `
                -CoverageGapMilestoneTarget $Context.CoverageGapMilestoneTarget `
                -CoverageGapFilterArgs $Context.CoverageGapFilterContext.FilterArgs `
                -InspectIndex $Context.InspectIndex `
                -InspectAct $Context.InspectAct `
                -InspectFloor $Context.InspectFloor `
                -InspectBoundary $Context.CampaignRequest.InspectBoundary `
                -InspectQuery $Context.InspectQuery `
                -ProbeBoss ([bool] $Context.ProbeBoss)
            $InspectEntryContext = New-CampaignInspectEntryContext `
                -CampaignRequest $Context.CampaignRequest `
                -CampaignSourceArtifact $Context.CampaignSourceArtifact `
                -InspectOptionContext $InspectOptionContext `
                -InspectArtifacts ([bool] $Context.InspectArtifacts) `
                -ExportLearningDataset $Context.ExportLearningDataset `
                -Seed $Context.Seed `
                -Ascension $Context.Ascension `
                -Class $Context.Class `
                -BuildContext $Context.BuildContext `
                -NeedsBuild ([bool] $Context.NeedsBuild) `
                -InspectCoverageGapMilestoneSummary ([bool] $Context.InspectCoverageGapMilestoneSummary) `
                -CoverageGapFilterLabel $Context.CoverageGapFilterContext.FilterLabel `
                -DryRun ([bool] $Context.DryRun) `
                -RepoRoot $Context.RepoRoot
            return Invoke-CampaignInspectEntry -Context $InspectEntryContext
        }

        { @("run", "continue_run") -contains $_ } {
            $RunDriverArgsContext = New-CampaignRunDriverArgsContext `
                -RunIdentityArgs $Context.CampaignRunIdentityArgs `
                -Request $Context.CampaignRequest `
                -RunOutputContext $Context.RunOutputContext `
                -RunRoundContext $Context.RunRoundContext `
                -OptionContext $Context.CampaignSharedDriverOptionContext `
                -ExportLearningDataset $Context.ExportLearningDataset
            $RunCommandContext = New-CampaignRunCommandContext `
                -CampaignRequest $Context.CampaignRequest `
                -WrapperScript $Context.WrapperScript `
                -RepoRoot $Context.RepoRoot `
                -Mode $Context.Mode `
                -Seed $Context.Seed `
                -Ascension $Context.Ascension `
                -Class $Context.Class `
                -Scratch ([bool] $Context.Scratch) `
                -BuildContext $Context.BuildContext `
                -RunOutputContext $Context.RunOutputContext `
                -BoundParameterContext $Context.BoundParameterContext `
                -RunRoundContext $Context.RunRoundContext `
                -DriverPassthroughContext $Context.DriverPassthroughContext `
                -DriverArgs $RunDriverArgsContext.DriverArgs `
                -NeedsBuild ([bool] $Context.NeedsBuild) `
                -DryRun ([bool] $Context.DryRun) `
                -Log ([bool] $Context.Log) `
                -BossRelicAxes ([bool] $Context.BossRelicAxes) `
                -CombatSegmentMode $RunDriverArgsContext.CombatSegmentMode `
                -UntilMilestone $Context.UntilMilestone `
                -MilestoneStepRounds $Context.MilestoneStepRounds `
                -MilestoneMaxRounds $Context.MilestoneMaxRounds `
                -ResolvedMilestoneStop $Context.RunRoundContext.ResolvedMilestoneStop
            Write-CampaignRunPreflight -Context $RunCommandContext
            return Invoke-CampaignRunCommand `
                -Context $RunCommandContext `
                -RunIdentityArgs $Context.CampaignRunIdentityArgs `
                -OptionContext $Context.CampaignSharedDriverOptionContext
        }

        default {
            throw "Unknown campaign command kind: $($Context.CampaignRequest.Kind)"
        }
    }
}
