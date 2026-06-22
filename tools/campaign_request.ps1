function Test-AnyCampaignFlag {
    param(
        [bool[]] $Flags
    )

    foreach ($Flag in $Flags) {
        if ($Flag) {
            return $true
        }
    }
    return $false
}

function Resolve-CampaignEntryRequest {
    param(
        [bool] $ContinueRun,
        [bool] $More,
        [bool] $Inspect,
        [bool[]] $InspectSelectorFlags,
        [bool] $InspectScratchLatest,
        [bool] $InspectShopChallenge,
        [bool] $InspectBoundaryBound,
        [string] $InspectBoundary,
        [bool] $PlanTargets,
        [bool] $ContinueTargets,
        [bool] $PlanCoverageGaps,
        [bool] $ContinueCoverageGaps,
        [bool] $Scratch
    )

    $ContinueCampaign = [bool] $ContinueRun
    if ($More) {
        throw "-More has been retired because it silently mixed latest source, output, and round semantics. Use '.\tools\campaign.ps1 -From latest -Continue' or '.\tools\campaign.ps1 -From run:<id> -Continue'."
    }

    $ScratchLatestIsContinuationSource = $InspectScratchLatest -and (
        $PlanTargets -or
        $ContinueTargets -or
        $PlanCoverageGaps -or
        $ContinueCoverageGaps
    )

    $ResolvedInspect = $Inspect
    if (
        (Test-AnyCampaignFlag -Flags $InspectSelectorFlags) -or
        ($InspectScratchLatest -and -not $ScratchLatestIsContinuationSource)
    ) {
        $ResolvedInspect = $true
    }

    $ResolvedInspectBoundary = $InspectBoundary
    if ($InspectShopChallenge -and -not $InspectBoundaryBound) {
        $ResolvedInspectBoundary = "Shop"
    }

    if (($PlanTargets -or $ContinueTargets) -and ($PlanCoverageGaps -or $ContinueCoverageGaps)) {
        throw "Choose either targeted continuation (-PlanTargets/-ContinueTargets) or coverage-gap continuation (-PlanCoverageGaps/-ContinueCoverageGaps), not both."
    }
    if (
        $Scratch -and
        -not (
            $ContinueCoverageGaps -or
            $ContinueTargets -or
            ((-not $ContinueCampaign) -and (-not $ResolvedInspect) -and (-not $PlanTargets) -and (-not $ContinueTargets) -and (-not $PlanCoverageGaps))
        )
    ) {
        throw "-Scratch currently supports normal campaign runs, -ContinueTargets, and -ContinueCoverageGaps only."
    }

    return [pscustomobject]@{
        ContinueCampaign = [bool] $ContinueCampaign
        Inspect = [bool] $ResolvedInspect
        InspectBoundary = $ResolvedInspectBoundary
        ScratchLatestIsContinuationSource = [bool] $ScratchLatestIsContinuationSource
        ReadsCampaignSource = [bool] ($ResolvedInspect -or $ContinueCampaign -or $PlanTargets -or $ContinueTargets -or $PlanCoverageGaps -or $ContinueCoverageGaps)
    }
}
