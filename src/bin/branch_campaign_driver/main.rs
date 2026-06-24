#[cfg(test)]
use clap::error::ErrorKind;

mod campaign_app;
mod campaign_artifact_source_info;
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

use campaign_app::CampaignAppV1;
use cli_args::parse_cli;
#[cfg(test)]
use cli_args::{
    parse_args_from, Args, BranchCampaignPresetV1, QUICK_PRESET_MAX_ACTIVE,
    QUICK_PRESET_MAX_ROUNDS, QUICK_PRESET_ROUND_DEPTH, QUICK_PRESET_SEARCH_MAX_NODES,
    QUICK_PRESET_SEARCH_WALL_MS,
};
#[cfg(test)]
use command_inputs::{
    campaign_config_from_args, round_budget_for_source_from_args, CoverageGapPlanCommandInput,
    InspectCommandInput, RoundBudgetModeV1,
};
#[cfg(test)]
use driver_command::{
    driver_command_from_args, driver_command_from_cli_input, BranchCampaignDriverCommandV1,
};
#[cfg(test)]
use sts_simulator::eval::run_control::RunControlCombatSegmentMode;

fn main() {
    let cli_input = parse_cli();
    if let Err(err) = CampaignAppV1::new().run_cli_input(cli_input) {
        eprintln!("error: {err}");
        std::process::exit(1);
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
        let input = CoverageGapPlanCommandInput::from_args(&args).expect("input builds");
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
