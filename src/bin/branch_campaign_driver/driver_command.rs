use super::Args;

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
    InspectFinalBossCombat,
    InspectCheckpoint,
    RunCampaign,
}

pub(super) fn driver_command_from_args(args: &Args) -> BranchCampaignDriverCommandV1 {
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
    if args.inspect_checkpoint.is_some() {
        return BranchCampaignDriverCommandV1::InspectCheckpoint;
    }
    BranchCampaignDriverCommandV1::RunCampaign
}
