function Invoke-CampaignEntryDispatch {
    param(
        [object] $Context
    )

    $CoverageGap = $Context.CoverageGapSwitchContext
    $Inspect = $Context.InspectSwitchContext
    $RunSwitch = $Context.RunSwitchContext

    switch ($Context.CampaignRequest.Kind) {
        { @("plan_coverage_gaps", "continue_coverage_gaps") -contains $_ } {
            $ContinuationEntryContext = New-CampaignContinuationEntryContext `
                -CampaignRequest $Context.CampaignRequest `
                -WrapperScript $Context.WrapperScript `
                -Mode $Context.Mode `
                -RunOutputContext $Context.RunOutputContext `
                -BoundParameterContext $Context.BoundParameterContext `
                -CampaignSourceArtifact $Context.CampaignSourceArtifact `
                -FromScratchLatest ([bool] $Context.FromScratchLatest) `
                -CoverageGapExecution $CoverageGap.Execution `
                -CoverageGapIntent $CoverageGap.Intent `
                -CoverageGapFilterLabel $CoverageGap.FilterContext.FilterLabel `
                -CoverageGapFilterArgs $CoverageGap.FilterContext.FilterArgs `
                -CoverageGapResultFilterArgs $CoverageGap.FilterContext.ResultFilterArgs `
                -CoverageGapResultFilterLabel $CoverageGap.FilterContext.ResultFilterLabel `
                -CampaignRunIdentityArgs $Context.CampaignRunIdentityArgs `
                -CampaignSharedDriverOptionContext $Context.CampaignSharedDriverOptionContext `
                -Seed $Context.Seed `
                -Ascension $Context.Ascension `
                -Class $Context.Class `
                -BuildContext $Context.BuildContext `
                -NeedsBuild ([bool] $Context.NeedsBuild) `
                -Scratch ([bool] $RunSwitch.Scratch) `
                -RunRoundContext $Context.RunRoundContext `
                -CoverageGapLimit $CoverageGap.Limit `
                -CoverageGapCandidatesPerDecision $CoverageGap.CandidatesPerDecision `
                -DryRun ([bool] $RunSwitch.DryRun) `
                -RepoRoot $Context.RepoRoot
            return Invoke-CampaignContinuationEntry -Context $ContinuationEntryContext
        }

        "inspect" {
            $InspectOptionContext = New-CampaignInspectOptionContext `
                -BoundParameters $Context.BoundParameterContext.CampaignBoundParameters `
                -InspectState ([bool] $Inspect.State) `
                -InspectShopEvidence ([bool] $Inspect.ShopEvidence) `
                -InspectShopChallenge ([bool] $Inspect.ShopChallenge) `
                -InspectCardRewardEvidence ([bool] $Inspect.CardRewardEvidence) `
                -InspectDecisionObservations ([bool] $Inspect.DecisionObservations) `
                -InspectJournal ([bool] $Inspect.Journal) `
                -InspectLineageDecisions ([bool] $Inspect.LineageDecisions) `
                -InspectCampfireEvidence ([bool] $Inspect.CampfireEvidence) `
                -InspectDeckMutation ([bool] $Inspect.DeckMutation) `
                -InspectRouteEvidence ([bool] $Inspect.RouteEvidence) `
                -InspectLastAutoCombat ([bool] $Inspect.LastAutoCombat) `
                -InspectCombatLab ([bool] $Inspect.CombatLab) `
                -InspectFinalBossCombat ([bool] $Inspect.FinalBossCombat) `
                -InspectCoverageGapMilestoneSummary ([bool] $Inspect.CoverageGapMilestoneSummary) `
                -InspectCoverageGapTargetState ([bool] $Inspect.CoverageGapTargetState) `
                -ExportLearningDataset $Context.ExportLearningDataset `
                -BranchExamples $Inspect.BranchExamples `
                -ChallengeMaxPlans $Inspect.ChallengeMaxPlans `
                -ChallengeDepth $Inspect.ChallengeDepth `
                -ChallengeMaxBranches $Inspect.ChallengeMaxBranches `
                -SearchWallMs $Inspect.SearchWallMs `
                -SearchMaxNodes $Inspect.SearchMaxNodes `
                -CoverageGapMilestoneTarget $CoverageGap.MilestoneTarget `
                -CoverageGapFilterArgs $CoverageGap.FilterContext.FilterArgs `
                -InspectIndex $Inspect.Index `
                -InspectAct $Inspect.Act `
                -InspectFloor $Inspect.Floor `
                -InspectBoundary $Context.CampaignRequest.InspectBoundary `
                -InspectQuery $Inspect.Query `
                -ProbeDetail $Inspect.ProbeDetail `
                -ProbeBoss ([bool] $Inspect.ProbeBoss)
            $InspectEntryContext = New-CampaignInspectEntryContext `
                -CampaignRequest $Context.CampaignRequest `
                -CampaignSourceArtifact $Context.CampaignSourceArtifact `
                -InspectOptionContext $InspectOptionContext `
                -InspectArtifacts ([bool] $Inspect.Artifacts) `
                -ExportLearningDataset $Context.ExportLearningDataset `
                -Seed $Context.Seed `
                -Ascension $Context.Ascension `
                -Class $Context.Class `
                -BuildContext $Context.BuildContext `
                -NeedsBuild ([bool] $Context.NeedsBuild) `
                -InspectCoverageGapMilestoneSummary ([bool] $Inspect.CoverageGapMilestoneSummary) `
                -CoverageGapFilterLabel $CoverageGap.FilterContext.FilterLabel `
                -DryRun ([bool] $RunSwitch.DryRun) `
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
                -Scratch ([bool] $RunSwitch.Scratch) `
                -BuildContext $Context.BuildContext `
                -RunOutputContext $Context.RunOutputContext `
                -BoundParameterContext $Context.BoundParameterContext `
                -RunRoundContext $Context.RunRoundContext `
                -DriverPassthroughContext $Context.DriverPassthroughContext `
                -DriverArgs $RunDriverArgsContext.DriverArgs `
                -NeedsBuild ([bool] $Context.NeedsBuild) `
                -DryRun ([bool] $RunSwitch.DryRun) `
                -Log ([bool] $RunSwitch.Log) `
                -BossRelicAxes ([bool] $RunSwitch.BossRelicAxes) `
                -CombatSegmentMode $RunDriverArgsContext.CombatSegmentMode `
                -UntilMilestone $RunSwitch.UntilMilestone `
                -MilestoneStepRounds $RunSwitch.MilestoneStepRounds `
                -MilestoneMaxRounds $RunSwitch.MilestoneMaxRounds `
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
