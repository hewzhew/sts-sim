function Get-CampaignInspectSelectorParameterNames {
    return @(
        "InspectArtifacts",
        "InspectState",
        "InspectShopEvidence",
        "InspectShopChallenge",
        "InspectCardRewardEvidence",
        "InspectDecisionObservations",
        "InspectJournal",
        "InspectLineageDecisions",
        "InspectCampfireEvidence",
        "InspectDeckMutation",
        "InspectRouteEvidence",
        "InspectLastAutoCombat",
        "InspectCombatLab",
        "InspectFinalBossCombat",
        "InspectCoverageGapMilestoneSummary",
        "InspectCoverageGapTargetState",
        "Probe"
    )
}

function Test-CampaignAnyInspectSelectorSwitch {
    param(
        [System.Collections.IDictionary] $BoundParameters
    )

    foreach ($Name in (Get-CampaignInspectSelectorParameterNames)) {
        if (-not $BoundParameters.ContainsKey($Name)) {
            continue
        }
        $Value = $BoundParameters[$Name]
        if ($Value -is [array]) {
            if ($Value.Count -gt 0) {
                return $true
            }
            continue
        }
        if ([bool] $Value) {
            return $true
        }
    }
    return $false
}

function New-CampaignInspectProbeContext {
    param(
        [string[]] $Probe
    )

    $Selected = @{}
    foreach ($Name in @($Probe | Where-Object { $_ })) {
        $Selected[$Name] = $true
    }

    return [pscustomobject]@{
        ShopEvidence = $Selected.ContainsKey("shop-evidence")
        ShopChallenge = $Selected.ContainsKey("shop-challenge")
        CardRewardEvidence = $Selected.ContainsKey("card-reward-evidence")
        CampfireEvidence = $Selected.ContainsKey("campfire-evidence")
        DeckMutation = $Selected.ContainsKey("deck-mutation")
        RouteEvidence = $Selected.ContainsKey("route-evidence")
        LastAutoCombat = $Selected.ContainsKey("last-auto-combat")
        CombatLab = $Selected.ContainsKey("combat-lab")
        FinalBossCombat = $Selected.ContainsKey("final-boss-combat")
    }
}

function New-CampaignInspectSwitchContext {
    param(
        [bool] $InspectArtifacts,
        [bool] $InspectState,
        [bool] $InspectShopEvidence,
        [bool] $InspectShopChallenge,
        [bool] $InspectCardRewardEvidence,
        [bool] $InspectDecisionObservations,
        [bool] $InspectJournal,
        [bool] $InspectLineageDecisions,
        [bool] $InspectCampfireEvidence,
        [bool] $InspectDeckMutation,
        [bool] $InspectRouteEvidence,
        [bool] $InspectLastAutoCombat,
        [bool] $InspectCombatLab,
        [bool] $InspectFinalBossCombat,
        [bool] $InspectCoverageGapMilestoneSummary,
        [bool] $InspectCoverageGapTargetState,
        [object] $ProbeContext,
        [int] $BranchExamples,
        [int] $ChallengeMaxPlans,
        [int] $ChallengeDepth,
        [int] $ChallengeMaxBranches,
        [int] $SearchWallMs,
        [int] $SearchMaxNodes,
        [int] $InspectIndex,
        [int] $InspectAct,
        [int] $InspectFloor,
        [string] $InspectQuery,
        [bool] $ProbeBoss
    )

    return [pscustomobject]@{
        Artifacts = [bool] $InspectArtifacts
        State = [bool] $InspectState
        ShopEvidence = [bool] ($InspectShopEvidence -or $ProbeContext.ShopEvidence)
        ShopChallenge = [bool] ($InspectShopChallenge -or $ProbeContext.ShopChallenge)
        CardRewardEvidence = [bool] ($InspectCardRewardEvidence -or $ProbeContext.CardRewardEvidence)
        DecisionObservations = [bool] $InspectDecisionObservations
        Journal = [bool] $InspectJournal
        LineageDecisions = [bool] $InspectLineageDecisions
        CampfireEvidence = [bool] ($InspectCampfireEvidence -or $ProbeContext.CampfireEvidence)
        DeckMutation = [bool] ($InspectDeckMutation -or $ProbeContext.DeckMutation)
        RouteEvidence = [bool] ($InspectRouteEvidence -or $ProbeContext.RouteEvidence)
        LastAutoCombat = [bool] ($InspectLastAutoCombat -or $ProbeContext.LastAutoCombat)
        CombatLab = [bool] ($InspectCombatLab -or $ProbeContext.CombatLab)
        FinalBossCombat = [bool] ($InspectFinalBossCombat -or $ProbeContext.FinalBossCombat)
        CoverageGapMilestoneSummary = [bool] $InspectCoverageGapMilestoneSummary
        CoverageGapTargetState = [bool] $InspectCoverageGapTargetState
        BranchExamples = $BranchExamples
        ChallengeMaxPlans = $ChallengeMaxPlans
        ChallengeDepth = $ChallengeDepth
        ChallengeMaxBranches = $ChallengeMaxBranches
        SearchWallMs = $SearchWallMs
        SearchMaxNodes = $SearchMaxNodes
        Index = $InspectIndex
        Act = $InspectAct
        Floor = $InspectFloor
        Query = $InspectQuery
        ProbeBoss = [bool] $ProbeBoss
    }
}

function New-CampaignEntryRequestDescriptor {
    param(
        [string] $Kind,
        [string] $SourceIntent,
        [string] $OutputIntent,
        [bool] $ContinueCampaign,
        [bool] $Inspect,
        [string] $InspectBoundary,
        [bool] $ScratchLatestIsContinuationSource,
        [bool] $ReadsCampaignSource,
        [bool] $IsContinuationFamily,
        [bool] $UsesCoverageGap
    )

    return [pscustomobject]@{
        SchemaName = "CampaignEntryRequestV1"
        Kind = $Kind
        SourceIntent = $SourceIntent
        OutputIntent = $OutputIntent
        PlanCoverageGaps = [bool] ($Kind -eq "plan_coverage_gaps")
        ContinueCoverageGaps = [bool] ($Kind -eq "continue_coverage_gaps")
        ContinueCampaign = [bool] $ContinueCampaign
        Inspect = [bool] $Inspect
        InspectBoundary = $InspectBoundary
        ScratchLatestIsContinuationSource = [bool] $ScratchLatestIsContinuationSource
        ReadsCampaignSource = [bool] $ReadsCampaignSource
        IsContinuationFamily = [bool] $IsContinuationFamily
        UsesCoverageGap = [bool] $UsesCoverageGap
    }
}

function Get-CampaignEntryRequestKind {
    param(
        [bool] $ContinueCampaign,
        [bool] $Inspect,
        [bool] $PlanCoverageGaps,
        [bool] $ContinueCoverageGaps
    )

    $Kinds = @()
    if ($ContinueCampaign) {
        $Kinds += "continue_run"
    }
    if ($Inspect) {
        $Kinds += "inspect"
    }
    if ($PlanCoverageGaps) {
        $Kinds += "plan_coverage_gaps"
    }
    if ($ContinueCoverageGaps) {
        $Kinds += "continue_coverage_gaps"
    }

    if ($Kinds.Count -gt 1) {
        throw "Choose one campaign request kind, not: $($Kinds -join ', ')."
    }
    if ($Kinds.Count -eq 1) {
        return $Kinds[0]
    }
    return "run"
}

function Resolve-CampaignEntryRequest {
    param(
        [bool] $ContinueRun,
        [bool] $More,
        [bool] $Inspect,
        [bool] $AnyInspectSelector,
        [bool] $InspectScratchLatest,
        [bool] $InspectShopChallenge,
        [bool] $InspectBoundaryBound,
        [string] $InspectBoundary,
        [bool] $PlanCoverageGaps,
        [bool] $ContinueCoverageGaps,
        [bool] $Scratch
    )

    $ContinueCampaign = [bool] $ContinueRun
    if ($More) {
        throw "-More has been retired because it silently mixed latest source, output, and round semantics. Use '.\tools\campaign.ps1 -From latest -Continue' or '.\tools\campaign.ps1 -From run:<id> -Continue'."
    }

    $ScratchLatestIsContinuationSource = $InspectScratchLatest -and (
        $ContinueCampaign -or
        $PlanCoverageGaps -or
        $ContinueCoverageGaps
    )

    $ResolvedInspect = $Inspect
    if (
        $AnyInspectSelector -or
        ($InspectScratchLatest -and -not $ScratchLatestIsContinuationSource)
    ) {
        $ResolvedInspect = $true
    }

    $ResolvedInspectBoundary = $InspectBoundary
    if ($InspectShopChallenge -and -not $InspectBoundaryBound) {
        $ResolvedInspectBoundary = "Shop"
    }

    $Kind = Get-CampaignEntryRequestKind `
        -ContinueCampaign $ContinueCampaign `
        -Inspect $ResolvedInspect `
        -PlanCoverageGaps $PlanCoverageGaps `
        -ContinueCoverageGaps $ContinueCoverageGaps

    $IsContinuationFamily = [bool] ($PlanCoverageGaps -or $ContinueCoverageGaps)
    $ReadsCampaignSource = [bool] ($ResolvedInspect -or $ContinueCampaign -or $IsContinuationFamily)
    $SourceIntent = if ($ReadsCampaignSource) {
        if ($InspectScratchLatest) {
            "scratch_latest"
        } else {
            "campaign_source_selector"
        }
    } else {
        "none"
    }
    $OutputIntent = switch ($Kind) {
        "run" { "campaign_output" }
        "continue_run" { "campaign_output" }
        "continue_coverage_gaps" { "campaign_output" }
        default { "none" }
    }

    if (
        $Scratch -and
        -not (
            $ContinueCoverageGaps -or
            $ContinueCampaign -or
            ((-not $ContinueCampaign) -and (-not $ResolvedInspect) -and (-not $PlanCoverageGaps))
        )
    ) {
        throw "-Scratch currently supports normal campaign runs, -Continue, and -ContinueCoverageGaps only."
    }

    return New-CampaignEntryRequestDescriptor `
        -Kind $Kind `
        -SourceIntent $SourceIntent `
        -OutputIntent $OutputIntent `
        -ContinueCampaign $ContinueCampaign `
        -Inspect $ResolvedInspect `
        -InspectBoundary $ResolvedInspectBoundary `
        -ScratchLatestIsContinuationSource $ScratchLatestIsContinuationSource `
        -ReadsCampaignSource $ReadsCampaignSource `
        -IsContinuationFamily $IsContinuationFamily `
        -UsesCoverageGap ($PlanCoverageGaps -or $ContinueCoverageGaps)
}
