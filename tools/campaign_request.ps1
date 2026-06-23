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
        [bool] $UsesCoverageGap,
        [bool] $UsesLegacyTargeted
    )

    return [pscustomobject]@{
        SchemaName = "CampaignEntryRequestV1"
        Kind = $Kind
        SourceIntent = $SourceIntent
        OutputIntent = $OutputIntent
        ContinueCampaign = [bool] $ContinueCampaign
        Inspect = [bool] $Inspect
        InspectBoundary = $InspectBoundary
        ScratchLatestIsContinuationSource = [bool] $ScratchLatestIsContinuationSource
        ReadsCampaignSource = [bool] $ReadsCampaignSource
        IsContinuationFamily = [bool] $IsContinuationFamily
        UsesCoverageGap = [bool] $UsesCoverageGap
        UsesLegacyTargeted = [bool] $UsesLegacyTargeted
    }
}

function Get-CampaignEntryRequestKind {
    param(
        [bool] $ContinueCampaign,
        [bool] $Inspect,
        [bool] $PlanTargets,
        [bool] $ContinueTargets,
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
    if ($PlanTargets) {
        $Kinds += "legacy_plan_targets"
    }
    if ($ContinueTargets) {
        $Kinds += "legacy_continue_targets"
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
    return "new_run"
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

    $Kind = Get-CampaignEntryRequestKind `
        -ContinueCampaign $ContinueCampaign `
        -Inspect $ResolvedInspect `
        -PlanTargets $PlanTargets `
        -ContinueTargets $ContinueTargets `
        -PlanCoverageGaps $PlanCoverageGaps `
        -ContinueCoverageGaps $ContinueCoverageGaps

    $IsContinuationFamily = [bool] ($PlanTargets -or $ContinueTargets -or $PlanCoverageGaps -or $ContinueCoverageGaps)
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
        "new_run" { "campaign_output" }
        "continue_run" { "campaign_output" }
        "legacy_continue_targets" { "campaign_output" }
        "continue_coverage_gaps" { "campaign_output" }
        default { "none" }
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
        -UsesCoverageGap ($PlanCoverageGaps -or $ContinueCoverageGaps) `
        -UsesLegacyTargeted ($PlanTargets -or $ContinueTargets)
}
