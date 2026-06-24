function Resolve-CampaignAdditionalRoundBudget {
    param(
        [int] $ResumeRoundsCompleted,
        [bool] $UntilMilestoneBound,
        [int] $MilestoneStepRounds,
        [bool] $RoundsBound,
        [int] $Rounds,
        [bool] $UntilRoundBound,
        [int] $UntilRound,
        [bool] $MaxRoundsBound,
        [int] $MaxRounds,
        [string] $MaxRoundsDriverFlag = "--max-rounds"
    )

    $ContinuationRounds = 1
    $RoundBudgetArgs = @("--rounds", "1")
    $TargetRounds = $null
    $Source = "default"
    if ($UntilMilestoneBound) {
        $ContinuationRounds = $MilestoneStepRounds
        $Source = "UntilMilestone"
        $RoundBudgetArgs = @("--rounds", "$MilestoneStepRounds")
    } elseif ($RoundsBound) {
        $ContinuationRounds = $Rounds
        $Source = "Rounds"
        $TargetRounds = $ResumeRoundsCompleted + $Rounds
        $RoundBudgetArgs = @("--rounds", "$Rounds")
    } elseif ($UntilRoundBound) {
        $TargetRounds = $UntilRound
        $ContinuationRounds = [Math]::Max(0, $TargetRounds - $ResumeRoundsCompleted)
        $Source = "UntilRound"
        $RoundBudgetArgs = @("--until-round", "$UntilRound")
    } elseif ($MaxRoundsBound) {
        $ContinuationRounds = $MaxRounds
        $Source = "MaxRounds"
        $TargetRounds = $ResumeRoundsCompleted + $MaxRounds
        $RoundBudgetArgs = @($MaxRoundsDriverFlag, "$ContinuationRounds")
    }

    return [pscustomobject]@{
        AdditionalRounds = $ContinuationRounds
        Args = @($RoundBudgetArgs)
        TargetRounds = $TargetRounds
        Source = $Source
    }
}

function Resolve-CampaignRunRoundContext {
    param(
        [object] $Request,
        [object] $CampaignSourceArtifact,
        [bool] $RoundsBound,
        [int] $Rounds,
        [bool] $UntilRoundBound,
        [int] $UntilRound,
        [bool] $UntilMilestoneBound,
        [string] $UntilMilestone,
        [int] $MilestoneStepRounds,
        [int] $MilestoneMaxRounds,
        [string] $MilestoneStop,
        [bool] $MaxRoundsBound,
        [int] $MaxRounds
    )

    if (-not $Request) {
        throw "Internal error: round context requires CampaignEntryRequestV1."
    }
    $ContinueCampaign = [bool] $Request.ContinueCampaign
    $ContinueCoverageGaps = [bool] $Request.ContinueCoverageGaps
    $PlanCoverageGaps = [bool] $Request.PlanCoverageGaps
    $Inspect = [bool] $Request.Inspect
    $IsContinuationRoundCommand = $ContinueCampaign -or $ContinueCoverageGaps
    $HasExplicitRoundBudget = $RoundsBound -or $UntilRoundBound -or $MaxRoundsBound

    if (($RoundsBound -and $UntilRoundBound) -or ($RoundsBound -and $MaxRoundsBound) -or ($UntilRoundBound -and $MaxRoundsBound)) {
        throw "Choose only one round budget: -Rounds N, -UntilRound N, or legacy -MaxRounds N."
    }
    if ($UntilMilestoneBound -and $HasExplicitRoundBudget) {
        throw "-UntilMilestone owns the round budget. Use -MilestoneStepRounds and -MilestoneMaxRounds instead of -Rounds, -UntilRound, or -MaxRounds."
    }
    if ($UntilMilestoneBound -and ($PlanCoverageGaps -or $Inspect)) {
        throw "-UntilMilestone requires an executing command (-Continue, -ContinueCoverageGaps, or a normal run), not a plan/inspect command."
    }

    $ResolvedMilestoneStop = $MilestoneStop
    if ($ResolvedMilestoneStop -eq "auto") {
        if ($ContinueCoverageGaps) {
            $ResolvedMilestoneStop = "round_cap"
        } else {
            $ResolvedMilestoneStop = "first_hit"
        }
    }
    $ConcreteUntilMilestone = $UntilMilestone
    if ($UntilMilestoneBound) {
        $ConcreteUntilMilestone = Resolve-CampaignConcreteMilestone -Milestone $UntilMilestone
    }

    $DriverRoundBudgetArgs = @()
    $RoundBudgetSource = if ($MaxRoundsBound) { "MaxRounds" } else { "preset" }
    $ResolvedMaxRounds = $MaxRounds
    $ResumeCampaignPath = $null
    $ResumeCheckpointPath = $null
    $ResumeRoundsCompleted = $null
    $TargetRounds = $null
    $ResumeDriverArgs = @()
    $RoundBudgetAdditionalRounds = $MaxRounds

    if ($UntilMilestoneBound) {
        $ResolvedMaxRounds = $MilestoneStepRounds
        $RoundBudgetAdditionalRounds = $MilestoneStepRounds
        $DriverRoundBudgetArgs = @("--rounds", "$MilestoneStepRounds")
        $RoundBudgetSource = "UntilMilestone"
    } elseif (-not $ContinueCampaign) {
        if ($RoundsBound) {
            $DriverRoundBudgetArgs = @("--rounds", "$Rounds")
            $RoundBudgetSource = "Rounds"
            $RoundBudgetAdditionalRounds = $Rounds
        } elseif ($UntilRoundBound) {
            $DriverRoundBudgetArgs = @("--until-round", "$UntilRound")
            $RoundBudgetSource = "UntilRound"
            $TargetRounds = $UntilRound
            $RoundBudgetAdditionalRounds = $UntilRound
        } elseif ($MaxRoundsBound) {
            $DriverRoundBudgetArgs = @("--max-rounds", "$MaxRounds")
            $RoundBudgetAdditionalRounds = $MaxRounds
        }
    }

    if ($IsContinuationRoundCommand) {
        $ResumeSource = $CampaignSourceArtifact
        if (-not $ResumeSource) {
            throw "Internal error: campaign continuation did not resolve a source artifact."
        }
        if (-not (Test-Path $ResumeSource.ReportPath)) {
            throw "No campaign report found for source '$($ResumeSource.Label)' at $($ResumeSource.ReportPath)."
        }

        $ResumeCampaignPath = $ResumeSource.ReportPath
        $ResumeReport = Read-CampaignJsonArtifactOrThrow -Path $ResumeCampaignPath -Role "campaign report"
        $ResumeRoundsCompleted = [int] $ResumeReport.rounds_completed
        if ($UntilMilestoneBound) {
            $ConcreteUntilMilestone = Resolve-CampaignConcreteMilestone `
                -Milestone $UntilMilestone `
                -ReportPath $ResumeCampaignPath
        }
        if ($UntilMilestoneBound -or $HasExplicitRoundBudget -or $ContinueCoverageGaps) {
            $ContinuationMaxRoundsDriverFlag = if ($ContinueCoverageGaps) { "--max-rounds" } else { "--rounds" }
            $RunContinuationRoundBudget = Resolve-CampaignAdditionalRoundBudget `
                -ResumeRoundsCompleted $ResumeRoundsCompleted `
                -UntilMilestoneBound $UntilMilestoneBound `
                -MilestoneStepRounds $MilestoneStepRounds `
                -RoundsBound $RoundsBound `
                -Rounds $Rounds `
                -UntilRoundBound $UntilRoundBound `
                -UntilRound $UntilRound `
                -MaxRoundsBound $MaxRoundsBound `
                -MaxRounds $MaxRounds `
                -MaxRoundsDriverFlag $ContinuationMaxRoundsDriverFlag
            $DriverRoundBudgetArgs = @($RunContinuationRoundBudget.Args)
            $TargetRounds = $RunContinuationRoundBudget.TargetRounds
            $ResolvedMaxRounds = $RunContinuationRoundBudget.AdditionalRounds
            $RoundBudgetAdditionalRounds = $RunContinuationRoundBudget.AdditionalRounds
            $RoundBudgetSource = $RunContinuationRoundBudget.Source
        }

        $ResumeDriverArgs += @("--resume", "$ResumeCampaignPath")
        if (Test-Path $ResumeSource.CheckpointPath) {
            $ResumeCheckpointPath = $ResumeSource.CheckpointPath
            $ResumeDriverArgs += @("--resume-checkpoint", "$ResumeCheckpointPath")
        }
    }

    return [pscustomobject]@{
        DriverRoundBudgetArgs = @($DriverRoundBudgetArgs)
        RoundBudgetSource = $RoundBudgetSource
        RoundBudgetAdditionalRounds = $RoundBudgetAdditionalRounds
        HasExplicitRoundBudget = [bool] $HasExplicitRoundBudget
        UntilMilestoneBound = [bool] $UntilMilestoneBound
        UntilMilestone = $ConcreteUntilMilestone
        MilestoneStepRounds = $MilestoneStepRounds
        MilestoneMaxRounds = $MilestoneMaxRounds
        MaxRounds = $ResolvedMaxRounds
        ResolvedMilestoneStop = $ResolvedMilestoneStop
        ResumeCampaignPath = $ResumeCampaignPath
        ResumeCheckpointPath = $ResumeCheckpointPath
        ResumeRoundsCompleted = $ResumeRoundsCompleted
        TargetRounds = $TargetRounds
        ResumeDriverArgs = @($ResumeDriverArgs)
    }
}
