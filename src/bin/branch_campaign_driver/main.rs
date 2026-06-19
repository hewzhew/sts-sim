#[cfg(test)]
use clap::error::ErrorKind;
use std::fs;
use std::path::PathBuf;

mod campaign_run;
mod checkpoint_evidence;
mod checkpoint_inspection;
mod cli_args;
mod combat_lab;
mod command_inputs;
mod driver_command;
mod final_boss_combat;
mod inspect_summary;
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
#[cfg(test)]
use driver_command::driver_command_from_args;
use driver_command::{driver_command_from_cli_input, BranchCampaignDriverCommandV1};
use outcome_dataset::{
    run_branch_outcome_dataset_analysis, run_branch_outcome_dataset_export,
    run_continuation_effect_report, run_decision_outcome_dataset_analysis,
    run_decision_outcome_dataset_export, run_learning_dataset_export, run_learning_readiness_probe,
    run_targeted_continuation_execution, run_targeted_continuation_plan,
};
#[cfg(test)]
use sts_simulator::eval::branch_campaign::BranchCampaignCombatRetryPolicyV1;
use sts_simulator::eval::branch_campaign::{
    BranchCampaignCheckpointV1, BranchCampaignReportV1, BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME,
    BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
};
#[cfg(test)]
use sts_simulator::eval::run_control::RunControlCombatSegmentMode;
#[cfg(test)]
use sts_simulator::eval::run_control::RunControlHpLossLimit;

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
        BranchCampaignDriverCommandV1::InspectCheckpoint => {
            run_checkpoint_inspection(&InspectCommandInput::from_args(args)?)
        }
        BranchCampaignDriverCommandV1::RunCampaign => {
            run_campaign_command(&RunCommandInput::from_args(args)?)
        }
    }
}

fn read_campaign_report_v1(path: &PathBuf) -> Result<BranchCampaignReportV1, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("failed to read --resume {}: {err}", path.display()))?;
    serde_json::from_str(&text).map_err(|err| {
        format!(
            "failed to parse --resume {} as BranchCampaignV1: {err}",
            path.display()
        )
    })
}

fn read_campaign_checkpoint_v1(path: &PathBuf) -> Result<BranchCampaignCheckpointV1, String> {
    let text = fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read --resume-checkpoint {}: {err}",
            path.display()
        )
    })?;
    let checkpoint: BranchCampaignCheckpointV1 = serde_json::from_str(&text).map_err(|err| {
        format!(
            "failed to parse --resume-checkpoint {} as {BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME}: {err}",
            path.display()
        )
    })?;
    if checkpoint.schema_name != BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME
        || checkpoint.schema_version != BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION
    {
        return Err(format!(
            "checkpoint {} uses {} v{}; expected {} v{}. Rerun campaign to create a fresh checkpoint.",
            path.display(),
            checkpoint.schema_name,
            checkpoint.schema_version,
            BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME,
            BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION
        ));
    }
    Ok(checkpoint)
}

fn write_campaign_report_v1(path: &PathBuf, report: &BranchCampaignReportV1) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create --out directory {}: {err}",
                parent.display()
            )
        })?;
    }
    let text = serde_json::to_string_pretty(report)
        .map_err(|err| format!("failed to serialize BranchCampaignV1 report: {err}"))?;
    fs::write(path, text).map_err(|err| format!("failed to write --out {}: {err}", path.display()))
}

fn write_campaign_checkpoint_v1(
    path: &PathBuf,
    checkpoint: &BranchCampaignCheckpointV1,
) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create --checkpoint-out directory {}: {err}",
                parent.display()
            )
        })?;
    }
    let text = serde_json::to_string_pretty(checkpoint)
        .map_err(|err| format!("failed to serialize BranchCampaignCheckpointV2: {err}"))?;
    fs::write(path, text)
        .map_err(|err| format!("failed to write --checkpoint-out {}: {err}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campaign_cli_defaults_to_bounded_reward_branching() {
        let args = Args::try_parse_from(["branch_campaign_driver"]).expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.max_reward_options_per_branch, Some(2));
        assert_eq!(
            config.search_options.segment_mode,
            Some(RunControlCombatSegmentMode::NonBossTurnBoundary)
        );
        assert_eq!(config.max_active, 8);
        assert_eq!(config.max_frozen, 32);
        assert_eq!(config.round_depth, 1);
        assert_eq!(config.active_lineage_diversity_slots, 0);
    }

    #[test]
    fn campaign_cli_can_disable_segment_combat_fallback() {
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
    fn campaign_cli_accepts_resume_and_out_paths() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--resume",
            "old.campaign.json",
            "--resume-checkpoint",
            "old.checkpoint.json",
            "--out",
            "new.campaign.json",
            "--checkpoint-out",
            "new.checkpoint.json",
        ])
        .expect("args parse");

        assert_eq!(args.resume, Some(PathBuf::from("old.campaign.json")));
        assert_eq!(
            args.resume_checkpoint,
            Some(PathBuf::from("old.checkpoint.json"))
        );
        assert_eq!(args.out, Some(PathBuf::from("new.campaign.json")));
        assert_eq!(
            args.checkpoint_out,
            Some(PathBuf::from("new.checkpoint.json"))
        );
    }

    #[test]
    fn campaign_cli_accepts_ancestor_replay_self_check() {
        let args = parse_args_from(["branch_campaign_driver", "--self-check-ancestor-replay"])
            .expect("args parse");

        assert!(args.self_check_ancestor_replay);
    }

    #[test]
    fn driver_command_defaults_to_campaign_run() {
        let args = Args::try_parse_from(["branch_campaign_driver"]).expect("args parse");

        assert_eq!(
            driver_command_from_args(&args),
            BranchCampaignDriverCommandV1::RunCampaign
        );
    }

    #[test]
    fn driver_command_classifies_checkpoint_inspection() {
        let args = Args::try_parse_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
        ])
        .expect("args parse");

        assert_eq!(
            driver_command_from_args(&args),
            BranchCampaignDriverCommandV1::InspectCheckpoint
        );
    }

    #[test]
    fn driver_command_classifies_learning_dataset_modes() {
        let analyze_args = Args::try_parse_from([
            "branch_campaign_driver",
            "--analyze-decision-outcome-dataset",
            "decision_outcomes.jsonl",
        ])
        .expect("args parse");
        let continue_args = Args::try_parse_from([
            "branch_campaign_driver",
            "--execute-targeted-continuation",
            "decision_outcomes.jsonl",
        ])
        .expect("args parse");

        assert_eq!(
            driver_command_from_args(&analyze_args),
            BranchCampaignDriverCommandV1::AnalyzeDecisionOutcomeDataset
        );
        assert_eq!(
            driver_command_from_args(&continue_args),
            BranchCampaignDriverCommandV1::ExecuteTargetedContinuation
        );
    }

    #[test]
    fn driver_command_keeps_legacy_self_check_precedence() {
        let args = Args::try_parse_from([
            "branch_campaign_driver",
            "--self-check-ancestor-replay",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
        ])
        .expect("args parse");

        assert_eq!(
            driver_command_from_args(&args),
            BranchCampaignDriverCommandV1::SelfCheckAncestorReplay
        );
    }

    #[test]
    fn driver_subcommands_classify_primary_modes() {
        let run_args = parse_args_from([
            "branch_campaign_driver",
            "run",
            "--preset",
            "focused",
            "--seed",
            "521",
        ])
        .expect("run args parse");
        let inspect_args = parse_args_from([
            "branch_campaign_driver",
            "inspect",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
        ])
        .expect("inspect args parse");
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
        assert_eq!(
            input.args().analyze_decision_outcome_dataset,
            Some(PathBuf::from("decision_outcomes.jsonl"))
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
    fn run_subcommand_applies_preset_defaults_after_conversion() {
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

    #[test]
    fn campaign_cli_accepts_checkpoint_summary_inspection_paths() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-summary",
            "--branch-examples",
            "2",
        ])
        .expect("args parse");

        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
        assert_eq!(
            args.inspect_report,
            Some(PathBuf::from("latest.campaign.json"))
        );
        assert!(args.inspect_summary);
        assert_eq!(args.branch_examples, 2);
        assert_eq!(args.inspect_index, None);
    }

    #[test]
    fn campaign_cli_accepts_optional_checkpoint_inspect_index() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-summary",
            "--inspect-index",
            "18",
        ])
        .expect("args parse");

        assert_eq!(args.inspect_index, Some(18));
    }

    #[test]
    fn campaign_cli_accepts_checkpoint_shop_evidence_inspection() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-act",
            "2",
            "--inspect-floor",
            "18",
            "--inspect-shop-evidence",
        ])
        .expect("args parse");

        assert_eq!(args.inspect_act, Some(2));
        assert_eq!(args.inspect_floor, Some(18));
        assert!(args.inspect_shop_evidence);
    }

    #[test]
    fn campaign_cli_accepts_checkpoint_shop_plan_challenge() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-boundary",
            "Shop",
            "--challenge-shop-plans",
            "--challenge-max-plans",
            "5",
            "--challenge-depth",
            "3",
        ])
        .expect("args parse");

        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
        assert_eq!(args.inspect_boundary.as_deref(), Some("Shop"));
        assert!(args.challenge_shop_plans);
        assert_eq!(args.challenge_max_plans, 5);
        assert_eq!(args.challenge_depth, 3);
    }

    #[test]
    fn campaign_cli_accepts_checkpoint_card_reward_evidence_inspection() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-act",
            "1",
            "--inspect-floor",
            "11",
            "--inspect-card-reward-evidence",
        ])
        .expect("args parse");

        assert_eq!(args.inspect_act, Some(1));
        assert_eq!(args.inspect_floor, Some(11));
        assert!(args.inspect_card_reward_evidence);
    }

    #[test]
    fn inspect_search_keeps_combat_search_wall_ms_option() {
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
    fn campaign_cli_accepts_checkpoint_deck_mutation_inspection() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-deck-mutation",
        ]);

        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
        assert_eq!(
            args.inspect_report,
            Some(PathBuf::from("latest.campaign.json"))
        );
        assert!(args.inspect_deck_mutation);
    }

    #[test]
    fn campaign_cli_accepts_checkpoint_campfire_evidence_inspection() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-campfire-evidence",
        ]);

        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
        assert_eq!(
            args.inspect_report,
            Some(PathBuf::from("latest.campaign.json"))
        );
        assert!(args.inspect_campfire_evidence);
    }

    #[test]
    fn campaign_cli_accepts_checkpoint_route_evidence_inspection() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-route-evidence",
        ]);

        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
        assert_eq!(
            args.inspect_report,
            Some(PathBuf::from("latest.campaign.json"))
        );
        assert!(args.inspect_route_evidence);
    }

    #[test]
    fn campaign_cli_accepts_checkpoint_last_auto_combat_inspection() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-last-auto-combat",
        ]);

        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
        assert!(args.inspect_last_auto_combat);
    }

    #[test]
    fn campaign_cli_accepts_checkpoint_combat_lab_inspection() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-combat-lab",
            "--probe-boss",
            "--combat-search-option",
            "wall_ms=1000",
        ]);

        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
        assert!(args.inspect_combat_lab);
        assert!(args.probe_boss);
        assert_eq!(args.combat_search_options, vec!["wall_ms=1000"]);
    }

    #[test]
    fn campaign_cli_accepts_final_boss_combat_report_inspection() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-final-boss-combat",
        ]);

        assert_eq!(
            args.inspect_report,
            Some(PathBuf::from("latest.campaign.json"))
        );
        assert!(args.inspect_final_boss_combat);
    }

    #[test]
    fn campaign_cli_accepts_branch_outcome_dataset_export() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--export-outcome-dataset",
            "branch_outcomes.jsonl",
        ]);

        assert_eq!(
            args.export_outcome_dataset,
            Some(PathBuf::from("branch_outcomes.jsonl"))
        );
        assert_eq!(
            args.inspect_report,
            Some(PathBuf::from("latest.campaign.json"))
        );
        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
    }

    #[test]
    fn campaign_cli_accepts_branch_outcome_dataset_analysis() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--analyze-outcome-dataset",
            "branch_outcomes.jsonl",
        ]);

        assert_eq!(
            args.analyze_outcome_dataset,
            Some(PathBuf::from("branch_outcomes.jsonl"))
        );
    }
    #[test]
    fn campaign_cli_accepts_learning_dataset_export() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--export-learning-dataset",
            "learning.jsonl",
        ]);

        assert_eq!(
            args.export_learning_dataset,
            Some(PathBuf::from("learning.jsonl"))
        );
        assert_eq!(
            args.inspect_report,
            Some(PathBuf::from("latest.campaign.json"))
        );
        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
    }

    #[test]
    fn campaign_cli_accepts_decision_outcome_dataset_export() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--export-decision-outcome-dataset",
            "decision_outcomes.jsonl",
        ]);

        assert_eq!(
            args.export_decision_outcome_dataset,
            Some(PathBuf::from("decision_outcomes.jsonl"))
        );
        assert_eq!(
            args.inspect_report,
            Some(PathBuf::from("latest.campaign.json"))
        );
        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
    }

    #[test]
    fn campaign_cli_accepts_decision_outcome_dataset_analysis() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--analyze-decision-outcome-dataset",
            "decision_outcomes.jsonl",
        ]);

        assert_eq!(
            args.analyze_decision_outcome_dataset,
            Some(PathBuf::from("decision_outcomes.jsonl"))
        );
    }

    #[test]
    fn campaign_cli_accepts_learning_readiness_probe() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--probe-learning-readiness",
            "decision_outcomes.jsonl",
        ]);

        assert_eq!(
            args.probe_learning_readiness,
            Some(PathBuf::from("decision_outcomes.jsonl"))
        );
    }

    #[test]
    fn campaign_cli_accepts_targeted_continuation_plan() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--plan-targeted-continuation",
            "decision_outcomes.jsonl",
        ]);

        assert_eq!(
            args.plan_targeted_continuation,
            Some(PathBuf::from("decision_outcomes.jsonl"))
        );
    }

    #[test]
    fn campaign_cli_accepts_targeted_continuation_execution() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--resume",
            "latest.campaign.json",
            "--resume-checkpoint",
            "latest.checkpoint.json",
            "--execute-targeted-continuation",
            "decision_outcomes.jsonl",
            "--targeted-continuation-limit",
            "3",
            "--targeted-continuation-candidates-per-target",
            "2",
        ]);

        assert_eq!(
            args.execute_targeted_continuation,
            Some(PathBuf::from("decision_outcomes.jsonl"))
        );
        assert_eq!(args.targeted_continuation_limit, 3);
        assert_eq!(args.targeted_continuation_candidates_per_target, 2);
    }

    #[test]
    fn campaign_cli_accepts_continuation_effect_report() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--continuation-effect-before",
            "before.jsonl",
            "--continuation-effect-after",
            "after.jsonl",
        ]);

        assert_eq!(
            args.continuation_effect_before,
            Some(PathBuf::from("before.jsonl"))
        );
        assert_eq!(
            args.continuation_effect_after,
            Some(PathBuf::from("after.jsonl"))
        );
    }

    #[test]
    fn campaign_cli_can_branch_all_reward_options() {
        let args = Args::try_parse_from(["branch_campaign_driver", "--all-reward-options"])
            .expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.max_reward_options_per_branch, None);
    }

    #[test]
    fn focused_preset_uses_deeper_fewer_active_branches() {
        let args =
            parse_args_from(["branch_campaign_driver", "--preset", "focused"]).expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.max_rounds, 6);
        assert_eq!(config.round_depth, 2);
        assert_eq!(config.max_active, 2);
        assert_eq!(config.max_frozen, 16);
        assert_eq!(config.max_branches_per_active, 8);
        assert_eq!(config.active_lineage_diversity_slots, 2);
        assert_eq!(config.experiment_wall_ms, Some(10_000));
        assert_eq!(config.search_wall_ms, Some(300));
        assert_eq!(config.search_max_nodes, Some(50_000));
        assert_eq!(args.branch_examples, 4);
    }

    #[test]
    fn quick_preset_uses_short_smoke_budgets() {
        let args =
            parse_args_from(["branch_campaign_driver", "--preset", "quick"]).expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.max_rounds, 2);
        assert_eq!(config.round_depth, 2);
        assert_eq!(config.max_active, 2);
        assert_eq!(config.max_frozen, 16);
        assert_eq!(config.max_branches_per_active, 8);
        assert_eq!(config.active_lineage_diversity_slots, 0);
        assert_eq!(config.experiment_wall_ms, Some(5_000));
        assert_eq!(config.search_wall_ms, Some(300));
        assert_eq!(config.search_max_nodes, Some(50_000));
        assert_eq!(
            config.search_max_hp_loss,
            Some(RunControlHpLossLimit::Unlimited)
        );
        assert_eq!(
            config.combat_retry_policy,
            BranchCampaignCombatRetryPolicyV1::OnStall
        );
        assert_eq!(args.branch_examples, 3);
    }

    #[test]
    fn ascension_domain_sets_curriculum_ascension_when_not_explicit() {
        let args = parse_args_from(["branch_campaign_driver", "--ascension-domain", "a20"])
            .expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.ascension_level, 20);
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
    fn deep_preset_uses_larger_budgets() {
        let args =
            parse_args_from(["branch_campaign_driver", "--preset", "deep"]).expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.max_rounds, 10);
        assert_eq!(config.round_depth, 2);
        assert_eq!(config.max_active, 2);
        assert_eq!(config.max_frozen, 16);
        assert_eq!(config.max_branches_per_active, 8);
        assert_eq!(config.active_lineage_diversity_slots, 2);
        assert_eq!(config.experiment_wall_ms, Some(30_000));
        assert_eq!(config.search_wall_ms, Some(1_000));
        assert_eq!(config.search_max_nodes, Some(200_000));
        assert_eq!(
            config.search_max_hp_loss,
            Some(RunControlHpLossLimit::Unlimited)
        );
        assert_eq!(args.branch_examples, 6);
    }

    #[test]
    fn explore_preset_uses_wider_shallower_branching() {
        let args =
            parse_args_from(["branch_campaign_driver", "--preset", "explore"]).expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.max_rounds, 4);
        assert_eq!(config.round_depth, 1);
        assert_eq!(config.max_active, 6);
        assert_eq!(config.max_frozen, 48);
        assert_eq!(config.max_branches_per_active, 6);
        assert_eq!(config.active_lineage_diversity_slots, 4);
        assert_eq!(config.experiment_wall_ms, Some(8_000));
        assert_eq!(config.search_wall_ms, Some(200));
        assert_eq!(config.search_max_nodes, Some(30_000));
        assert_eq!(
            config.search_max_hp_loss,
            Some(RunControlHpLossLimit::Unlimited)
        );
        assert_eq!(args.branch_examples, 8);
    }

    #[test]
    fn campaign_cli_keeps_explicit_hp_loss_limit() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--preset",
            "quick",
            "--max-hp-loss",
            "12",
        ])
        .expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(
            config.search_max_hp_loss,
            Some(RunControlHpLossLimit::Limit(12))
        );
    }

    #[test]
    fn campaign_cli_can_enable_immediate_combat_retry_for_comparison() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--preset",
            "quick",
            "--combat-retry",
            "immediate",
        ])
        .expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(
            config.combat_retry_policy,
            BranchCampaignCombatRetryPolicyV1::Immediate
        );
    }

    #[test]
    fn campaign_cli_accepts_explicit_combat_retry_wall_budget() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--preset",
            "quick",
            "--combat-retry-wall-ms",
            "1000",
        ])
        .expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.combat_retry_wall_ms, Some(1_000));
    }

    #[test]
    fn focused_preset_keeps_explicit_branch_overrides() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--preset",
            "focused",
            "--round-depth",
            "1",
            "--max-active",
            "4",
            "--max-frozen",
            "8",
            "--max-branches-per-active",
            "12",
        ])
        .expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.round_depth, 1);
        assert_eq!(config.max_active, 4);
        assert_eq!(config.max_frozen, 8);
        assert_eq!(config.max_branches_per_active, 12);
    }
}
