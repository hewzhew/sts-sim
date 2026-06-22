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
.\tools\campaign.ps1 -FromScratchLatest -PlanCoverageGaps -CoverageGapRouteMissing
Plans missing route/map coverage-gap targets from the latest scratch campaign artifact.

.EXAMPLE
.\tools\campaign.ps1 -InspectScratchLatest -InspectState -InspectIndex 0
Prints the full checkpoint state for a selected latest scratch session.

.EXAMPLE
.\tools\campaign.ps1 -InspectArtifacts
Prints artifact sizes and top-level shape for the latest campaign manifest/report/checkpoint/log bundle.

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
.\tools\campaign.ps1 -InspectScratchLatest -InspectCoverageGapMilestoneSummary -CoverageGapRouteMissing
Summarizes milestone progress for the latest scratch campaign artifact.

.EXAMPLE
.\tools\campaign.ps1 -InspectScratchLatest -InspectCoverageGapMilestoneSummary -CoverageGapRoute
Summarizes route/map coverage-gap progress for the latest scratch campaign artifact without filtering by current progress.

.EXAMPLE
.\tools\campaign.ps1 -InspectScratchLatest -InspectCoverageGapTargetState -CoverageGapRoute -InspectIndex 1
Prints the full checkpoint state for a selected coverage-gap target group from the latest scratch campaign artifact.

.EXAMPLE
.\tools\campaign.ps1 -ContinueCoverageGaps -Rounds 1
Resumes selected unobserved journal candidate branches and advances one round.

.EXAMPLE
.\tools\campaign.ps1 -ContinueCoverageGaps -Scratch -RunLabel gap-probe -Rounds 1
Runs coverage-gap continuation into a scratch report/checkpoint without overwriting latest.

.EXAMPLE
.\tools\campaign.ps1 -FromScratchLatest -ContinueCoverageGaps -OutScratch -RunLabel gap-probe -CoverageGapExecution target_only
Runs coverage-gap continuation from the latest scratch campaign artifact into a new scratch report/checkpoint.

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
.\tools\campaign.ps1 -Mode quick -Scratch -Log
Runs a scratch campaign and tees the full driver output into a sibling .log file.

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

function Convert-CampaignWrapperParameterValue {
    param(
        [object] $Value
    )

    if ($null -eq $Value) {
        return $null
    }
    if ($Value -is [System.Management.Automation.SwitchParameter]) {
        return [bool] $Value
    }
    if ($Value -is [System.Array]) {
        $Converted = @()
        foreach ($Item in $Value) {
            $Converted += (Convert-CampaignWrapperParameterValue -Value $Item)
        }
        return ,$Converted
    }
    return $Value
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

function Get-LatestScratchCampaignArtifact {
    $ScratchReport = Get-ChildItem -LiteralPath $ScratchCampaignDir -Filter "*.campaign.json" -File -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $ScratchReport) {
        throw "No scratch campaign report found under $ScratchCampaignDir."
    }

    $ScratchCheckpointPath = $ScratchReport.FullName -replace '\.campaign\.json$', '.checkpoint.json'
    if (-not (Test-Path -LiteralPath $ScratchCheckpointPath)) {
        throw "Latest scratch report has no matching checkpoint: $ScratchCheckpointPath"
    }

    return [pscustomobject]@{
        ReportPath = $ScratchReport.FullName
        CheckpointPath = $ScratchCheckpointPath
        ManifestPath = $ScratchReport.FullName -replace '\.campaign\.json$', '.manifest.json'
        LogPath = $ScratchReport.FullName -replace '\.campaign\.json$', '.log'
        CommandPath = $ScratchReport.FullName -replace '\.campaign\.json$', '.command.txt'
        Label = $ScratchReport.BaseName -replace '\.campaign$', ''
    }
}

function Get-CampaignArtifactRunConfig {
    param(
        [string] $CheckpointPath,
        [string] $ManifestPath
    )

    $Config = [ordered]@{
        Seed = $null
        Ascension = $null
        Class = $null
        Mode = $null
    }

    if ($CheckpointPath -and (Test-Path -LiteralPath $CheckpointPath)) {
        try {
            $Checkpoint = Get-Content -LiteralPath $CheckpointPath -Raw | ConvertFrom-Json
            if ($Checkpoint.sessions -and $Checkpoint.sessions.Count -gt 0) {
                $RunState = $Checkpoint.sessions[0].session.run_state
                if ($RunState) {
                    if ($RunState.seed -ne $null) { $Config.Seed = [long] $RunState.seed }
                    if ($RunState.ascension_level -ne $null) { $Config.Ascension = [int] $RunState.ascension_level }
                    if ($RunState.player_class) { $Config.Class = ([string] $RunState.player_class).ToLowerInvariant() }
                }
            }
        } catch {
            # Older checkpoints may not expose run_state; leave fields unset.
        }
    }

    if ($ManifestPath -and (Test-Path -LiteralPath $ManifestPath)) {
        try {
            $Manifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
            if ($Manifest.mode) { $Config.Mode = ([string] $Manifest.mode).ToLowerInvariant() }
        } catch {
            # Latest artifacts can lack a manifest; existing sidecar mode fallback remains in effect.
        }
    }

    return [pscustomobject] $Config
}

function Get-CampaignSourceArtifact {
    param(
        [bool] $UseScratchLatest
    )

    if ($UseScratchLatest) {
        $ScratchArtifact = Get-LatestScratchCampaignArtifact
        return [pscustomobject]@{
            ReportPath = $ScratchArtifact.ReportPath
            CheckpointPath = $ScratchArtifact.CheckpointPath
            ManifestPath = $ScratchArtifact.ManifestPath
            LogPath = $ScratchArtifact.LogPath
            CommandPath = $ScratchArtifact.CommandPath
            Label = "scratch:$($ScratchArtifact.Label)"
        }
    }

    return [pscustomobject]@{
        ReportPath = $LatestCampaignPath
        CheckpointPath = $LatestCheckpointPath
        ManifestPath = $LatestManifestPath
        LogPath = $LatestLogPath
        CommandPath = $LatestCommandPath
        Label = "latest"
    }
}

function Format-CampaignArtifactSize {
    param(
        [long] $Bytes
    )

    if ($Bytes -ge 1048576) {
        return "{0:n1} MiB" -f ($Bytes / 1048576.0)
    }
    if ($Bytes -ge 1024) {
        return "{0:n1} KiB" -f ($Bytes / 1024.0)
    }
    return "$Bytes B"
}

function Get-CampaignValueCount {
    param(
        [object] $Value
    )

    if ($null -eq $Value) {
        return 0
    }
    if ($Value -is [System.Array]) {
        return $Value.Count
    }
    if ($Value -is [System.Collections.ICollection]) {
        return $Value.Count
    }
    return 1
}

function Get-CampaignJsonTopFields {
    param(
        [object] $Json,
        [int] $Limit = 10
    )

    if ($null -eq $Json) {
        return "-"
    }
    $Names = @($Json.PSObject.Properties.Name)
    if ($Names.Count -eq 0) {
        return "-"
    }
    $Shown = @($Names | Select-Object -First $Limit)
    $Suffix = if ($Names.Count -gt $Limit) { ", ..." } else { "" }
    return ($Shown -join ", ") + $Suffix
}

function Read-CampaignJsonArtifact {
    param(
        [string] $Path
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        return $null
    }
    try {
        return Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
    } catch {
        return $null
    }
}

function Get-CampaignArtifactShape {
    param(
        [string] $Kind,
        [string] $Path
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        return "missing"
    }

    if ($Kind -eq "log") {
        return "text log"
    }
    if ($Kind -eq "command") {
        return "primary_driver_command"
    }

    $Json = Read-CampaignJsonArtifact -Path $Path
    if ($null -eq $Json) {
        return "unreadable_json"
    }

    if ($Kind -eq "manifest") {
        $WrapperParams = 0
        if ($Json.wrapper_invocation -and $Json.wrapper_invocation.bound_parameters) {
            $WrapperParams = @($Json.wrapper_invocation.bound_parameters.PSObject.Properties).Count
        }
        $DriverArgs = Get-CampaignValueCount -Value $Json.primary_driver.args
        return "stage=$($Json.stage) kind=$($Json.command_kind) wrapper_params=$WrapperParams driver_args=$DriverArgs"
    }

    if ($Kind -eq "report") {
        $Active = Get-CampaignValueCount -Value $Json.active
        $Frozen = Get-CampaignValueCount -Value $Json.frozen
        $Journal = Get-CampaignValueCount -Value $Json.journal
        $Rounds = Get-CampaignValueCount -Value $Json.rounds
        $StateSessions = 0
        $StateNodes = 0
        $DecisionSessions = 0
        $RouteDecisionSessions = 0
        $SessionsPruned = 0
        if ($Json.state_store) {
            $StateSessions = $Json.state_store.sessions
            $StateNodes = $Json.state_store.nodes
            $DecisionSessions = $Json.state_store.decision_coordinate_sessions
            $RouteDecisionSessions = $Json.state_store.route_decision_coordinate_sessions
            $SessionsPruned = $Json.state_store.sessions_pruned
        }
        return "rounds=$($Json.rounds_completed) stop=$($Json.stop_reason) active=$Active frozen=$Frozen journal=$Journal round_entries=$Rounds state_sessions=$StateSessions state_nodes=$StateNodes decision_sessions=$DecisionSessions route_decision_sessions=$RouteDecisionSessions pruned=$SessionsPruned"
    }

    if ($Kind -eq "checkpoint") {
        $Nodes = Get-CampaignValueCount -Value $Json.nodes
        $Sessions = Get-CampaignValueCount -Value $Json.sessions
        $AnchorPaths = Get-CampaignValueCount -Value $Json.decision_parent_anchor_commands
        $PreludeCommands = 0
        if ($Json.run_prelude -and $Json.run_prelude.commands) {
            $PreludeCommands = Get-CampaignValueCount -Value $Json.run_prelude.commands
        }
        $ApproxSessionBytes = "-"
        if ($Sessions -gt 0) {
            $CheckpointBytes = (Get-Item -LiteralPath $Path).Length
            $ApproxSessionBytes = Format-CampaignArtifactSize -Bytes ([long]($CheckpointBytes / $Sessions))
        }
        return "rounds=$($Json.rounds_completed) nodes=$Nodes sessions=$Sessions anchor_paths=$AnchorPaths approx_bytes_per_session=$ApproxSessionBytes prelude_commands=$PreludeCommands"
    }

    return "json_fields=$(Get-CampaignJsonTopFields -Json $Json -Limit 6)"
}

function Write-CampaignArtifactSummary {
    param(
        [string] $SourceLabel,
        [string] $ReportPath,
        [string] $CheckpointPath,
        [string] $ManifestPath,
        [string] $LogPath,
        [string] $CommandPath
    )

    Write-Host "CampaignArtifactContractV1 source=$SourceLabel"
    $Artifacts = @(
        [pscustomobject]@{ Kind = "manifest"; Path = $ManifestPath; Contract = "run provenance" },
        [pscustomobject]@{ Kind = "report"; Path = $ReportPath; Contract = "campaign summary" },
        [pscustomobject]@{ Kind = "checkpoint"; Path = $CheckpointPath; Contract = "continuation state" },
        [pscustomobject]@{ Kind = "log"; Path = $LogPath; Contract = "optional stream log" },
        [pscustomobject]@{ Kind = "command"; Path = $CommandPath; Contract = "primary driver command" }
    )

    foreach ($Artifact in $Artifacts) {
        if (Test-Path -LiteralPath $Artifact.Path) {
            $Item = Get-Item -LiteralPath $Artifact.Path
            $Size = Format-CampaignArtifactSize -Bytes $Item.Length
            $Shape = Get-CampaignArtifactShape -Kind $Artifact.Kind -Path $Artifact.Path
            Write-Host ("  {0,-10} {1,10} | {2,-22} | {3}" -f $Artifact.Kind, $Size, $Artifact.Contract, $Shape)
            Write-Host "    path=$($Artifact.Path)"
        } else {
            Write-Host ("  {0,-10} {1,10} | {2,-22} | missing" -f $Artifact.Kind, "-", $Artifact.Contract)
            Write-Host "    path=$($Artifact.Path)"
        }
    }
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
        ((-not $More) -and (-not $Inspect) -and (-not $PlanTargets) -and (-not $ContinueTargets) -and (-not $PlanCoverageGaps))
    )
) {
    throw "-Scratch currently supports normal campaign runs and -ContinueCoverageGaps only."
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
$ScratchManifestPath = $LatestManifestPath
$ScratchLogPath = $LatestLogPath
if ($Scratch) {
    $ScratchStamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $BaseLabel = if ($RunLabel) { $RunLabel } elseif ($PlanCoverageGaps -or $ContinueCoverageGaps) { "coverage-gap-seed$Seed" } else { "campaign-seed$Seed" }
    $ScratchLabel = "$(Convert-ToCampaignArtifactSlug $BaseLabel)-$ScratchStamp"
    New-Item -ItemType Directory -Force -Path $ScratchCampaignDir | Out-Null
    $ScratchCampaignPath = Join-Path $ScratchCampaignDir "$ScratchLabel.campaign.json"
    $ScratchCheckpointPath = Join-Path $ScratchCampaignDir "$ScratchLabel.checkpoint.json"
    $ScratchCommandPath = Join-Path $ScratchCampaignDir "$ScratchLabel.command.txt"
    $ScratchManifestPath = Join-Path $ScratchCampaignDir "$ScratchLabel.manifest.json"
    $ScratchLogPath = Join-Path $ScratchCampaignDir "$ScratchLabel.log"
}
$RunOutputCampaignPath = $ScratchCampaignPath
$RunOutputCheckpointPath = $ScratchCheckpointPath
$RunCommandPath = $ScratchCommandPath
$RunManifestPath = $ScratchManifestPath
$RunLogPath = $ScratchLogPath

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

$DriverArgs += @("--out", "$RunOutputCampaignPath", "--checkpoint-out", "$RunOutputCheckpointPath")

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

function Write-CampaignWrapperManifest {
    param(
        [string] $Path,
        [object] $Manifest
    )

    if (-not $Path) {
        return
    }
    $Parent = Split-Path -Parent $Path
    if ($Parent) {
        New-Item -ItemType Directory -Force -Path $Parent | Out-Null
    }
    $Manifest | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $Path
}

function Write-CampaignPrimaryDriverCommandRecord {
    param(
        [string] $PrimaryDriverCommandLine
    )

    if ($Scratch) {
        Set-Content -LiteralPath $RunCommandPath -Value $PrimaryDriverCommandLine
        Write-Host "scratch-primary-driver-command=$RunCommandPath"
        Write-Host "scratch-manifest=$RunManifestPath"
        return
    }

    Set-Content -LiteralPath $LatestSeedPath -Value $Seed
    Set-Content -LiteralPath $LatestAscensionPath -Value $Ascension
    Set-Content -LiteralPath $LatestClassPath -Value $Class
    Set-Content -LiteralPath $LatestModePath -Value $Mode
    Set-Content -LiteralPath $LatestCommandPath -Value $PrimaryDriverCommandLine
}

function Invoke-CampaignLoggedDriverCommand {
    param(
        [string] $ExePath,
        [string[]] $Arguments,
        [string] $LogPath
    )

    $LogParent = Split-Path -Parent $LogPath
    if ($LogParent) {
        New-Item -ItemType Directory -Force -Path $LogParent | Out-Null
    }
    $DriverStderrLogPath = "$LogPath.stderr.tmp"
    Remove-Item -LiteralPath $LogPath, $DriverStderrLogPath -Force -ErrorAction SilentlyContinue
    $PreviousErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = "Continue"
        & $ExePath @Arguments 2> $DriverStderrLogPath |
            Tee-Object -FilePath $LogPath |
            ForEach-Object { Write-Host $_ }
        $ExitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $PreviousErrorActionPreference
    }
    if (Test-Path -LiteralPath $DriverStderrLogPath) {
        $DriverStderrText = Get-Content -LiteralPath $DriverStderrLogPath -Raw
        if ($DriverStderrText) {
            Add-Content -LiteralPath $LogPath -Value ""
            Add-Content -LiteralPath $LogPath -Value "[stderr]"
            Add-Content -LiteralPath $LogPath -Value $DriverStderrText
        }
        Remove-Item -LiteralPath $DriverStderrLogPath -Force -ErrorAction SilentlyContinue
    }
    return $ExitCode
}

function New-CampaignWrapperManifestBase {
    param(
        [int] $ExitCode,
        [string] $Stage,
        [string] $CommandKind,
        [string[]] $PrimaryDriverArgs,
        [string] $PrimaryDriverCommand
    )

    return [ordered]@{
        schema_name = "CampaignWrapperManifestV1"
        schema_version = 1
        created_at = (Get-Date).ToString("o")
        stage = $Stage
        exit_code = $ExitCode
        wrapper_script = $PSCommandPath
        command_kind = $CommandKind
        mode = $Mode
        seed = $Seed
        ascension = $Ascension
        class = $Class
        build_profile = $BuildProfile
        driver_exe = "$DriverExe"
        scratch = [bool] $Scratch
        scratch_label = $ScratchLabel
        output_report = "$RunOutputCampaignPath"
        output_checkpoint = "$RunOutputCheckpointPath"
        command_file_semantics = "primary_driver_command"
        command_file = "$RunCommandPath"
        manifest_file = "$RunManifestPath"
        wrapper_invocation = [ordered]@{
            line = $CampaignWrapperInvocationLine
            bound_parameters = $CampaignWrapperBoundParameters
        }
        primary_driver = [ordered]@{
            args = @($PrimaryDriverArgs)
            command = $PrimaryDriverCommand
            command_file = "$RunCommandPath"
        }
    }
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
    $SummaryArgs += $CoverageGapResultFilterArgs
    if (Test-Path -LiteralPath $RunOutputCheckpointPath) {
        $SummaryArgs += @("--inspect-checkpoint", "$RunOutputCheckpointPath")
    }
    Write-Host "coverage-gap-milestone-summary:"
    & $DriverExe @SummaryArgs | ForEach-Object { Write-Host $_ }
    $SummaryExitCode = $LASTEXITCODE
    return $SummaryExitCode
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
    if ($InspectScratchLatest -and ($PlanTargets -or $ContinueTargets)) {
        throw "-InspectScratchLatest is not supported for targeted continuation yet; use inspect or coverage-gap continuation."
    }
    if ($InspectScratchLatest -and $ContinueCoverageGaps -and -not $Scratch) {
        throw "-InspectScratchLatest with -ContinueCoverageGaps requires -Scratch so scratch source data is not written back to latest."
    }

    $ContinuationSource = Get-CampaignSourceArtifact -UseScratchLatest $InspectScratchLatest
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
    $CoveragePlanArgs = @(
        "dataset",
        "--inspect-report", "$SourceCampaignPath",
        "--inspect-checkpoint", "$SourceCheckpointPath",
        "--plan-coverage-gap-continuation",
        "--coverage-gap-limit", "$CoverageGapLimit",
        "--coverage-gap-candidates-per-decision", "$CoverageGapCandidatesPerDecision"
    )
    $CoveragePlanArgs += $CoverageGapFilterArgs
    $ExportDecisionArgs = @(
        "dataset",
        "--inspect-report", "$SourceCampaignPath",
        "--inspect-checkpoint", "$SourceCheckpointPath",
        "--export-decision-outcome-dataset", "$TargetDecisionOutcomePath"
    )
    $PlanTargetArgs = @("continue", "--plan-targeted-continuation", "$TargetDecisionOutcomePath")
    $ExportDecisionAfterArgs = @(
        "dataset",
        "--inspect-report", "$RunOutputCampaignPath",
        "--inspect-checkpoint", "$RunOutputCheckpointPath",
        "--export-decision-outcome-dataset", "$LatestDecisionOutcomeAfterPath"
    )
    $ContinuationEffectArgs = @(
        "continue",
        "--continuation-effect-before", "$TargetDecisionOutcomePath",
        "--continuation-effect-after", "$LatestDecisionOutcomeAfterPath"
    )

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
        "--resume", "$SourceCampaignPath",
        "--resume-checkpoint", "$SourceCheckpointPath",
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
        "--resume", "$SourceCampaignPath",
        "--resume-checkpoint", "$SourceCheckpointPath",
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

    function New-CoverageGapMilestoneSummaryArgs {
        $Args = @(
            "inspect",
            "--inspect-report", "$RunOutputCampaignPath",
            "--inspect-checkpoint", "$RunOutputCheckpointPath",
            "--inspect-coverage-gap-milestone-summary",
            "--coverage-gap-milestone-target", "$UntilMilestone"
        )
        $Args += $CoverageGapResultFilterArgs
        return $Args
    }

    function New-CoverageGapWrapperManifest {
        param(
            [int] $ExitCode,
            [string] $Stage
        )

        $Manifest = New-CampaignWrapperManifestBase `
            -ExitCode $ExitCode `
            -Stage $Stage `
            -CommandKind "coverage_gap_continuation" `
            -PrimaryDriverArgs $ContinueCoverageGapArgs `
            -PrimaryDriverCommand (Format-CommandLine -ExePath $DriverExe -Arguments $ContinueCoverageGapArgs)
        $Manifest["source_label"] = "$SourceLabel"
        $Manifest["source_report"] = "$SourceCampaignPath"
        $Manifest["source_checkpoint"] = "$SourceCheckpointPath"
        $Manifest["coverage_gap"] = [ordered]@{
            limit = $CoverageGapLimit
            candidates_per_decision = $CoverageGapCandidatesPerDecision
            intent = $CoverageGapIntent
            execution = $CoverageGapExecutionLabel
            seed_execution = $CoverageGapDriverExecution
            filter = $CoverageGapFilterLabel
            result_filter = $CoverageGapResultFilterLabel
        }

        if ($UntilMilestoneBound) {
            $MilestoneResumeArgs = New-MilestoneResumeDriverArgs -StepRounds $MilestoneStepRounds
            $MilestoneSummaryArgs = New-CoverageGapMilestoneSummaryArgs
            $Manifest["milestone"] = [ordered]@{
                target = $UntilMilestone
                stop = $ResolvedMilestoneStop
                step_rounds = $MilestoneStepRounds
                max_additional_rounds = $MilestoneMaxRounds
                initial_spent_rounds = $CoverageGapInitialSpentRounds
                resume_driver_args_template = @($MilestoneResumeArgs)
                resume_driver_command_template = (Format-CommandLine -ExePath $DriverExe -Arguments $MilestoneResumeArgs)
                summary_driver_args = @($MilestoneSummaryArgs)
                summary_driver_command = (Format-CommandLine -ExePath $DriverExe -Arguments $MilestoneSummaryArgs)
            }
        }

        return $Manifest
    }

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
    $InspectCampaignPath = $LatestCampaignPath
    $InspectCheckpointPath = $LatestCheckpointPath
    $InspectManifestPath = $LatestManifestPath
    $InspectLogPath = $LatestLogPath
    $InspectCommandPath = $LatestCommandPath
    $InspectSourceLabel = "latest"
    if ($InspectScratchLatest) {
        $ScratchArtifact = Get-LatestScratchCampaignArtifact
        $InspectCampaignPath = $ScratchArtifact.ReportPath
        $InspectCheckpointPath = $ScratchArtifact.CheckpointPath
        $InspectManifestPath = $ScratchArtifact.ManifestPath
        $InspectLogPath = $ScratchArtifact.LogPath
        $InspectCommandPath = $ScratchArtifact.CommandPath
        $InspectSourceLabel = "scratch:$($ScratchArtifact.Label)"
    }

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

    if ($ExportLearningDataset) {
        $InspectArgs = @(
            "dataset",
            "--inspect-checkpoint", "$InspectCheckpointPath",
            "--inspect-report", "$InspectCampaignPath",
            "--export-learning-dataset", "$ExportLearningDataset"
        )
    } else {
        $InspectArgs = @(
            "inspect",
            "--inspect-checkpoint", "$InspectCheckpointPath",
            "--inspect-report", "$InspectCampaignPath",
            "--branch-examples", "$BranchExamples"
        )
    }
    $DetailedInspect =
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
    if ((-not $ExportLearningDataset) -and $InspectCoverageGapTargetState) {
        $InspectArgs += @(
            "--inspect-coverage-gap-target-state",
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
