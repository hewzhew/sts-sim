use std::io::Read;

use super::campaign_artifact_source_info::{
    campaign_artifact_source_info_v1, render_campaign_artifact_source_info_v1,
};
use super::campaign_artifact_store::{
    render_campaign_artifact_manifest_ref_v1, render_campaign_artifact_prune_report_v1,
    render_campaign_artifact_ref_v1, write_campaign_artifact_manifest_from_payload_text_v1,
    CampaignArtifactKindV1, CampaignArtifactStoreV1,
};
use super::campaign_run::{
    run_ancestor_replay_self_check, run_campaign_command, run_continue_campaign_command,
};
use super::checkpoint_inspection::{
    run_checkpoint_inspection, run_final_boss_combat_report_inspection,
};
use super::cli_args::{ArtifactKindArgV1, BranchCampaignCliInputV1};
use super::command_inputs::ArtifactCommandInput;
use super::coverage_gap_milestone_summary::{
    resolve_coverage_gap_target_group_checkpoint_index_from_input_v1,
    run_coverage_gap_milestone_summary_inspection,
};
use super::decision_observations::run_decision_observation_inspection;
use super::driver_command::{driver_request_from_cli_input, BranchCampaignDriverRequestV1};
use super::journal_inspection::{
    run_campaign_journal_inspection, run_campaign_lineage_decision_inspection,
};
use super::outcome_dataset::{
    run_branch_outcome_dataset_analysis, run_branch_outcome_dataset_export,
    run_continuation_effect_report, run_coverage_gap_continuation_execution,
    run_coverage_gap_continuation_plan, run_decision_candidate_coverage_inspection,
    run_decision_outcome_dataset_analysis, run_decision_outcome_dataset_export,
    run_learning_dataset_export, run_learning_readiness_probe, run_targeted_continuation_execution,
    run_targeted_continuation_plan,
};

pub(super) struct CampaignAppV1;

impl CampaignAppV1 {
    pub(super) fn new() -> Self {
        Self
    }

    pub(super) fn run_cli_input(&self, cli_input: BranchCampaignCliInputV1) -> Result<(), String> {
        self.run_request(driver_request_from_cli_input(cli_input)?)
    }

    pub(super) fn run_request(&self, request: BranchCampaignDriverRequestV1) -> Result<(), String> {
        match request {
            BranchCampaignDriverRequestV1::SelfCheckAncestorReplay => {
                run_ancestor_replay_self_check()
            }
            BranchCampaignDriverRequestV1::AnalyzeOutcomeDataset(input) => {
                run_branch_outcome_dataset_analysis(&input)
            }
            BranchCampaignDriverRequestV1::AnalyzeDecisionOutcomeDataset(input) => {
                run_decision_outcome_dataset_analysis(&input)
            }
            BranchCampaignDriverRequestV1::ProbeLearningReadiness(input) => {
                run_learning_readiness_probe(&input)
            }
            BranchCampaignDriverRequestV1::PlanTargetedContinuation(input) => {
                run_targeted_continuation_plan(&input)
            }
            BranchCampaignDriverRequestV1::ExecuteTargetedContinuation(input) => {
                run_targeted_continuation_execution(&input)
            }
            BranchCampaignDriverRequestV1::ContinueCampaign(input) => {
                run_continue_campaign_command(&input)
            }
            BranchCampaignDriverRequestV1::ResolveCampaignArtifact(input) => {
                self.run_artifact_command(input)
            }
            BranchCampaignDriverRequestV1::PlanCoverageGapContinuation(input) => {
                run_coverage_gap_continuation_plan(&input)
            }
            BranchCampaignDriverRequestV1::ExecuteCoverageGapContinuation(input) => {
                run_coverage_gap_continuation_execution(&input)
            }
            BranchCampaignDriverRequestV1::ContinuationEffectReport(input) => {
                run_continuation_effect_report(&input)
            }
            BranchCampaignDriverRequestV1::ExportOutcomeDataset(input) => {
                run_branch_outcome_dataset_export(&input)
            }
            BranchCampaignDriverRequestV1::ExportLearningDataset(input) => {
                run_learning_dataset_export(&input)
            }
            BranchCampaignDriverRequestV1::ExportDecisionOutcomeDataset(input) => {
                run_decision_outcome_dataset_export(&input)
            }
            BranchCampaignDriverRequestV1::InspectFinalBossCombat(input) => {
                run_final_boss_combat_report_inspection(&input)
            }
            BranchCampaignDriverRequestV1::InspectJournal(input) => {
                run_campaign_journal_inspection(&input)
            }
            BranchCampaignDriverRequestV1::InspectLineageDecisions(input) => {
                run_campaign_lineage_decision_inspection(&input)
            }
            BranchCampaignDriverRequestV1::InspectDecisionCoverage(input) => {
                run_decision_candidate_coverage_inspection(&input)
            }
            BranchCampaignDriverRequestV1::InspectCoverageGapMilestoneSummary(input) => {
                run_coverage_gap_milestone_summary_inspection(&input)
            }
            BranchCampaignDriverRequestV1::InspectCoverageGapTargetState(mut input) => {
                let checkpoint_index =
                    resolve_coverage_gap_target_group_checkpoint_index_from_input_v1(&input)?;
                input.filters.index = Some(checkpoint_index);
                input.summary = false;
                run_checkpoint_inspection(&input)
            }
            BranchCampaignDriverRequestV1::InspectDecisionObservations(input) => {
                run_decision_observation_inspection(&input)
            }
            BranchCampaignDriverRequestV1::InspectCheckpoint(input) => {
                run_checkpoint_inspection(&input)
            }
            BranchCampaignDriverRequestV1::RunCampaign(input) => run_campaign_command(&input),
        }
    }

    fn run_artifact_command(&self, input: ArtifactCommandInput) -> Result<(), String> {
        match input {
            ArtifactCommandInput::Resolve {
                campaign_dir,
                selector,
                json,
            } => {
                let store = CampaignArtifactStoreV1::new(campaign_dir);
                let artifact = store.resolve_source_selector_v1(&selector)?;
                println!("{}", render_campaign_artifact_ref_v1(&artifact, json)?);
                Ok(())
            }
            ArtifactCommandInput::SourceInfo {
                campaign_dir,
                selector,
                json,
            } => {
                let store = CampaignArtifactStoreV1::new(campaign_dir);
                let info = campaign_artifact_source_info_v1(&store, &selector)?;
                println!("{}", render_campaign_artifact_source_info_v1(&info, json)?);
                Ok(())
            }
            ArtifactCommandInput::Allocate {
                campaign_dir,
                kind,
                label,
                stamp,
                suffix,
                json,
            } => {
                let store = CampaignArtifactStoreV1::new(campaign_dir);
                let artifact_kind = match kind {
                    ArtifactKindArgV1::Run => CampaignArtifactKindV1::Run,
                    ArtifactKindArgV1::Scratch => CampaignArtifactKindV1::Scratch,
                };
                let artifact = store.allocate_output_ref_v1(
                    artifact_kind,
                    &label,
                    stamp.as_deref(),
                    suffix.as_deref(),
                )?;
                println!("{}", render_campaign_artifact_ref_v1(&artifact, json)?);
                Ok(())
            }
            ArtifactCommandInput::WriteLatest {
                campaign_dir,
                kind,
                artifact_id,
                updated_at,
                json,
            } => {
                let store = CampaignArtifactStoreV1::new(campaign_dir);
                let artifact = match kind {
                    ArtifactKindArgV1::Run => {
                        let artifact = store.run_artifact_ref_v1(&artifact_id);
                        store.write_latest_pointer_v1(&artifact, &updated_at)?;
                        artifact
                    }
                    ArtifactKindArgV1::Scratch => {
                        let artifact = store.scratch_artifact_ref_v1(&artifact_id);
                        store.write_scratch_latest_pointer_v1(&artifact, &updated_at)?;
                        artifact
                    }
                };
                println!("{}", render_campaign_artifact_ref_v1(&artifact, json)?);
                Ok(())
            }
            ArtifactCommandInput::WriteManifest {
                manifest_path,
                payload_schema_name,
                created_at,
                json,
            } => {
                let mut payload_text = String::new();
                std::io::stdin()
                    .read_to_string(&mut payload_text)
                    .map_err(|err| format!("failed to read manifest payload from stdin: {err}"))?;
                let manifest_ref = write_campaign_artifact_manifest_from_payload_text_v1(
                    &manifest_path,
                    &payload_schema_name,
                    &created_at,
                    &payload_text,
                )?;
                println!(
                    "{}",
                    render_campaign_artifact_manifest_ref_v1(&manifest_ref, json)?
                );
                Ok(())
            }
            ArtifactCommandInput::Prune {
                campaign_dir,
                keep_runs,
                keep_scratch,
                apply,
                json,
            } => {
                let store = CampaignArtifactStoreV1::new(campaign_dir);
                let report = store.prune_campaign_artifacts_v1(keep_runs, keep_scratch, apply)?;
                println!(
                    "{}",
                    render_campaign_artifact_prune_report_v1(&report, json)?
                );
                Ok(())
            }
        }
    }
}
