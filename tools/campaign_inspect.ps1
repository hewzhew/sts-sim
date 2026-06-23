function Test-CampaignDetailedInspect {
    param(
        [object] $Options
    )

    return (
        $Options.InspectState -or
        $Options.InspectShopEvidence -or
        $Options.InspectShopChallenge -or
        $Options.InspectCardRewardEvidence -or
        $Options.InspectDecisionObservations -or
        $Options.InspectJournal -or
        $Options.InspectLineageDecisions -or
        $Options.InspectCampfireEvidence -or
        $Options.InspectDeckMutation -or
        $Options.InspectRouteEvidence -or
        $Options.InspectLastAutoCombat -or
        $Options.InspectCombatLab -or
        $Options.InspectFinalBossCombat -or
        $Options.InspectCoverageGapMilestoneSummary -or
        $Options.InspectCoverageGapTargetState
    )
}

function New-CampaignInspectOptionContext {
    param(
        [System.Collections.IDictionary] $BoundParameters,
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
        [string] $ExportLearningDataset,
        [int] $BranchExamples,
        [int] $ChallengeMaxPlans,
        [int] $ChallengeDepth,
        [int] $ChallengeMaxBranches,
        [int] $SearchWallMs,
        [int] $SearchMaxNodes,
        [string] $CoverageGapMilestoneTarget,
        [string[]] $CoverageGapFilterArgs,
        [int] $InspectIndex,
        [int] $InspectAct,
        [int] $InspectFloor,
        [string] $InspectBoundary,
        [string] $InspectQuery,
        [bool] $ProbeBoss
    )

    return [pscustomobject]@{
        BoundParameters = $BoundParameters
        InspectState = $InspectState
        InspectShopEvidence = $InspectShopEvidence
        InspectShopChallenge = $InspectShopChallenge
        InspectCardRewardEvidence = $InspectCardRewardEvidence
        InspectDecisionObservations = $InspectDecisionObservations
        InspectJournal = $InspectJournal
        InspectLineageDecisions = $InspectLineageDecisions
        InspectCampfireEvidence = $InspectCampfireEvidence
        InspectDeckMutation = $InspectDeckMutation
        InspectRouteEvidence = $InspectRouteEvidence
        InspectLastAutoCombat = $InspectLastAutoCombat
        InspectCombatLab = $InspectCombatLab
        InspectFinalBossCombat = $InspectFinalBossCombat
        InspectCoverageGapMilestoneSummary = $InspectCoverageGapMilestoneSummary
        InspectCoverageGapTargetState = $InspectCoverageGapTargetState
        ExportLearningDataset = $ExportLearningDataset
        BranchExamples = $BranchExamples
        ChallengeMaxPlans = $ChallengeMaxPlans
        ChallengeDepth = $ChallengeDepth
        ChallengeMaxBranches = $ChallengeMaxBranches
        SearchWallMs = $SearchWallMs
        SearchMaxNodes = $SearchMaxNodes
        CoverageGapMilestoneTarget = $CoverageGapMilestoneTarget
        CoverageGapFilterArgs = @($CoverageGapFilterArgs | Where-Object { $_ })
        InspectIndex = $InspectIndex
        InspectAct = $InspectAct
        InspectFloor = $InspectFloor
        InspectBoundary = $InspectBoundary
        InspectQuery = $InspectQuery
        ProbeBoss = $ProbeBoss
    }
}

function New-CampaignInspectEntryContext {
    param(
        [object] $CampaignRequest,
        [object] $CampaignSourceArtifact,
        [object] $InspectOptionContext,
        [bool] $InspectArtifacts,
        [string] $ExportLearningDataset,
        [long] $Seed,
        [int] $Ascension,
        [string] $Class,
        [object] $BuildContext,
        [bool] $NeedsBuild,
        [bool] $InspectCoverageGapMilestoneSummary,
        [string] $CoverageGapFilterLabel,
        [bool] $DryRun,
        [string] $RepoRoot
    )

    return [pscustomobject]@{
        CampaignRequest = $CampaignRequest
        CampaignSourceArtifact = $CampaignSourceArtifact
        InspectOptionContext = $InspectOptionContext
        InspectArtifacts = $InspectArtifacts
        ExportLearningDataset = $ExportLearningDataset
        Seed = $Seed
        Ascension = $Ascension
        Class = $Class
        BuildProfile = $BuildContext.BuildProfile
        DriverExe = $BuildContext.DriverExe
        NeedsBuild = $NeedsBuild
        InspectCoverageGapMilestoneSummary = $InspectCoverageGapMilestoneSummary
        CoverageGapFilterLabel = $CoverageGapFilterLabel
        DryRun = $DryRun
        BuildArgs = @($BuildContext.BuildArgs)
        RepoRoot = $RepoRoot
    }
}

function New-CampaignInspectDriverArgs {
    param(
        [string] $InspectCheckpointPath,
        [string] $InspectCampaignPath,
        [object] $Options
    )

    if ($Options.ExportLearningDataset) {
        return @(
            "dataset",
            "--inspect-checkpoint", "$InspectCheckpointPath",
            "--inspect-report", "$InspectCampaignPath",
            "--export-learning-dataset", "$($Options.ExportLearningDataset)"
        )
    }

    $Args = @(
        "inspect",
        "--inspect-checkpoint", "$InspectCheckpointPath",
        "--inspect-report", "$InspectCampaignPath",
        "--branch-examples", "$($Options.BranchExamples)"
    )

    if (-not (Test-CampaignDetailedInspect -Options $Options)) {
        $Args += "--inspect-summary"
    }
    if ($Options.InspectShopEvidence) {
        $Args += "--inspect-shop-evidence"
    }
    if ($Options.InspectShopChallenge) {
        $Args += @(
            "--challenge-shop-plans",
            "--challenge-max-plans", "$($Options.ChallengeMaxPlans)",
            "--challenge-depth", "$($Options.ChallengeDepth)",
            "--challenge-max-branches", "$($Options.ChallengeMaxBranches)",
            "--search-wall-ms", "$($Options.SearchWallMs)",
            "--search-max-nodes", "$($Options.SearchMaxNodes)"
        )
    }
    if ($Options.InspectCardRewardEvidence) {
        $Args += "--inspect-card-reward-evidence"
    }
    if ($Options.InspectDecisionObservations) {
        $Args += "--inspect-decision-observations"
    }
    if ($Options.InspectJournal) {
        $Args += "--inspect-journal"
    }
    if ($Options.InspectLineageDecisions) {
        $Args += "--inspect-lineage-decisions"
    }
    if ($Options.InspectCampfireEvidence) {
        $Args += "--inspect-campfire-evidence"
    }
    if ($Options.InspectDeckMutation) {
        $Args += "--inspect-deck-mutation"
    }
    if ($Options.InspectRouteEvidence) {
        $Args += "--inspect-route-evidence"
    }
    if ($Options.InspectLastAutoCombat) {
        $Args += "--inspect-last-auto-combat"
    }
    if ($Options.InspectFinalBossCombat) {
        $Args += "--inspect-final-boss-combat"
    }
    if ($Options.InspectCoverageGapMilestoneSummary) {
        $Args += @(
            "--inspect-coverage-gap-milestone-summary",
            "--coverage-gap-milestone-target", "$($Options.CoverageGapMilestoneTarget)"
        )
        $Args += $Options.CoverageGapFilterArgs
    }
    if ($Options.InspectCoverageGapTargetState) {
        $Args += @(
            "--inspect-coverage-gap-target-state",
            "--coverage-gap-milestone-target", "$($Options.CoverageGapMilestoneTarget)"
        )
        $Args += $Options.CoverageGapFilterArgs
    }
    if ($Options.InspectCombatLab) {
        $Args += @(
            "--inspect-combat-lab",
            "--combat-search-option", "wall_ms=$($Options.SearchWallMs)",
            "--combat-search-option", "max_nodes=$($Options.SearchMaxNodes)"
        )
        if ($Options.ProbeBoss) {
            $Args += "--probe-boss"
        }
    }
    if ($Options.BoundParameters.ContainsKey("InspectIndex") -and $Options.InspectIndex -ge 0) {
        $Args += @("--inspect-index", "$($Options.InspectIndex)")
    }
    if ($Options.BoundParameters.ContainsKey("InspectAct") -and $Options.InspectAct -gt 0) {
        $Args += @("--inspect-act", "$($Options.InspectAct)")
    }
    if ($Options.BoundParameters.ContainsKey("InspectFloor") -and $Options.InspectFloor -gt 0) {
        $Args += @("--inspect-floor", "$($Options.InspectFloor)")
    }
    if ($Options.InspectBoundary) {
        $Args += @("--inspect-boundary", "$($Options.InspectBoundary)")
    }
    if ($Options.InspectQuery) {
        $Args += @("--inspect-query", "$($Options.InspectQuery)")
    }

    return $Args
}

function Write-CampaignInspectPreflight {
    param(
        [string] $ModeLabel,
        [string] $SourceLabel,
        [long] $Seed,
        [int] $Ascension,
        [string] $Class,
        [string] $BuildProfile,
        [string] $DriverExe,
        [bool] $NeedsBuild,
        [bool] $CoverageGapMilestoneSummary,
        [string] $CoverageGapFilterLabel,
        [string] $InspectCampaignPath,
        [string] $InspectCheckpointPath
    )

    Write-Host "mode=$ModeLabel $SourceLabel branch campaign"
    if ($Seed -gt 0) {
        Write-Host "seed=$Seed"
    }
    Write-Host "ascension=A$Ascension domain=a$Ascension class=$Class"
    Write-Host "build=$BuildProfile exe=$DriverExe"
    if ($NeedsBuild) {
        Write-Host "build-needed=yes"
    } else {
        Write-Host "build-needed=no"
    }
    if ($CoverageGapMilestoneSummary) {
        Write-Host "coverage-gap-filter=$CoverageGapFilterLabel"
    }
    Write-Host "report=$InspectCampaignPath"
    Write-Host "checkpoint=$InspectCheckpointPath"
}

function Invoke-CampaignInspectCommand {
    param(
        [bool] $DryRun,
        [bool] $NeedsBuild,
        [string[]] $BuildArgs,
        [string] $RepoRoot,
        [string] $DriverExe,
        [string[]] $InspectArgs
    )

    if ($DryRun) {
        if ($NeedsBuild) {
            Write-CampaignBuildCommandPreview -BuildArgs $BuildArgs
        }
        Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments $InspectArgs)
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
        & $DriverExe @InspectArgs
        return $LASTEXITCODE
    } finally {
        Pop-Location
    }
}

function Invoke-CampaignInspectEntry {
    param(
        [object] $Context
    )

    $InspectSource = $Context.CampaignSourceArtifact
    if (-not $InspectSource) {
        throw "Internal error: campaign inspect did not resolve a source artifact."
    }
    $InspectCampaignPath = $InspectSource.ReportPath
    $InspectCheckpointPath = $InspectSource.CheckpointPath
    $InspectManifestPath = $InspectSource.ManifestPath
    $InspectLogPath = $InspectSource.LogPath
    $InspectCommandPath = $InspectSource.CommandPath
    $InspectSourceLabel = $InspectSource.Label

    if ($Context.InspectArtifacts) {
        Write-CampaignArtifactSummary `
            -SourceLabel $InspectSourceLabel `
            -ReportPath $InspectCampaignPath `
            -CheckpointPath $InspectCheckpointPath `
            -ManifestPath $InspectManifestPath `
            -LogPath $InspectLogPath `
            -CommandPath $InspectCommandPath
        return 0
    }

    if (-not (Test-Path $InspectCheckpointPath)) {
        throw "No previous campaign checkpoint found at $InspectCheckpointPath. Run .\tools\campaign.ps1 first."
    }
    if (-not (Test-Path $InspectCampaignPath)) {
        throw "No previous campaign report found at $InspectCampaignPath. Run .\tools\campaign.ps1 first."
    }

    $InspectArgs = New-CampaignInspectDriverArgs `
        -InspectCheckpointPath $InspectCheckpointPath `
        -InspectCampaignPath $InspectCampaignPath `
        -Options $Context.InspectOptionContext

    $InspectModeLabel = if ($Context.ExportLearningDataset) { "dataset" } else { "inspect" }
    Write-CampaignInspectPreflight `
        -ModeLabel $InspectModeLabel `
        -SourceLabel $InspectSourceLabel `
        -Seed $Context.Seed `
        -Ascension $Context.Ascension `
        -Class $Context.Class `
        -BuildProfile $Context.BuildProfile `
        -DriverExe $Context.DriverExe `
        -NeedsBuild $Context.NeedsBuild `
        -CoverageGapMilestoneSummary $Context.InspectCoverageGapMilestoneSummary `
        -CoverageGapFilterLabel $Context.CoverageGapFilterLabel `
        -InspectCampaignPath $InspectCampaignPath `
        -InspectCheckpointPath $InspectCheckpointPath
    return Invoke-CampaignInspectCommand `
        -DryRun $Context.DryRun `
        -NeedsBuild $Context.NeedsBuild `
        -BuildArgs $Context.BuildArgs `
        -RepoRoot $Context.RepoRoot `
        -DriverExe $Context.DriverExe `
        -InspectArgs $InspectArgs
}
