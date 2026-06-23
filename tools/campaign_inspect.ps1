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
        -InspectCampaignPath $InspectCampaignPath

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
