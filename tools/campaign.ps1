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

.EXAMPLE
.\tools\campaign.ps1 -PruneArtifacts
Shows campaign artifacts that are outside the current retention window.

.NOTES
Detailed examples live in docs/CAMPAIGN_WRAPPER_USAGE.md.
#>
param(
    [Parameter(Position = 0)]
    [long] $Seed = 0,

    [switch] $Last,
    [Parameter(DontShow = $true)]
    [switch] $More,
    [Alias("Continue")]
    [switch] $ContinueRun,
    [switch] $Inspect,
    [switch] $InspectArtifacts,
    [switch] $InspectState,
    [Parameter(DontShow = $true)]
    [switch] $InspectShopEvidence,
    [Parameter(DontShow = $true)]
    [switch] $InspectShopChallenge,
    [Parameter(DontShow = $true)]
    [switch] $InspectCardRewardEvidence,
    [switch] $InspectDecisionObservations,
    [switch] $InspectJournal,
    [switch] $InspectLineageDecisions,
    [Parameter(DontShow = $true)]
    [switch] $InspectCampfireEvidence,
    [Parameter(DontShow = $true)]
    [switch] $InspectDeckMutation,
    [Parameter(DontShow = $true)]
    [switch] $InspectRouteEvidence,
    [Parameter(DontShow = $true)]
    [switch] $InspectLastAutoCombat,
    [Parameter(DontShow = $true)]
    [switch] $InspectCombatLab,
    [Parameter(DontShow = $true)]
    [switch] $InspectFinalBossCombat,
    [switch] $InspectCoverageGapMilestoneSummary,
    [switch] $InspectCoverageGapTargetState,
    [Alias("InspectScratchLatest")]
    [switch] $FromScratchLatest,
    [switch] $ProbeBoss,
    [switch] $DryRun,
    [switch] $PruneArtifacts,
    [switch] $PruneApply,
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
    [switch] $PlanCoverageGaps,
    [switch] $ContinueCoverageGaps,
    [switch] $CoverageGapRoute,
    [switch] $CoverageGapRouteMissing,
    [switch] $CoverageGapEventBoundary,
    [switch] $CoverageGapEventBoundaryMissing,
    [Alias("OutScratch")]
    [switch] $Scratch,

    [string] $ExportLearningDataset = "",
    [string] $AutoCaptureRoot = "",
    [string] $RunLabel = "",
    [string] $From = "",

    [ValidateSet(
        "shop-evidence",
        "shop-challenge",
        "card-reward-evidence",
        "campfire-evidence",
        "deck-mutation",
        "route-evidence",
        "last-auto-combat",
        "combat-lab",
        "final-boss-combat"
    )]
    [string[]] $Probe = @(),

    [ValidateSet("compact", "full")]
    [string] $ProbeDetail = "compact",

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

    [Parameter(DontShow = $true)]
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
    [ValidateRange(0, 1000)]
    [int] $KeepArtifactRuns = 5,
    [ValidateRange(0, 1000)]
    [int] $KeepArtifactScratch = 1,

    [Alias("Passthrough")]
    [string[]] $DriverArgs = @(),

    [Parameter(ValueFromRemainingArguments = $true, DontShow = $true)]
    [string[]] $ExtraArgs
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot

. (Join-Path $PSScriptRoot "campaign_artifacts.ps1")
$CampaignPathContext = New-CampaignPathContext -RepoRoot $RepoRoot
Initialize-CampaignArtifactPaths -PathContext $CampaignPathContext
New-Item -ItemType Directory -Force -Path $CampaignPathContext.CampaignDir | Out-Null

. (Join-Path $PSScriptRoot "campaign_artifact_summary.ps1")
. (Join-Path $PSScriptRoot "campaign_artifact_prune.ps1")
. (Join-Path $PSScriptRoot "campaign_invocation.ps1")
. (Join-Path $PSScriptRoot "campaign_preflight.ps1")
. (Join-Path $PSScriptRoot "campaign_milestones.ps1")
. (Join-Path $PSScriptRoot "campaign_manifest.ps1")
. (Join-Path $PSScriptRoot "campaign_run_execution.ps1")
. (Join-Path $PSScriptRoot "campaign_rounds.ps1")
. (Join-Path $PSScriptRoot "campaign_coverage_gaps.ps1")
. (Join-Path $PSScriptRoot "campaign_coverage_gap_manifest.ps1")
. (Join-Path $PSScriptRoot "campaign_coverage_gap_execution.ps1")
. (Join-Path $PSScriptRoot "campaign_continuation.ps1")
. (Join-Path $PSScriptRoot "campaign_inspect.ps1")
. (Join-Path $PSScriptRoot "campaign_build.ps1")
. (Join-Path $PSScriptRoot "campaign_source.ps1")
. (Join-Path $PSScriptRoot "campaign_request.ps1")
. (Join-Path $PSScriptRoot "campaign_entry_dispatch.ps1")

if ($PruneArtifacts) {
    exit (Invoke-CampaignArtifactPrune `
        -KeepRuns $KeepArtifactRuns `
        -KeepScratch $KeepArtifactScratch `
        -Apply ([bool] $PruneApply))
}

$DriverPassthroughContext = Resolve-CampaignDriverPassthroughContext `
    -DriverArgs $DriverArgs `
    -CompatibilityExtraArgs $ExtraArgs

$CampaignRequest = Resolve-CampaignEntryRequest `
    -ContinueRun ([bool] $ContinueRun) `
    -More ([bool] $More) `
    -Inspect ([bool] $Inspect) `
    -AnyInspectSelector (Test-CampaignAnyInspectSelectorSwitch -BoundParameters $PSBoundParameters) `
    -FromScratchLatest ([bool] $FromScratchLatest) `
    -InspectShopChallenge ([bool] $InspectShopChallenge) `
    -InspectBoundaryBound ($PSBoundParameters.ContainsKey("InspectBoundary")) `
    -InspectBoundary $InspectBoundary `
    -PlanCoverageGaps ([bool] $PlanCoverageGaps) `
    -ContinueCoverageGaps ([bool] $ContinueCoverageGaps) `
    -Scratch ([bool] $Scratch)
$InspectProbeContext = New-CampaignInspectProbeContext -Probe $Probe
$InspectSwitchContext = New-CampaignInspectSwitchContext `
    -InspectArtifacts ([bool] $InspectArtifacts) `
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
    -ProbeContext $InspectProbeContext `
    -BranchExamples $BranchExamples `
    -ChallengeMaxPlans $ChallengeMaxPlans `
    -ChallengeDepth $ChallengeDepth `
    -ChallengeMaxBranches $ChallengeMaxBranches `
    -SearchWallMs $SearchWallMs `
    -SearchMaxNodes $SearchMaxNodes `
    -InspectIndex $InspectIndex `
    -InspectAct $InspectAct `
    -InspectFloor $InspectFloor `
    -InspectQuery $InspectQuery `
    -ProbeDetail $ProbeDetail `
    -ProbeBoss ([bool] $ProbeBoss)
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
    -DriverPassthroughContext $DriverPassthroughContext `
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
    -MaxRounds $MaxRounds

$RunOutputContext = Resolve-CampaignOutputArtifactContext `
    -Request $CampaignRequest `
    -Scratch ([bool] $Scratch) `
    -RunLabel $RunLabel `
    -Seed $Seed
Ensure-CampaignOutputArtifactDirectory -OutputContext $RunOutputContext -DryRun ([bool] $DryRun)

$NeedsBuild = $Build -or (Test-DriverNeedsBuild $BuildContext.DriverExe)

$EntryDispatchContext = [pscustomobject]@{
    CampaignRequest = $CampaignRequest
    WrapperScript = $PSCommandPath
    RepoRoot = $RepoRoot
    Mode = $Mode
    Seed = $Seed
    Ascension = $Ascension
    Class = $Class
    CampaignSourceArtifact = $CampaignSourceArtifact
    BuildContext = $BuildContext
    NeedsBuild = [bool] $NeedsBuild
    BoundParameterContext = $BoundParameterContext
    DriverPassthroughContext = $DriverPassthroughContext
    CampaignRunIdentityArgs = @($CampaignRunIdentityArgs)
    CampaignSharedDriverOptionContext = $CampaignSharedDriverOptionContext
    RunRoundContext = $RunRoundContext
    RunOutputContext = $RunOutputContext
    FromScratchLatest = [bool] $FromScratchLatest
    ExportLearningDataset = $ExportLearningDataset
    RunSwitchContext = [pscustomobject]@{
        Scratch = [bool] $Scratch
        DryRun = [bool] $DryRun
        Log = [bool] $Log
        BossRelicAxes = [bool] $BossRelicAxes
        UntilMilestone = $UntilMilestone
        MilestoneStepRounds = $MilestoneStepRounds
        MilestoneMaxRounds = $MilestoneMaxRounds
    }
    CoverageGapSwitchContext = [pscustomobject]@{
        Execution = $CoverageGapExecution
        Intent = $CoverageGapIntent
        Limit = $CoverageGapLimit
        CandidatesPerDecision = $CoverageGapCandidatesPerDecision
        MilestoneTarget = $CoverageGapMilestoneTarget
        FilterContext = $CoverageGapFilterContext
    }
    InspectSwitchContext = $InspectSwitchContext
}

$DriverExitCode = Invoke-CampaignEntryDispatch -Context $EntryDispatchContext
exit $DriverExitCode
