use super::cli_args::{Args, BranchCampaignCliInputV1, BranchCampaignExplicitCommandV1};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BranchCampaignDriverCommandV1 {
    SelfCheckAncestorReplay,
    AnalyzeOutcomeDataset,
    AnalyzeDecisionOutcomeDataset,
    ProbeLearningReadiness,
    PlanTargetedContinuation,
    ExecuteTargetedContinuation,
    ContinuationEffectReport,
    ExportOutcomeDataset,
    ExportLearningDataset,
    ExportDecisionOutcomeDataset,
    InspectJournal,
    InspectDecisionCoverage,
    InspectDecisionObservations,
    InspectFinalBossCombat,
    InspectCheckpoint,
    RunCampaign,
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
    if args.inspect_decision_coverage {
        return BranchCampaignDriverCommandV1::InspectDecisionCoverage;
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
    } else if args.inspect_decision_coverage {
        BranchCampaignDriverCommandV1::InspectDecisionCoverage
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
    } else if args.continuation_effect_before.is_some() || args.continuation_effect_after.is_some()
    {
        BranchCampaignDriverCommandV1::ContinuationEffectReport
    } else {
        BranchCampaignDriverCommandV1::PlanTargetedContinuation
    }
}
