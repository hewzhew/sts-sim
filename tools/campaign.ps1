<#
.SYNOPSIS
Runs the explore branch campaign with baby-friendly defaults.

.EXAMPLE
.\tools\campaign.ps1
Runs an explore campaign on a random seed.

.EXAMPLE
.\tools\campaign.ps1 521
Runs the same explore campaign on seed 521.

.EXAMPLE
.\tools\campaign.ps1 -Last
Reuses the last non-dry-run campaign seed.

.EXAMPLE
.\tools\campaign.ps1 -More
Resumes the latest saved campaign report with the previous mode.

.EXAMPLE
.\tools\campaign.ps1 -More -Rounds 1
Resumes the latest saved campaign report and advances exactly one additional round.

.EXAMPLE
.\tools\campaign.ps1 -More -UntilRound 33
Resumes the latest saved campaign report and advances until total round 33.

.EXAMPLE
.\tools\campaign.ps1 -Inspect
Summarizes the latest saved campaign checkpoint with active/frozen/abandoned deck context.

.EXAMPLE
.\tools\campaign.ps1 -InspectShopEvidence -InspectIndex 0
Prints shop compiler evidence for a selected checkpoint branch.

.EXAMPLE
.\tools\campaign.ps1 -InspectShopChallenge -InspectIndex 0
Runs selected and alternative shop plans from a selected checkpoint branch, then rolls each forward briefly.

.EXAMPLE
.\tools\campaign.ps1 -InspectCardRewardEvidence -InspectIndex 0
Prints card reward compiler evidence for a selected checkpoint branch.

.EXAMPLE
.\tools\campaign.ps1 -InspectDecisionObservations -InspectQuery "Iron Wave"
Prints saved historical reward option portfolios matching a candidate or semantic class.

.EXAMPLE
.\tools\campaign.ps1 -InspectJournal -InspectQuery "shop"
Prints saved CampaignJournal decision events matching a boundary, candidate, or semantic field.

.EXAMPLE
.\tools\campaign.ps1 -InspectDeckMutation -InspectIndex 0
Prints DeckMutationCompiler evidence for a selected checkpoint branch.

.EXAMPLE
.\tools\campaign.ps1 -InspectCampfireEvidence -InspectIndex 0
Prints campfire compiler evidence for a selected checkpoint branch.

.EXAMPLE
.\tools\campaign.ps1 -InspectRouteEvidence -InspectIndex 0
Prints route planner evidence for a selected checkpoint branch.

.EXAMPLE
.\tools\campaign.ps1 -InspectLastAutoCombat -InspectIndex 0
Prints the last saved automated combat trajectory for a selected checkpoint branch.

.EXAMPLE
.\tools\campaign.ps1 -InspectCombatLab -InspectIndex 0
Prints a report-only combat lab packet for a selected checkpoint branch.

.EXAMPLE
.\tools\campaign.ps1 -InspectCombatLab -ProbeBoss -InspectIndex 0
Runs a report-only current-act boss preview for a selected checkpoint branch.

.EXAMPLE
.\tools\campaign.ps1 -Inspect -ExportLearningDataset tools\artifacts\learning\latest.learning.jsonl
Exports LearningBranchSampleV1 JSONL from the latest campaign report/checkpoint.

.EXAMPLE
.\tools\campaign.ps1 -PlanTargets
Exports latest decision outcomes and prints targeted sibling continuation groups.

.EXAMPLE
.\tools\campaign.ps1 -ContinueTargets -Rounds 1
Exports latest decision outcomes, resumes selected censored sibling branches, and advances one round.

.EXAMPLE
.\tools\campaign.ps1 -PlanCoverageGaps
Prints unobserved journal candidate coverage-gap continuation targets.

.EXAMPLE
.\tools\campaign.ps1 -ContinueCoverageGaps -Rounds 1
Resumes selected unobserved journal candidate branches and advances one round.

.EXAMPLE
.\tools\campaign.ps1 -Mode quick
Runs a shorter random-seed campaign for fast smoke testing.

.EXAMPLE
.\tools\campaign.ps1 -Ascension 20 -Mode quick
Runs a high-ascension stress campaign on a random seed.

.EXAMPLE
.\tools\campaign.ps1 -Domain a20 -Mode quick
Runs the current target-domain high-ascension campaign shortcut.

.EXAMPLE
.\tools\campaign.ps1 -Domain a20 -Mode explore -BossRelicAxes
Runs a high-ascension campaign where each boss relic lineage gets separate active/frozen branch budgets.

.EXAMPLE
.\tools\campaign.ps1 -Mode deep
Runs a larger random-seed campaign when you want to leave it working longer.

.EXAMPLE
.\tools\campaign.ps1 -Mode explore
Runs a wider, shallower campaign for branch comparison and strategy diagnosis.

.EXAMPLE
.\tools\campaign.ps1 -More -VictoryHpPercent 50
Resumes the latest campaign but keeps exploring until it finds a victory at 50% HP or better.

.EXAMPLE
.\tools\campaign.ps1 -DryRun
Prints the cargo command without updating the last seed or running it.

.EXAMPLE
.\tools\campaign.ps1 -NoProgress
Runs without coarse campaign progress messages.

.EXAMPLE
.\tools\campaign.ps1 -Perf
Prints campaign performance diagnostics in the final report.

.EXAMPLE
.\tools\campaign.ps1 -Diagnose
Prints strategy and branch diagnostics in the final report.

.EXAMPLE
.\tools\campaign.ps1 -VerboseProgress
Prints branch-by-branch progress messages while running.

.EXAMPLE
.\tools\campaign.ps1 -NoBossSegments
Compatibility switch; boss combats already stay on complete-win search by default.

.EXAMPLE
.\tools\campaign.ps1 -BossSegments
Allows turn-segment continuation inside boss combats. This is slower, but can push through bosses while debugging combat strategy.

.EXAMPLE
.\tools\campaign.ps1 -DebugBuild
Runs the slower debug build when you are debugging compilation or assertions.

.EXAMPLE
.\tools\campaign.ps1 -BuildProfile release-final
Runs with the slow-to-build final-performance profile.

.EXAMPLE
.\tools\campaign.ps1 -Build
Rebuilds the branch campaign driver before running it.

.EXAMPLE
.\tools\campaign.ps1 -Mode quick -AutoCaptureCombat -AutoCaptureRoot tools\artifacts\tmp\ml_capture_seed123
Runs a campaign and registers fresh combat captures under the selected benchmark root.
#>
param(
    [Parameter(Position = 0)]
    [long] $Seed = 0,

    [switch] $Last,
    [switch] $More,
    [switch] $Inspect,
    [switch] $InspectShopEvidence,
    [switch] $InspectShopChallenge,
    [switch] $InspectCardRewardEvidence,
    [switch] $InspectDecisionObservations,
    [switch] $InspectJournal,
    [switch] $InspectCampfireEvidence,
    [switch] $InspectDeckMutation,
    [switch] $InspectRouteEvidence,
    [switch] $InspectLastAutoCombat,
    [switch] $InspectCombatLab,
    [switch] $InspectFinalBossCombat,
    [switch] $ProbeBoss,
    [switch] $DryRun,
    [switch] $NoProgress,
    [switch] $VerboseProgress,
    [switch] $Diagnose,
    [switch] $Perf,
    [switch] $NoBossSegments,
    [switch] $BossSegments,
    [switch] $BossRelicAxes,
    [switch] $AutoCaptureCombat,
    [switch] $DebugBuild,
    [switch] $Build,
    [switch] $PlanTargets,
    [switch] $ContinueTargets,
    [switch] $PlanCoverageGaps,
    [switch] $ContinueCoverageGaps,

    [string] $ExportLearningDataset = "",
    [string] $DecisionOutcomeDataset = "",
    [string] $AutoCaptureRoot = "",

    [ValidateSet("fast-run", "release-final", "release", "dev-opt", "debug")]
    [string] $BuildProfile = "fast-run",

    [ValidateSet("quick", "focused", "explore", "deep")]
    [string] $Mode = "explore",

    [ValidateRange(0, 100000)]
    [int] $Rounds = 0,

    [ValidateRange(0, 100000)]
    [int] $UntilRound = 0,

    [ValidateRange(0, 20)]
    [int] $Ascension = 0,

    [ValidateSet("a0", "a10", "a15", "a17", "a20")]
    [string] $Domain = "",

    [ValidateSet("ironclad", "silent", "defect", "watcher")]
    [string] $Class = "ironclad",

    [int] $MaxRounds = 6,
    [int] $ExperimentWallMs = 10000,
    [int] $SearchWallMs = 300,
    [int] $SearchMaxNodes = 50000,
    [int] $CombatRetryWallMs = 0,
    [int] $ActiveLineageDiversity = -1,
    [int] $BranchExamples = 4,
    [int] $InspectIndex = -1,
    [int] $InspectAct = 0,
    [int] $InspectFloor = 0,
    [string] $InspectBoundary = "",
    [string] $InspectQuery = "",
    [int] $ChallengeMaxPlans = 6,
    [int] $ChallengeDepth = 3,
    [int] $ChallengeMaxBranches = 10,
    [int] $TargetedContinuationLimit = 4,
    [int] $TargetedContinuationCandidatesPerTarget = 1,
    [int] $CoverageGapLimit = 8,
    [int] $CoverageGapCandidatesPerDecision = 1,
    [ValidateRange(0, 100)]
    [int] $VictoryHpPercent = 20,

    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $ExtraArgs
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
$CampaignDir = Join-Path $RepoRoot "tools\artifacts\campaigns"
$LatestSeedPath = Join-Path $CampaignDir "latest.seed.txt"
$LatestAscensionPath = Join-Path $CampaignDir "latest.ascension.txt"
$LatestClassPath = Join-Path $CampaignDir "latest.class.txt"
$LatestModePath = Join-Path $CampaignDir "latest.mode.txt"
$LatestCommandPath = Join-Path $CampaignDir "latest.command.txt"
$LatestCampaignPath = Join-Path $CampaignDir "latest.campaign.json"
$LatestCheckpointPath = Join-Path $CampaignDir "latest.checkpoint.json"
$LatestDecisionOutcomePath = Join-Path $CampaignDir "latest.decision_outcomes.jsonl"
$LatestDecisionOutcomeBeforePath = Join-Path $CampaignDir "latest.decision_outcomes.before.jsonl"
$LatestDecisionOutcomeAfterPath = Join-Path $CampaignDir "latest.decision_outcomes.after.jsonl"

New-Item -ItemType Directory -Force -Path $CampaignDir | Out-Null

function Read-LatestCheckpointRunConfig {
    if (-not (Test-Path -LiteralPath $LatestCheckpointPath)) {
        return $null
    }
    try {
        $Checkpoint = Get-Content -LiteralPath $LatestCheckpointPath -Raw | ConvertFrom-Json
        if ($Checkpoint.sessions -and $Checkpoint.sessions.Count -gt 0) {
            return $Checkpoint.sessions[0].session.run_state
        }
    } catch {
        return $null
    }
    return $null
}

function Read-LatestCampaignMode {
    if (Test-Path -LiteralPath $LatestModePath) {
        $ModeText = (Get-Content -LiteralPath $LatestModePath -Raw).Trim().ToLowerInvariant()
        if (@("quick", "focused", "explore", "deep") -contains $ModeText) {
            return $ModeText
        }
    }
    if (Test-Path -LiteralPath $LatestCommandPath) {
        $CommandText = Get-Content -LiteralPath $LatestCommandPath -Raw
        if ($CommandText -match "--preset\s+('?)(quick|focused|explore|deep)\1") {
            return $Matches[2].ToLowerInvariant()
        }
    }
    return $null
}

if (
    $InspectShopEvidence -or
    $InspectShopChallenge -or
    $InspectCardRewardEvidence -or
    $InspectDecisionObservations -or
    $InspectJournal -or
    $InspectCampfireEvidence -or
    $InspectDeckMutation -or
    $InspectRouteEvidence -or
    $InspectLastAutoCombat -or
    $InspectCombatLab -or
    $InspectFinalBossCombat
) {
    $Inspect = $true
}
if ($InspectShopChallenge -and -not $PSBoundParameters.ContainsKey("InspectBoundary")) {
    $InspectBoundary = "Shop"
}

if (($PlanTargets -or $ContinueTargets) -and ($PlanCoverageGaps -or $ContinueCoverageGaps)) {
    throw "Choose either targeted continuation (-PlanTargets/-ContinueTargets) or coverage-gap continuation (-PlanCoverageGaps/-ContinueCoverageGaps), not both."
}

if ($PlanTargets -or $ContinueTargets -or $PlanCoverageGaps -or $ContinueCoverageGaps) {
    $Last = $true
    if (-not $PSBoundParameters.ContainsKey("Mode")) {
        $SavedMode = Read-LatestCampaignMode
        if ($SavedMode) {
            $Mode = $SavedMode
        } else {
            $Mode = "focused"
        }
    }
}

if ($More) {
    $Last = $true
    if (-not $PSBoundParameters.ContainsKey("Mode")) {
        $SavedMode = Read-LatestCampaignMode
        if ($SavedMode) {
            $Mode = $SavedMode
        } else {
            $Mode = "deep"
        }
    }
}

if ($Inspect) {
    if ($Seed -le 0 -and (Test-Path $LatestSeedPath)) {
        $SeedText = (Get-Content -LiteralPath $LatestSeedPath -Raw).Trim()
        [void] [long]::TryParse($SeedText, [ref] $Seed)
    }
} elseif ($Last) {
    if (-not (Test-Path $LatestSeedPath)) {
        throw "No previous campaign seed found at $LatestSeedPath. Run .\tools\campaign.ps1 first."
    }
    $SeedText = (Get-Content -LiteralPath $LatestSeedPath -Raw).Trim()
    if (-not [long]::TryParse($SeedText, [ref] $Seed)) {
        throw "Invalid previous campaign seed in $LatestSeedPath`: $SeedText"
    }
} elseif ($Seed -le 0) {
    $Seed = Get-Random -Minimum 1 -Maximum 2147483647
}

$AscensionBound = $PSBoundParameters.ContainsKey("Ascension")
$ClassBound = $PSBoundParameters.ContainsKey("Class")
$DomainBound = $PSBoundParameters.ContainsKey("Domain") -and $Domain
if ($DomainBound) {
    $DomainAscension = [int] $Domain.Substring(1)
    if ($AscensionBound -and $Ascension -ne $DomainAscension) {
        throw "-Domain $Domain conflicts with -Ascension $Ascension."
    }
    $Ascension = $DomainAscension
    $AscensionBound = $true
}
if ($Last -or $Inspect) {
    if (-not $AscensionBound) {
        if (Test-Path -LiteralPath $LatestAscensionPath) {
            $AscensionText = (Get-Content -LiteralPath $LatestAscensionPath -Raw).Trim()
            [void] [int]::TryParse($AscensionText, [ref] $Ascension)
        } else {
            $SavedConfig = Read-LatestCheckpointRunConfig
            if ($SavedConfig -and $SavedConfig.ascension_level -ne $null) {
                $Ascension = [int] $SavedConfig.ascension_level
            }
        }
    }
    if (-not $ClassBound) {
        if (Test-Path -LiteralPath $LatestClassPath) {
            $Class = (Get-Content -LiteralPath $LatestClassPath -Raw).Trim().ToLowerInvariant()
        } else {
            $SavedConfig = Read-LatestCheckpointRunConfig
            if ($SavedConfig -and $SavedConfig.player_class) {
                $Class = ([string] $SavedConfig.player_class).ToLowerInvariant()
            }
        }
    }
}

$ExplicitBuildProfile = $PSBoundParameters.ContainsKey("BuildProfile")
if ($DebugBuild) {
    if ($ExplicitBuildProfile -and $BuildProfile -ne "debug") {
        throw "-DebugBuild conflicts with -BuildProfile $BuildProfile. Use only one build profile selector."
    }
    $BuildProfile = "debug"
}

$DriverExe = Join-Path $RepoRoot "target\$BuildProfile\branch_campaign_driver.exe"
$BuildArgs = @("build", "--quiet", "--bin", "branch_campaign_driver")
switch ($BuildProfile) {
    "debug" {
        # Default cargo dev profile.
    }
    "release" {
        $BuildArgs += "--release"
    }
    default {
        $BuildArgs += @("--profile", "$BuildProfile")
    }
}

$DriverArgs = @(
    "run",
    "--preset", "$Mode",
    "--seed", "$Seed",
    "--ascension", "$Ascension",
    "--class", "$Class"
)
if (@(0, 10, 15, 17, 20) -contains $Ascension) {
    $DriverArgs += @("--ascension-domain", "a$Ascension")
}

$CampaignBoundParameters = @{}
foreach ($ParameterName in $PSBoundParameters.Keys) {
    $CampaignBoundParameters[$ParameterName] = $true
}

$RoundsBound = $CampaignBoundParameters.ContainsKey("Rounds")
$UntilRoundBound = $CampaignBoundParameters.ContainsKey("UntilRound")
$MaxRoundsBound = $CampaignBoundParameters.ContainsKey("MaxRounds")
if (($RoundsBound -and $UntilRoundBound) -or ($RoundsBound -and $MaxRoundsBound) -or ($UntilRoundBound -and $MaxRoundsBound)) {
    throw "Choose only one round budget: -Rounds N, -UntilRound N, or legacy -MaxRounds N."
}

$DriverRoundBudgetArgs = @()
$RoundBudgetSource = if ($MaxRoundsBound) { "MaxRounds" } else { "preset" }
if (-not $More) {
    if ($RoundsBound) {
        $DriverRoundBudgetArgs = @("--rounds", "$Rounds")
        $RoundBudgetSource = "Rounds"
    } elseif ($UntilRoundBound) {
        $DriverRoundBudgetArgs = @("--until-round", "$UntilRound")
        $RoundBudgetSource = "UntilRound"
    } elseif ($MaxRoundsBound) {
        $DriverRoundBudgetArgs = @("--max-rounds", "$MaxRounds")
    }
}

$ResumeCampaignPath = $null
$ResumeCheckpointPath = $null
$ResumeRoundsCompleted = $null
$TargetRounds = $null
if ($More) {
    if (-not (Test-Path $LatestCampaignPath)) {
        throw "No previous campaign report found at $LatestCampaignPath. Run .\tools\campaign.ps1 first, or use -Last to rerun the previous seed from the start."
    }
    $ResumeCampaignPath = $LatestCampaignPath
    $ResumeReport = Get-Content -LiteralPath $ResumeCampaignPath -Raw | ConvertFrom-Json
    $ResumeRoundsCompleted = [int] $ResumeReport.rounds_completed
    if ($RoundsBound) {
        $TargetRounds = $ResumeRoundsCompleted + $Rounds
        $MaxRounds = $Rounds
        $DriverRoundBudgetArgs = @("--rounds", "$Rounds")
        $RoundBudgetSource = "Rounds"
    } elseif ($UntilRoundBound) {
        $TargetRounds = $UntilRound
        $MaxRounds = [Math]::Max(0, $TargetRounds - $ResumeRoundsCompleted)
        $DriverRoundBudgetArgs = @("--until-round", "$UntilRound")
        $RoundBudgetSource = "UntilRound"
    } elseif ($MaxRoundsBound) {
        $TargetRounds = $MaxRounds
        $MaxRounds = [Math]::Max(0, $TargetRounds - $ResumeRoundsCompleted)
        $DriverRoundBudgetArgs = @("--until-round", "$TargetRounds")
        $RoundBudgetSource = "LegacyMaxRounds"
    }
    $DriverArgs += @("--resume", "$ResumeCampaignPath")
    if (Test-Path $LatestCheckpointPath) {
        $ResumeCheckpointPath = $LatestCheckpointPath
        $DriverArgs += @("--resume-checkpoint", "$ResumeCheckpointPath")
    }
}

$DriverArgs += @("--out", "$LatestCampaignPath", "--checkpoint-out", "$LatestCheckpointPath")

function Add-DriverArgIfBound {
    param(
        [string] $ParameterName,
        [string] $Flag,
        [object] $Value
    )

    if ($CampaignBoundParameters.ContainsKey($ParameterName)) {
        $script:DriverArgs += @($Flag, "$Value")
    }
}

if ($DriverRoundBudgetArgs.Count -gt 0) {
    $DriverArgs += $DriverRoundBudgetArgs
}
Add-DriverArgIfBound "ExperimentWallMs" "--experiment-wall-ms" $ExperimentWallMs
Add-DriverArgIfBound "SearchWallMs" "--search-wall-ms" $SearchWallMs
Add-DriverArgIfBound "SearchMaxNodes" "--search-max-nodes" $SearchMaxNodes
if ($CampaignBoundParameters.ContainsKey("ActiveLineageDiversity") -and $ActiveLineageDiversity -ge 0) {
    $DriverArgs += @("--active-lineage-diversity", "$ActiveLineageDiversity")
}
if ($BossRelicAxes) {
    $DriverArgs += "--boss-relic-axes"
}
if ($CampaignBoundParameters.ContainsKey("CombatRetryWallMs") -and $CombatRetryWallMs -gt 0) {
    $DriverArgs += @("--combat-retry-wall-ms", "$CombatRetryWallMs")
}
Add-DriverArgIfBound "BranchExamples" "--branch-examples" $BranchExamples
Add-DriverArgIfBound "VictoryHpPercent" "--min-acceptable-victory-hp-percent" $VictoryHpPercent
if ($AutoCaptureCombat) {
    $DriverArgs += "--auto-capture-combat"
    if ($AutoCaptureRoot) {
        $DriverArgs += @("--auto-capture-root", "$AutoCaptureRoot")
    }
}

function Test-ExtraCombatOptionKey {
    param(
        [string[]] $Tokens,
        [string[]] $Keys
    )

    foreach ($Arg in $Tokens) {
        foreach ($Key in $Keys) {
            if ($Arg -match "(^|\s|=)$([regex]::Escape($Key))=") {
                return $true
            }
        }
    }
    return $false
}

if ($BossSegments -and $NoBossSegments) {
    throw "-BossSegments and -NoBossSegments conflict; choose one segment policy."
}

$CombatSegmentMode = "custom"
if (-not (Test-ExtraCombatOptionKey -Tokens $ExtraArgs -Keys @("segment", "segment_mode", "partial", "partial_mode"))) {
    if ($BossSegments) {
        $DriverArgs += @("--combat-search-option", "segment=turn")
        $CombatSegmentMode = "turn"
    } else {
        $DriverArgs += @("--combat-search-option", "segment=non_boss_turn")
        $CombatSegmentMode = "non_boss_turn"
    }
}

if (-not $NoProgress) {
    $DriverArgs += "--progress"
    if ($VerboseProgress) {
        $DriverArgs += @("--progress-detail", "verbose")
    }
}

if ($Perf) {
    $DriverArgs += @("--report-detail", "perf")
} elseif ($Diagnose) {
    $DriverArgs += @("--report-detail", "diagnose")
}

if ($ExportLearningDataset -and -not $Inspect) {
    $DriverArgs += @("--export-learning-dataset", "$ExportLearningDataset")
}

if ($ExtraArgs) {
    $DriverArgs += $ExtraArgs
}

function Test-DriverNeedsBuild {
    param(
        [string] $ExePath
    )

    if (-not (Test-Path -LiteralPath $ExePath)) {
        return $true
    }

    $ExeTime = (Get-Item -LiteralPath $ExePath).LastWriteTimeUtc
    foreach ($Path in @("Cargo.toml", "Cargo.lock")) {
        $FullPath = Join-Path $RepoRoot $Path
        if ((Test-Path -LiteralPath $FullPath) -and (Get-Item -LiteralPath $FullPath).LastWriteTimeUtc -gt $ExeTime) {
            return $true
        }
    }
    foreach ($SourceFile in Get-ChildItem -LiteralPath (Join-Path $RepoRoot "src") -Recurse -File -Filter *.rs) {
        if ($SourceFile.LastWriteTimeUtc -gt $ExeTime) {
            return $true
        }
    }
    return $false
}

$NeedsBuild = $Build -or (Test-DriverNeedsBuild $DriverExe)

if ($PlanTargets -or $ContinueTargets -or $PlanCoverageGaps -or $ContinueCoverageGaps) {
    if (-not (Test-Path $LatestCampaignPath)) {
        throw "No previous campaign report found at $LatestCampaignPath. Run .\tools\campaign.ps1 first."
    }
    if (-not (Test-Path $LatestCheckpointPath)) {
        throw "No previous campaign checkpoint found at $LatestCheckpointPath. Run .\tools\campaign.ps1 first."
    }

    $TargetDecisionOutcomePath = if ($DecisionOutcomeDataset) {
        $DecisionOutcomeDataset
    } elseif ($ContinueTargets) {
        $LatestDecisionOutcomeBeforePath
    } else {
        $LatestDecisionOutcomePath
    }
    $CoveragePlanArgs = @(
        "dataset",
        "--inspect-report", "$LatestCampaignPath",
        "--inspect-checkpoint", "$LatestCheckpointPath",
        "--plan-coverage-gap-continuation",
        "--coverage-gap-limit", "$CoverageGapLimit",
        "--coverage-gap-candidates-per-decision", "$CoverageGapCandidatesPerDecision"
    )
    $ExportDecisionArgs = @(
        "dataset",
        "--inspect-report", "$LatestCampaignPath",
        "--inspect-checkpoint", "$LatestCheckpointPath",
        "--export-decision-outcome-dataset", "$TargetDecisionOutcomePath"
    )
    $PlanTargetArgs = @("continue", "--plan-targeted-continuation", "$TargetDecisionOutcomePath")
    $ExportDecisionAfterArgs = @(
        "dataset",
        "--inspect-report", "$LatestCampaignPath",
        "--inspect-checkpoint", "$LatestCheckpointPath",
        "--export-decision-outcome-dataset", "$LatestDecisionOutcomeAfterPath"
    )
    $ContinuationEffectArgs = @(
        "continue",
        "--continuation-effect-before", "$TargetDecisionOutcomePath",
        "--continuation-effect-after", "$LatestDecisionOutcomeAfterPath"
    )

    $ResumeReport = Get-Content -LiteralPath $LatestCampaignPath -Raw | ConvertFrom-Json
    $ResumeRoundsCompleted = [int] $ResumeReport.rounds_completed
    $ContinuationRounds = 1
    $ContinuationRoundBudgetArgs = @("--rounds", "1")
    $TargetRounds = $null
    $ContinuationRoundSource = "default"
    if ($RoundsBound) {
        $ContinuationRounds = $Rounds
        $ContinuationRoundSource = "Rounds"
        $TargetRounds = $ResumeRoundsCompleted + $Rounds
        $ContinuationRoundBudgetArgs = @("--rounds", "$Rounds")
    } elseif ($UntilRoundBound) {
        $TargetRounds = $UntilRound
        $ContinuationRounds = [Math]::Max(0, $TargetRounds - $ResumeRoundsCompleted)
        $ContinuationRoundSource = "UntilRound"
        $ContinuationRoundBudgetArgs = @("--until-round", "$UntilRound")
    } elseif ($MaxRoundsBound) {
        $ContinuationRounds = $MaxRounds
        $ContinuationRoundSource = "MaxRounds"
        $TargetRounds = $ResumeRoundsCompleted + $MaxRounds
        $ContinuationRoundBudgetArgs = @("--max-rounds", "$ContinuationRounds")
    }

    $ContinueTargetArgs = @(
        "continue",
        "--preset", "$Mode",
        "--seed", "$Seed",
        "--ascension", "$Ascension",
        "--class", "$Class"
    )
    if (@(0, 10, 15, 17, 20) -contains $Ascension) {
        $ContinueTargetArgs += @("--ascension-domain", "a$Ascension")
    }
    $ContinueTargetArgs += @(
        "--resume", "$LatestCampaignPath",
        "--resume-checkpoint", "$LatestCheckpointPath",
        "--execute-targeted-continuation", "$TargetDecisionOutcomePath",
        "--targeted-continuation-limit", "$TargetedContinuationLimit",
        "--targeted-continuation-candidates-per-target", "$TargetedContinuationCandidatesPerTarget",
        "--out", "$LatestCampaignPath",
        "--checkpoint-out", "$LatestCheckpointPath"
    )
    $ContinueTargetArgs += $ContinuationRoundBudgetArgs
    $ContinueCoverageGapArgs = @(
        "continue",
        "--preset", "$Mode",
        "--seed", "$Seed",
        "--ascension", "$Ascension",
        "--class", "$Class"
    )
    if (@(0, 10, 15, 17, 20) -contains $Ascension) {
        $ContinueCoverageGapArgs += @("--ascension-domain", "a$Ascension")
    }
    $ContinueCoverageGapArgs += @(
        "--resume", "$LatestCampaignPath",
        "--resume-checkpoint", "$LatestCheckpointPath",
        "--execute-coverage-gap-continuation",
        "--coverage-gap-limit", "$CoverageGapLimit",
        "--coverage-gap-candidates-per-decision", "$CoverageGapCandidatesPerDecision",
        "--out", "$LatestCampaignPath",
        "--checkpoint-out", "$LatestCheckpointPath"
    )
    $ContinueCoverageGapArgs += $ContinuationRoundBudgetArgs
    if ($CampaignBoundParameters.ContainsKey("ExperimentWallMs")) {
        $ContinueTargetArgs += @("--experiment-wall-ms", "$ExperimentWallMs")
        $ContinueCoverageGapArgs += @("--experiment-wall-ms", "$ExperimentWallMs")
    }
    if ($CampaignBoundParameters.ContainsKey("SearchWallMs")) {
        $ContinueTargetArgs += @("--search-wall-ms", "$SearchWallMs")
        $ContinueCoverageGapArgs += @("--search-wall-ms", "$SearchWallMs")
    }
    if ($CampaignBoundParameters.ContainsKey("SearchMaxNodes")) {
        $ContinueTargetArgs += @("--search-max-nodes", "$SearchMaxNodes")
        $ContinueCoverageGapArgs += @("--search-max-nodes", "$SearchMaxNodes")
    }
    if ($CampaignBoundParameters.ContainsKey("CombatRetryWallMs") -and $CombatRetryWallMs -gt 0) {
        $ContinueTargetArgs += @("--combat-retry-wall-ms", "$CombatRetryWallMs")
        $ContinueCoverageGapArgs += @("--combat-retry-wall-ms", "$CombatRetryWallMs")
    }
    if ($CampaignBoundParameters.ContainsKey("BranchExamples")) {
        $ContinueTargetArgs += @("--branch-examples", "$BranchExamples")
        $ContinueCoverageGapArgs += @("--branch-examples", "$BranchExamples")
    }
    if ($CampaignBoundParameters.ContainsKey("VictoryHpPercent")) {
        $ContinueTargetArgs += @("--min-acceptable-victory-hp-percent", "$VictoryHpPercent")
        $ContinueCoverageGapArgs += @("--min-acceptable-victory-hp-percent", "$VictoryHpPercent")
    }
    if (-not (Test-ExtraCombatOptionKey -Tokens $ExtraArgs -Keys @("segment", "segment_mode", "partial", "partial_mode"))) {
        if ($BossSegments) {
            $ContinueTargetArgs += @("--combat-search-option", "segment=turn")
            $ContinueCoverageGapArgs += @("--combat-search-option", "segment=turn")
        } else {
            $ContinueTargetArgs += @("--combat-search-option", "segment=non_boss_turn")
            $ContinueCoverageGapArgs += @("--combat-search-option", "segment=non_boss_turn")
        }
    }
    if (-not $NoProgress) {
        $ContinueTargetArgs += "--progress"
        $ContinueCoverageGapArgs += "--progress"
        if ($VerboseProgress) {
            $ContinueTargetArgs += @("--progress-detail", "verbose")
            $ContinueCoverageGapArgs += @("--progress-detail", "verbose")
        }
    }
    if ($Perf) {
        $ContinueTargetArgs += @("--report-detail", "perf")
        $ContinueCoverageGapArgs += @("--report-detail", "perf")
    } elseif ($Diagnose) {
        $ContinueTargetArgs += @("--report-detail", "diagnose")
        $ContinueCoverageGapArgs += @("--report-detail", "diagnose")
    }
    if ($AutoCaptureCombat) {
        $ContinueTargetArgs += "--auto-capture-combat"
        $ContinueCoverageGapArgs += "--auto-capture-combat"
        if ($AutoCaptureRoot) {
            $ContinueTargetArgs += @("--auto-capture-root", "$AutoCaptureRoot")
            $ContinueCoverageGapArgs += @("--auto-capture-root", "$AutoCaptureRoot")
        }
    }
    if ($ExtraArgs) {
        $ContinueTargetArgs += $ExtraArgs
        $ContinueCoverageGapArgs += $ExtraArgs
    }

    function Format-CommandLine {
        param(
            [string] $ExePath,
            [string[]] $Arguments
        )

        $RenderedExe = if ($ExePath -match '^[A-Za-z0-9_./:=\\-]+$') { $ExePath } else { "'$($ExePath -replace "'", "''")'" }
        $RenderedArgs = $Arguments | ForEach-Object {
            if ($_ -match '^[A-Za-z0-9_./:=\\-]+$') { $_ } else { "'$($_ -replace "'", "''")'" }
        }
        return $RenderedExe + " " + ($RenderedArgs -join " ")
    }

    $ContinuationModeLabel = if ($PlanCoverageGaps -or $ContinueCoverageGaps) { "coverage-gap-continuation" } else { "targeted-continuation" }
    Write-Host "mode=$ContinuationModeLabel latest branch campaign"
    Write-Host "seed=$Seed"
    Write-Host "ascension=A$Ascension domain=a$Ascension class=$Class"
    Write-Host "build=$BuildProfile exe=$DriverExe"
    if ($NeedsBuild) {
        Write-Host "build-needed=yes"
    } else {
        Write-Host "build-needed=no"
    }
    Write-Host "report=$LatestCampaignPath"
    Write-Host "checkpoint=$LatestCheckpointPath"
    if ($PlanTargets -or $ContinueTargets) {
        Write-Host "decision-outcomes=$TargetDecisionOutcomePath"
    }
    if ($ContinueTargets) {
        Write-Host "decision-outcomes-after=$LatestDecisionOutcomeAfterPath"
        Write-Host "continue-targets=$TargetedContinuationLimit candidates-per-target=$TargetedContinuationCandidatesPerTarget"
        Write-Host "resume-rounds=$ResumeRoundsCompleted"
        if ($TargetRounds -ne $null) {
            Write-Host "round-budget=$ContinuationRoundSource target-rounds=$TargetRounds additional-rounds=$ContinuationRounds"
        } else {
            Write-Host "round-budget=$ContinuationRoundSource additional-rounds=$ContinuationRounds"
        }
    }
    if ($PlanCoverageGaps) {
        Write-Host "coverage-gap-plan=$CoverageGapLimit candidates-per-decision=$CoverageGapCandidatesPerDecision"
    }
    if ($ContinueCoverageGaps) {
        Write-Host "coverage-gap-continue=$CoverageGapLimit candidates-per-decision=$CoverageGapCandidatesPerDecision"
        Write-Host "resume-rounds=$ResumeRoundsCompleted"
        if ($TargetRounds -ne $null) {
            Write-Host "round-budget=$ContinuationRoundSource target-rounds=$TargetRounds additional-rounds=$ContinuationRounds"
        } else {
            Write-Host "round-budget=$ContinuationRoundSource additional-rounds=$ContinuationRounds"
        }
    }

    if ($DryRun) {
        if ($NeedsBuild) {
            $RenderedBuildArgs = $BuildArgs | ForEach-Object {
                if ($_ -match '^[A-Za-z0-9_./:=\\-]+$') { $_ } else { "'$($_ -replace "'", "''")'" }
            }
            Write-Host ("cargo " + ($RenderedBuildArgs -join " "))
        }
        if ($PlanTargets -or $ContinueTargets) {
            Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments $ExportDecisionArgs)
        }
        if ($PlanTargets) {
            Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments $PlanTargetArgs)
        }
        if ($PlanCoverageGaps) {
            Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments $CoveragePlanArgs)
        }
        if ($ContinueTargets) {
            Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments $ContinueTargetArgs)
            Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments $ExportDecisionAfterArgs)
            Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments $ContinuationEffectArgs)
        }
        if ($ContinueCoverageGaps) {
            Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments $ContinueCoverageGapArgs)
        }
        exit 0
    }

    if (($ContinueTargets -or $ContinueCoverageGaps) -and $ContinuationRounds -eq 0) {
        Write-Host "already-at-target-rounds=yes; nothing to run"
        exit 0
    }

    Push-Location $RepoRoot
    try {
        if ($NeedsBuild) {
            & cargo @BuildArgs
            if ($LASTEXITCODE -ne 0) {
                exit $LASTEXITCODE
            }
        }
        if ($PlanTargets -or $ContinueTargets) {
            & $DriverExe @ExportDecisionArgs
            if ($LASTEXITCODE -ne 0) {
                exit $LASTEXITCODE
            }
        }
        if ($PlanTargets) {
            & $DriverExe @PlanTargetArgs
            if ($LASTEXITCODE -ne 0) {
                exit $LASTEXITCODE
            }
        }
        if ($PlanCoverageGaps) {
            & $DriverExe @CoveragePlanArgs
            if ($LASTEXITCODE -ne 0) {
                exit $LASTEXITCODE
            }
        }
        if ($ContinueTargets) {
            & $DriverExe @ContinueTargetArgs
            $DriverExitCode = $LASTEXITCODE
            if ($DriverExitCode -eq 0) {
                & $DriverExe @ExportDecisionAfterArgs
                if ($LASTEXITCODE -ne 0) {
                    exit $LASTEXITCODE
                }
                & $DriverExe @ContinuationEffectArgs
                if ($LASTEXITCODE -ne 0) {
                    exit $LASTEXITCODE
                }
                Set-Content -LiteralPath $LatestSeedPath -Value $Seed
                Set-Content -LiteralPath $LatestAscensionPath -Value $Ascension
                Set-Content -LiteralPath $LatestClassPath -Value $Class
                Set-Content -LiteralPath $LatestModePath -Value $Mode
                Set-Content -LiteralPath $LatestCommandPath -Value (Format-CommandLine -ExePath $DriverExe -Arguments $ContinueTargetArgs)
            }
            exit $DriverExitCode
        }
        if ($ContinueCoverageGaps) {
            & $DriverExe @ContinueCoverageGapArgs
            $DriverExitCode = $LASTEXITCODE
            if ($DriverExitCode -eq 0) {
                Set-Content -LiteralPath $LatestSeedPath -Value $Seed
                Set-Content -LiteralPath $LatestAscensionPath -Value $Ascension
                Set-Content -LiteralPath $LatestClassPath -Value $Class
                Set-Content -LiteralPath $LatestModePath -Value $Mode
                Set-Content -LiteralPath $LatestCommandPath -Value (Format-CommandLine -ExePath $DriverExe -Arguments $ContinueCoverageGapArgs)
            }
            exit $DriverExitCode
        }
        exit 0
    } finally {
        Pop-Location
    }
}

if ($Inspect) {
    if (-not (Test-Path $LatestCheckpointPath)) {
        throw "No previous campaign checkpoint found at $LatestCheckpointPath. Run .\tools\campaign.ps1 first."
    }
    if (-not (Test-Path $LatestCampaignPath)) {
        throw "No previous campaign report found at $LatestCampaignPath. Run .\tools\campaign.ps1 first."
    }

    if ($ExportLearningDataset) {
        $InspectArgs = @(
            "dataset",
            "--inspect-checkpoint", "$LatestCheckpointPath",
            "--inspect-report", "$LatestCampaignPath",
            "--export-learning-dataset", "$ExportLearningDataset"
        )
    } else {
        $InspectArgs = @(
            "inspect",
            "--inspect-checkpoint", "$LatestCheckpointPath",
            "--inspect-report", "$LatestCampaignPath",
            "--branch-examples", "$BranchExamples"
        )
    }
    $DetailedInspect =
        $InspectShopEvidence -or
        $InspectShopChallenge -or
        $InspectCardRewardEvidence -or
        $InspectDecisionObservations -or
        $InspectJournal -or
        $InspectCampfireEvidence -or
        $InspectDeckMutation -or
        $InspectRouteEvidence -or
        $InspectLastAutoCombat -or
        $InspectCombatLab -or
        $InspectFinalBossCombat
    if ((-not $ExportLearningDataset) -and (-not $DetailedInspect)) {
        $InspectArgs += "--inspect-summary"
    }
    if ((-not $ExportLearningDataset) -and $InspectShopEvidence) {
        $InspectArgs += "--inspect-shop-evidence"
    }
    if ((-not $ExportLearningDataset) -and $InspectShopChallenge) {
        $InspectArgs += @(
            "--challenge-shop-plans",
            "--challenge-max-plans", "$ChallengeMaxPlans",
            "--challenge-depth", "$ChallengeDepth",
            "--challenge-max-branches", "$ChallengeMaxBranches",
            "--search-wall-ms", "$SearchWallMs",
            "--search-max-nodes", "$SearchMaxNodes"
        )
    }
    if ((-not $ExportLearningDataset) -and $InspectCardRewardEvidence) {
        $InspectArgs += "--inspect-card-reward-evidence"
    }
    if ((-not $ExportLearningDataset) -and $InspectDecisionObservations) {
        $InspectArgs += "--inspect-decision-observations"
    }
    if ((-not $ExportLearningDataset) -and $InspectJournal) {
        $InspectArgs += "--inspect-journal"
    }
    if ((-not $ExportLearningDataset) -and $InspectCampfireEvidence) {
        $InspectArgs += "--inspect-campfire-evidence"
    }
    if ((-not $ExportLearningDataset) -and $InspectDeckMutation) {
        $InspectArgs += "--inspect-deck-mutation"
    }
    if ((-not $ExportLearningDataset) -and $InspectRouteEvidence) {
        $InspectArgs += "--inspect-route-evidence"
    }
    if ((-not $ExportLearningDataset) -and $InspectLastAutoCombat) {
        $InspectArgs += "--inspect-last-auto-combat"
    }
    if ((-not $ExportLearningDataset) -and $InspectFinalBossCombat) {
        $InspectArgs += "--inspect-final-boss-combat"
    }
    if ((-not $ExportLearningDataset) -and $InspectCombatLab) {
        $InspectArgs += @(
            "--inspect-combat-lab",
            "--combat-search-option", "wall_ms=$SearchWallMs",
            "--combat-search-option", "max_nodes=$SearchMaxNodes"
        )
        if ($ProbeBoss) {
            $InspectArgs += "--probe-boss"
        }
    }
    if ((-not $ExportLearningDataset) -and $CampaignBoundParameters.ContainsKey("InspectIndex") -and $InspectIndex -ge 0) {
        $InspectArgs += @("--inspect-index", "$InspectIndex")
    }
    if ((-not $ExportLearningDataset) -and $CampaignBoundParameters.ContainsKey("InspectAct") -and $InspectAct -gt 0) {
        $InspectArgs += @("--inspect-act", "$InspectAct")
    }
    if ((-not $ExportLearningDataset) -and $CampaignBoundParameters.ContainsKey("InspectFloor") -and $InspectFloor -gt 0) {
        $InspectArgs += @("--inspect-floor", "$InspectFloor")
    }
    if ((-not $ExportLearningDataset) -and $InspectBoundary) {
        $InspectArgs += @("--inspect-boundary", "$InspectBoundary")
    }
    if ((-not $ExportLearningDataset) -and $InspectQuery) {
        $InspectArgs += @("--inspect-query", "$InspectQuery")
    }

    $RenderedInspectArgs = $InspectArgs | ForEach-Object {
        if ($_ -match '^[A-Za-z0-9_./:=\\-]+$') { $_ } else { "'$($_ -replace "'", "''")'" }
    }
    $RenderedExe = if ($DriverExe -match '^[A-Za-z0-9_./:=\\-]+$') { $DriverExe } else { "'$($DriverExe -replace "'", "''")'" }
    $RenderedCommand = $RenderedExe + " " + ($RenderedInspectArgs -join " ")

    $InspectModeLabel = if ($ExportLearningDataset) { "dataset" } else { "inspect" }
    Write-Host "mode=$InspectModeLabel latest branch campaign"
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
    Write-Host "report=$LatestCampaignPath"
    Write-Host "checkpoint=$LatestCheckpointPath"

    if ($DryRun) {
        if ($NeedsBuild) {
            $RenderedBuildArgs = $BuildArgs | ForEach-Object {
                if ($_ -match '^[A-Za-z0-9_./:=\\-]+$') { $_ } else { "'$($_ -replace "'", "''")'" }
            }
            Write-Host ("cargo " + ($RenderedBuildArgs -join " "))
        }
        Write-Host $RenderedCommand
        exit 0
    }

    Push-Location $RepoRoot
    try {
        if ($NeedsBuild) {
            & cargo @BuildArgs
            if ($LASTEXITCODE -ne 0) {
                exit $LASTEXITCODE
            }
        }
        & $DriverExe @InspectArgs
        exit $LASTEXITCODE
    } finally {
        Pop-Location
    }
}

Write-Host "seed=$Seed"
Write-Host "ascension=A$Ascension domain=a$Ascension class=$Class"
$RenderedArgs = $DriverArgs | ForEach-Object {
    if ($_ -match '^[A-Za-z0-9_./:=\\-]+$') { $_ } else { "'$($_ -replace "'", "''")'" }
}
$RenderedExe = if ($DriverExe -match '^[A-Za-z0-9_./:=\\-]+$') { $DriverExe } else { "'$($DriverExe -replace "'", "''")'" }
$RenderedCommand = $RenderedExe + " " + ($RenderedArgs -join " ")

Write-Host "mode=$Mode branch campaign"
Write-Host "build=$BuildProfile exe=$DriverExe"
if ($NeedsBuild) {
    Write-Host "build-needed=yes"
} else {
    Write-Host "build-needed=no"
}
if ($BossRelicAxes) {
    Write-Host "boss-relic-axes=on active/frozen budgets are per boss relic lineage"
}
Write-Host "rerun-last=.\tools\campaign.ps1 -Last"
Write-Host "run-more=.\tools\campaign.ps1 -More"
Write-Host "run-one-more=.\tools\campaign.ps1 -More -Rounds 1"
Write-Host "report=$LatestCampaignPath"
Write-Host "checkpoint=$LatestCheckpointPath"
Write-Host "combat-segment=$CombatSegmentMode"
if ($ResumeCampaignPath) {
    Write-Host "resume=$ResumeCampaignPath"
    Write-Host "resume-rounds=$ResumeRoundsCompleted"
    if ($RoundBudgetSource -eq "LegacyMaxRounds") {
        Write-Warning "-MaxRounds with -More uses legacy target-total semantics. Prefer -Rounds N for additional rounds or -UntilRound N for a total-round target."
    }
    if ($TargetRounds -ne $null) {
        Write-Host "round-budget=$RoundBudgetSource target-rounds=$TargetRounds additional-rounds=$MaxRounds"
    } else {
        Write-Host "round-budget=preset additional-rounds=mode-default"
    }
    if ($ResumeCheckpointPath) {
        Write-Host "resume-checkpoint=$ResumeCheckpointPath"
    } else {
        Write-Host "resume-checkpoint=missing; falling back to replay"
    }
}

if ($More -and $TargetRounds -ne $null -and $MaxRounds -eq 0) {
    Write-Host "already-at-target-rounds=yes; nothing to run"
    exit 0
}

if ($DryRun) {
    if ($NeedsBuild) {
        $RenderedBuildArgs = $BuildArgs | ForEach-Object {
            if ($_ -match '^[A-Za-z0-9_./:=\\-]+$') { $_ } else { "'$($_ -replace "'", "''")'" }
        }
        Write-Host ("cargo " + ($RenderedBuildArgs -join " "))
    }
    Write-Host $RenderedCommand
    exit 0
}

Push-Location $RepoRoot
try {
    if ($NeedsBuild) {
        & cargo @BuildArgs
        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }
    }
    & $DriverExe @DriverArgs
    $DriverExitCode = $LASTEXITCODE
    if ($DriverExitCode -eq 0) {
        Set-Content -LiteralPath $LatestSeedPath -Value $Seed
        Set-Content -LiteralPath $LatestAscensionPath -Value $Ascension
        Set-Content -LiteralPath $LatestClassPath -Value $Class
        Set-Content -LiteralPath $LatestModePath -Value $Mode
        Set-Content -LiteralPath $LatestCommandPath -Value $RenderedCommand
    }
    exit $DriverExitCode
} finally {
    Pop-Location
}
