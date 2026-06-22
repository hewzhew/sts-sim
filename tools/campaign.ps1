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
. (Join-Path $PSScriptRoot "campaign_preflight.ps1")
. (Join-Path $PSScriptRoot "campaign_milestones.ps1")
. (Join-Path $PSScriptRoot "campaign_rounds.ps1")
. (Join-Path $PSScriptRoot "campaign_coverage_gaps.ps1")
. (Join-Path $PSScriptRoot "campaign_continuation.ps1")
. (Join-Path $PSScriptRoot "campaign_inspect.ps1")
. (Join-Path $PSScriptRoot "campaign_targets.ps1")
. (Join-Path $PSScriptRoot "campaign_build.ps1")
. (Join-Path $PSScriptRoot "campaign_source.ps1")
. (Join-Path $PSScriptRoot "campaign_request.ps1")

$InspectSelectorFlags = @(
    [bool] $InspectArtifacts,
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
    [bool] $InspectCoverageGapTargetState
)
$CampaignRequest = Resolve-CampaignEntryRequest `
    -ContinueRun ([bool] $ContinueRun) `
    -More ([bool] $More) `
    -Inspect ([bool] $Inspect) `
    -InspectSelectorFlags $InspectSelectorFlags `
    -InspectScratchLatest ([bool] $InspectScratchLatest) `
    -InspectShopChallenge ([bool] $InspectShopChallenge) `
    -InspectBoundaryBound ($PSBoundParameters.ContainsKey("InspectBoundary")) `
    -InspectBoundary $InspectBoundary `
    -PlanTargets ([bool] $PlanTargets) `
    -ContinueTargets ([bool] $ContinueTargets) `
    -PlanCoverageGaps ([bool] $PlanCoverageGaps) `
    -ContinueCoverageGaps ([bool] $ContinueCoverageGaps) `
    -Scratch ([bool] $Scratch)
$ContinueCampaign = $CampaignRequest.ContinueCampaign
$Inspect = $CampaignRequest.Inspect
$InspectBoundary = $CampaignRequest.InspectBoundary
$ScratchLatestIsContinuationSource = $CampaignRequest.ScratchLatestIsContinuationSource
$ReadsCampaignSource = $CampaignRequest.ReadsCampaignSource
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

$BuildContext = Resolve-CampaignBuildContext `
    -RepoRoot $RepoRoot `
    -BuildProfile $BuildProfile `
    -DebugBuild ([bool] $DebugBuild) `
    -BuildProfileBound ($PSBoundParameters.ContainsKey("BuildProfile"))
$BuildProfile = $BuildContext.BuildProfile
$DriverExe = $BuildContext.DriverExe
$BuildArgs = @($BuildContext.BuildArgs)

$CampaignRunIdentityArgs = New-CampaignRunDriverIdentityArgs -Mode $Mode -Seed $Seed -Ascension $Ascension -Class $Class
$DriverArgs = @($CampaignRunIdentityArgs)

$RunOutputContext = Resolve-CampaignOutputArtifactContext `
    -Inspect ([bool] $Inspect) `
    -PlanTargets ([bool] $PlanTargets) `
    -PlanCoverageGaps ([bool] $PlanCoverageGaps) `
    -Scratch ([bool] $Scratch) `
    -RunLabel $RunLabel `
    -ContinueCoverageGaps ([bool] $ContinueCoverageGaps) `
    -ContinueTargets ([bool] $ContinueTargets) `
    -ContinueCampaign ([bool] $ContinueCampaign) `
    -Seed $Seed
$WritesCampaignOutput = $RunOutputContext.WritesCampaignOutput
$RunOutputArtifact = $RunOutputContext.Artifact
$ScratchLabel = $RunOutputContext.ScratchLabel
$RunOutputCampaignPath = $RunOutputContext.CampaignPath
$RunOutputCheckpointPath = $RunOutputContext.CheckpointPath
$RunCommandPath = $RunOutputContext.CommandPath
$RunManifestPath = $RunOutputContext.ManifestPath
$RunLogPath = $RunOutputContext.LogPath
$RunDecisionOutcomePath = $RunOutputContext.DecisionOutcomePath
$RunDecisionOutcomeBeforePath = $RunOutputContext.DecisionOutcomeBeforePath
$RunDecisionOutcomeAfterPath = $RunOutputContext.DecisionOutcomeAfterPath
Ensure-CampaignOutputArtifactDirectory -OutputContext $RunOutputContext -DryRun ([bool] $DryRun)

$BoundParameterContext = Resolve-CampaignBoundParameterContext `
    -BoundParameters $PSBoundParameters `
    -InvocationLine $MyInvocation.Line `
    -UntilMilestone $UntilMilestone
$CampaignBoundParameters = $BoundParameterContext.CampaignBoundParameters
$CampaignWrapperInvocationLine = $BoundParameterContext.WrapperInvocationLine
$CampaignWrapperBoundParameters = $BoundParameterContext.WrapperBoundParameters
$RoundsBound = $BoundParameterContext.RoundsBound
$UntilRoundBound = $BoundParameterContext.UntilRoundBound
$UntilMilestoneBound = $BoundParameterContext.UntilMilestoneBound
$MaxRoundsBound = $BoundParameterContext.MaxRoundsBound
$CampaignSharedDriverOptionContext = New-CampaignSharedDriverOptionContext `
    -CampaignBoundParameters $CampaignBoundParameters `
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
$CoverageGapBucket = $CoverageGapFilterContext.Bucket
$CoverageGapEventId = $CoverageGapFilterContext.EventId
$CoverageGapLane = $CoverageGapFilterContext.Lane
$CoverageGapOriginSource = $CoverageGapFilterContext.OriginSource
$CoverageGapProgress = $CoverageGapFilterContext.Progress
$CoverageGapFilterArgs = @($CoverageGapFilterContext.FilterArgs)
$CoverageGapFilterLabel = $CoverageGapFilterContext.FilterLabel
$CoverageGapResultFilterArgs = @($CoverageGapFilterContext.ResultFilterArgs)
$CoverageGapResultFilterLabel = $CoverageGapFilterContext.ResultFilterLabel

$RunRoundContext = Resolve-CampaignRunRoundContext `
    -ContinueCampaign ([bool] $ContinueCampaign) `
    -CampaignSourceArtifact $CampaignSourceArtifact `
    -RoundsBound $RoundsBound `
    -Rounds $Rounds `
    -UntilRoundBound $UntilRoundBound `
    -UntilRound $UntilRound `
    -UntilMilestoneBound $UntilMilestoneBound `
    -UntilMilestone $UntilMilestone `
    -MilestoneStepRounds $MilestoneStepRounds `
    -MilestoneMaxRounds $MilestoneMaxRounds `
    -MilestoneStop $MilestoneStop `
    -MaxRoundsBound $MaxRoundsBound `
    -MaxRounds $MaxRounds `
    -ContinueCoverageGaps ([bool] $ContinueCoverageGaps) `
    -PlanTargets ([bool] $PlanTargets) `
    -PlanCoverageGaps ([bool] $PlanCoverageGaps) `
    -Inspect ([bool] $Inspect)
$DriverRoundBudgetArgs = @($RunRoundContext.DriverRoundBudgetArgs)
$RoundBudgetSource = $RunRoundContext.RoundBudgetSource
$RoundBudgetAdditionalRounds = $RunRoundContext.RoundBudgetAdditionalRounds
$MaxRounds = $RunRoundContext.MaxRounds
$ResolvedMilestoneStop = $RunRoundContext.ResolvedMilestoneStop
$ResumeCampaignPath = $RunRoundContext.ResumeCampaignPath
$ResumeCheckpointPath = $RunRoundContext.ResumeCheckpointPath
$ResumeRoundsCompleted = $RunRoundContext.ResumeRoundsCompleted
$TargetRounds = $RunRoundContext.TargetRounds
$DriverArgs += @($RunRoundContext.ResumeDriverArgs)

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
    -IncludeAutoCaptureCombat $true `
    -OptionContext $CampaignSharedDriverOptionContext

$NeedsBuild = $Build -or (Test-DriverNeedsBuild $DriverExe)

if ($PlanTargets -or $ContinueTargets -or $PlanCoverageGaps -or $ContinueCoverageGaps) {
    $ContinuationEntryContext = [pscustomobject]@{
        WrapperScript = $PSCommandPath
        Mode = $Mode
        OutputArtifact = $RunOutputArtifact
        RunCommandPath = $RunCommandPath
        RunManifestPath = $RunManifestPath
        WrapperInvocationLine = $CampaignWrapperInvocationLine
        WrapperBoundParameters = $CampaignWrapperBoundParameters
        LatestSeedPath = $LatestSeedPath
        LatestAscensionPath = $LatestAscensionPath
        LatestClassPath = $LatestClassPath
        LatestModePath = $LatestModePath
        LatestCommandPath = $LatestCommandPath
        InspectScratchLatest = [bool] $InspectScratchLatest
        PlanTargets = [bool] $PlanTargets
        ContinueTargets = [bool] $ContinueTargets
        PlanCoverageGaps = [bool] $PlanCoverageGaps
        ContinueCoverageGaps = [bool] $ContinueCoverageGaps
        CampaignSourceArtifact = $CampaignSourceArtifact
        DecisionOutcomeDataset = $DecisionOutcomeDataset
        DecisionOutcomeBeforePath = $(if ($RunDecisionOutcomeBeforePath) { $RunDecisionOutcomeBeforePath } else { $LatestDecisionOutcomeBeforePath })
        DecisionOutcomePath = $(if ($RunDecisionOutcomePath) { $RunDecisionOutcomePath } else { $LatestDecisionOutcomePath })
        DecisionOutcomeAfterPath = $(if ($RunDecisionOutcomeAfterPath) { $RunDecisionOutcomeAfterPath } else { $LatestDecisionOutcomeAfterPath })
        RunOutputCampaignPath = $RunOutputCampaignPath
        RunOutputCheckpointPath = $RunOutputCheckpointPath
        UntilMilestoneBound = $UntilMilestoneBound
        MilestoneStepRounds = $MilestoneStepRounds
        RoundsBound = $RoundsBound
        Rounds = $Rounds
        UntilRoundBound = $UntilRoundBound
        UntilRound = $UntilRound
        MaxRoundsBound = $MaxRoundsBound
        MaxRounds = $MaxRounds
        CoverageGapExecution = $CoverageGapExecution
        CoverageGapIntent = $CoverageGapIntent
        CampaignRunIdentityArgs = @($CampaignRunIdentityArgs)
        CampaignSharedDriverOptionContext = $CampaignSharedDriverOptionContext
        Seed = $Seed
        Ascension = $Ascension
        Class = $Class
        BuildProfile = $BuildProfile
        DriverExe = $DriverExe
        NeedsBuild = [bool] $NeedsBuild
        Scratch = [bool] $Scratch
        ScratchLabel = $ScratchLabel
        TargetedContinuationLimit = $TargetedContinuationLimit
        TargetedContinuationCandidatesPerTarget = $TargetedContinuationCandidatesPerTarget
        UntilMilestone = $UntilMilestone
        MilestoneMaxRounds = $MilestoneMaxRounds
        ResolvedMilestoneStop = $ResolvedMilestoneStop
        CoverageGapLimit = $CoverageGapLimit
        CoverageGapCandidatesPerDecision = $CoverageGapCandidatesPerDecision
        CoverageGapFilterLabel = $CoverageGapFilterLabel
        CoverageGapResultFilterArgs = @($CoverageGapResultFilterArgs)
        CoverageGapResultFilterLabel = $CoverageGapResultFilterLabel
        DryRun = [bool] $DryRun
        BuildArgs = @($BuildArgs)
        RepoRoot = $RepoRoot
    }
    $ContinuationExitCode = Invoke-CampaignContinuationEntry -Context $ContinuationEntryContext
    exit $ContinuationExitCode
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

$RenderedCommand = Format-CommandLine -ExePath $DriverExe -Arguments $DriverArgs
Write-CampaignRunPreflight
$RunCommandContext = [pscustomobject]@{
    WrapperScript = $PSCommandPath
    Mode = $Mode
    Seed = $Seed
    Ascension = $Ascension
    Class = $Class
    BuildProfile = $BuildProfile
    Scratch = [bool] $Scratch
    ScratchLabel = $ScratchLabel
    OutputArtifact = $RunOutputArtifact
    RunOutputCampaignPath = $RunOutputCampaignPath
    RunOutputCheckpointPath = $RunOutputCheckpointPath
    RunCommandPath = $RunCommandPath
    RunManifestPath = $RunManifestPath
    WrapperInvocationLine = $CampaignWrapperInvocationLine
    WrapperBoundParameters = $CampaignWrapperBoundParameters
    LatestSeedPath = $LatestSeedPath
    LatestAscensionPath = $LatestAscensionPath
    LatestClassPath = $LatestClassPath
    LatestModePath = $LatestModePath
    LatestCommandPath = $LatestCommandPath
    ContinueCampaign = [bool] $ContinueCampaign
    TargetRounds = $TargetRounds
    MaxRounds = $MaxRounds
    UntilMilestoneBound = $UntilMilestoneBound
    ResumeCampaignPath = $ResumeCampaignPath
    ResumeCheckpointPath = $ResumeCheckpointPath
    UntilMilestone = $UntilMilestone
    MilestoneStepRounds = $MilestoneStepRounds
    MilestoneMaxRounds = $MilestoneMaxRounds
    ResolvedMilestoneStop = $ResolvedMilestoneStop
    NeedsBuild = [bool] $NeedsBuild
    BuildArgs = @($BuildArgs)
    DryRun = [bool] $DryRun
    RepoRoot = $RepoRoot
    DriverExe = $DriverExe
    DriverArgs = @($DriverArgs)
    RenderedCommand = $RenderedCommand
    Log = [bool] $Log
    RunLogPath = $RunLogPath
    RoundBudgetSource = $RoundBudgetSource
    RoundBudgetAdditionalRounds = $RoundBudgetAdditionalRounds
}
$DriverExitCode = Invoke-CampaignRunCommand -Context $RunCommandContext -RunIdentityArgs $CampaignRunIdentityArgs -OptionContext $CampaignSharedDriverOptionContext
exit $DriverExitCode
