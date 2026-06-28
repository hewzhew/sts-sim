#[cfg(test)]
use super::cli_args::DatasetCommandArgs;
use super::cli_args::{Args, BranchCampaignCliInputV1};
use super::command_inputs::{
    ArtifactCommandInput, ContinuationCommandInput, CoverageGapExecutionCommandInput,
    CoverageGapPlanCommandInput, DatasetCommandInput, InspectCommandInput, RunCommandInput,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BranchCampaignDriverCommandV1 {
    SelfCheckAncestorReplay,
    AnalyzeOutcomeDataset,
    AnalyzeDecisionOutcomeDataset,
    ProbeLearningReadiness,
    PlanTargetedContinuation,
    ExecuteTargetedContinuation,
    ContinueCampaign,
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
    ContinueCampaign(RunCommandInput),
    ResolveCampaignArtifact(ArtifactCommandInput),
    PlanCoverageGapContinuation(CoverageGapPlanCommandInput),
    ExecuteCoverageGapContinuation(CoverageGapExecutionCommandInput),
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
    InspectCheckpoint(InspectCommandInput),
    RunCampaign(RunCommandInput),
}

pub(super) fn driver_request_from_cli_input(
    input: BranchCampaignCliInputV1,
) -> Result<BranchCampaignDriverRequestV1, String> {
    match input {
        BranchCampaignCliInputV1::CampaignRun(args) => Ok(
            BranchCampaignDriverRequestV1::RunCampaign(RunCommandInput::from_run_args(args)?),
        ),
        BranchCampaignCliInputV1::CampaignContinue(args) => {
            continuation_request_from_continue_args(args)
        }
        BranchCampaignCliInputV1::CampaignInspect(args) => inspect_request_from_inspect_args(args),
        BranchCampaignCliInputV1::CampaignArtifact(args) => {
            Ok(BranchCampaignDriverRequestV1::ResolveCampaignArtifact(
                ArtifactCommandInput::from_artifact_command_args(args),
            ))
        }
        BranchCampaignCliInputV1::CampaignCoveragePlan(args) => {
            Ok(BranchCampaignDriverRequestV1::PlanCoverageGapContinuation(
                CoverageGapPlanCommandInput::from_coverage_plan_args(args)?,
            ))
        }
        BranchCampaignCliInputV1::CampaignCoverageExecute(args) => Ok(
            BranchCampaignDriverRequestV1::ExecuteCoverageGapContinuation(
                CoverageGapExecutionCommandInput::from_coverage_execute_args(args)?,
            ),
        ),
        BranchCampaignCliInputV1::CampaignDataset(args) => Ok(dataset_request_from_input(
            DatasetCommandInput::from_dataset_args(args),
        )),
        BranchCampaignCliInputV1::SelfCheck => {
            Ok(BranchCampaignDriverRequestV1::SelfCheckAncestorReplay)
        }
        BranchCampaignCliInputV1::Legacy(args) => legacy_driver_request_from_args(&args),
    }
}

fn dataset_request_from_input(input: DatasetCommandInput) -> BranchCampaignDriverRequestV1 {
    match dataset_command_from_input(&input) {
        BranchCampaignDriverCommandV1::AnalyzeOutcomeDataset => {
            BranchCampaignDriverRequestV1::AnalyzeOutcomeDataset(input)
        }
        BranchCampaignDriverCommandV1::AnalyzeDecisionOutcomeDataset => {
            BranchCampaignDriverRequestV1::AnalyzeDecisionOutcomeDataset(input)
        }
        BranchCampaignDriverCommandV1::ProbeLearningReadiness => {
            BranchCampaignDriverRequestV1::ProbeLearningReadiness(input)
        }
        BranchCampaignDriverCommandV1::ExportOutcomeDataset => {
            BranchCampaignDriverRequestV1::ExportOutcomeDataset(input)
        }
        BranchCampaignDriverCommandV1::ExportLearningDataset => {
            BranchCampaignDriverRequestV1::ExportLearningDataset(input)
        }
        BranchCampaignDriverCommandV1::ExportDecisionOutcomeDataset => {
            BranchCampaignDriverRequestV1::ExportDecisionOutcomeDataset(input)
        }
        _ => BranchCampaignDriverRequestV1::AnalyzeDecisionOutcomeDataset(input),
    }
}

fn continuation_request_from_continue_args(
    args: super::cli_args::ContinueCommandArgs,
) -> Result<BranchCampaignDriverRequestV1, String> {
    if args.continuation.execute_coverage_gap_continuation {
        return Ok(
            BranchCampaignDriverRequestV1::ExecuteCoverageGapContinuation(
                CoverageGapExecutionCommandInput::from_continue_args(args)?,
            ),
        );
    }
    if has_targeted_continuation_intent(&args.continuation) {
        return continuation_request_from_input(ContinuationCommandInput::from_continue_args(
            args,
        )?);
    }
    Ok(BranchCampaignDriverRequestV1::ContinueCampaign(
        RunCommandInput::from_continue_args(args)?,
    ))
}

fn continuation_request_from_input(
    input: ContinuationCommandInput,
) -> Result<BranchCampaignDriverRequestV1, String> {
    match continuation_command_from_input(&input) {
        BranchCampaignDriverCommandV1::PlanTargetedContinuation => Ok(
            BranchCampaignDriverRequestV1::PlanTargetedContinuation(input),
        ),
        BranchCampaignDriverCommandV1::ExecuteTargetedContinuation => Ok(
            BranchCampaignDriverRequestV1::ExecuteTargetedContinuation(input),
        ),
        BranchCampaignDriverCommandV1::ContinuationEffectReport => Ok(
            BranchCampaignDriverRequestV1::ContinuationEffectReport(input),
        ),
        _ => Err("ordinary campaign continue must be built from ContinueCommandArgs".to_string()),
    }
}

fn has_targeted_continuation_intent(args: &super::cli_args::ContinuationArgs) -> bool {
    args.plan_targeted_continuation.is_some()
        || args.execute_targeted_continuation.is_some()
        || args.continuation_effect_before.is_some()
        || args.continuation_effect_after.is_some()
}

fn inspect_request_from_inspect_args(
    args: super::cli_args::InspectCommandArgs,
) -> Result<BranchCampaignDriverRequestV1, String> {
    let command = inspect_command_from_inspect_args(&args);
    if matches!(
        command,
        BranchCampaignDriverCommandV1::InspectDecisionCoverage
    ) {
        return Ok(BranchCampaignDriverRequestV1::InspectDecisionCoverage(
            DatasetCommandInput::from_inspect_args_for_decision_coverage(args),
        ));
    }
    inspect_request_from_input(command, InspectCommandInput::from_inspect_args(args)?)
}

fn inspect_request_from_input(
    command: BranchCampaignDriverCommandV1,
    input: InspectCommandInput,
) -> Result<BranchCampaignDriverRequestV1, String> {
    Ok(match command {
        BranchCampaignDriverCommandV1::InspectJournal => {
            BranchCampaignDriverRequestV1::InspectJournal(input)
        }
        BranchCampaignDriverCommandV1::InspectLineageDecisions => {
            BranchCampaignDriverRequestV1::InspectLineageDecisions(input)
        }
        BranchCampaignDriverCommandV1::InspectCoverageGapMilestoneSummary => {
            BranchCampaignDriverRequestV1::InspectCoverageGapMilestoneSummary(input)
        }
        BranchCampaignDriverCommandV1::InspectCoverageGapTargetState => {
            BranchCampaignDriverRequestV1::InspectCoverageGapTargetState(input)
        }
        BranchCampaignDriverCommandV1::InspectDecisionObservations => {
            BranchCampaignDriverRequestV1::InspectDecisionObservations(input)
        }
        BranchCampaignDriverCommandV1::InspectCheckpoint => {
            BranchCampaignDriverRequestV1::InspectCheckpoint(input)
        }
        other => {
            return Err(format!(
                "non-inspect command reached inspect request builder: {other:?}"
            ));
        }
    })
}

fn legacy_driver_request_from_args(args: &Args) -> Result<BranchCampaignDriverRequestV1, String> {
    driver_request_for_command(legacy_command_from_args(args), args)
}

fn driver_request_for_command(
    command: BranchCampaignDriverCommandV1,
    args: &Args,
) -> Result<BranchCampaignDriverRequestV1, String> {
    Ok(match command {
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
        BranchCampaignDriverCommandV1::ContinueCampaign => {
            BranchCampaignDriverRequestV1::ContinueCampaign(RunCommandInput::from_args(args)?)
        }
        BranchCampaignDriverCommandV1::PlanCoverageGapContinuation => {
            BranchCampaignDriverRequestV1::PlanCoverageGapContinuation(
                CoverageGapPlanCommandInput::from_args(args)?,
            )
        }
        BranchCampaignDriverCommandV1::ExecuteCoverageGapContinuation => {
            BranchCampaignDriverRequestV1::ExecuteCoverageGapContinuation(
                CoverageGapExecutionCommandInput::from_args(args)?,
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
        BranchCampaignDriverCommandV1::InspectCheckpoint => {
            BranchCampaignDriverRequestV1::InspectCheckpoint(InspectCommandInput::from_args(args)?)
        }
        BranchCampaignDriverCommandV1::RunCampaign => {
            BranchCampaignDriverRequestV1::RunCampaign(RunCommandInput::from_args(args)?)
        }
    })
}

#[cfg(test)]
pub(super) fn driver_command_from_cli_input(
    input: &BranchCampaignCliInputV1,
) -> BranchCampaignDriverCommandV1 {
    if matches!(input, BranchCampaignCliInputV1::SelfCheck) {
        return BranchCampaignDriverCommandV1::SelfCheckAncestorReplay;
    }
    if matches!(input, BranchCampaignCliInputV1::CampaignRun(_)) {
        return BranchCampaignDriverCommandV1::RunCampaign;
    }
    if let BranchCampaignCliInputV1::CampaignContinue(args) = input {
        if args.continuation.execute_coverage_gap_continuation {
            return BranchCampaignDriverCommandV1::ExecuteCoverageGapContinuation;
        }
        if args.continuation.continuation_effect_before.is_some()
            || args.continuation.continuation_effect_after.is_some()
        {
            return BranchCampaignDriverCommandV1::ContinuationEffectReport;
        }
        if args.continuation.plan_targeted_continuation.is_some() {
            return BranchCampaignDriverCommandV1::PlanTargetedContinuation;
        }
        if args.continuation.execute_targeted_continuation.is_some() {
            return BranchCampaignDriverCommandV1::ExecuteTargetedContinuation;
        }
        return BranchCampaignDriverCommandV1::ContinueCampaign;
    }
    if matches!(input, BranchCampaignCliInputV1::CampaignCoveragePlan(_)) {
        return BranchCampaignDriverCommandV1::PlanCoverageGapContinuation;
    }
    if let BranchCampaignCliInputV1::CampaignDataset(args) = input {
        return dataset_command_from_dataset_args(args);
    }
    if let BranchCampaignCliInputV1::CampaignInspect(args) = input {
        return inspect_command_from_inspect_args(args);
    }
    legacy_command_from_args(input.args())
}

#[cfg(test)]
pub(super) fn driver_command_from_args(args: &Args) -> BranchCampaignDriverCommandV1 {
    legacy_command_from_args(args)
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

fn inspect_command_from_inspect_args(
    args: &super::cli_args::InspectCommandArgs,
) -> BranchCampaignDriverCommandV1 {
    if args.modes.inspect_journal {
        BranchCampaignDriverCommandV1::InspectJournal
    } else if args.modes.inspect_lineage_decisions {
        BranchCampaignDriverCommandV1::InspectLineageDecisions
    } else if args.modes.inspect_decision_coverage {
        BranchCampaignDriverCommandV1::InspectDecisionCoverage
    } else if args.modes.inspect_coverage_gap_target_state {
        BranchCampaignDriverCommandV1::InspectCoverageGapTargetState
    } else if args.modes.inspect_coverage_gap_milestone_summary {
        BranchCampaignDriverCommandV1::InspectCoverageGapMilestoneSummary
    } else if args.modes.inspect_decision_observations {
        BranchCampaignDriverCommandV1::InspectDecisionObservations
    } else {
        BranchCampaignDriverCommandV1::InspectCheckpoint
    }
}

#[cfg(test)]
fn dataset_command_from_dataset_args(args: &DatasetCommandArgs) -> BranchCampaignDriverCommandV1 {
    let paths = &args.paths;
    if paths.analyze_outcome_dataset.is_some() {
        BranchCampaignDriverCommandV1::AnalyzeOutcomeDataset
    } else if paths.analyze_decision_outcome_dataset.is_some() {
        BranchCampaignDriverCommandV1::AnalyzeDecisionOutcomeDataset
    } else if paths.probe_learning_readiness.is_some() {
        BranchCampaignDriverCommandV1::ProbeLearningReadiness
    } else if paths.export_outcome_dataset.is_some() {
        BranchCampaignDriverCommandV1::ExportOutcomeDataset
    } else if paths.export_learning_dataset.is_some() {
        BranchCampaignDriverCommandV1::ExportLearningDataset
    } else if paths.export_decision_outcome_dataset.is_some() {
        BranchCampaignDriverCommandV1::ExportDecisionOutcomeDataset
    } else {
        BranchCampaignDriverCommandV1::AnalyzeDecisionOutcomeDataset
    }
}

fn dataset_command_from_input(input: &DatasetCommandInput) -> BranchCampaignDriverCommandV1 {
    if input.analyze_outcome_dataset.is_some() {
        BranchCampaignDriverCommandV1::AnalyzeOutcomeDataset
    } else if input.analyze_decision_outcome_dataset.is_some() {
        BranchCampaignDriverCommandV1::AnalyzeDecisionOutcomeDataset
    } else if input.probe_learning_readiness.is_some() {
        BranchCampaignDriverCommandV1::ProbeLearningReadiness
    } else if input.export_outcome_dataset.is_some() {
        BranchCampaignDriverCommandV1::ExportOutcomeDataset
    } else if input.export_learning_dataset.is_some() {
        BranchCampaignDriverCommandV1::ExportLearningDataset
    } else if input.export_decision_outcome_dataset.is_some() {
        BranchCampaignDriverCommandV1::ExportDecisionOutcomeDataset
    } else {
        BranchCampaignDriverCommandV1::AnalyzeDecisionOutcomeDataset
    }
}

fn continuation_command_from_input(
    input: &ContinuationCommandInput,
) -> BranchCampaignDriverCommandV1 {
    if input.plan_targeted_continuation.is_some() {
        BranchCampaignDriverCommandV1::PlanTargetedContinuation
    } else if input.execute_targeted_continuation.is_some() {
        BranchCampaignDriverCommandV1::ExecuteTargetedContinuation
    } else if input.continuation_effect_before.is_some()
        || input.continuation_effect_after.is_some()
    {
        BranchCampaignDriverCommandV1::ContinuationEffectReport
    } else {
        BranchCampaignDriverCommandV1::ContinueCampaign
    }
}
