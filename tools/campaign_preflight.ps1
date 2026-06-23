function Write-CampaignRunPreflight {
    param(
        [object] $Context
    )

    Write-Host "seed=$($Context.Seed)"
    Write-Host "ascension=A$($Context.Ascension) domain=a$($Context.Ascension) class=$($Context.Class)"
    Write-Host "mode=$($Context.Mode) branch campaign"
    Write-Host "build=$($Context.BuildProfile) exe=$($Context.DriverExe)"
    if ($Context.NeedsBuild) {
        Write-Host "build-needed=yes"
    } else {
        Write-Host "build-needed=no"
    }
    if ($Context.BossRelicAxes) {
        Write-Host "boss-relic-axes=on active/frozen budgets are per boss relic lineage"
    }
    Write-Host "rerun-last=.\tools\campaign.ps1 -Last"
    Write-Host "continue-latest=.\tools\campaign.ps1 -From latest -Continue"
    Write-Host "continue-one-round=.\tools\campaign.ps1 -From latest -Continue -Rounds 1"
    Write-Host "report=$($Context.RunOutputCampaignPath)"
    Write-Host "checkpoint=$($Context.RunOutputCheckpointPath)"
    Write-Host "manifest=$($Context.RunManifestPath)"
    if ($Context.Log) {
        Write-Host "log=$($Context.RunLogPath)"
    }
    Write-Host "combat-segment=$($Context.CombatSegmentMode)"
    if ($Context.ResumeCampaignPath) {
        Write-Host "resume=$($Context.ResumeCampaignPath)"
        Write-Host "resume-rounds=$($Context.ResumeRoundsCompleted)"
        if ($Context.TargetRounds -ne $null) {
            Write-Host "round-budget=$($Context.RoundBudgetSource) target-rounds=$($Context.TargetRounds) additional-rounds=$($Context.MaxRounds)"
        } elseif ($Context.RoundBudgetSource -ne "preset") {
            Write-Host "round-budget=$($Context.RoundBudgetSource) additional-rounds=$($Context.MaxRounds)"
        } else {
            Write-Host "round-budget=preset additional-rounds=mode-default"
        }
        if ($Context.ResumeCheckpointPath) {
            Write-Host "resume-checkpoint=$($Context.ResumeCheckpointPath)"
        } else {
            Write-Host "resume-checkpoint=missing; falling back to replay"
        }
    }
    if ($Context.UntilMilestoneBound) {
        Write-Host "until-milestone=$($Context.UntilMilestone) step-rounds=$($Context.MilestoneStepRounds) max-additional-rounds=$($Context.MilestoneMaxRounds)"
    }
}

function New-CampaignContinuationPreflightContext {
    param(
        [bool] $PlanTargets,
        [bool] $ContinueTargets,
        [bool] $PlanCoverageGaps,
        [bool] $ContinueCoverageGaps,
        [long] $Seed,
        [int] $Ascension,
        [string] $Class,
        [string] $BuildProfile,
        [string] $DriverExe,
        [bool] $NeedsBuild,
        [string] $SourceLabel,
        [string] $SourceCampaignPath,
        [string] $SourceCheckpointPath,
        [bool] $Scratch,
        [string] $ScratchLabel,
        [string] $RunOutputCampaignPath,
        [string] $RunOutputCheckpointPath,
        [string] $TargetDecisionOutcomePath,
        [string] $DecisionOutcomeAfterPath,
        [int] $TargetedContinuationLimit,
        [int] $TargetedContinuationCandidatesPerTarget,
        [int] $ResumeRoundsCompleted,
        [object] $TargetRounds,
        [string] $ContinuationRoundSource,
        [int] $ContinuationRounds,
        [bool] $UntilMilestoneBound,
        [string] $UntilMilestone,
        [int] $MilestoneStepRounds,
        [int] $MilestoneMaxRounds,
        [string] $ResolvedMilestoneStop,
        [int] $CoverageGapLimit,
        [int] $CoverageGapCandidatesPerDecision,
        [string] $CoverageGapIntent,
        [string] $CoverageGapExecutionLabel,
        [string] $CoverageGapDriverExecution,
        [string] $CoverageGapFilterLabel,
        [int] $CoverageGapInitialSpentRounds,
        [string] $CoverageGapResultFilterLabel
    )

    return [pscustomobject]@{
        PlanTargets = [bool] $PlanTargets
        ContinueTargets = [bool] $ContinueTargets
        PlanCoverageGaps = [bool] $PlanCoverageGaps
        ContinueCoverageGaps = [bool] $ContinueCoverageGaps
        ModeLabel = if ($PlanCoverageGaps -or $ContinueCoverageGaps) { "coverage-gap-continuation" } else { "targeted-continuation" }
        Seed = $Seed
        Ascension = $Ascension
        Class = $Class
        BuildProfile = $BuildProfile
        DriverExe = $DriverExe
        NeedsBuild = [bool] $NeedsBuild
        SourceLabel = $SourceLabel
        SourceCampaignPath = $SourceCampaignPath
        SourceCheckpointPath = $SourceCheckpointPath
        Scratch = [bool] $Scratch
        ScratchLabel = $ScratchLabel
        RunOutputCampaignPath = $RunOutputCampaignPath
        RunOutputCheckpointPath = $RunOutputCheckpointPath
        TargetDecisionOutcomePath = $TargetDecisionOutcomePath
        DecisionOutcomeAfterPath = $DecisionOutcomeAfterPath
        TargetedContinuationLimit = $TargetedContinuationLimit
        TargetedContinuationCandidatesPerTarget = $TargetedContinuationCandidatesPerTarget
        ResumeRoundsCompleted = $ResumeRoundsCompleted
        TargetRounds = $TargetRounds
        ContinuationRoundSource = $ContinuationRoundSource
        ContinuationRounds = $ContinuationRounds
        UntilMilestoneBound = [bool] $UntilMilestoneBound
        UntilMilestone = $UntilMilestone
        MilestoneStepRounds = $MilestoneStepRounds
        MilestoneMaxRounds = $MilestoneMaxRounds
        ResolvedMilestoneStop = $ResolvedMilestoneStop
        CoverageGapLimit = $CoverageGapLimit
        CoverageGapCandidatesPerDecision = $CoverageGapCandidatesPerDecision
        CoverageGapIntent = $CoverageGapIntent
        CoverageGapExecutionLabel = $CoverageGapExecutionLabel
        CoverageGapDriverExecution = $CoverageGapDriverExecution
        CoverageGapFilterLabel = $CoverageGapFilterLabel
        CoverageGapInitialSpentRounds = $CoverageGapInitialSpentRounds
        CoverageGapResultFilterLabel = $CoverageGapResultFilterLabel
    }
}

function Write-CampaignContinuationPreflight {
    param(
        [object] $Context
    )

    Write-Host "mode=$($Context.ModeLabel) branch campaign"
    Write-Host "seed=$($Context.Seed)"
    Write-Host "ascension=A$($Context.Ascension) domain=a$($Context.Ascension) class=$($Context.Class)"
    Write-Host "build=$($Context.BuildProfile) exe=$($Context.DriverExe)"
    if ($Context.NeedsBuild) {
        Write-Host "build-needed=yes"
    } else {
        Write-Host "build-needed=no"
    }
    Write-Host "source=$($Context.SourceLabel)"
    Write-Host "source-report=$($Context.SourceCampaignPath)"
    Write-Host "source-checkpoint=$($Context.SourceCheckpointPath)"
    if ($Context.Scratch) {
        Write-Host "scratch=yes label=$($Context.ScratchLabel)"
        Write-Host "report=$($Context.RunOutputCampaignPath)"
        Write-Host "checkpoint=$($Context.RunOutputCheckpointPath)"
    } elseif ($Context.ContinueTargets -or $Context.ContinueCoverageGaps) {
        Write-Host "report=$($Context.RunOutputCampaignPath)"
        Write-Host "checkpoint=$($Context.RunOutputCheckpointPath)"
    }

    if ($Context.PlanTargets -or $Context.ContinueTargets) {
        Write-Host "decision-outcomes=$($Context.TargetDecisionOutcomePath)"
    }
    if ($Context.ContinueTargets) {
        Write-Host "decision-outcomes-after=$($Context.DecisionOutcomeAfterPath)"
        Write-Host "continue-targets=$($Context.TargetedContinuationLimit) candidates-per-target=$($Context.TargetedContinuationCandidatesPerTarget)"
        Write-Host "resume-rounds=$($Context.ResumeRoundsCompleted)"
        if ($Context.TargetRounds -ne $null) {
            Write-Host "round-budget=$($Context.ContinuationRoundSource) target-rounds=$($Context.TargetRounds) additional-rounds=$($Context.ContinuationRounds)"
        } else {
            Write-Host "round-budget=$($Context.ContinuationRoundSource) additional-rounds=$($Context.ContinuationRounds)"
        }
    }
    if ($Context.UntilMilestoneBound) {
        Write-Host "until-milestone=$($Context.UntilMilestone) step-rounds=$($Context.MilestoneStepRounds) max-additional-rounds=$($Context.MilestoneMaxRounds) stop=$($Context.ResolvedMilestoneStop)"
    }
    if ($Context.PlanCoverageGaps) {
        Write-Host "coverage-gap-plan=$($Context.CoverageGapLimit) candidates-per-decision=$($Context.CoverageGapCandidatesPerDecision)"
        Write-Host "coverage-gap-filter=$($Context.CoverageGapFilterLabel)"
    }
    if ($Context.ContinueCoverageGaps) {
        if ($Context.CoverageGapExecutionLabel -eq $Context.CoverageGapDriverExecution) {
            Write-Host "coverage-gap-continue=$($Context.CoverageGapLimit) candidates-per-decision=$($Context.CoverageGapCandidatesPerDecision) intent=$($Context.CoverageGapIntent) execution=$($Context.CoverageGapExecutionLabel)"
        } else {
            Write-Host "coverage-gap-continue=$($Context.CoverageGapLimit) candidates-per-decision=$($Context.CoverageGapCandidatesPerDecision) intent=$($Context.CoverageGapIntent) execution=$($Context.CoverageGapExecutionLabel) seed-execution=$($Context.CoverageGapDriverExecution)"
        }
        Write-Host "coverage-gap-filter=$($Context.CoverageGapFilterLabel)"
        Write-Host "resume-rounds=$($Context.ResumeRoundsCompleted)"
        if ($Context.TargetRounds -ne $null) {
            Write-Host "round-budget=$($Context.ContinuationRoundSource) target-rounds=$($Context.TargetRounds) additional-rounds=$($Context.ContinuationRounds)"
        } else {
            Write-Host "round-budget=$($Context.ContinuationRoundSource) additional-rounds=$($Context.ContinuationRounds)"
        }
        if ($Context.UntilMilestoneBound) {
            Write-Host "milestone-initial-spent-rounds=$($Context.CoverageGapInitialSpentRounds)"
            if ($Context.CoverageGapResultFilterLabel -ne $Context.CoverageGapFilterLabel) {
                Write-Host "coverage-gap-result-filter=$($Context.CoverageGapResultFilterLabel)"
            }
        }
    }
}
