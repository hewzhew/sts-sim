function Test-CampaignDetailedInspect {
    return (
        $InspectState -or
        $InspectShopEvidence -or
        $InspectShopChallenge -or
        $InspectCardRewardEvidence -or
        $InspectDecisionObservations -or
        $InspectJournal -or
        $InspectLineageDecisions -or
        $InspectCampfireEvidence -or
        $InspectDeckMutation -or
        $InspectRouteEvidence -or
        $InspectLastAutoCombat -or
        $InspectCombatLab -or
        $InspectFinalBossCombat -or
        $InspectCoverageGapMilestoneSummary -or
        $InspectCoverageGapTargetState
    )
}

function New-CampaignInspectDriverArgs {
    param(
        [string] $InspectCheckpointPath,
        [string] $InspectCampaignPath
    )

    if ($ExportLearningDataset) {
        return @(
            "dataset",
            "--inspect-checkpoint", "$InspectCheckpointPath",
            "--inspect-report", "$InspectCampaignPath",
            "--export-learning-dataset", "$ExportLearningDataset"
        )
    }

    $Args = @(
        "inspect",
        "--inspect-checkpoint", "$InspectCheckpointPath",
        "--inspect-report", "$InspectCampaignPath",
        "--branch-examples", "$BranchExamples"
    )

    if (-not (Test-CampaignDetailedInspect)) {
        $Args += "--inspect-summary"
    }
    if ($InspectShopEvidence) {
        $Args += "--inspect-shop-evidence"
    }
    if ($InspectShopChallenge) {
        $Args += @(
            "--challenge-shop-plans",
            "--challenge-max-plans", "$ChallengeMaxPlans",
            "--challenge-depth", "$ChallengeDepth",
            "--challenge-max-branches", "$ChallengeMaxBranches",
            "--search-wall-ms", "$SearchWallMs",
            "--search-max-nodes", "$SearchMaxNodes"
        )
    }
    if ($InspectCardRewardEvidence) {
        $Args += "--inspect-card-reward-evidence"
    }
    if ($InspectDecisionObservations) {
        $Args += "--inspect-decision-observations"
    }
    if ($InspectJournal) {
        $Args += "--inspect-journal"
    }
    if ($InspectLineageDecisions) {
        $Args += "--inspect-lineage-decisions"
    }
    if ($InspectCampfireEvidence) {
        $Args += "--inspect-campfire-evidence"
    }
    if ($InspectDeckMutation) {
        $Args += "--inspect-deck-mutation"
    }
    if ($InspectRouteEvidence) {
        $Args += "--inspect-route-evidence"
    }
    if ($InspectLastAutoCombat) {
        $Args += "--inspect-last-auto-combat"
    }
    if ($InspectFinalBossCombat) {
        $Args += "--inspect-final-boss-combat"
    }
    if ($InspectCoverageGapMilestoneSummary) {
        $Args += @(
            "--inspect-coverage-gap-milestone-summary",
            "--coverage-gap-milestone-target", "$CoverageGapMilestoneTarget"
        )
        $Args += $CoverageGapFilterArgs
    }
    if ($InspectCoverageGapTargetState) {
        $Args += @(
            "--inspect-coverage-gap-target-state",
            "--coverage-gap-milestone-target", "$CoverageGapMilestoneTarget"
        )
        $Args += $CoverageGapFilterArgs
    }
    if ($InspectCombatLab) {
        $Args += @(
            "--inspect-combat-lab",
            "--combat-search-option", "wall_ms=$SearchWallMs",
            "--combat-search-option", "max_nodes=$SearchMaxNodes"
        )
        if ($ProbeBoss) {
            $Args += "--probe-boss"
        }
    }
    if ($CampaignBoundParameters.ContainsKey("InspectIndex") -and $InspectIndex -ge 0) {
        $Args += @("--inspect-index", "$InspectIndex")
    }
    if ($CampaignBoundParameters.ContainsKey("InspectAct") -and $InspectAct -gt 0) {
        $Args += @("--inspect-act", "$InspectAct")
    }
    if ($CampaignBoundParameters.ContainsKey("InspectFloor") -and $InspectFloor -gt 0) {
        $Args += @("--inspect-floor", "$InspectFloor")
    }
    if ($InspectBoundary) {
        $Args += @("--inspect-boundary", "$InspectBoundary")
    }
    if ($InspectQuery) {
        $Args += @("--inspect-query", "$InspectQuery")
    }

    return $Args
}
