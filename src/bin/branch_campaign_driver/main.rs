#[cfg(test)]
use clap::error::ErrorKind;

mod campaign_artifacts;
mod campaign_run;
mod checkpoint_evidence;
mod checkpoint_inspection;
mod cli_args;
mod combat_lab;
mod command_inputs;
mod decision_observations;
mod driver_command;
mod final_boss_combat;
mod inspect_summary;
mod journal_inspection;
mod outcome_dataset;
mod shop_challenge;

use campaign_run::{run_ancestor_replay_self_check, run_campaign_command};
use checkpoint_inspection::{run_checkpoint_inspection, run_final_boss_combat_report_inspection};
#[cfg(test)]
use cli_args::{
    parse_args_from, Args, BranchCampaignExplicitCommandV1, BranchCampaignPresetV1,
    QUICK_PRESET_MAX_ACTIVE, QUICK_PRESET_MAX_ROUNDS, QUICK_PRESET_ROUND_DEPTH,
    QUICK_PRESET_SEARCH_MAX_NODES, QUICK_PRESET_SEARCH_WALL_MS,
};
use cli_args::{parse_cli, BranchCampaignCliInputV1};
#[cfg(test)]
use command_inputs::campaign_config_from_args;
use command_inputs::{
    ContinuationCommandInput, DatasetCommandInput, InspectCommandInput, RunCommandInput,
};
use decision_observations::run_decision_observation_inspection;
#[cfg(test)]
use driver_command::driver_command_from_args;
use driver_command::{driver_command_from_cli_input, BranchCampaignDriverCommandV1};
use journal_inspection::run_campaign_journal_inspection;
use outcome_dataset::{
    run_branch_outcome_dataset_analysis, run_branch_outcome_dataset_export,
    run_continuation_effect_report, run_decision_outcome_dataset_analysis,
    run_decision_outcome_dataset_export, run_learning_dataset_export, run_learning_readiness_probe,
    run_targeted_continuation_execution, run_targeted_continuation_plan,
};
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
            "latest.checkpoint.json",
        ])
        .expect("inspect args parse");
        let decision_observation_args = parse_args_from([
            "branch_campaign_driver",
            "inspect",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-decision-observations",
        ])
        .expect("decision observation inspect args parse");
        let journal_args = parse_args_from([
            "branch_campaign_driver",
            "inspect",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-journal",
        ])
        .expect("journal inspect args parse");
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
            driver_command_from_args(&dataset_args),
            BranchCampaignDriverCommandV1::AnalyzeDecisionOutcomeDataset
        );
        assert_eq!(
            driver_command_from_args(&continue_args),
            BranchCampaignDriverCommandV1::ExecuteTargetedContinuation
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
            "latest.checkpoint.json",
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
            "latest.checkpoint.json",
            "--preset",
            "quick",
        ])
        .expect("legacy flattened args remain accepted");

        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
        assert_eq!(args.preset, Some(BranchCampaignPresetV1::Quick));
        assert_eq!(args.explicit_command, None);
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
            "latest.checkpoint.json",
            "--inspect-report",
            "latest.campaign.json",
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
