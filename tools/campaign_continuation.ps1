function Resolve-CampaignContinuationOperation {
    param(
        [object] $Context
    )

    if (-not $Context.CampaignRequest) {
        throw "Internal error: continuation operation requires CampaignEntryRequestV1."
    }

    return [pscustomobject]@{
        Kind = $Context.CampaignRequest.Kind
        PlanCoverageGaps = [bool] $Context.CampaignRequest.PlanCoverageGaps
        ContinueCoverageGaps = [bool] $Context.CampaignRequest.ContinueCoverageGaps
        UsesCoverageGap = [bool] $Context.CampaignRequest.UsesCoverageGap
    }
}

function New-CampaignContinuationEntryContext {
    param(
        [object] $CampaignRequest,
        [string] $WrapperScript,
        [string] $Mode,
        [object] $RunOutputContext,
        [object] $BoundParameterContext,
        [object] $CampaignSourceArtifact,
        [bool] $InspectScratchLatest,
        [string] $CoverageGapExecution,
        [string] $CoverageGapIntent,
        [string] $CoverageGapFilterLabel,
        [string[]] $CoverageGapFilterArgs,
        [string[]] $CoverageGapResultFilterArgs,
        [string] $CoverageGapResultFilterLabel,
        [string[]] $CampaignRunIdentityArgs,
        [object] $CampaignSharedDriverOptionContext,
        [long] $Seed,
        [int] $Ascension,
        [string] $Class,
        [object] $BuildContext,
        [bool] $NeedsBuild,
        [bool] $Scratch,
        [int] $Rounds,
        [int] $UntilRound,
        [string] $UntilMilestone,
        [int] $MilestoneStepRounds,
        [int] $MilestoneMaxRounds,
        [string] $ResolvedMilestoneStop,
        [int] $MaxRounds,
        [int] $CoverageGapLimit,
        [int] $CoverageGapCandidatesPerDecision,
        [bool] $DryRun,
        [string] $RepoRoot
    )

    return [pscustomobject]@{
        CampaignRequest = $CampaignRequest
        WrapperScript = $WrapperScript
        Mode = $Mode
        OutputArtifact = $RunOutputContext.Artifact
        RunCommandPath = $RunOutputContext.CommandPath
        RunManifestPath = $RunOutputContext.ManifestPath
        WrapperInvocationLine = $BoundParameterContext.WrapperInvocationLine
        WrapperBoundParameters = $BoundParameterContext.WrapperBoundParameters
        InspectScratchLatest = $InspectScratchLatest
        CampaignSourceArtifact = $CampaignSourceArtifact
        RunOutputCampaignPath = $RunOutputContext.CampaignPath
        RunOutputCheckpointPath = $RunOutputContext.CheckpointPath
        UntilMilestoneBound = $BoundParameterContext.UntilMilestoneBound
        MilestoneStepRounds = $MilestoneStepRounds
        RoundsBound = $BoundParameterContext.RoundsBound
        Rounds = $Rounds
        UntilRoundBound = $BoundParameterContext.UntilRoundBound
        UntilRound = $UntilRound
        MaxRoundsBound = $BoundParameterContext.MaxRoundsBound
        MaxRounds = $MaxRounds
        CoverageGapExecution = $CoverageGapExecution
        CoverageGapIntent = $CoverageGapIntent
        CampaignRunIdentityArgs = @($CampaignRunIdentityArgs)
        CampaignSharedDriverOptionContext = $CampaignSharedDriverOptionContext
        Seed = $Seed
        Ascension = $Ascension
        Class = $Class
        BuildProfile = $BuildContext.BuildProfile
        DriverExe = $BuildContext.DriverExe
        NeedsBuild = $NeedsBuild
        Scratch = $Scratch
        ScratchLabel = $RunOutputContext.ScratchLabel
        UntilMilestone = $UntilMilestone
        MilestoneMaxRounds = $MilestoneMaxRounds
        ResolvedMilestoneStop = $ResolvedMilestoneStop
        CoverageGapLimit = $CoverageGapLimit
        CoverageGapCandidatesPerDecision = $CoverageGapCandidatesPerDecision
        CoverageGapFilterLabel = $CoverageGapFilterLabel
        CoverageGapFilterArgs = @($CoverageGapFilterArgs | Where-Object { $_ })
        CoverageGapResultFilterArgs = @($CoverageGapResultFilterArgs | Where-Object { $_ })
        CoverageGapResultFilterLabel = $CoverageGapResultFilterLabel
        DryRun = $DryRun
        BuildArgs = @($BuildContext.BuildArgs)
        RepoRoot = $RepoRoot
    }
}

function Resolve-CampaignContinuationSourceContext {
    param(
        [object] $Context
    )

    $Source = $Context.CampaignSourceArtifact
    if (-not $Source) {
        throw "Internal error: campaign continuation did not resolve a source artifact."
    }

    if (-not (Test-Path $Source.ReportPath)) {
        throw "No previous campaign report found at $($Source.ReportPath). Run .\tools\campaign.ps1 first."
    }
    if (-not (Test-Path $Source.CheckpointPath)) {
        throw "No previous campaign checkpoint found at $($Source.CheckpointPath). Run .\tools\campaign.ps1 first."
    }

    $Report = Get-Content -LiteralPath $Source.ReportPath -Raw | ConvertFrom-Json
    return [pscustomobject]@{
        Label = $Source.Label
        CampaignPath = $Source.ReportPath
        CheckpointPath = $Source.CheckpointPath
        RoundsCompleted = [int] $Report.rounds_completed
    }
}

function New-CampaignContinuationCommandContext {
    param(
        [object] $Context,
        [object] $SourceContext
    )

    $Operation = Resolve-CampaignContinuationOperation -Context $Context
    $RoundBudget = Resolve-CampaignAdditionalRoundBudget `
        -ResumeRoundsCompleted $SourceContext.RoundsCompleted `
        -UntilMilestoneBound $Context.UntilMilestoneBound `
        -MilestoneStepRounds $Context.MilestoneStepRounds `
        -RoundsBound $Context.RoundsBound `
        -Rounds $Context.Rounds `
        -UntilRoundBound $Context.UntilRoundBound `
        -UntilRound $Context.UntilRound `
        -MaxRoundsBound $Context.MaxRoundsBound `
        -MaxRounds $Context.MaxRounds `
        -MaxRoundsDriverFlag "--max-rounds"
    $RoundBudgetArgs = @($RoundBudget.Args)
    $CoverageExecutionContext = Resolve-CoverageGapExecutionContext `
        -Execution $Context.CoverageGapExecution `
        -UntilMilestoneBound $Context.UntilMilestoneBound `
        -ContinueCoverageGaps $Operation.ContinueCoverageGaps `
        -HasExplicitRoundBudget ($Context.RoundsBound -or $Context.UntilRoundBound -or $Context.MaxRoundsBound) `
        -Intent $Context.CoverageGapIntent `
        -ContinuationRounds $RoundBudget.AdditionalRounds

    $CoveragePlanArgs = New-CoverageGapPlanDriverArgs `
        -SourceCampaignPath $SourceContext.CampaignPath `
        -SourceCheckpointPath $SourceContext.CheckpointPath `
        -CoverageGapLimit $Context.CoverageGapLimit `
        -CoverageGapCandidatesPerDecision $Context.CoverageGapCandidatesPerDecision `
        -CoverageGapFilterArgs $Context.CoverageGapFilterArgs
    $ContinueCoverageGapArgs = New-CoverageGapContinueDriverArgs `
        -RunIdentityArgs $Context.CampaignRunIdentityArgs `
        -SourceCampaignPath $SourceContext.CampaignPath `
        -SourceCheckpointPath $SourceContext.CheckpointPath `
        -RunOutputCampaignPath $Context.RunOutputCampaignPath `
        -RunOutputCheckpointPath $Context.RunOutputCheckpointPath `
        -RoundBudgetArgs $RoundBudgetArgs `
        -DriverExecution $CoverageExecutionContext.DriverExecution `
        -CoverageGapLimit $Context.CoverageGapLimit `
        -CoverageGapCandidatesPerDecision $Context.CoverageGapCandidatesPerDecision `
        -CoverageGapIntent $Context.CoverageGapIntent `
        -CoverageGapFilterArgs $Context.CoverageGapFilterArgs `
        -OptionContext $Context.CampaignSharedDriverOptionContext
    $CoverageGapMilestoneSummaryArgs = @(
        "inspect",
        "--inspect-report", "$($Context.RunOutputCampaignPath)",
        "--inspect-checkpoint", "$($Context.RunOutputCheckpointPath)",
        "--inspect-coverage-gap-milestone-summary",
        "--coverage-gap-milestone-target", "$($Context.UntilMilestone)"
    )
    $CoverageGapMilestoneSummaryArgs += @($Context.CoverageGapResultFilterArgs)
    $MilestoneContext = New-CampaignMilestoneContext `
        -ReportPath $Context.RunOutputCampaignPath `
        -CheckpointPath $Context.RunOutputCheckpointPath `
        -DriverExe $Context.DriverExe `
        -UntilMilestone $Context.UntilMilestone `
        -ResolvedMilestoneStop $Context.ResolvedMilestoneStop `
        -MilestoneStepRounds $Context.MilestoneStepRounds `
        -MilestoneMaxRounds $Context.MilestoneMaxRounds `
        -RunIdentityArgs $Context.CampaignRunIdentityArgs `
        -OptionContext $Context.CampaignSharedDriverOptionContext

    $PreflightContext = New-CampaignContinuationPreflightContext `
        -PlanCoverageGaps $Operation.PlanCoverageGaps `
        -ContinueCoverageGaps $Operation.ContinueCoverageGaps `
        -Seed $Context.Seed `
        -Ascension $Context.Ascension `
        -Class $Context.Class `
        -BuildProfile $Context.BuildProfile `
        -DriverExe $Context.DriverExe `
        -NeedsBuild ([bool] $Context.NeedsBuild) `
        -SourceLabel $SourceContext.Label `
        -SourceCampaignPath $SourceContext.CampaignPath `
        -SourceCheckpointPath $SourceContext.CheckpointPath `
        -Scratch ([bool] $Context.Scratch) `
        -ScratchLabel $Context.ScratchLabel `
        -RunOutputCampaignPath $Context.RunOutputCampaignPath `
        -RunOutputCheckpointPath $Context.RunOutputCheckpointPath `
        -ResumeRoundsCompleted $SourceContext.RoundsCompleted `
        -TargetRounds $RoundBudget.TargetRounds `
        -ContinuationRoundSource $RoundBudget.Source `
        -ContinuationRounds $RoundBudget.AdditionalRounds `
        -UntilMilestoneBound $Context.UntilMilestoneBound `
        -UntilMilestone $Context.UntilMilestone `
        -MilestoneStepRounds $Context.MilestoneStepRounds `
        -MilestoneMaxRounds $Context.MilestoneMaxRounds `
        -ResolvedMilestoneStop $Context.ResolvedMilestoneStop `
        -CoverageGapLimit $Context.CoverageGapLimit `
        -CoverageGapCandidatesPerDecision $Context.CoverageGapCandidatesPerDecision `
        -CoverageGapIntent $Context.CoverageGapIntent `
        -CoverageGapExecutionLabel $CoverageExecutionContext.Label `
        -CoverageGapDriverExecution $CoverageExecutionContext.DriverExecution `
        -CoverageGapFilterLabel $Context.CoverageGapFilterLabel `
        -CoverageGapInitialSpentRounds $CoverageExecutionContext.InitialSpentRounds `
        -CoverageGapResultFilterLabel $Context.CoverageGapResultFilterLabel

    return [pscustomobject]@{
        CoveragePlanArgs = @($CoveragePlanArgs)
        ContinueCoverageGapArgs = @($ContinueCoverageGapArgs)
        ContinuationRounds = [int] $RoundBudget.AdditionalRounds
        CoverageGapInitialSpentRounds = [int] $CoverageExecutionContext.InitialSpentRounds
        CoverageGapMilestoneSummaryArgs = @($CoverageGapMilestoneSummaryArgs)
        MilestoneContext = $MilestoneContext
        CoverageGapManifestContext = [pscustomobject]@{
            SourceLabel = $SourceContext.Label
            SourceCampaignPath = $SourceContext.CampaignPath
            SourceCheckpointPath = $SourceContext.CheckpointPath
            CoverageGapLimit = $Context.CoverageGapLimit
            CoverageGapCandidatesPerDecision = $Context.CoverageGapCandidatesPerDecision
            CoverageGapIntent = $Context.CoverageGapIntent
            CoverageGapExecutionLabel = $CoverageExecutionContext.Label
            CoverageGapDriverExecution = $CoverageExecutionContext.DriverExecution
            CoverageGapFilterLabel = $Context.CoverageGapFilterLabel
            CoverageGapResultFilterLabel = $Context.CoverageGapResultFilterLabel
            ContinueCoverageGapArgs = @($ContinueCoverageGapArgs)
            DriverExe = $Context.DriverExe
            UntilMilestoneBound = [bool] $Context.UntilMilestoneBound
            UntilMilestone = $Context.UntilMilestone
            ResolvedMilestoneStop = $Context.ResolvedMilestoneStop
            MilestoneStepRounds = $Context.MilestoneStepRounds
            MilestoneMaxRounds = $Context.MilestoneMaxRounds
            CoverageGapInitialSpentRounds = [int] $CoverageExecutionContext.InitialSpentRounds
            CoverageGapMilestoneSummaryArgs = @($CoverageGapMilestoneSummaryArgs)
            MilestoneContext = $MilestoneContext
        }
        PreflightContext = $PreflightContext
    }
}

function Write-CampaignContinuationDryRunCommandSet {
    param(
        [object] $Context,
        [object] $CommandContext
    )

    $Operation = Resolve-CampaignContinuationOperation -Context $Context
    if ($Context.NeedsBuild) {
        Write-CampaignBuildCommandPreview -BuildArgs $Context.BuildArgs
    }
    Write-CoverageGapContinuationDryRunCommands `
        -PlanCoverageGaps $Operation.PlanCoverageGaps `
        -ContinueCoverageGaps $Operation.ContinueCoverageGaps `
        -UntilMilestoneBound $Context.UntilMilestoneBound `
        -DriverExe $Context.DriverExe `
        -CoveragePlanArgs $CommandContext.CoveragePlanArgs `
        -ContinueCoverageGapArgs $CommandContext.ContinueCoverageGapArgs `
        -MilestoneContext $CommandContext.MilestoneContext `
        -CoverageGapMilestoneSummaryArgs $CommandContext.CoverageGapMilestoneSummaryArgs
}

function Invoke-CampaignContinuationCommandSet {
    param(
        [object] $Context,
        [object] $CommandContext
    )

    $Operation = Resolve-CampaignContinuationOperation -Context $Context

    if ($Operation.ContinueCoverageGaps -and $CommandContext.ContinuationRounds -eq 0) {
        Write-Host "already-at-target-rounds=yes; nothing to run"
        return 0
    }

    Push-Location $Context.RepoRoot
    try {
        if ($Context.NeedsBuild) {
            & cargo @($Context.BuildArgs)
            if ($LASTEXITCODE -ne 0) {
                return $LASTEXITCODE
            }
        }
        if ($Operation.UsesCoverageGap) {
            return Invoke-CoverageGapContinuationCommands `
                -PlanCoverageGaps $Operation.PlanCoverageGaps `
                -ContinueCoverageGaps $Operation.ContinueCoverageGaps `
                -DriverExe $Context.DriverExe `
                -CoveragePlanArgs $CommandContext.CoveragePlanArgs `
                -ContinueCoverageGapArgs $CommandContext.ContinueCoverageGapArgs `
                -UntilMilestoneBound $Context.UntilMilestoneBound `
                -CoverageGapInitialSpentRounds $CommandContext.CoverageGapInitialSpentRounds `
                -RunIdentityArgs $Context.CampaignRunIdentityArgs `
                -OptionContext $Context.CampaignSharedDriverOptionContext `
                -MilestoneContext $CommandContext.MilestoneContext `
                -RecordContext $Context `
                -ManifestContext $CommandContext.CoverageGapManifestContext `
                -CoverageGapMilestoneSummaryArgs $CommandContext.CoverageGapMilestoneSummaryArgs
        }
        return 0
    } finally {
        Pop-Location
    }
}

function Invoke-CampaignContinuationEntry {
    param(
        [object] $Context
    )

    $SourceContext = Resolve-CampaignContinuationSourceContext -Context $Context
    $CommandContext = New-CampaignContinuationCommandContext -Context $Context -SourceContext $SourceContext
    Write-CampaignContinuationPreflight -Context $CommandContext.PreflightContext

    if ($Context.DryRun) {
        Write-CampaignContinuationDryRunCommandSet -Context $Context -CommandContext $CommandContext
        return 0
    }

    return Invoke-CampaignContinuationCommandSet -Context $Context -CommandContext $CommandContext
}
