#[cfg(test)]
use clap::error::ErrorKind;

mod campaign_artifact_store;
mod campaign_artifacts;
mod campaign_milestones;
mod campaign_run;
mod checkpoint_evidence;
mod checkpoint_inspection;
mod checkpoint_shop_evidence;
mod cli_args;
mod combat_lab;
mod command_inputs;
mod coverage_gap_milestone_summary;
mod decision_observations;
mod driver_command;
mod final_boss_combat;
mod inspect_summary;
mod journal_inspection;
mod outcome_dataset;
mod shop_challenge;

use campaign_artifact_store::{
    render_campaign_artifact_manifest_ref_v1, render_campaign_artifact_ref_v1,
    write_campaign_artifact_manifest_from_payload_text_v1, CampaignArtifactKindV1,
    CampaignArtifactStoreV1,
};
use campaign_run::{run_ancestor_replay_self_check, run_campaign_command};
use checkpoint_inspection::{run_checkpoint_inspection, run_final_boss_combat_report_inspection};
#[cfg(test)]
use cli_args::{
    parse_args_from, Args, BranchCampaignExplicitCommandV1, BranchCampaignPresetV1,
    QUICK_PRESET_MAX_ACTIVE, QUICK_PRESET_MAX_ROUNDS, QUICK_PRESET_ROUND_DEPTH,
    QUICK_PRESET_SEARCH_MAX_NODES, QUICK_PRESET_SEARCH_WALL_MS,
};
use cli_args::{parse_cli, ArtifactKindArgV1, BranchCampaignCliInputV1};
#[cfg(test)]
use command_inputs::{
    campaign_config_from_args, round_budget_for_source_from_args, RoundBudgetModeV1,
};
use command_inputs::{
    ArtifactCommandInput, ContinuationCommandInput, DatasetCommandInput, InspectCommandInput,
    RunCommandInput,
};
use coverage_gap_milestone_summary::{
    resolve_coverage_gap_target_group_checkpoint_index_from_input_v1,
    run_coverage_gap_milestone_summary_inspection,
};
use decision_observations::run_decision_observation_inspection;
#[cfg(test)]
use driver_command::driver_command_from_args;
use driver_command::{driver_command_from_cli_input, BranchCampaignDriverCommandV1};
use journal_inspection::{
    run_campaign_journal_inspection, run_campaign_lineage_decision_inspection,
};
use outcome_dataset::{
    run_branch_outcome_dataset_analysis, run_branch_outcome_dataset_export,
    run_continuation_effect_report, run_coverage_gap_continuation_execution,
    run_coverage_gap_continuation_plan, run_decision_candidate_coverage_inspection,
    run_decision_outcome_dataset_analysis, run_decision_outcome_dataset_export,
    run_learning_dataset_export, run_learning_readiness_probe, run_targeted_continuation_execution,
    run_targeted_continuation_plan,
};
use std::io::Read;
#[cfg(test)]
use sts_simulator::eval::run_control::RunControlCombatSegmentMode;

fn main() {
    let cli_input = parse_cli();
    if let Err(err) = run(cli_input) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run(cli_input: BranchCampaignCliInputV1) -> Result<(), String> {
    let args = cli_input.args();
    match driver_command_from_cli_input(&cli_input) {
        BranchCampaignDriverCommandV1::SelfCheckAncestorReplay => run_ancestor_replay_self_check(),
        BranchCampaignDriverCommandV1::AnalyzeOutcomeDataset => {
            run_branch_outcome_dataset_analysis(&DatasetCommandInput::from_args(args))
        }
        BranchCampaignDriverCommandV1::AnalyzeDecisionOutcomeDataset => {
            run_decision_outcome_dataset_analysis(&DatasetCommandInput::from_args(args))
        }
        BranchCampaignDriverCommandV1::ProbeLearningReadiness => {
            run_learning_readiness_probe(&DatasetCommandInput::from_args(args))
        }
        BranchCampaignDriverCommandV1::PlanTargetedContinuation => {
            run_targeted_continuation_plan(&ContinuationCommandInput::from_args(args)?)
        }
        BranchCampaignDriverCommandV1::ExecuteTargetedContinuation => {
            run_targeted_continuation_execution(&ContinuationCommandInput::from_args(args)?)
        }
        BranchCampaignDriverCommandV1::ResolveCampaignArtifact => {
            run_artifact_command(ArtifactCommandInput::from_args(args)?)
        }
        BranchCampaignDriverCommandV1::PlanCoverageGapContinuation => {
            run_coverage_gap_continuation_plan(&DatasetCommandInput::from_args(args))
        }
        BranchCampaignDriverCommandV1::ExecuteCoverageGapContinuation => {
            run_coverage_gap_continuation_execution(&ContinuationCommandInput::from_args(args)?)
        }
        BranchCampaignDriverCommandV1::ContinuationEffectReport => {
            run_continuation_effect_report(&ContinuationCommandInput::from_args(args)?)
        }
        BranchCampaignDriverCommandV1::ExportOutcomeDataset => {
            run_branch_outcome_dataset_export(&DatasetCommandInput::from_args(args))
        }
        BranchCampaignDriverCommandV1::ExportLearningDataset => {
            run_learning_dataset_export(&DatasetCommandInput::from_args(args))
        }
        BranchCampaignDriverCommandV1::ExportDecisionOutcomeDataset => {
            run_decision_outcome_dataset_export(&DatasetCommandInput::from_args(args))
        }
        BranchCampaignDriverCommandV1::InspectFinalBossCombat => {
            run_final_boss_combat_report_inspection(&InspectCommandInput::from_args(args)?)
        }
        BranchCampaignDriverCommandV1::InspectJournal => {
            run_campaign_journal_inspection(&InspectCommandInput::from_args(args)?)
        }
        BranchCampaignDriverCommandV1::InspectLineageDecisions => {
            run_campaign_lineage_decision_inspection(&InspectCommandInput::from_args(args)?)
        }
        BranchCampaignDriverCommandV1::InspectDecisionCoverage => {
            run_decision_candidate_coverage_inspection(&DatasetCommandInput::from_args(args))
        }
        BranchCampaignDriverCommandV1::InspectCoverageGapMilestoneSummary => {
            run_coverage_gap_milestone_summary_inspection(&InspectCommandInput::from_args(args)?)
        }
        BranchCampaignDriverCommandV1::InspectCoverageGapTargetState => {
            let mut input = InspectCommandInput::from_args(args)?;
            let checkpoint_index =
                resolve_coverage_gap_target_group_checkpoint_index_from_input_v1(&input)?;
            input.filters.index = Some(checkpoint_index);
            input.summary = false;
            run_checkpoint_inspection(&input)
        }
        BranchCampaignDriverCommandV1::InspectDecisionObservations => {
            run_decision_observation_inspection(&InspectCommandInput::from_args(args)?)
        }
        BranchCampaignDriverCommandV1::InspectCheckpoint => {
            run_checkpoint_inspection(&InspectCommandInput::from_args(args)?)
        }
        BranchCampaignDriverCommandV1::RunCampaign => {
            run_campaign_command(&RunCommandInput::from_args(args)?)
        }
    }
}

fn run_artifact_command(input: ArtifactCommandInput) -> Result<(), String> {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign_artifacts::read_campaign_checkpoint_v1;

    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn campaign_cli_default_run_config_smoke() {
        let args = Args::try_parse_from(["branch_campaign_driver"]).expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.max_reward_options_per_branch, Some(2));
        assert_eq!(
            config.search_options.segment_mode,
            Some(RunControlCombatSegmentMode::NonBossTurnBoundary)
        );
        assert_eq!(config.max_active, 8);
    }

    #[test]
    fn explicit_subcommands_select_primary_modes() {
        let run_args = parse_args_from(["branch_campaign_driver", "run", "--preset", "focused"])
            .expect("run args parse");
        let inspect_args = parse_args_from([
            "branch_campaign_driver",
            "inspect",
            "--inspect-checkpoint",
            "artifact.checkpoint.json",
        ])
        .expect("inspect args parse");
        let decision_observation_args = parse_args_from([
            "branch_campaign_driver",
            "inspect",
            "--inspect-report",
            "artifact.campaign.json",
            "--inspect-decision-observations",
        ])
        .expect("decision observation inspect args parse");
        let journal_args = parse_args_from([
            "branch_campaign_driver",
            "inspect",
            "--inspect-report",
            "artifact.campaign.json",
            "--inspect-journal",
        ])
        .expect("journal inspect args parse");
        let lineage_decision_args = parse_args_from([
            "branch_campaign_driver",
            "inspect",
            "--inspect-report",
            "artifact.campaign.json",
            "--inspect-lineage-decisions",
        ])
        .expect("lineage decision inspect args parse");
        let coverage_gap_milestone_args = parse_args_from([
            "branch_campaign_driver",
            "inspect",
            "--inspect-report",
            "artifact.campaign.json",
            "--inspect-coverage-gap-milestone-summary",
            "--coverage-gap-milestone-target",
            "Act2Start",
            "--coverage-gap-bucket",
            "route",
            "--coverage-gap-origin-source",
            "map_decision_packet",
            "--coverage-gap-progress",
            "missing",
        ])
        .expect("coverage gap milestone summary inspect args parse");
        let coverage_gap_target_state_args = parse_args_from([
            "branch_campaign_driver",
            "inspect",
            "--inspect-checkpoint",
            "artifact.checkpoint.json",
            "--inspect-report",
            "artifact.campaign.json",
            "--inspect-coverage-gap-target-state",
            "--coverage-gap-milestone-target",
            "Act2Start",
            "--inspect-index",
            "1",
        ])
        .expect("coverage gap target state inspect args parse");
        let dataset_args = parse_args_from([
            "branch_campaign_driver",
            "dataset",
            "--analyze-decision-outcome-dataset",
            "decision_outcomes.jsonl",
        ])
        .expect("dataset args parse");
        let continue_args = parse_args_from([
            "branch_campaign_driver",
            "continue",
            "--execute-targeted-continuation",
            "decision_outcomes.jsonl",
        ])
        .expect("continue args parse");
        let coverage_gap_continue_args = parse_args_from([
            "branch_campaign_driver",
            "continue",
            "--execute-coverage-gap-continuation",
        ])
        .expect("coverage gap continuation args parse");
        let self_check_args =
            parse_args_from(["branch_campaign_driver", "self-check"]).expect("self-check parse");

        assert_eq!(
            driver_command_from_args(&run_args),
            BranchCampaignDriverCommandV1::RunCampaign
        );
        assert_eq!(
            driver_command_from_args(&inspect_args),
            BranchCampaignDriverCommandV1::InspectCheckpoint
        );
        assert_eq!(
            driver_command_from_args(&decision_observation_args),
            BranchCampaignDriverCommandV1::InspectDecisionObservations
        );
        assert_eq!(
            driver_command_from_args(&journal_args),
            BranchCampaignDriverCommandV1::InspectJournal
        );
        assert_eq!(
            driver_command_from_args(&lineage_decision_args),
            BranchCampaignDriverCommandV1::InspectLineageDecisions
        );
        assert_eq!(
            driver_command_from_args(&coverage_gap_milestone_args),
            BranchCampaignDriverCommandV1::InspectCoverageGapMilestoneSummary
        );
        assert_eq!(
            driver_command_from_args(&coverage_gap_target_state_args),
            BranchCampaignDriverCommandV1::InspectCoverageGapTargetState
        );
        assert_eq!(
            driver_command_from_args(&dataset_args),
            BranchCampaignDriverCommandV1::AnalyzeDecisionOutcomeDataset
        );
        assert_eq!(
            driver_command_from_args(&continue_args),
            BranchCampaignDriverCommandV1::ExecuteTargetedContinuation
        );
        assert_eq!(
            driver_command_from_args(&coverage_gap_continue_args),
            BranchCampaignDriverCommandV1::ExecuteCoverageGapContinuation
        );
        assert_eq!(
            driver_command_from_args(&self_check_args),
            BranchCampaignDriverCommandV1::SelfCheckAncestorReplay
        );
    }

    #[test]
    fn typed_cli_input_drives_subcommand_dispatch() {
        let input = cli_args::parse_cli_from([
            "branch_campaign_driver",
            "dataset",
            "--analyze-decision-outcome-dataset",
            "decision_outcomes.jsonl",
        ])
        .expect("typed cli input parse");

        assert_eq!(
            input.explicit_command(),
            Some(BranchCampaignExplicitCommandV1::Dataset)
        );
        assert_eq!(
            driver_command_from_cli_input(&input),
            BranchCampaignDriverCommandV1::AnalyzeDecisionOutcomeDataset
        );
    }

    #[test]
    fn driver_subcommands_reject_unrelated_mode_flags() {
        let dataset_err =
            parse_args_from(["branch_campaign_driver", "dataset", "--max-rounds", "2"])
                .expect_err("dataset should not expose campaign run budget flags");
        let run_err = parse_args_from([
            "branch_campaign_driver",
            "run",
            "--inspect-checkpoint",
            "artifact.checkpoint.json",
        ])
        .expect_err("run should not expose inspection flags");
        let inspect_err =
            parse_args_from(["branch_campaign_driver", "inspect", "--preset", "quick"])
                .expect_err("inspect should not expose campaign preset flags");

        assert_eq!(dataset_err.kind(), ErrorKind::UnknownArgument);
        assert_eq!(run_err.kind(), ErrorKind::UnknownArgument);
        assert_eq!(inspect_err.kind(), ErrorKind::UnknownArgument);
    }

    #[test]
    fn legacy_top_level_flags_remain_compatible() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "artifact.checkpoint.json",
            "--preset",
            "quick",
        ])
        .expect("legacy flattened args remain accepted");

        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("artifact.checkpoint.json"))
        );
        assert_eq!(args.preset, Some(BranchCampaignPresetV1::Quick));
        assert_eq!(args.explicit_command, None);
    }

    #[test]
    fn coverage_gap_plan_accepts_resume_checkpoint_preview_source() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--inspect-report",
            "artifact.campaign.json",
            "--resume-checkpoint",
            "artifact.checkpoint.json",
            "--plan-coverage-gap-continuation",
        ])
        .expect("coverage gap plan args parse");

        assert_eq!(
            driver_command_from_args(&args),
            BranchCampaignDriverCommandV1::PlanCoverageGapContinuation
        );
        let input = DatasetCommandInput::from_args(&args);
        assert_eq!(
            input.resume_checkpoint,
            Some(PathBuf::from("artifact.checkpoint.json"))
        );
    }

    #[test]
    fn run_subcommand_applies_quick_preset_smoke() {
        let args = parse_args_from(["branch_campaign_driver", "run", "--preset", "quick"])
            .expect("run args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(
            args.explicit_command,
            Some(BranchCampaignExplicitCommandV1::Run)
        );
        assert_eq!(config.max_rounds, QUICK_PRESET_MAX_ROUNDS);
        assert_eq!(config.round_depth, QUICK_PRESET_ROUND_DEPTH);
        assert_eq!(config.max_active, QUICK_PRESET_MAX_ACTIVE);
        assert_eq!(config.search_wall_ms, Some(QUICK_PRESET_SEARCH_WALL_MS));
        assert_eq!(config.search_max_nodes, Some(QUICK_PRESET_SEARCH_MAX_NODES));
    }

    #[test]
    fn rounds_alias_sets_per_invocation_round_budget() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "run",
            "--preset",
            "quick",
            "--rounds",
            "3",
        ])
        .expect("run args parse");
        let config = campaign_config_from_args(&args).expect("config builds");
        let budget = round_budget_for_source_from_args(&args, 7).expect("budget resolves");

        assert_eq!(config.max_rounds, 3);
        assert_eq!(budget.mode, RoundBudgetModeV1::Rounds);
        assert_eq!(budget.source_rounds, 7);
        assert_eq!(budget.round_budget, 3);
        assert_eq!(budget.target_total_rounds, 10);
    }

    #[test]
    fn until_round_sets_remaining_round_budget_from_source_rounds() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "continue",
            "--execute-coverage-gap-continuation",
            "--until-round",
            "9",
        ])
        .expect("continuation args parse");
        let budget = round_budget_for_source_from_args(&args, 6).expect("budget resolves");

        assert_eq!(budget.mode, RoundBudgetModeV1::UntilRound);
        assert_eq!(budget.source_rounds, 6);
        assert_eq!(budget.round_budget, 3);
        assert_eq!(budget.target_total_rounds, 9);
    }

    #[test]
    fn campaign_search_options_can_override_segment_mode() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--combat-search-option",
            "segment=off",
        ])
        .expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.search_options.segment_mode, None);
    }

    #[test]
    fn inspect_search_options_are_forwarded_to_inspect_input() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "artifact.checkpoint.json",
            "--inspect-report",
            "artifact.campaign.json",
            "--inspect-search",
            "--combat-search-option",
            "wall_ms=5000",
        ])
        .expect("args parse");

        let input = InspectCommandInput::from_args(&args).expect("inspect input builds");

        assert_eq!(input.search_options.wall_ms, Some(5_000));
    }

    #[test]
    fn ascension_domain_rejects_conflicting_explicit_ascension() {
        let err = parse_args_from([
            "branch_campaign_driver",
            "--ascension-domain",
            "a20",
            "--ascension",
            "10",
        ])
        .expect_err("conflicting ascension should fail");

        assert_eq!(err.kind(), ErrorKind::ValueValidation);
    }

    #[test]
    fn campaign_checkpoint_reader_rejects_v1_schema() {
        let path = std::env::temp_dir().join(format!(
            "old-branch-campaign-checkpoint-{}.json",
            std::process::id()
        ));
        fs::write(
            &path,
            r#"{
  "schema_name": "BranchCampaignCheckpointV1",
  "schema_version": 1,
  "seed": 1,
  "rounds_completed": 0,
  "sessions": []
}"#,
        )
        .expect("write old checkpoint fixture");

        let err =
            read_campaign_checkpoint_v1(&path).expect_err("old checkpoint should be rejected");
        let _ = fs::remove_file(&path);

        assert!(err.contains("BranchCampaignCheckpointV2"));
    }
}
