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
.\tools\campaign.ps1 -ContinueCoverageGaps -UntilMilestone Act2Start
Runs coverage-gap branches, then keeps resuming in small round chunks until the milestone round cap is exhausted by default.

.EXAMPLE
.\tools\campaign.ps1 -ContinueCoverageGaps -CoverageGapExecution milestone -UntilMilestone Act2Start -Scratch
Seeds selected coverage-gap targets without spending a campaign round, then continues those branches to the requested milestone in a scratch report.

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
.\tools\campaign.ps1 -InspectLineageDecisions -InspectIndex 0
Prints historical CampaignJournal candidate pools along a selected active/frozen branch lineage.

.EXAMPLE
.\tools\campaign.ps1 -InspectLineageDecisions -InspectQuery "CompleteWithinBudget"
Prints current branch lineages where historical candidates match a typed route/reward/shop/event query.

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
.\tools\campaign.ps1 -PlanCoverageGaps -CoverageGapRouteMissing
Prints only missing route/map coverage-gap continuation targets.

.EXAMPLE
.\tools\campaign.ps1 -InspectCoverageGapMilestoneSummary -CoverageGapRouteMissing
Summarizes milestone progress for only missing route/map coverage-gap targets.

.EXAMPLE
.\tools\campaign.ps1 -ContinueCoverageGaps -Rounds 1
Resumes selected unobserved journal candidate branches and advances one round.

.EXAMPLE
.\tools\campaign.ps1 -ContinueCoverageGaps -Scratch -RunLabel gap-probe -Rounds 1
Runs coverage-gap continuation into a scratch report/checkpoint without overwriting latest.

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
    [switch] $InspectLineageDecisions,
    [switch] $InspectCampfireEvidence,
    [switch] $InspectDeckMutation,
    [switch] $InspectRouteEvidence,
    [switch] $InspectLastAutoCombat,
    [switch] $InspectCombatLab,
    [switch] $InspectFinalBossCombat,
    [switch] $InspectCoverageGapMilestoneSummary,
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
    [switch] $CoverageGapRouteMissing,
    [switch] $CoverageGapEventBoundaryMissing,
    [switch] $Scratch,

    [string] $ExportLearningDataset = "",
    [string] $DecisionOutcomeDataset = "",
    [string] $AutoCaptureRoot = "",
    [string] $RunLabel = "",

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
$LatestCampaignPath = Join-Path $CampaignDir "latest.campaign.json"
$LatestCheckpointPath = Join-Path $CampaignDir "latest.checkpoint.json"
$LatestDecisionOutcomePath = Join-Path $CampaignDir "latest.decision_outcomes.jsonl"
$LatestDecisionOutcomeBeforePath = Join-Path $CampaignDir "latest.decision_outcomes.before.jsonl"
$LatestDecisionOutcomeAfterPath = Join-Path $CampaignDir "latest.decision_outcomes.after.jsonl"

New-Item -ItemType Directory -Force -Path $CampaignDir | Out-Null

function Convert-ToCampaignArtifactSlug {
    param(
        [string] $Value
    )

    $Slug = ($Value.Trim() -replace '[^A-Za-z0-9_.-]+', '-').Trim('-')
    if (-not $Slug) {
        return "scratch"
    }
    return $Slug
}

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

function Assert-CoverageGapPresetCompatible {
    param(
        [string] $Preset,
        [string] $Name,
        [string] $Actual,
        [string] $Expected
    )

    if ($Actual -and $Actual -ne $Expected) {
        throw "$Preset conflicts with -$Name $Actual; expected $Expected."
    }
}

function New-CoverageGapFilterArgs {
    param(
        [string] $Bucket,
        [string] $EventId,
        [string] $Lane,
        [string] $OriginSource,
        [string] $Progress
    )

    $Args = @()
    if ($Bucket) {
        $Args += @("--coverage-gap-bucket", "$Bucket")
    }
    if ($EventId) {
        $Args += @("--coverage-gap-event-id", "$EventId")
    }
    if ($Lane) {
        $Args += @("--coverage-gap-lane", "$Lane")
    }
    if ($OriginSource) {
        $Args += @("--coverage-gap-origin-source", "$OriginSource")
    }
    if ($Progress) {
        $Args += @("--coverage-gap-progress", "$Progress")
    }
    return $Args
}

function Format-CoverageGapFilterLabel {
    param(
        [string] $Bucket,
        [string] $EventId,
        [string] $Lane,
        [string] $OriginSource,
        [string] $Progress
    )

    $Parts = @()
    if ($Bucket) {
        $Parts += "bucket=$Bucket"
    }
    if ($EventId) {
        $Parts += "event_id=$EventId"
    }
    if ($Lane) {
        $Parts += "lane=$Lane"
    }
    if ($OriginSource) {
        $Parts += "origin_source=$OriginSource"
    }
    if ($Progress) {
        $Parts += "progress=$Progress"
    }
    if ($Parts.Count -eq 0) {
        return "-"
    }
    return $Parts -join " "
}

if (
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
    $InspectCoverageGapMilestoneSummary
) {
    $Inspect = $true
}
if ($InspectShopChallenge -and -not $PSBoundParameters.ContainsKey("InspectBoundary")) {
    $InspectBoundary = "Shop"
}

if (($PlanTargets -or $ContinueTargets) -and ($PlanCoverageGaps -or $ContinueCoverageGaps)) {
    throw "Choose either targeted continuation (-PlanTargets/-ContinueTargets) or coverage-gap continuation (-PlanCoverageGaps/-ContinueCoverageGaps), not both."
}
if ($Scratch -and -not $ContinueCoverageGaps) {
    throw "-Scratch currently supports -ContinueCoverageGaps only."
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

$ScratchLabel = ""
$ScratchCampaignPath = $LatestCampaignPath
$ScratchCheckpointPath = $LatestCheckpointPath
$ScratchCommandPath = $LatestCommandPath
if ($Scratch) {
    $ScratchStamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $BaseLabel = if ($RunLabel) { $RunLabel } else { "coverage-gap-seed$Seed" }
    $ScratchLabel = "$(Convert-ToCampaignArtifactSlug $BaseLabel)-$ScratchStamp"
    New-Item -ItemType Directory -Force -Path $ScratchCampaignDir | Out-Null
    $ScratchCampaignPath = Join-Path $ScratchCampaignDir "$ScratchLabel.campaign.json"
    $ScratchCheckpointPath = Join-Path $ScratchCampaignDir "$ScratchLabel.checkpoint.json"
    $ScratchCommandPath = Join-Path $ScratchCampaignDir "$ScratchLabel.command.txt"
}
$RunOutputCampaignPath = $ScratchCampaignPath
$RunOutputCheckpointPath = $ScratchCheckpointPath
$RunCommandPath = $ScratchCommandPath

$CampaignBoundParameters = @{}
foreach ($ParameterName in $PSBoundParameters.Keys) {
    $CampaignBoundParameters[$ParameterName] = $true
}

$RoundsBound = $CampaignBoundParameters.ContainsKey("Rounds")
$UntilRoundBound = $CampaignBoundParameters.ContainsKey("UntilRound")
$UntilMilestoneBound = $CampaignBoundParameters.ContainsKey("UntilMilestone") -and $UntilMilestone
$MaxRoundsBound = $CampaignBoundParameters.ContainsKey("MaxRounds")
if ($CoverageGapRouteMissing -and $CoverageGapEventBoundaryMissing) {
    throw "Choose either -CoverageGapRouteMissing or -CoverageGapEventBoundaryMissing, not both."
}
if ($CoverageGapRouteMissing) {
    Assert-CoverageGapPresetCompatible -Preset "-CoverageGapRouteMissing" -Name "CoverageGapBucket" -Actual $CoverageGapBucket -Expected "route"
    Assert-CoverageGapPresetCompatible -Preset "-CoverageGapRouteMissing" -Name "CoverageGapOriginSource" -Actual $CoverageGapOriginSource -Expected "map_decision_packet"
    Assert-CoverageGapPresetCompatible -Preset "-CoverageGapRouteMissing" -Name "CoverageGapProgress" -Actual $CoverageGapProgress -Expected "missing"
    if (-not $CoverageGapBucket) {
        $CoverageGapBucket = "route"
    }
    if (-not $CoverageGapOriginSource) {
        $CoverageGapOriginSource = "map_decision_packet"
    }
    if (-not $CoverageGapProgress) {
        $CoverageGapProgress = "missing"
    }
}
if ($CoverageGapEventBoundaryMissing) {
    Assert-CoverageGapPresetCompatible -Preset "-CoverageGapEventBoundaryMissing" -Name "CoverageGapBucket" -Actual $CoverageGapBucket -Expected "event"
    Assert-CoverageGapPresetCompatible -Preset "-CoverageGapEventBoundaryMissing" -Name "CoverageGapOriginSource" -Actual $CoverageGapOriginSource -Expected "event_boundary_packet"
    Assert-CoverageGapPresetCompatible -Preset "-CoverageGapEventBoundaryMissing" -Name "CoverageGapProgress" -Actual $CoverageGapProgress -Expected "missing"
    if (-not $CoverageGapBucket) {
        $CoverageGapBucket = "event"
    }
    if (-not $CoverageGapOriginSource) {
        $CoverageGapOriginSource = "event_boundary_packet"
    }
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

if (($RoundsBound -and $UntilRoundBound) -or ($RoundsBound -and $MaxRoundsBound) -or ($UntilRoundBound -and $MaxRoundsBound)) {
    throw "Choose only one round budget: -Rounds N, -UntilRound N, or legacy -MaxRounds N."
}
if ($UntilMilestoneBound -and ($RoundsBound -or $UntilRoundBound -or $MaxRoundsBound)) {
    throw "-UntilMilestone owns the round budget. Use -MilestoneStepRounds and -MilestoneMaxRounds instead of -Rounds, -UntilRound, or -MaxRounds."
}
if ($UntilMilestoneBound -and ($PlanTargets -or $PlanCoverageGaps -or $Inspect)) {
    throw "-UntilMilestone requires an executing command (-More, -ContinueTargets, -ContinueCoverageGaps, or a normal run), not a plan/inspect command."
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
} elseif (-not $More) {
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

function Get-CampaignMilestoneStatus {
    param(
        [string] $ReportPath,
        [string] $Milestone
    )

    if (-not (Test-Path -LiteralPath $ReportPath)) {
        return [pscustomobject]@{
            Reached = $false
            FurthestAct = 0
            FurthestFloor = 0
            HitCount = 0
            RoundsCompleted = 0
        }
    }

    $Report = Get-Content -LiteralPath $ReportPath -Raw | ConvertFrom-Json
    $Branches = @()
    foreach ($Bucket in @("active", "frozen", "stuck", "victories", "dead", "abandoned")) {
        if ($Report.$Bucket) {
            $Branches += @($Report.$Bucket)
        }
    }

    $FurthestAct = 0
    $FurthestFloor = 0
    $HitCount = 0
    foreach ($Branch in $Branches) {
        if (-not $Branch.summary) {
            continue
        }
        $Act = [int] $Branch.summary.act
        $Floor = [int] $Branch.summary.floor
        if (($Act -gt $FurthestAct) -or (($Act -eq $FurthestAct) -and ($Floor -gt $FurthestFloor))) {
            $FurthestAct = $Act
            $FurthestFloor = $Floor
        }
        $Hit = switch ($Milestone) {
            "Act1Boss" { ($Act -gt 1) -or (($Act -eq 1) -and ($Floor -ge 16)) }
            "Act2Start" { $Act -ge 2 }
            default { $false }
        }
        if ($Hit) {
            $HitCount += 1
        }
    }

    return [pscustomobject]@{
        Reached = $HitCount -gt 0
        FurthestAct = $FurthestAct
        FurthestFloor = $FurthestFloor
        HitCount = $HitCount
        RoundsCompleted = [int] $Report.rounds_completed
    }
}

function New-MilestoneResumeDriverArgs {
    param(
        [int] $StepRounds
    )

    $Args = @(
        "run",
        "--preset", "$Mode",
        "--seed", "$Seed",
        "--ascension", "$Ascension",
        "--class", "$Class"
    )
    if (@(0, 10, 15, 17, 20) -contains $Ascension) {
        $Args += @("--ascension-domain", "a$Ascension")
    }
    $Args += @(
        "--resume", "$RunOutputCampaignPath",
        "--resume-checkpoint", "$RunOutputCheckpointPath",
        "--out", "$RunOutputCampaignPath",
        "--checkpoint-out", "$RunOutputCheckpointPath",
        "--rounds", "$StepRounds"
    )
    if ($CampaignBoundParameters.ContainsKey("ExperimentWallMs")) {
        $Args += @("--experiment-wall-ms", "$ExperimentWallMs")
    }
    if ($CampaignBoundParameters.ContainsKey("SearchWallMs")) {
        $Args += @("--search-wall-ms", "$SearchWallMs")
    }
    if ($CampaignBoundParameters.ContainsKey("SearchMaxNodes")) {
        $Args += @("--search-max-nodes", "$SearchMaxNodes")
    }
    if ($CampaignBoundParameters.ContainsKey("ActiveLineageDiversity") -and $ActiveLineageDiversity -ge 0) {
        $Args += @("--active-lineage-diversity", "$ActiveLineageDiversity")
    }
    if ($BossRelicAxes) {
        $Args += "--boss-relic-axes"
    }
    if ($CampaignBoundParameters.ContainsKey("CombatRetryWallMs") -and $CombatRetryWallMs -gt 0) {
        $Args += @("--combat-retry-wall-ms", "$CombatRetryWallMs")
    }
    if ($CampaignBoundParameters.ContainsKey("BranchExamples")) {
        $Args += @("--branch-examples", "$BranchExamples")
    }
    if ($CampaignBoundParameters.ContainsKey("VictoryHpPercent")) {
        $Args += @("--min-acceptable-victory-hp-percent", "$VictoryHpPercent")
    }
    if (-not (Test-ExtraCombatOptionKey -Tokens $ExtraArgs -Keys @("segment", "segment_mode", "partial", "partial_mode"))) {
        if ($BossSegments) {
            $Args += @("--combat-search-option", "segment=turn")
        } else {
            $Args += @("--combat-search-option", "segment=non_boss_turn")
        }
    }
    if (-not $NoProgress) {
        $Args += "--progress"
        if ($VerboseProgress) {
            $Args += @("--progress-detail", "verbose")
        }
    }
    if ($Perf) {
        $Args += @("--report-detail", "perf")
    } elseif ($Diagnose) {
        $Args += @("--report-detail", "diagnose")
    }
    if ($ExtraArgs) {
        $Args += $ExtraArgs
    }
    return $Args
}

function Invoke-CampaignUntilMilestone {
    param(
        [int] $AlreadySpentRounds = 0
    )

    $script:CampaignMilestoneExitCode = 0
    $SpentRounds = $AlreadySpentRounds
    while ($SpentRounds -lt $MilestoneMaxRounds) {
        $Status = Get-CampaignMilestoneStatus -ReportPath $RunOutputCampaignPath -Milestone $UntilMilestone
        Write-Host "milestone-status target=$UntilMilestone stop=$ResolvedMilestoneStop reached=$($Status.Reached) hits=$($Status.HitCount) furthest=A$($Status.FurthestAct)F$($Status.FurthestFloor) report-rounds=$($Status.RoundsCompleted) spent-rounds=$SpentRounds cap=$MilestoneMaxRounds"
        if ($Status.Reached -and $ResolvedMilestoneStop -eq "first_hit") {
            $script:CampaignMilestoneExitCode = 0
            return
        }
        $StepRounds = [Math]::Min($MilestoneStepRounds, $MilestoneMaxRounds - $SpentRounds)
        $ResumeArgs = New-MilestoneResumeDriverArgs -StepRounds $StepRounds
        Write-Host "milestone-step target=$UntilMilestone additional-rounds=$StepRounds"
        & $DriverExe @ResumeArgs
        if ($LASTEXITCODE -ne 0) {
            $script:CampaignMilestoneExitCode = $LASTEXITCODE
            return
        }
        $SpentRounds += $StepRounds
    }

    $FinalStatus = Get-CampaignMilestoneStatus -ReportPath $RunOutputCampaignPath -Milestone $UntilMilestone
    Write-Host "milestone-status target=$UntilMilestone stop=$ResolvedMilestoneStop reached=$($FinalStatus.Reached) hits=$($FinalStatus.HitCount) furthest=A$($FinalStatus.FurthestAct)F$($FinalStatus.FurthestFloor) report-rounds=$($FinalStatus.RoundsCompleted) spent-rounds=$SpentRounds cap=$MilestoneMaxRounds"
    $script:CampaignMilestoneExitCode = 0
    return
}

function Invoke-CoverageGapMilestoneSummary {
    param(
        [string] $Target
    )

    if (-not (Test-Path -LiteralPath $RunOutputCampaignPath)) {
        Write-Host "coverage-gap-milestone-summary=skipped missing-report=$RunOutputCampaignPath"
        return 0
    }

    $SummaryArgs = @(
        "inspect",
        "--inspect-report", "$RunOutputCampaignPath",
        "--inspect-coverage-gap-milestone-summary",
        "--coverage-gap-milestone-target", "$Target"
    )
    $SummaryArgs += $CoverageGapFilterArgs
    if (Test-Path -LiteralPath $RunOutputCheckpointPath) {
        $SummaryArgs += @("--inspect-checkpoint", "$RunOutputCheckpointPath")
    }
    Write-Host "coverage-gap-milestone-summary:"
    & $DriverExe @SummaryArgs
    return $LASTEXITCODE
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
    $CoveragePlanArgs += $CoverageGapFilterArgs
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
        "--coverage-gap-budget-intent", "$CoverageGapIntent",
        "--coverage-gap-execution-mode", "$CoverageGapDriverExecution",
        "--out", "$RunOutputCampaignPath",
        "--checkpoint-out", "$RunOutputCheckpointPath"
    )
    $ContinueCoverageGapArgs += $CoverageGapFilterArgs
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
    if ($Scratch) {
        Write-Host "scratch=yes label=$ScratchLabel"
        Write-Host "source-report=$LatestCampaignPath"
        Write-Host "source-checkpoint=$LatestCheckpointPath"
        Write-Host "report=$RunOutputCampaignPath"
        Write-Host "checkpoint=$RunOutputCheckpointPath"
    } else {
        Write-Host "report=$LatestCampaignPath"
        Write-Host "checkpoint=$LatestCheckpointPath"
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
                $SummaryArgs += $CoverageGapFilterArgs
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
                Set-Content -LiteralPath $LatestSeedPath -Value $Seed
                Set-Content -LiteralPath $LatestAscensionPath -Value $Ascension
                Set-Content -LiteralPath $LatestClassPath -Value $Class
                Set-Content -LiteralPath $LatestModePath -Value $Mode
                Set-Content -LiteralPath $LatestCommandPath -Value (Format-CommandLine -ExePath $DriverExe -Arguments $ContinueTargetArgs)
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
                if ($Scratch) {
                    Set-Content -LiteralPath $RunCommandPath -Value (Format-CommandLine -ExePath $DriverExe -Arguments $ContinueCoverageGapArgs)
                    Write-Host "scratch-command=$RunCommandPath"
                } else {
                    Set-Content -LiteralPath $LatestSeedPath -Value $Seed
                    Set-Content -LiteralPath $LatestAscensionPath -Value $Ascension
                    Set-Content -LiteralPath $LatestClassPath -Value $Class
                    Set-Content -LiteralPath $LatestModePath -Value $Mode
                    Set-Content -LiteralPath $LatestCommandPath -Value (Format-CommandLine -ExePath $DriverExe -Arguments $ContinueCoverageGapArgs)
                }
                if ($UntilMilestoneBound) {
                    Invoke-CampaignUntilMilestone -AlreadySpentRounds $CoverageGapInitialSpentRounds
                    $DriverExitCode = $script:CampaignMilestoneExitCode
                    if ($DriverExitCode -eq 0) {
                        $DriverExitCode = Invoke-CoverageGapMilestoneSummary -Target $UntilMilestone
                    }
                }
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
        $InspectLineageDecisions -or
        $InspectCampfireEvidence -or
        $InspectDeckMutation -or
        $InspectRouteEvidence -or
        $InspectLastAutoCombat -or
        $InspectCombatLab -or
        $InspectFinalBossCombat -or
        $InspectCoverageGapMilestoneSummary
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
    if ((-not $ExportLearningDataset) -and $InspectLineageDecisions) {
        $InspectArgs += "--inspect-lineage-decisions"
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
    if ((-not $ExportLearningDataset) -and $InspectCoverageGapMilestoneSummary) {
        $InspectArgs += @(
            "--inspect-coverage-gap-milestone-summary",
            "--coverage-gap-milestone-target", "$CoverageGapMilestoneTarget"
        )
        $InspectArgs += $CoverageGapFilterArgs
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
    if ($InspectCoverageGapMilestoneSummary) {
        Write-Host "coverage-gap-filter=$CoverageGapFilterLabel"
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
if ($UntilMilestoneBound) {
    Write-Host "until-milestone=$UntilMilestone step-rounds=$MilestoneStepRounds max-additional-rounds=$MilestoneMaxRounds"
}

if ($More -and $TargetRounds -ne $null -and $MaxRounds -eq 0) {
    Write-Host "already-at-target-rounds=yes; nothing to run"
    exit 0
}
if ($More -and $UntilMilestoneBound) {
    $InitialMilestoneStatus = Get-CampaignMilestoneStatus -ReportPath $LatestCampaignPath -Milestone $UntilMilestone
    if ($InitialMilestoneStatus.Reached) {
        Write-Host "already-at-milestone=yes target=$UntilMilestone hits=$($InitialMilestoneStatus.HitCount) furthest=A$($InitialMilestoneStatus.FurthestAct)F$($InitialMilestoneStatus.FurthestFloor)"
        exit 0
    }
}

if ($DryRun) {
    if ($NeedsBuild) {
        $RenderedBuildArgs = $BuildArgs | ForEach-Object {
            if ($_ -match '^[A-Za-z0-9_./:=\\-]+$') { $_ } else { "'$($_ -replace "'", "''")'" }
        }
        Write-Host ("cargo " + ($RenderedBuildArgs -join " "))
    }
    Write-Host $RenderedCommand
    if ($UntilMilestoneBound) {
        Write-Host "milestone-loop-command-template:"
        Write-Host (Format-CommandLine -ExePath $DriverExe -Arguments (New-MilestoneResumeDriverArgs -StepRounds $MilestoneStepRounds))
    }
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
        if ($UntilMilestoneBound) {
            Invoke-CampaignUntilMilestone -AlreadySpentRounds $MaxRounds
            $DriverExitCode = $script:CampaignMilestoneExitCode
        }
    }
    exit $DriverExitCode
} finally {
    Pop-Location
}
