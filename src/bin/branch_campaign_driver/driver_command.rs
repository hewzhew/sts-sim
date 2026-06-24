use super::cli_args::{Args, BranchCampaignCliInputV1, BranchCampaignExplicitCommandV1};
use super::command_inputs::{
    ArtifactCommandInput, ContinuationCommandInput, DatasetCommandInput, InspectCommandInput,
    RunCommandInput,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BranchCampaignDriverCommandV1 {
    SelfCheckAncestorReplay,
    AnalyzeOutcomeDataset,
    AnalyzeDecisionOutcomeDataset,
    ProbeLearningReadiness,
    PlanTargetedContinuation,
    ExecuteTargetedContinuation,
    ResolveCampaignArtifact,
    PlanCoverageGapContinuation,
    ExecuteCoverageGapContinuation,
    ContinuationEffectReport,
    ExportOutcomeDataset,
    ExportLearningDataset,
    ExportDecisionOutcomeDataset,
    InspectJournal,
    InspectLineageDecisions,
    InspectDecisionCoverage,
    InspectCoverageGapMilestoneSummary,
    InspectCoverageGapTargetState,
    InspectDecisionObservations,
    InspectFinalBossCombat,
    InspectCheckpoint,
    RunCampaign,
}

#[derive(Clone, Debug)]
pub(super) enum BranchCampaignDriverRequestV1 {
    SelfCheckAncestorReplay,
    AnalyzeOutcomeDataset(DatasetCommandInput),
    AnalyzeDecisionOutcomeDataset(DatasetCommandInput),
    ProbeLearningReadiness(DatasetCommandInput),
    PlanTargetedContinuation(ContinuationCommandInput),
    ExecuteTargetedContinuation(ContinuationCommandInput),
    ResolveCampaignArtifact(ArtifactCommandInput),
    PlanCoverageGapContinuation(DatasetCommandInput),
    ExecuteCoverageGapContinuation(ContinuationCommandInput),
    ContinuationEffectReport(ContinuationCommandInput),
    ExportOutcomeDataset(DatasetCommandInput),
    ExportLearningDataset(DatasetCommandInput),
    ExportDecisionOutcomeDataset(DatasetCommandInput),
    InspectJournal(InspectCommandInput),
    InspectLineageDecisions(InspectCommandInput),
    InspectDecisionCoverage(DatasetCommandInput),
    InspectCoverageGapMilestoneSummary(InspectCommandInput),
    InspectCoverageGapTargetState(InspectCommandInput),
    InspectDecisionObservations(InspectCommandInput),
    InspectFinalBossCombat(InspectCommandInput),
    InspectCheckpoint(InspectCommandInput),
    RunCampaign(RunCommandInput),
}

pub(super) fn driver_request_from_cli_input(
    input: &BranchCampaignCliInputV1,
) -> Result<BranchCampaignDriverRequestV1, String> {
    let args = input.args();
    Ok(match driver_command_from_cli_input(input) {
        BranchCampaignDriverCommandV1::SelfCheckAncestorReplay => {
            BranchCampaignDriverRequestV1::SelfCheckAncestorReplay
        }
        BranchCampaignDriverCommandV1::AnalyzeOutcomeDataset => {
            BranchCampaignDriverRequestV1::AnalyzeOutcomeDataset(DatasetCommandInput::from_args(
                args,
            ))
        }
        BranchCampaignDriverCommandV1::AnalyzeDecisionOutcomeDataset => {
            BranchCampaignDriverRequestV1::AnalyzeDecisionOutcomeDataset(
                DatasetCommandInput::from_args(args),
            )
        }
        BranchCampaignDriverCommandV1::ProbeLearningReadiness => {
            BranchCampaignDriverRequestV1::ProbeLearningReadiness(DatasetCommandInput::from_args(
                args,
            ))
        }
        BranchCampaignDriverCommandV1::PlanTargetedContinuation => {
            BranchCampaignDriverRequestV1::PlanTargetedContinuation(
                ContinuationCommandInput::from_args(args)?,
            )
        }
        BranchCampaignDriverCommandV1::ExecuteTargetedContinuation => {
            BranchCampaignDriverRequestV1::ExecuteTargetedContinuation(
                ContinuationCommandInput::from_args(args)?,
            )
        }
        BranchCampaignDriverCommandV1::ResolveCampaignArtifact => {
            BranchCampaignDriverRequestV1::ResolveCampaignArtifact(ArtifactCommandInput::from_args(
                args,
            )?)
        }
        BranchCampaignDriverCommandV1::PlanCoverageGapContinuation => {
            BranchCampaignDriverRequestV1::PlanCoverageGapContinuation(
                DatasetCommandInput::from_args(args),
            )
        }
        BranchCampaignDriverCommandV1::ExecuteCoverageGapContinuation => {
            BranchCampaignDriverRequestV1::ExecuteCoverageGapContinuation(
                ContinuationCommandInput::from_args(args)?,
            )
        }
        BranchCampaignDriverCommandV1::ContinuationEffectReport => {
            BranchCampaignDriverRequestV1::ContinuationEffectReport(
                ContinuationCommandInput::from_args(args)?,
            )
        }
        BranchCampaignDriverCommandV1::ExportOutcomeDataset => {
            BranchCampaignDriverRequestV1::ExportOutcomeDataset(DatasetCommandInput::from_args(
                args,
            ))
        }
        BranchCampaignDriverCommandV1::ExportLearningDataset => {
            BranchCampaignDriverRequestV1::ExportLearningDataset(DatasetCommandInput::from_args(
                args,
            ))
        }
        BranchCampaignDriverCommandV1::ExportDecisionOutcomeDataset => {
            BranchCampaignDriverRequestV1::ExportDecisionOutcomeDataset(
                DatasetCommandInput::from_args(args),
            )
        }
        BranchCampaignDriverCommandV1::InspectJournal => {
            BranchCampaignDriverRequestV1::InspectJournal(InspectCommandInput::from_args(args)?)
        }
        BranchCampaignDriverCommandV1::InspectLineageDecisions => {
            BranchCampaignDriverRequestV1::InspectLineageDecisions(InspectCommandInput::from_args(
                args,
            )?)
        }
        BranchCampaignDriverCommandV1::InspectDecisionCoverage => {
            BranchCampaignDriverRequestV1::InspectDecisionCoverage(DatasetCommandInput::from_args(
                args,
            ))
        }
        BranchCampaignDriverCommandV1::InspectCoverageGapMilestoneSummary => {
            BranchCampaignDriverRequestV1::InspectCoverageGapMilestoneSummary(
                InspectCommandInput::from_args(args)?,
            )
        }
        BranchCampaignDriverCommandV1::InspectCoverageGapTargetState => {
            BranchCampaignDriverRequestV1::InspectCoverageGapTargetState(
                InspectCommandInput::from_args(args)?,
            )
        }
        BranchCampaignDriverCommandV1::InspectDecisionObservations => {
            BranchCampaignDriverRequestV1::InspectDecisionObservations(
                InspectCommandInput::from_args(args)?,
            )
        }
        BranchCampaignDriverCommandV1::InspectFinalBossCombat => {
            BranchCampaignDriverRequestV1::InspectFinalBossCombat(InspectCommandInput::from_args(
                args,
            )?)
        }
        BranchCampaignDriverCommandV1::InspectCheckpoint => {
            BranchCampaignDriverRequestV1::InspectCheckpoint(InspectCommandInput::from_args(args)?)
        }
        BranchCampaignDriverCommandV1::RunCampaign => {
            BranchCampaignDriverRequestV1::RunCampaign(RunCommandInput::from_args(args)?)
        }
    })
}

pub(super) fn driver_command_from_cli_input(
    input: &BranchCampaignCliInputV1,
) -> BranchCampaignDriverCommandV1 {
    if let Some(explicit) = input.explicit_command() {
        return explicit_driver_command_from_args(explicit, input.args());
    }
    legacy_command_from_args(input.args())
}

#[cfg(test)]
pub(super) fn driver_command_from_args(args: &Args) -> BranchCampaignDriverCommandV1 {
    if let Some(explicit) = args.explicit_command {
        return explicit_driver_command_from_args(explicit, args);
    }
    legacy_command_from_args(args)
}

fn explicit_driver_command_from_args(
    explicit: BranchCampaignExplicitCommandV1,
    args: &Args,
) -> BranchCampaignDriverCommandV1 {
    match explicit {
        BranchCampaignExplicitCommandV1::Run => BranchCampaignDriverCommandV1::RunCampaign,
        BranchCampaignExplicitCommandV1::Inspect => inspect_command_from_args(args),
        BranchCampaignExplicitCommandV1::Dataset => dataset_command_from_args(args),
        BranchCampaignExplicitCommandV1::Continue => continuation_command_from_args(args),
        BranchCampaignExplicitCommandV1::PlanCoverageGapContinuation => {
            BranchCampaignDriverCommandV1::PlanCoverageGapContinuation
        }
        BranchCampaignExplicitCommandV1::ExecuteCoverageGapContinuation => {
            BranchCampaignDriverCommandV1::ExecuteCoverageGapContinuation
        }
        BranchCampaignExplicitCommandV1::Artifact => {
            BranchCampaignDriverCommandV1::ResolveCampaignArtifact
        }
        BranchCampaignExplicitCommandV1::SelfCheck => {
            BranchCampaignDriverCommandV1::SelfCheckAncestorReplay
        }
    }
}

fn legacy_command_from_args(args: &Args) -> BranchCampaignDriverCommandV1 {
    if args.self_check_ancestor_replay {
        return BranchCampaignDriverCommandV1::SelfCheckAncestorReplay;
    }
    if args.analyze_outcome_dataset.is_some() {
        return BranchCampaignDriverCommandV1::AnalyzeOutcomeDataset;
    }
    if args.analyze_decision_outcome_dataset.is_some() {
        return BranchCampaignDriverCommandV1::AnalyzeDecisionOutcomeDataset;
    }
    if args.probe_learning_readiness.is_some() {
        return BranchCampaignDriverCommandV1::ProbeLearningReadiness;
    }
    if args.plan_targeted_continuation.is_some() {
        return BranchCampaignDriverCommandV1::PlanTargetedContinuation;
    }
    if args.execute_targeted_continuation.is_some() {
        return BranchCampaignDriverCommandV1::ExecuteTargetedContinuation;
    }
    if args.plan_coverage_gap_continuation {
        return BranchCampaignDriverCommandV1::PlanCoverageGapContinuation;
    }
    if args.execute_coverage_gap_continuation {
        return BranchCampaignDriverCommandV1::ExecuteCoverageGapContinuation;
    }
    if args.continuation_effect_before.is_some() || args.continuation_effect_after.is_some() {
        return BranchCampaignDriverCommandV1::ContinuationEffectReport;
    }
    if args.export_outcome_dataset.is_some() && args.inspect_report.is_some() {
        return BranchCampaignDriverCommandV1::ExportOutcomeDataset;
    }
    if args.export_learning_dataset.is_some() && args.inspect_report.is_some() {
        return BranchCampaignDriverCommandV1::ExportLearningDataset;
    }
    if args.export_decision_outcome_dataset.is_some() && args.inspect_report.is_some() {
        return BranchCampaignDriverCommandV1::ExportDecisionOutcomeDataset;
    }
    if args.inspect_final_boss_combat {
        return BranchCampaignDriverCommandV1::InspectFinalBossCombat;
    }
    if args.inspect_journal {
        return BranchCampaignDriverCommandV1::InspectJournal;
    }
    if args.inspect_lineage_decisions {
        return BranchCampaignDriverCommandV1::InspectLineageDecisions;
    }
    if args.inspect_decision_coverage {
        return BranchCampaignDriverCommandV1::InspectDecisionCoverage;
    }
    if args.inspect_coverage_gap_target_state {
        return BranchCampaignDriverCommandV1::InspectCoverageGapTargetState;
    }
    if args.inspect_coverage_gap_milestone_summary {
        return BranchCampaignDriverCommandV1::InspectCoverageGapMilestoneSummary;
    }
    if args.inspect_decision_observations {
        return BranchCampaignDriverCommandV1::InspectDecisionObservations;
    }
    if args.inspect_checkpoint.is_some() {
        return BranchCampaignDriverCommandV1::InspectCheckpoint;
    }
    BranchCampaignDriverCommandV1::RunCampaign
}

fn inspect_command_from_args(args: &Args) -> BranchCampaignDriverCommandV1 {
    if args.inspect_journal {
        BranchCampaignDriverCommandV1::InspectJournal
    } else if args.inspect_lineage_decisions {
        BranchCampaignDriverCommandV1::InspectLineageDecisions
    } else if args.inspect_decision_coverage {
        BranchCampaignDriverCommandV1::InspectDecisionCoverage
    } else if args.inspect_coverage_gap_target_state {
        BranchCampaignDriverCommandV1::InspectCoverageGapTargetState
    } else if args.inspect_coverage_gap_milestone_summary {
        BranchCampaignDriverCommandV1::InspectCoverageGapMilestoneSummary
    } else if args.inspect_decision_observations {
        BranchCampaignDriverCommandV1::InspectDecisionObservations
    } else if args.inspect_final_boss_combat {
        BranchCampaignDriverCommandV1::InspectFinalBossCombat
    } else {
        BranchCampaignDriverCommandV1::InspectCheckpoint
    }
}

fn dataset_command_from_args(args: &Args) -> BranchCampaignDriverCommandV1 {
    if args.analyze_outcome_dataset.is_some() {
        BranchCampaignDriverCommandV1::AnalyzeOutcomeDataset
    } else if args.analyze_decision_outcome_dataset.is_some() {
        BranchCampaignDriverCommandV1::AnalyzeDecisionOutcomeDataset
    } else if args.probe_learning_readiness.is_some() {
        BranchCampaignDriverCommandV1::ProbeLearningReadiness
    } else if args.plan_coverage_gap_continuation {
        BranchCampaignDriverCommandV1::PlanCoverageGapContinuation
    } else if args.export_outcome_dataset.is_some() {
        BranchCampaignDriverCommandV1::ExportOutcomeDataset
    } else if args.export_learning_dataset.is_some() {
        BranchCampaignDriverCommandV1::ExportLearningDataset
    } else if args.export_decision_outcome_dataset.is_some() {
        BranchCampaignDriverCommandV1::ExportDecisionOutcomeDataset
    } else {
        BranchCampaignDriverCommandV1::AnalyzeDecisionOutcomeDataset
    }
}

fn continuation_command_from_args(args: &Args) -> BranchCampaignDriverCommandV1 {
    if args.plan_targeted_continuation.is_some() {
        BranchCampaignDriverCommandV1::PlanTargetedContinuation
    } else if args.execute_targeted_continuation.is_some() {
        BranchCampaignDriverCommandV1::ExecuteTargetedContinuation
    } else if args.execute_coverage_gap_continuation {
        BranchCampaignDriverCommandV1::ExecuteCoverageGapContinuation
    } else if args.continuation_effect_before.is_some() || args.continuation_effect_after.is_some()
    {
        BranchCampaignDriverCommandV1::ContinuationEffectReport
    } else {
        BranchCampaignDriverCommandV1::PlanTargetedContinuation
    }
}
