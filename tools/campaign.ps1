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

New-Item -ItemType Directory -Force -Path $CampaignDir | Out-Null

. (Join-Path $PSScriptRoot "campaign_artifacts.ps1")
. (Join-Path $PSScriptRoot "campaign_invocation.ps1")
. (Join-Path $PSScriptRoot "campaign_milestones.ps1")
. (Join-Path $PSScriptRoot "campaign_coverage_gaps.ps1")
. (Join-Path $PSScriptRoot "campaign_inspect.ps1")
. (Join-Path $PSScriptRoot "campaign_targets.ps1")
. (Join-Path $PSScriptRoot "campaign_build.ps1")
. (Join-Path $PSScriptRoot "campaign_source.ps1")

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
$CampaignSourceContext = Get-CampaignSourceContext `
    -ReadsCampaignSource $ReadsCampaignSource `
    -Last $Last `
    -From $From `
    -UseScratchLatest $InspectScratchLatest
$CampaignSourceArtifact = $CampaignSourceContext.Artifact
$CampaignSourceRunConfig = $CampaignSourceContext.RunConfig
$Mode = Resolve-CampaignMode `
    -Mode $Mode `
    -ModeBound ($PSBoundParameters.ContainsKey("Mode")) `
    -IsContinuationFamily ($PlanTargets -or $ContinueTargets -or $PlanCoverageGaps -or $ContinueCoverageGaps) `
    -ContinueCampaign $ContinueCampaign `
    -SourceArtifact $CampaignSourceArtifact
$Seed = Resolve-CampaignSeed `
    -Seed $Seed `
    -ReadsCampaignSource $ReadsCampaignSource `
    -Last $Last `
    -SourceArtifact $CampaignSourceArtifact `
    -SourceRunConfig $CampaignSourceRunConfig

$AscensionBound = $PSBoundParameters.ContainsKey("Ascension")
$ClassBound = $PSBoundParameters.ContainsKey("Class")
$DomainBound = $PSBoundParameters.ContainsKey("Domain") -and $Domain
$RunIdentity = Resolve-CampaignRunIdentity `
    -Ascension $Ascension `
    -Class $Class `
    -Domain $Domain `
    -AscensionBound $AscensionBound `
    -ClassBound $ClassBound `
    -DomainBound $DomainBound `
    -Last $Last `
    -Inspect $Inspect `
    -ReadsCampaignSource $ReadsCampaignSource `
    -SourceRunConfig $CampaignSourceRunConfig
$Ascension = $RunIdentity.Ascension
$Class = $RunIdentity.Class
$AscensionBound = $RunIdentity.AscensionBound
$ClassBound = $RunIdentity.ClassBound

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
$CoverageGapFilterContext = Resolve-CoverageGapFilterContext `
    -Route ([bool] $CoverageGapRoute) `
    -RouteMissing ([bool] $CoverageGapRouteMissing) `
    -EventBoundary ([bool] $CoverageGapEventBoundary) `
    -EventBoundaryMissing ([bool] $CoverageGapEventBoundaryMissing) `
    -Bucket $CoverageGapBucket `
    -EventId $CoverageGapEventId `
    -Lane $CoverageGapLane `
    -OriginSource $CoverageGapOriginSource `
    -Progress $CoverageGapProgress
$CoverageGapBucket = $CoverageGapFilterContext.Bucket
$CoverageGapEventId = $CoverageGapFilterContext.EventId
$CoverageGapLane = $CoverageGapFilterContext.Lane
$CoverageGapOriginSource = $CoverageGapFilterContext.OriginSource
$CoverageGapProgress = $CoverageGapFilterContext.Progress
$CoverageGapFilterArgs = @($CoverageGapFilterContext.FilterArgs)
$CoverageGapFilterLabel = $CoverageGapFilterContext.FilterLabel
$CoverageGapResultFilterArgs = @($CoverageGapFilterContext.ResultFilterArgs)
$CoverageGapResultFilterLabel = $CoverageGapFilterContext.ResultFilterLabel

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
    $ResumeSource = $CampaignSourceArtifact
    if (-not $ResumeSource) {
        throw "Internal error: campaign continuation did not resolve a source artifact."
    }
    if (-not (Test-Path $ResumeSource.ReportPath)) {
        throw "No campaign report found for source '$($ResumeSource.Label)' at $($ResumeSource.ReportPath)."
    }
    $ResumeCampaignPath = $ResumeSource.ReportPath
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
    $ContinuationSource = $CampaignSourceArtifact
    if (-not $ContinuationSource) {
        throw "Internal error: campaign continuation did not resolve a source artifact."
    }
    $SourceCampaignPath = $ContinuationSource.ReportPath
    $SourceCheckpointPath = $ContinuationSource.CheckpointPath
    $SourceLabel = $ContinuationSource.Label

    if (-not (Test-Path $SourceCampaignPath)) {
        throw "No previous campaign report found at $SourceCampaignPath. Run .\tools\campaign.ps1 first."
    }
    if (-not (Test-Path $SourceCheckpointPath)) {
        throw "No previous campaign checkpoint found at $SourceCheckpointPath. Run .\tools\campaign.ps1 first."
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
    $ContinuationRoundBudget = Resolve-CampaignAdditionalRoundBudget `
        -ResumeRoundsCompleted $ResumeRoundsCompleted `
        -UntilMilestoneBound $UntilMilestoneBound `
        -MilestoneStepRounds $MilestoneStepRounds `
        -RoundsBound $RoundsBound `
        -Rounds $Rounds `
        -UntilRoundBound $UntilRoundBound `
        -UntilRound $UntilRound `
        -MaxRoundsBound $MaxRoundsBound `
        -MaxRounds $MaxRounds `
        -MaxRoundsDriverFlag "--max-rounds"
    $ContinuationRounds = $ContinuationRoundBudget.AdditionalRounds
    $ContinuationRoundBudgetArgs = @($ContinuationRoundBudget.Args)
    $TargetRounds = $ContinuationRoundBudget.TargetRounds
    $ContinuationRoundSource = $ContinuationRoundBudget.Source
    $CoverageGapExecutionContext = Resolve-CoverageGapExecutionContext `
        -Execution $CoverageGapExecution `
        -UntilMilestoneBound $UntilMilestoneBound `
        -ContinueCoverageGaps ([bool] $ContinueCoverageGaps) `
        -HasExplicitRoundBudget ($RoundsBound -or $UntilRoundBound -or $MaxRoundsBound) `
        -Intent $CoverageGapIntent `
        -ContinuationRounds $ContinuationRounds
    $CoverageGapExecutionLabel = $CoverageGapExecutionContext.Label
    $CoverageGapDriverExecution = $CoverageGapExecutionContext.DriverExecution
    $CoverageGapInitialSpentRounds = $CoverageGapExecutionContext.InitialSpentRounds

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
        Write-TargetedContinuationDryRunCommands `
            -PlanTargets ([bool] $PlanTargets) `
            -ContinueTargets ([bool] $ContinueTargets) `
            -DriverExe $DriverExe `
            -ExportDecisionArgs $ExportDecisionArgs `
            -PlanTargetArgs $PlanTargetArgs `
            -ContinueTargetArgs $ContinueTargetArgs `
            -ExportDecisionAfterArgs $ExportDecisionAfterArgs `
            -ContinuationEffectArgs $ContinuationEffectArgs
        Write-CoverageGapContinuationDryRunCommands `
            -PlanCoverageGaps ([bool] $PlanCoverageGaps) `
            -ContinueCoverageGaps ([bool] $ContinueCoverageGaps) `
            -UntilMilestoneBound $UntilMilestoneBound `
            -DriverExe $DriverExe `
            -CoveragePlanArgs $CoveragePlanArgs `
            -ContinueCoverageGapArgs $ContinueCoverageGapArgs `
            -MilestoneStepRounds $MilestoneStepRounds
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
            $DriverExitCode = Invoke-TargetedContinuationCommands `
                -PlanTargets ([bool] $PlanTargets) `
                -ContinueTargets ([bool] $ContinueTargets) `
                -DriverExe $DriverExe `
                -ExportDecisionArgs $ExportDecisionArgs `
                -PlanTargetArgs $PlanTargetArgs `
                -ContinueTargetArgs $ContinueTargetArgs `
                -ExportDecisionAfterArgs $ExportDecisionAfterArgs `
                -ContinuationEffectArgs $ContinuationEffectArgs `
                -UntilMilestoneBound $UntilMilestoneBound `
                -ContinuationRounds $ContinuationRounds
            exit $DriverExitCode
        }
        if ($PlanCoverageGaps -or $ContinueCoverageGaps) {
            $DriverExitCode = Invoke-CoverageGapContinuationCommands `
                -PlanCoverageGaps ([bool] $PlanCoverageGaps) `
                -ContinueCoverageGaps ([bool] $ContinueCoverageGaps) `
                -DriverExe $DriverExe `
                -CoveragePlanArgs $CoveragePlanArgs `
                -ContinueCoverageGapArgs $ContinueCoverageGapArgs `
                -UntilMilestoneBound $UntilMilestoneBound `
                -CoverageGapInitialSpentRounds $CoverageGapInitialSpentRounds
            exit $DriverExitCode
        }
        exit 0
    } finally {
        Pop-Location
    }
}

if ($Inspect) {
    $InspectSource = $CampaignSourceArtifact
    if (-not $InspectSource) {
        throw "Internal error: campaign inspect did not resolve a source artifact."
    }
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

    $InspectModeLabel = if ($ExportLearningDataset) { "dataset" } else { "inspect" }
    Write-CampaignInspectPreflight `
        -ModeLabel $InspectModeLabel `
        -SourceLabel $InspectSourceLabel `
        -Seed $Seed `
        -Ascension $Ascension `
        -Class $Class `
        -BuildProfile $BuildProfile `
        -DriverExe $DriverExe `
        -NeedsBuild $NeedsBuild `
        -CoverageGapMilestoneSummary ([bool] $InspectCoverageGapMilestoneSummary) `
        -CoverageGapFilterLabel $CoverageGapFilterLabel `
        -InspectCampaignPath $InspectCampaignPath `
        -InspectCheckpointPath $InspectCheckpointPath
    $DriverExitCode = Invoke-CampaignInspectCommand `
        -DryRun ([bool] $DryRun) `
        -NeedsBuild $NeedsBuild `
        -BuildArgs $BuildArgs `
        -RepoRoot $RepoRoot `
        -DriverExe $DriverExe `
        -InspectArgs $InspectArgs
    exit $DriverExitCode
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
