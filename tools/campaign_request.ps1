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
        "InspectCoverageGapTargetState"
    )
}

function Test-CampaignAnyInspectSelectorSwitch {
    param(
        [System.Collections.IDictionary] $BoundParameters
    )

    foreach ($Name in (Get-CampaignInspectSelectorParameterNames)) {
        if ($BoundParameters.ContainsKey($Name) -and [bool] $BoundParameters[$Name]) {
            return $true
        }
    }
    return $false
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
            ((-not $ContinueCampaign) -and (-not $ResolvedInspect) -and (-not $PlanCoverageGaps))
        )
    ) {
        throw "-Scratch currently supports normal campaign runs and -ContinueCoverageGaps only."
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
