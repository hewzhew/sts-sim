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
. (Join-Path $PSScriptRoot "campaign_artifact_summary.ps1")
. (Join-Path $PSScriptRoot "campaign_invocation.ps1")
. (Join-Path $PSScriptRoot "campaign_preflight.ps1")
. (Join-Path $PSScriptRoot "campaign_milestones.ps1")
. (Join-Path $PSScriptRoot "campaign_manifest.ps1")
. (Join-Path $PSScriptRoot "campaign_run_execution.ps1")
. (Join-Path $PSScriptRoot "campaign_rounds.ps1")
. (Join-Path $PSScriptRoot "campaign_coverage_gaps.ps1")
. (Join-Path $PSScriptRoot "campaign_continuation.ps1")
. (Join-Path $PSScriptRoot "campaign_inspect.ps1")
. (Join-Path $PSScriptRoot "campaign_targets.ps1")
. (Join-Path $PSScriptRoot "campaign_build.ps1")
. (Join-Path $PSScriptRoot "campaign_source.ps1")
. (Join-Path $PSScriptRoot "campaign_request.ps1")

$CampaignRequest = Resolve-CampaignEntryRequest `
    -ContinueRun ([bool] $ContinueRun) `
    -More ([bool] $More) `
    -Inspect ([bool] $Inspect) `
    -AnyInspectSelector (Test-CampaignAnyInspectSelectorSwitch -BoundParameters $PSBoundParameters) `
    -InspectScratchLatest ([bool] $InspectScratchLatest) `
    -InspectShopChallenge ([bool] $InspectShopChallenge) `
    -InspectBoundaryBound ($PSBoundParameters.ContainsKey("InspectBoundary")) `
    -InspectBoundary $InspectBoundary `
    -PlanTargets ([bool] $PlanTargets) `
    -ContinueTargets ([bool] $ContinueTargets) `
    -PlanCoverageGaps ([bool] $PlanCoverageGaps) `
    -ContinueCoverageGaps ([bool] $ContinueCoverageGaps) `
    -Scratch ([bool] $Scratch)
$CampaignSourceRunContext = Resolve-CampaignSourceRunContext `
    -Request $CampaignRequest `
    -Last ([bool] $Last) `
    -From $From `
    -Mode $Mode `
    -ModeBound ($PSBoundParameters.ContainsKey("Mode")) `
    -Seed $Seed `
    -Ascension $Ascension `
    -Class $Class `
    -Domain $Domain `
    -BoundParameters $PSBoundParameters
$CampaignSourceContext = $CampaignSourceRunContext.SourceContext
$CampaignSourceArtifact = $CampaignSourceRunContext.SourceArtifact
$Mode = $CampaignSourceRunContext.Mode
$Seed = $CampaignSourceRunContext.Seed
$Ascension = $CampaignSourceRunContext.Ascension
$Class = $CampaignSourceRunContext.Class

$BuildContext = Resolve-CampaignBuildContext `
    -RepoRoot $RepoRoot `
    -BuildProfile $BuildProfile `
    -DebugBuild ([bool] $DebugBuild) `
    -BuildProfileBound ($PSBoundParameters.ContainsKey("BuildProfile"))

$CampaignRunIdentityArgs = New-CampaignRunDriverIdentityArgs -Mode $Mode -Seed $Seed -Ascension $Ascension -Class $Class

$RunOutputContext = Resolve-CampaignOutputArtifactContext `
    -Request $CampaignRequest `
    -Inspect ([bool] $CampaignRequest.Inspect) `
    -PlanTargets ([bool] $CampaignRequest.PlanTargets) `
    -PlanCoverageGaps ([bool] $CampaignRequest.PlanCoverageGaps) `
    -Scratch ([bool] $Scratch) `
    -RunLabel $RunLabel `
    -ContinueCoverageGaps ([bool] $CampaignRequest.ContinueCoverageGaps) `
    -ContinueTargets ([bool] $CampaignRequest.ContinueTargets) `
    -ContinueCampaign ([bool] $CampaignRequest.ContinueCampaign) `
    -Seed $Seed
Ensure-CampaignOutputArtifactDirectory -OutputContext $RunOutputContext -DryRun ([bool] $DryRun)

$PlanDecisionOutcomePath = Resolve-TargetedContinuationPlanOutcomePath `
    -Request $CampaignRequest `
    -DecisionOutcomeDataset $DecisionOutcomeDataset `
    -Seed $Seed `
    -DryRun ([bool] $DryRun)

$BoundParameterContext = Resolve-CampaignBoundParameterContext `
    -BoundParameters $PSBoundParameters `
    -InvocationLine $MyInvocation.Line `
    -UntilMilestone $UntilMilestone
$CampaignSharedDriverOptionContext = New-CampaignSharedDriverOptionContext `
    -CampaignBoundParameters $BoundParameterContext.CampaignBoundParameters `
    -ExperimentWallMs $ExperimentWallMs `
    -SearchWallMs $SearchWallMs `
    -SearchMaxNodes $SearchMaxNodes `
    -ActiveLineageDiversity $ActiveLineageDiversity `
    -BossRelicAxes ([bool] $BossRelicAxes) `
    -CombatRetryWallMs $CombatRetryWallMs `
    -BranchExamples $BranchExamples `
    -VictoryHpPercent $VictoryHpPercent `
    -AutoCaptureCombat ([bool] $AutoCaptureCombat) `
    -AutoCaptureRoot $AutoCaptureRoot `
    -ExtraArgs $ExtraArgs `
    -BossSegments ([bool] $BossSegments) `
    -NoProgress ([bool] $NoProgress) `
    -VerboseProgress ([bool] $VerboseProgress) `
    -Perf ([bool] $Perf) `
    -Diagnose ([bool] $Diagnose)
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

$RunRoundContext = Resolve-CampaignRunRoundContext `
    -Request $CampaignRequest `
    -ContinueCampaign ([bool] $CampaignRequest.ContinueCampaign) `
    -CampaignSourceArtifact $CampaignSourceArtifact `
    -RoundsBound $BoundParameterContext.RoundsBound `
    -Rounds $Rounds `
    -UntilRoundBound $BoundParameterContext.UntilRoundBound `
    -UntilRound $UntilRound `
    -UntilMilestoneBound $BoundParameterContext.UntilMilestoneBound `
    -UntilMilestone $UntilMilestone `
    -MilestoneStepRounds $MilestoneStepRounds `
    -MilestoneMaxRounds $MilestoneMaxRounds `
    -MilestoneStop $MilestoneStop `
    -MaxRoundsBound $BoundParameterContext.MaxRoundsBound `
    -MaxRounds $MaxRounds `
    -ContinueCoverageGaps ([bool] $CampaignRequest.ContinueCoverageGaps) `
    -PlanTargets ([bool] $CampaignRequest.PlanTargets) `
    -PlanCoverageGaps ([bool] $CampaignRequest.PlanCoverageGaps) `
    -Inspect ([bool] $CampaignRequest.Inspect)
$RunDriverArgsContext = New-CampaignRunDriverArgsContext `
    -RunIdentityArgs $CampaignRunIdentityArgs `
    -Request $CampaignRequest `
    -RunOutputContext $RunOutputContext `
    -RunRoundContext $RunRoundContext `
    -OptionContext $CampaignSharedDriverOptionContext `
    -ExportLearningDataset $ExportLearningDataset

$NeedsBuild = $Build -or (Test-DriverNeedsBuild $BuildContext.DriverExe)

if ($CampaignRequest.IsContinuationFamily) {
    $ContinuationEntryContext = New-CampaignContinuationEntryContext `
        -CampaignRequest $CampaignRequest `
        -WrapperScript $PSCommandPath `
        -Mode $Mode `
        -RunOutputContext $RunOutputContext `
        -BoundParameterContext $BoundParameterContext `
        -CampaignSourceArtifact $CampaignSourceArtifact `
        -DecisionOutcomeDataset $DecisionOutcomeDataset `
        -LatestDecisionOutcomeBeforePath $LatestDecisionOutcomeBeforePath `
        -LatestDecisionOutcomePath $LatestDecisionOutcomePath `
        -LatestDecisionOutcomeAfterPath $LatestDecisionOutcomeAfterPath `
        -PlanDecisionOutcomePath $PlanDecisionOutcomePath `
        -InspectScratchLatest ([bool] $InspectScratchLatest) `
        -CoverageGapExecution $CoverageGapExecution `
        -CoverageGapIntent $CoverageGapIntent `
        -CoverageGapFilterLabel $CoverageGapFilterContext.FilterLabel `
        -CoverageGapFilterArgs $CoverageGapFilterContext.FilterArgs `
        -CoverageGapResultFilterArgs $CoverageGapFilterContext.ResultFilterArgs `
        -CoverageGapResultFilterLabel $CoverageGapFilterContext.ResultFilterLabel `
        -CampaignRunIdentityArgs $CampaignRunIdentityArgs `
        -CampaignSharedDriverOptionContext $CampaignSharedDriverOptionContext `
        -Seed $Seed `
        -Ascension $Ascension `
        -Class $Class `
        -BuildContext $BuildContext `
        -NeedsBuild ([bool] $NeedsBuild) `
        -Scratch ([bool] $Scratch) `
        -TargetedContinuationLimit $TargetedContinuationLimit `
        -TargetedContinuationCandidatesPerTarget $TargetedContinuationCandidatesPerTarget `
        -Rounds $Rounds `
        -UntilRound $UntilRound `
        -UntilMilestone $UntilMilestone `
        -MilestoneStepRounds $MilestoneStepRounds `
        -MilestoneMaxRounds $MilestoneMaxRounds `
        -ResolvedMilestoneStop $RunRoundContext.ResolvedMilestoneStop `
        -MaxRounds $RunRoundContext.MaxRounds `
        -CoverageGapLimit $CoverageGapLimit `
        -CoverageGapCandidatesPerDecision $CoverageGapCandidatesPerDecision `
        -DryRun ([bool] $DryRun) `
        -RepoRoot $RepoRoot
    $ContinuationExitCode = Invoke-CampaignContinuationEntry -Context $ContinuationEntryContext
    exit $ContinuationExitCode
}

if ($CampaignRequest.Kind -eq "inspect") {
    $InspectOptionContext = New-CampaignInspectOptionContext `
        -BoundParameters $BoundParameterContext.CampaignBoundParameters `
        -InspectState ([bool] $InspectState) `
        -InspectShopEvidence ([bool] $InspectShopEvidence) `
        -InspectShopChallenge ([bool] $InspectShopChallenge) `
        -InspectCardRewardEvidence ([bool] $InspectCardRewardEvidence) `
        -InspectDecisionObservations ([bool] $InspectDecisionObservations) `
        -InspectJournal ([bool] $InspectJournal) `
        -InspectLineageDecisions ([bool] $InspectLineageDecisions) `
        -InspectCampfireEvidence ([bool] $InspectCampfireEvidence) `
        -InspectDeckMutation ([bool] $InspectDeckMutation) `
        -InspectRouteEvidence ([bool] $InspectRouteEvidence) `
        -InspectLastAutoCombat ([bool] $InspectLastAutoCombat) `
        -InspectCombatLab ([bool] $InspectCombatLab) `
        -InspectFinalBossCombat ([bool] $InspectFinalBossCombat) `
        -InspectCoverageGapMilestoneSummary ([bool] $InspectCoverageGapMilestoneSummary) `
        -InspectCoverageGapTargetState ([bool] $InspectCoverageGapTargetState) `
        -ExportLearningDataset $ExportLearningDataset `
        -BranchExamples $BranchExamples `
        -ChallengeMaxPlans $ChallengeMaxPlans `
        -ChallengeDepth $ChallengeDepth `
        -ChallengeMaxBranches $ChallengeMaxBranches `
        -SearchWallMs $SearchWallMs `
        -SearchMaxNodes $SearchMaxNodes `
        -CoverageGapMilestoneTarget $CoverageGapMilestoneTarget `
        -CoverageGapFilterArgs $CoverageGapFilterContext.FilterArgs `
        -InspectIndex $InspectIndex `
        -InspectAct $InspectAct `
        -InspectFloor $InspectFloor `
        -InspectBoundary $CampaignRequest.InspectBoundary `
        -InspectQuery $InspectQuery `
        -ProbeBoss ([bool] $ProbeBoss)
    $InspectEntryContext = New-CampaignInspectEntryContext `
        -CampaignRequest $CampaignRequest `
        -CampaignSourceArtifact $CampaignSourceArtifact `
        -InspectOptionContext $InspectOptionContext `
        -InspectArtifacts ([bool] $InspectArtifacts) `
        -ExportLearningDataset $ExportLearningDataset `
        -Seed $Seed `
        -Ascension $Ascension `
        -Class $Class `
        -BuildContext $BuildContext `
        -NeedsBuild ([bool] $NeedsBuild) `
        -InspectCoverageGapMilestoneSummary ([bool] $InspectCoverageGapMilestoneSummary) `
        -CoverageGapFilterLabel $CoverageGapFilterContext.FilterLabel `
        -DryRun ([bool] $DryRun) `
        -RepoRoot $RepoRoot
    $DriverExitCode = Invoke-CampaignInspectEntry -Context $InspectEntryContext
    exit $DriverExitCode
}

$RunCommandContext = New-CampaignRunCommandContext `
    -CampaignRequest $CampaignRequest `
    -WrapperScript $PSCommandPath `
    -RepoRoot $RepoRoot `
    -Mode $Mode `
    -Seed $Seed `
    -Ascension $Ascension `
    -Class $Class `
    -Scratch ([bool] $Scratch) `
    -BuildContext $BuildContext `
    -RunOutputContext $RunOutputContext `
    -BoundParameterContext $BoundParameterContext `
    -RunRoundContext $RunRoundContext `
    -DriverArgs $RunDriverArgsContext.DriverArgs `
    -NeedsBuild ([bool] $NeedsBuild) `
    -DryRun ([bool] $DryRun) `
    -Log ([bool] $Log) `
    -BossRelicAxes ([bool] $BossRelicAxes) `
    -CombatSegmentMode $RunDriverArgsContext.CombatSegmentMode `
    -UntilMilestone $UntilMilestone `
    -MilestoneStepRounds $MilestoneStepRounds `
    -MilestoneMaxRounds $MilestoneMaxRounds `
    -ResolvedMilestoneStop $RunRoundContext.ResolvedMilestoneStop
Write-CampaignRunPreflight -Context $RunCommandContext
$DriverExitCode = Invoke-CampaignRunCommand -Context $RunCommandContext -RunIdentityArgs $CampaignRunIdentityArgs -OptionContext $CampaignSharedDriverOptionContext
exit $DriverExitCode
