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
.\tools\campaign.ps1 -From latest -Continue
Continues the latest campaign artifact into a new run artifact.

.EXAMPLE
.\tools\campaign.ps1 -Inspect
Summarizes the latest saved campaign checkpoint with active/frozen/abandoned deck context.

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
.\tools\campaign.ps1 -DryRun
Prints the cargo command without updating the last seed or running it.

.NOTES
Detailed examples live in docs/CAMPAIGN_WRAPPER_USAGE.md.
#>
param(
    [Parameter(Position = 0)]
    [long] $Seed = 0,

    [switch] $Last,
    [switch] $More,
    [Alias("Continue")]
    [switch] $ContinueRun,
    [switch] $Inspect,
    [switch] $InspectArtifacts,
    [switch] $InspectState,
    [switch] $InspectShopEvidence,
    [switch] $InspectShopChallenge,
    [switch] $InspectCardRewardEvidence,
    [switch] $InspectDecisionObservations,
    [switch] $InspectJournal,
    [switch] $InspectLineageDecisions,
    [switch] $InspectCampfireEvidence,
    [switch] $InspectDeckMutation,
    [switch] $InspectRouteEvidence,
    [switch] $InspectLastAutoCombat,
    [switch] $InspectCombatLab,
    [switch] $InspectFinalBossCombat,
    [switch] $InspectCoverageGapMilestoneSummary,
    [switch] $InspectCoverageGapTargetState,
    [Alias("FromScratchLatest")]
    [switch] $InspectScratchLatest,
    [switch] $ProbeBoss,
    [switch] $DryRun,
    [switch] $Log,
    [switch] $NoProgress,
    [switch] $VerboseProgress,
    [switch] $Diagnose,
    [switch] $Perf,
    [switch] $BossSegments,
    [switch] $BossRelicAxes,
    [switch] $AutoCaptureCombat,
    [switch] $DebugBuild,
    [switch] $Build,
    [switch] $PlanTargets,
    [switch] $ContinueTargets,
    [switch] $PlanCoverageGaps,
    [switch] $ContinueCoverageGaps,
    [switch] $CoverageGapRoute,
    [switch] $CoverageGapRouteMissing,
    [switch] $CoverageGapEventBoundary,
    [switch] $CoverageGapEventBoundaryMissing,
    [Alias("OutScratch")]
    [switch] $Scratch,

    [string] $ExportLearningDataset = "",
    [string] $DecisionOutcomeDataset = "",
    [string] $AutoCaptureRoot = "",
    [string] $RunLabel = "",
    [string] $From = "",

    [ValidateSet("fast-run", "release-final", "release", "dev-opt", "debug")]
    [string] $BuildProfile = "fast-run",

    [ValidateSet("quick", "focused", "explore", "deep")]
    [string] $Mode = "explore",

    [ValidateRange(0, 100000)]
    [int] $Rounds = 0,

    [ValidateRange(0, 100000)]
    [int] $UntilRound = 0,

    [ValidateSet("", "Act1Boss", "Act2Start")]
    [string] $UntilMilestone = "",

    [ValidateSet("Act1Boss", "Act2Start")]
    [string] $CoverageGapMilestoneTarget = "Act2Start",

    [ValidateRange(1, 100000)]
    [int] $MilestoneStepRounds = 2,

    [ValidateRange(1, 100000)]
    [int] $MilestoneMaxRounds = 24,

    [ValidateSet("auto", "first_hit", "round_cap")]
    [string] $MilestoneStop = "auto",

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
    [string] $CoverageGapBucket = "",
    [string] $CoverageGapEventId = "",
    [string] $CoverageGapLane = "",
    [string] $CoverageGapOriginSource = "",
    [ValidateSet("", "missing", "target_only", "extended")]
    [string] $CoverageGapProgress = "",
    [ValidateSet("gap_closure", "frontier_expansion")]
    [string] $CoverageGapIntent = "gap_closure",
    [ValidateSet("auto", "target_only", "advance_rounds", "milestone")]
    [string] $CoverageGapExecution = "auto",
    [ValidateRange(0, 100)]
    [int] $VictoryHpPercent = 20,

    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $ExtraArgs
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
$CampaignDir = Join-Path $RepoRoot "tools\artifacts\campaigns"
$ScratchCampaignDir = Join-Path $CampaignDir "scratch"
$LatestSeedPath = Join-Path $CampaignDir "latest.seed.txt"
$LatestAscensionPath = Join-Path $CampaignDir "latest.ascension.txt"
$LatestClassPath = Join-Path $CampaignDir "latest.class.txt"
$LatestModePath = Join-Path $CampaignDir "latest.mode.txt"
$LatestCommandPath = Join-Path $CampaignDir "latest.command.txt"
$LatestManifestPath = Join-Path $CampaignDir "latest.manifest.json"
$LatestLogPath = Join-Path $CampaignDir "latest.log"
$LatestCampaignPath = Join-Path $CampaignDir "latest.campaign.json"
$LatestCheckpointPath = Join-Path $CampaignDir "latest.checkpoint.json"
$LatestDecisionOutcomePath = Join-Path $CampaignDir "latest.decision_outcomes.jsonl"
$LatestDecisionOutcomeBeforePath = Join-Path $CampaignDir "latest.decision_outcomes.before.jsonl"
$LatestDecisionOutcomeAfterPath = Join-Path $CampaignDir "latest.decision_outcomes.after.jsonl"

New-Item -ItemType Directory -Force -Path $CampaignDir | Out-Null

. (Join-Path $PSScriptRoot "campaign_artifacts.ps1")
. (Join-Path $PSScriptRoot "campaign_invocation.ps1")
. (Join-Path $PSScriptRoot "campaign_milestones.ps1")
. (Join-Path $PSScriptRoot "campaign_coverage_gaps.ps1")
. (Join-Path $PSScriptRoot "campaign_inspect.ps1")
. (Join-Path $PSScriptRoot "campaign_targets.ps1")
. (Join-Path $PSScriptRoot "campaign_build.ps1")

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

if (
    $InspectArtifacts -or
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
    $InspectCoverageGapTargetState -or
    ($InspectScratchLatest -and -not $ScratchLatestIsContinuationSource)
) {
    $Inspect = $true
}
if ($InspectShopChallenge -and -not $PSBoundParameters.ContainsKey("InspectBoundary")) {
    $InspectBoundary = "Shop"
}

if (($PlanTargets -or $ContinueTargets) -and ($PlanCoverageGaps -or $ContinueCoverageGaps)) {
    throw "Choose either targeted continuation (-PlanTargets/-ContinueTargets) or coverage-gap continuation (-PlanCoverageGaps/-ContinueCoverageGaps), not both."
}
if (
    $Scratch -and
    -not (
        $ContinueCoverageGaps -or
        ((-not $ContinueCampaign) -and (-not $Inspect) -and (-not $PlanTargets) -and (-not $ContinueTargets) -and (-not $PlanCoverageGaps))
    )
) {
    throw "-Scratch currently supports normal campaign runs and -ContinueCoverageGaps only."
}

$ReadsCampaignSource = $Inspect -or $ContinueCampaign -or $PlanTargets -or $ContinueTargets -or $PlanCoverageGaps -or $ContinueCoverageGaps
$CampaignSourceArtifact = $null
$CampaignSourceRunConfig = $null
if ($ReadsCampaignSource -or $Last) {
    $CampaignSourceArtifact = Get-CampaignSourceArtifact -Selector $From -UseScratchLatest $InspectScratchLatest
    $CampaignSourceRunConfig = Get-CampaignArtifactRunConfig `
        -CheckpointPath $CampaignSourceArtifact.CheckpointPath `
        -ManifestPath $CampaignSourceArtifact.ManifestPath
}

if ($PlanTargets -or $ContinueTargets -or $PlanCoverageGaps -or $ContinueCoverageGaps) {
    if (-not $PSBoundParameters.ContainsKey("Mode")) {
        $SavedMode = Get-CampaignArtifactMode -Artifact $CampaignSourceArtifact
        if ($SavedMode) {
            $Mode = $SavedMode
        } else {
            $Mode = "focused"
        }
    }
}

if ($ContinueCampaign) {
    if (-not $PSBoundParameters.ContainsKey("Mode")) {
        $SavedMode = Get-CampaignArtifactMode -Artifact $CampaignSourceArtifact
        if ($SavedMode) {
            $Mode = $SavedMode
        } else {
            $Mode = "deep"
        }
    }
}

if (($ReadsCampaignSource -or $Last) -and $Seed -le 0 -and $CampaignSourceRunConfig -and $CampaignSourceRunConfig.Seed -ne $null) {
    $Seed = [long] $CampaignSourceRunConfig.Seed
} elseif ($Last -and $Seed -le 0) {
    throw "No reusable campaign seed found in source artifact '$($CampaignSourceArtifact.Label)'. Use -Seed or a source with checkpoint run_state."
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
if ($Last -or $Inspect -or $ReadsCampaignSource) {
    if (-not $AscensionBound) {
        if ($CampaignSourceRunConfig -and $CampaignSourceRunConfig.Ascension -ne $null) {
            $Ascension = [int] $CampaignSourceRunConfig.Ascension
        } else {
            $SavedConfig = Read-LatestCheckpointRunConfig
            if ($SavedConfig -and $SavedConfig.ascension_level -ne $null) {
                $Ascension = [int] $SavedConfig.ascension_level
            }
        }
    }
    if (-not $ClassBound) {
        if ($CampaignSourceRunConfig -and $CampaignSourceRunConfig.Class) {
            $Class = ([string] $CampaignSourceRunConfig.Class).ToLowerInvariant()
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

$WritesCampaignOutput = (-not $Inspect) -and (-not $PlanTargets) -and (-not $PlanCoverageGaps)
$RunOutputArtifact = $null
$ScratchLabel = ""
$RunOutputCampaignPath = ""
$RunOutputCheckpointPath = ""
$RunCommandPath = ""
$RunManifestPath = ""
$RunLogPath = ""
if ($WritesCampaignOutput) {
    $OutputBaseLabel = if ($RunLabel) {
        $RunLabel
    } elseif ($ContinueCoverageGaps) {
        "coverage-gap-seed$Seed"
    } elseif ($ContinueTargets) {
        "targeted-continuation-seed$Seed"
    } elseif ($ContinueCampaign) {
        "continue-seed$Seed"
    } else {
        "campaign-seed$Seed"
    }
    $RunOutputArtifact = if ($Scratch) {
        New-CampaignScratchArtifact -BaseLabel $OutputBaseLabel
    } else {
        New-CampaignRunArtifact -BaseLabel $OutputBaseLabel
    }
    $ScratchLabel = if ($Scratch) { $RunOutputArtifact.Id } else { "" }
    $RunOutputCampaignPath = $RunOutputArtifact.ReportPath
    $RunOutputCheckpointPath = $RunOutputArtifact.CheckpointPath
    $RunCommandPath = $RunOutputArtifact.CommandPath
    $RunManifestPath = $RunOutputArtifact.ManifestPath
    $RunLogPath = $RunOutputArtifact.LogPath
    if (-not $DryRun) {
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $RunOutputCampaignPath) | Out-Null
    }
}

$CampaignBoundParameters = @{}
foreach ($ParameterName in $PSBoundParameters.Keys) {
    $CampaignBoundParameters[$ParameterName] = $true
}
$CampaignWrapperInvocationLine = if ($MyInvocation.Line) { $MyInvocation.Line.Trim() } else { "" }
$CampaignWrapperBoundParameters = [ordered]@{}
foreach ($ParameterName in ($PSBoundParameters.Keys | Sort-Object)) {
    $CampaignWrapperBoundParameters[$ParameterName] =
        Convert-CampaignWrapperParameterValue -Value $PSBoundParameters[$ParameterName]
}

$RoundsBound = $CampaignBoundParameters.ContainsKey("Rounds")
$UntilRoundBound = $CampaignBoundParameters.ContainsKey("UntilRound")
$UntilMilestoneBound = $CampaignBoundParameters.ContainsKey("UntilMilestone") -and $UntilMilestone
$MaxRoundsBound = $CampaignBoundParameters.ContainsKey("MaxRounds")
$CoverageGapRoutePreset = $CoverageGapRoute -or $CoverageGapRouteMissing
$CoverageGapEventBoundaryPreset = $CoverageGapEventBoundary -or $CoverageGapEventBoundaryMissing
if ($CoverageGapRoutePreset -and $CoverageGapEventBoundaryPreset) {
    throw "Choose a route coverage-gap preset or an event-boundary coverage-gap preset, not both."
}
if ($CoverageGapRoutePreset) {
    $PresetName = if ($CoverageGapRouteMissing) { "-CoverageGapRouteMissing" } else { "-CoverageGapRoute" }
    Assert-CoverageGapPresetCompatible -Preset $PresetName -Name "CoverageGapBucket" -Actual $CoverageGapBucket -Expected "route"
    Assert-CoverageGapPresetCompatible -Preset $PresetName -Name "CoverageGapOriginSource" -Actual $CoverageGapOriginSource -Expected "map_decision_packet"
    if (-not $CoverageGapBucket) {
        $CoverageGapBucket = "route"
    }
    if (-not $CoverageGapOriginSource) {
        $CoverageGapOriginSource = "map_decision_packet"
    }
}
if ($CoverageGapRouteMissing) {
    Assert-CoverageGapPresetCompatible -Preset "-CoverageGapRouteMissing" -Name "CoverageGapProgress" -Actual $CoverageGapProgress -Expected "missing"
    if (-not $CoverageGapProgress) {
        $CoverageGapProgress = "missing"
    }
}
if ($CoverageGapEventBoundaryPreset) {
    $PresetName = if ($CoverageGapEventBoundaryMissing) { "-CoverageGapEventBoundaryMissing" } else { "-CoverageGapEventBoundary" }
    Assert-CoverageGapPresetCompatible -Preset $PresetName -Name "CoverageGapBucket" -Actual $CoverageGapBucket -Expected "event"
    Assert-CoverageGapPresetCompatible -Preset $PresetName -Name "CoverageGapOriginSource" -Actual $CoverageGapOriginSource -Expected "event_boundary_packet"
    if (-not $CoverageGapBucket) {
        $CoverageGapBucket = "event"
    }
    if (-not $CoverageGapOriginSource) {
        $CoverageGapOriginSource = "event_boundary_packet"
    }
}
if ($CoverageGapEventBoundaryMissing) {
    Assert-CoverageGapPresetCompatible -Preset "-CoverageGapEventBoundaryMissing" -Name "CoverageGapProgress" -Actual $CoverageGapProgress -Expected "missing"
    if (-not $CoverageGapProgress) {
        $CoverageGapProgress = "missing"
    }
}

$CoverageGapFilterArgs = @(New-CoverageGapFilterArgs `
    -Bucket $CoverageGapBucket `
    -EventId $CoverageGapEventId `
    -Lane $CoverageGapLane `
    -OriginSource $CoverageGapOriginSource `
    -Progress $CoverageGapProgress)
$CoverageGapFilterLabel = Format-CoverageGapFilterLabel `
    -Bucket $CoverageGapBucket `
    -EventId $CoverageGapEventId `
    -Lane $CoverageGapLane `
    -OriginSource $CoverageGapOriginSource `
    -Progress $CoverageGapProgress
$CoverageGapResultFilterArgs = @(New-CoverageGapFilterArgs `
    -Bucket $CoverageGapBucket `
    -EventId $CoverageGapEventId `
    -Lane $CoverageGapLane `
    -OriginSource $CoverageGapOriginSource `
    -Progress "")
$CoverageGapResultFilterLabel = Format-CoverageGapFilterLabel `
    -Bucket $CoverageGapBucket `
    -EventId $CoverageGapEventId `
    -Lane $CoverageGapLane `
    -OriginSource $CoverageGapOriginSource `
    -Progress ""

if (($RoundsBound -and $UntilRoundBound) -or ($RoundsBound -and $MaxRoundsBound) -or ($UntilRoundBound -and $MaxRoundsBound)) {
    throw "Choose only one round budget: -Rounds N, -UntilRound N, or legacy -MaxRounds N."
}
if ($UntilMilestoneBound -and ($RoundsBound -or $UntilRoundBound -or $MaxRoundsBound)) {
    throw "-UntilMilestone owns the round budget. Use -MilestoneStepRounds and -MilestoneMaxRounds instead of -Rounds, -UntilRound, or -MaxRounds."
}
if ($UntilMilestoneBound -and ($PlanTargets -or $PlanCoverageGaps -or $Inspect)) {
    throw "-UntilMilestone requires an executing command (-Continue, -ContinueTargets, -ContinueCoverageGaps, or a normal run), not a plan/inspect command."
}
$ResolvedMilestoneStop = $MilestoneStop
if ($ResolvedMilestoneStop -eq "auto") {
    if ($ContinueCoverageGaps) {
        $ResolvedMilestoneStop = "round_cap"
    } else {
        $ResolvedMilestoneStop = "first_hit"
    }
}

$DriverRoundBudgetArgs = @()
$RoundBudgetSource = if ($MaxRoundsBound) { "MaxRounds" } else { "preset" }
if ($UntilMilestoneBound) {
    $MaxRounds = $MilestoneStepRounds
    $DriverRoundBudgetArgs = @("--rounds", "$MilestoneStepRounds")
    $RoundBudgetSource = "UntilMilestone"
} elseif (-not $ContinueCampaign) {
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
if ($ContinueCampaign) {
    if (-not $CampaignSourceArtifact) {
        $CampaignSourceArtifact = Get-CampaignSourceArtifact -Selector $From -UseScratchLatest $false
    }
    if (-not (Test-Path $CampaignSourceArtifact.ReportPath)) {
        throw "No campaign report found for source '$($CampaignSourceArtifact.Label)' at $($CampaignSourceArtifact.ReportPath)."
    }
    $ResumeCampaignPath = $CampaignSourceArtifact.ReportPath
    $ResumeReport = Get-Content -LiteralPath $ResumeCampaignPath -Raw | ConvertFrom-Json
    $ResumeRoundsCompleted = [int] $ResumeReport.rounds_completed
    if ($UntilMilestoneBound) {
        $TargetRounds = $null
        $MaxRounds = $MilestoneStepRounds
        $DriverRoundBudgetArgs = @("--rounds", "$MilestoneStepRounds")
        $RoundBudgetSource = "UntilMilestone"
    } elseif ($RoundsBound) {
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
        $TargetRounds = $ResumeRoundsCompleted + $MaxRounds
        $DriverRoundBudgetArgs = @("--rounds", "$MaxRounds")
        $RoundBudgetSource = "MaxRounds"
    }
    $DriverArgs += @("--resume", "$ResumeCampaignPath")
    if (Test-Path $CampaignSourceArtifact.CheckpointPath) {
        $ResumeCheckpointPath = $CampaignSourceArtifact.CheckpointPath
        $DriverArgs += @("--resume-checkpoint", "$ResumeCheckpointPath")
    }
}

$DriverArgs += @("--out", "$RunOutputCampaignPath", "--checkpoint-out", "$RunOutputCheckpointPath")

if ($DriverRoundBudgetArgs.Count -gt 0) {
    $DriverArgs += $DriverRoundBudgetArgs
}

$CombatSegmentMode = "custom"
if (-not (Test-ExtraCombatOptionKey -Tokens $ExtraArgs -Keys @("segment", "segment_mode", "partial", "partial_mode"))) {
    if ($BossSegments) {
        $CombatSegmentMode = "turn"
    } else {
        $CombatSegmentMode = "non_boss_turn"
    }
}

if ($ExportLearningDataset -and -not $Inspect) {
    $DriverArgs += @("--export-learning-dataset", "$ExportLearningDataset")
}
$DriverArgs = Add-CampaignSharedDriverOptions `
    -Arguments $DriverArgs `
    -IncludeActiveLineageDiversity $true `
    -IncludeBossRelicAxes $true `
    -IncludeAutoCaptureCombat $true

$NeedsBuild = $Build -or (Test-DriverNeedsBuild $DriverExe)

if ($PlanTargets -or $ContinueTargets -or $PlanCoverageGaps -or $ContinueCoverageGaps) {
    if ($InspectScratchLatest -and ($PlanTargets -or $ContinueTargets)) {
        throw "-InspectScratchLatest is not supported for targeted continuation yet; use inspect or coverage-gap continuation."
    }
    $ContinuationSource = Get-CampaignSourceArtifact -Selector $From -UseScratchLatest $InspectScratchLatest
    $SourceCampaignPath = $ContinuationSource.ReportPath
    $SourceCheckpointPath = $ContinuationSource.CheckpointPath
    $SourceManifestPath = $ContinuationSource.ManifestPath
    $SourceLabel = $ContinuationSource.Label

    if (-not (Test-Path $SourceCampaignPath)) {
        throw "No previous campaign report found at $SourceCampaignPath. Run .\tools\campaign.ps1 first."
    }
    if (-not (Test-Path $SourceCheckpointPath)) {
        throw "No previous campaign checkpoint found at $SourceCheckpointPath. Run .\tools\campaign.ps1 first."
    }

    $SourceRunConfig = Get-CampaignArtifactRunConfig `
        -CheckpointPath $SourceCheckpointPath `
        -ManifestPath $SourceManifestPath
    if ((-not $PSBoundParameters.ContainsKey("Seed") -or $Seed -le 0) -and $SourceRunConfig.Seed -ne $null) {
        $Seed = [long] $SourceRunConfig.Seed
    }
    if (-not $AscensionBound -and $SourceRunConfig.Ascension -ne $null) {
        $Ascension = [int] $SourceRunConfig.Ascension
    }
    if (-not $ClassBound -and $SourceRunConfig.Class) {
        $Class = [string] $SourceRunConfig.Class
    }
    if (-not $CampaignBoundParameters.ContainsKey("Mode") -and $SourceRunConfig.Mode) {
        $Mode = [string] $SourceRunConfig.Mode
    }

    $TargetDecisionOutcomePath = if ($DecisionOutcomeDataset) {
        $DecisionOutcomeDataset
    } elseif ($ContinueTargets) {
        $LatestDecisionOutcomeBeforePath
    } else {
        $LatestDecisionOutcomePath
    }
    $CoveragePlanArgs = New-CoverageGapPlanDriverArgs `
        -SourceCampaignPath $SourceCampaignPath `
        -SourceCheckpointPath $SourceCheckpointPath
    $ExportDecisionArgs = New-TargetedContinuationExportBeforeArgs `
        -SourceCampaignPath $SourceCampaignPath `
        -SourceCheckpointPath $SourceCheckpointPath `
        -TargetDecisionOutcomePath $TargetDecisionOutcomePath
    $PlanTargetArgs = New-TargetedContinuationPlanDriverArgs `
        -TargetDecisionOutcomePath $TargetDecisionOutcomePath
    $ExportDecisionAfterArgs = New-TargetedContinuationExportAfterArgs `
        -RunOutputCampaignPath $RunOutputCampaignPath `
        -RunOutputCheckpointPath $RunOutputCheckpointPath `
        -LatestDecisionOutcomeAfterPath $LatestDecisionOutcomeAfterPath
    $ContinuationEffectArgs = New-TargetedContinuationEffectArgs `
        -TargetDecisionOutcomePath $TargetDecisionOutcomePath `
        -LatestDecisionOutcomeAfterPath $LatestDecisionOutcomeAfterPath

    $ResumeReport = Get-Content -LiteralPath $SourceCampaignPath -Raw | ConvertFrom-Json
    $ResumeRoundsCompleted = [int] $ResumeReport.rounds_completed
    $ContinuationRounds = 1
    $ContinuationRoundBudgetArgs = @("--rounds", "1")
    $TargetRounds = $null
    $ContinuationRoundSource = "default"
    if ($UntilMilestoneBound) {
        $ContinuationRounds = $MilestoneStepRounds
        $ContinuationRoundSource = "UntilMilestone"
        $ContinuationRoundBudgetArgs = @("--rounds", "$MilestoneStepRounds")
    } elseif ($RoundsBound) {
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
    if ($CoverageGapExecution -eq "milestone" -and -not $UntilMilestoneBound) {
        throw "-CoverageGapExecution milestone requires -UntilMilestone."
    }
    $CoverageGapDriverExecution = $CoverageGapExecution
    $CoverageGapExecutionLabel = $CoverageGapExecution
    if ($CoverageGapExecution -eq "auto") {
        $HasExplicitRoundBudget = $RoundsBound -or $UntilRoundBound -or $MaxRoundsBound
        if ($UntilMilestoneBound -and $ContinueCoverageGaps) {
            $CoverageGapExecutionLabel = "milestone_continuation"
            $CoverageGapDriverExecution = "target_only"
        } elseif ($HasExplicitRoundBudget) {
            $CoverageGapExecutionLabel = "advance_rounds"
            $CoverageGapDriverExecution = "advance_rounds"
        } elseif ($CoverageGapIntent -eq "gap_closure") {
            $CoverageGapExecutionLabel = "target_only"
            $CoverageGapDriverExecution = "target_only"
        } else {
            $CoverageGapExecutionLabel = "advance_rounds"
            $CoverageGapDriverExecution = "advance_rounds"
        }
    } elseif ($CoverageGapExecution -eq "milestone") {
        $CoverageGapExecutionLabel = "milestone_continuation"
        $CoverageGapDriverExecution = "target_only"
    }
    $CoverageGapInitialSpentRounds = $ContinuationRounds
    if ($CoverageGapDriverExecution -eq "target_only") {
        $CoverageGapInitialSpentRounds = 0
    }

    $ContinueTargetArgs = New-TargetedContinuationContinueDriverArgs `
        -SourceCampaignPath $SourceCampaignPath `
        -SourceCheckpointPath $SourceCheckpointPath `
        -RunOutputCampaignPath $RunOutputCampaignPath `
        -RunOutputCheckpointPath $RunOutputCheckpointPath `
        -TargetDecisionOutcomePath $TargetDecisionOutcomePath `
        -RoundBudgetArgs $ContinuationRoundBudgetArgs
    $ContinueCoverageGapArgs = New-CoverageGapContinueDriverArgs `
        -SourceCampaignPath $SourceCampaignPath `
        -SourceCheckpointPath $SourceCheckpointPath `
        -RunOutputCampaignPath $RunOutputCampaignPath `
        -RunOutputCheckpointPath $RunOutputCheckpointPath `
        -RoundBudgetArgs $ContinuationRoundBudgetArgs `
        -DriverExecution $CoverageGapDriverExecution

    $ContinuationModeLabel = if ($PlanCoverageGaps -or $ContinueCoverageGaps) { "coverage-gap-continuation" } else { "targeted-continuation" }
    Write-Host "mode=$ContinuationModeLabel branch campaign"
    Write-Host "seed=$Seed"
    Write-Host "ascension=A$Ascension domain=a$Ascension class=$Class"
    Write-Host "build=$BuildProfile exe=$DriverExe"
    if ($NeedsBuild) {
        Write-Host "build-needed=yes"
    } else {
        Write-Host "build-needed=no"
    }
    Write-Host "source=$SourceLabel"
    Write-Host "source-report=$SourceCampaignPath"
    Write-Host "source-checkpoint=$SourceCheckpointPath"
    if ($Scratch) {
        Write-Host "scratch=yes label=$ScratchLabel"
        Write-Host "report=$RunOutputCampaignPath"
        Write-Host "checkpoint=$RunOutputCheckpointPath"
    } else {
        if ($ContinueTargets -or $ContinueCoverageGaps) {
            Write-Host "report=$RunOutputCampaignPath"
            Write-Host "checkpoint=$RunOutputCheckpointPath"
        }
    }
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
    if ($UntilMilestoneBound) {
        Write-Host "until-milestone=$UntilMilestone step-rounds=$MilestoneStepRounds max-additional-rounds=$MilestoneMaxRounds stop=$ResolvedMilestoneStop"
    }
    if ($PlanCoverageGaps) {
        Write-Host "coverage-gap-plan=$CoverageGapLimit candidates-per-decision=$CoverageGapCandidatesPerDecision"
        Write-Host "coverage-gap-filter=$CoverageGapFilterLabel"
    }
    if ($ContinueCoverageGaps) {
        if ($CoverageGapExecutionLabel -eq $CoverageGapDriverExecution) {
            Write-Host "coverage-gap-continue=$CoverageGapLimit candidates-per-decision=$CoverageGapCandidatesPerDecision intent=$CoverageGapIntent execution=$CoverageGapExecutionLabel"
        } else {
            Write-Host "coverage-gap-continue=$CoverageGapLimit candidates-per-decision=$CoverageGapCandidatesPerDecision intent=$CoverageGapIntent execution=$CoverageGapExecutionLabel seed-execution=$CoverageGapDriverExecution"
        }
        Write-Host "coverage-gap-filter=$CoverageGapFilterLabel"
        Write-Host "resume-rounds=$ResumeRoundsCompleted"
        if ($TargetRounds -ne $null) {
            Write-Host "round-budget=$ContinuationRoundSource target-rounds=$TargetRounds additional-rounds=$ContinuationRounds"
        } else {
            Write-Host "round-budget=$ContinuationRoundSource additional-rounds=$ContinuationRounds"
        }
        if ($UntilMilestoneBound) {
            Write-Host "milestone-initial-spent-rounds=$CoverageGapInitialSpentRounds"
            if ($CoverageGapResultFilterLabel -ne $CoverageGapFilterLabel) {
                Write-Host "coverage-gap-result-filter=$CoverageGapResultFilterLabel"
            }
        }
    }

    if ($DryRun) {
        if ($NeedsBuild) {
            Write-CampaignBuildCommandPreview -BuildArgs $BuildArgs
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
        if ($UntilMilestoneBound) {
            Write-Host "milestone-loop-command-template:"
            Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments (New-MilestoneResumeDriverArgs -StepRounds $MilestoneStepRounds))
            if ($ContinueCoverageGaps) {
                $SummaryArgs = @(
                    "inspect",
                    "--inspect-report", "$RunOutputCampaignPath",
                    "--inspect-coverage-gap-milestone-summary",
                    "--coverage-gap-milestone-target", "$UntilMilestone"
                )
                $SummaryArgs += $CoverageGapResultFilterArgs
                if (Test-Path -LiteralPath $RunOutputCheckpointPath) {
                    $SummaryArgs += @("--inspect-checkpoint", "$RunOutputCheckpointPath")
                }
                Write-Host "milestone-summary-command:"
                Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments $SummaryArgs)
            }
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
                Write-CampaignPrimaryDriverCommandRecord -PrimaryDriverCommandLine (Format-CommandLine -ExePath $DriverExe -Arguments $ContinueTargetArgs)
                if ($UntilMilestoneBound) {
                    Invoke-CampaignUntilMilestone -AlreadySpentRounds $ContinuationRounds
                    $DriverExitCode = $script:CampaignMilestoneExitCode
                }
            }
            exit $DriverExitCode
        }
        if ($ContinueCoverageGaps) {
            & $DriverExe @ContinueCoverageGapArgs
            $DriverExitCode = $LASTEXITCODE
            if ($DriverExitCode -eq 0) {
                Write-CampaignPrimaryDriverCommandRecord -PrimaryDriverCommandLine (Format-CommandLine -ExePath $DriverExe -Arguments $ContinueCoverageGapArgs)
                Write-CampaignWrapperManifest `
                    -Path $RunManifestPath `
                    -Manifest (New-CoverageGapWrapperManifest -ExitCode $DriverExitCode -Stage "initial_driver_completed")
                if ($UntilMilestoneBound) {
                    Invoke-CampaignUntilMilestone -AlreadySpentRounds $CoverageGapInitialSpentRounds
                    $DriverExitCode = $script:CampaignMilestoneExitCode
                    if ($DriverExitCode -eq 0) {
                        $DriverExitCode = Invoke-CoverageGapMilestoneSummary -Target $UntilMilestone
                    }
                }
                $ManifestStage = if ($UntilMilestoneBound) { "completed_with_milestone_loop" } else { "completed" }
                Write-CampaignWrapperManifest `
                    -Path $RunManifestPath `
                    -Manifest (New-CoverageGapWrapperManifest -ExitCode $DriverExitCode -Stage $ManifestStage)
            }
            exit $DriverExitCode
        }
        exit 0
    } finally {
        Pop-Location
    }
}

if ($Inspect) {
    $InspectSource = Get-CampaignSourceArtifact -Selector $From -UseScratchLatest $InspectScratchLatest
    $InspectCampaignPath = $InspectSource.ReportPath
    $InspectCheckpointPath = $InspectSource.CheckpointPath
    $InspectManifestPath = $InspectSource.ManifestPath
    $InspectLogPath = $InspectSource.LogPath
    $InspectCommandPath = $InspectSource.CommandPath
    $InspectSourceLabel = $InspectSource.Label

    if ($InspectArtifacts) {
        Write-CampaignArtifactSummary `
            -SourceLabel $InspectSourceLabel `
            -ReportPath $InspectCampaignPath `
            -CheckpointPath $InspectCheckpointPath `
            -ManifestPath $InspectManifestPath `
            -LogPath $InspectLogPath `
            -CommandPath $InspectCommandPath
        exit 0
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
    $RenderedCommand = Format-CommandLine -ExePath $DriverExe -Arguments $InspectArgs

    $InspectModeLabel = if ($ExportLearningDataset) { "dataset" } else { "inspect" }
    Write-Host "mode=$InspectModeLabel $InspectSourceLabel branch campaign"
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
    if ($InspectCoverageGapMilestoneSummary) {
        Write-Host "coverage-gap-filter=$CoverageGapFilterLabel"
    }
    Write-Host "report=$InspectCampaignPath"
    Write-Host "checkpoint=$InspectCheckpointPath"

    if ($DryRun) {
        if ($NeedsBuild) {
            Write-CampaignBuildCommandPreview -BuildArgs $BuildArgs
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
Write-Host "continue-latest=.\tools\campaign.ps1 -From latest -Continue"
Write-Host "continue-one-round=.\tools\campaign.ps1 -From latest -Continue -Rounds 1"
Write-Host "report=$RunOutputCampaignPath"
Write-Host "checkpoint=$RunOutputCheckpointPath"
Write-Host "manifest=$RunManifestPath"
if ($Log) {
    Write-Host "log=$RunLogPath"
}
Write-Host "combat-segment=$CombatSegmentMode"
if ($ResumeCampaignPath) {
    Write-Host "resume=$ResumeCampaignPath"
    Write-Host "resume-rounds=$ResumeRoundsCompleted"
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
if ($UntilMilestoneBound) {
    Write-Host "until-milestone=$UntilMilestone step-rounds=$MilestoneStepRounds max-additional-rounds=$MilestoneMaxRounds"
}

if ($ContinueCampaign -and $TargetRounds -ne $null -and $MaxRounds -eq 0) {
    Write-Host "already-at-target-rounds=yes; nothing to run"
    exit 0
}
if ($ContinueCampaign -and $UntilMilestoneBound) {
    $InitialMilestoneStatus = Get-CampaignMilestoneStatus -ReportPath $ResumeCampaignPath -Milestone $UntilMilestone
    if ($InitialMilestoneStatus.Reached) {
        Write-Host "already-at-milestone=yes target=$UntilMilestone hits=$($InitialMilestoneStatus.HitCount) furthest=A$($InitialMilestoneStatus.FurthestAct)F$($InitialMilestoneStatus.FurthestFloor)"
        exit 0
    }
}

if ($DryRun) {
    if ($NeedsBuild) {
        Write-CampaignBuildCommandPreview -BuildArgs $BuildArgs
    }
    Write-Host $RenderedCommand
    if ($UntilMilestoneBound) {
        Write-Host "milestone-loop-command-template:"
        Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments (New-MilestoneResumeDriverArgs -StepRounds $MilestoneStepRounds))
    }
    exit 0
}

function New-CampaignRunWrapperManifest {
    param(
        [int] $ExitCode,
        [string] $Stage
    )

    $Manifest = New-CampaignWrapperManifestBase `
        -ExitCode $ExitCode `
        -Stage $Stage `
        -CommandKind "campaign_run" `
        -PrimaryDriverArgs $DriverArgs `
        -PrimaryDriverCommand $RenderedCommand
    $Manifest["resume_report"] = if ($ResumeCampaignPath) { "$ResumeCampaignPath" } else { "" }
    $Manifest["resume_checkpoint"] = if ($ResumeCheckpointPath) { "$ResumeCheckpointPath" } else { "" }
    $Manifest["log_file"] = if ($Log) { "$RunLogPath" } else { "" }
    $Manifest["round_budget"] = [ordered]@{
        source = $RoundBudgetSource
        target_rounds = $TargetRounds
        additional_rounds = $MaxRounds
    }

    if ($UntilMilestoneBound) {
        $MilestoneResumeArgs = New-MilestoneResumeDriverArgs -StepRounds $MilestoneStepRounds
        $Manifest["milestone"] = [ordered]@{
            target = $UntilMilestone
            stop = $ResolvedMilestoneStop
            step_rounds = $MilestoneStepRounds
            max_additional_rounds = $MilestoneMaxRounds
            resume_driver_args_template = @($MilestoneResumeArgs)
            resume_driver_command_template = (Format-CommandLine -ExePath $DriverExe -Arguments $MilestoneResumeArgs)
        }
    }

    return $Manifest
}

Push-Location $RepoRoot
try {
    if ($NeedsBuild) {
        & cargo @BuildArgs
        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }
    }
    if ($Log) {
        $DriverExitCode = Invoke-CampaignLoggedDriverCommand -ExePath $DriverExe -Arguments $DriverArgs -LogPath $RunLogPath
    } else {
        & $DriverExe @DriverArgs
        $DriverExitCode = $LASTEXITCODE
    }
    if ($DriverExitCode -eq 0) {
        Write-CampaignPrimaryDriverCommandRecord -PrimaryDriverCommandLine $RenderedCommand
        Write-CampaignWrapperManifest `
            -Path $RunManifestPath `
            -Manifest (New-CampaignRunWrapperManifest -ExitCode $DriverExitCode -Stage "initial_driver_completed")
        if ($UntilMilestoneBound) {
            Invoke-CampaignUntilMilestone -AlreadySpentRounds $MaxRounds
            $DriverExitCode = $script:CampaignMilestoneExitCode
        }
        $ManifestStage = if ($UntilMilestoneBound) { "completed_with_milestone_loop" } else { "completed" }
        Write-CampaignWrapperManifest `
            -Path $RunManifestPath `
            -Manifest (New-CampaignRunWrapperManifest -ExitCode $DriverExitCode -Stage $ManifestStage)
    }
    exit $DriverExitCode
} finally {
    Pop-Location
}
