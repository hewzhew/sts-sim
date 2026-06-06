use std::fs;
use std::path::PathBuf;

use clap::Parser;

use sts_simulator::eval::card_reward_value_loop::{
    build_card_reward_closed_loop_report_v1, build_card_reward_runtime_calibration_pipeline_v1,
    calibrate_card_reward_outcomes_v1, calibrate_card_reward_route_risk_v1,
    extract_card_reward_value_loop_examples_v1, promote_card_reward_outcome_calibration_v1,
    replay_card_reward_records_with_calibration_v1, summarize_card_reward_value_loop_examples_v1,
    CardRewardOutcomeCalibrationPromotionConfigV1, CardRewardOutcomeCalibrationV1,
};
use sts_simulator::eval::run_control::load_session_trace_v1;

#[derive(Debug, Parser)]
#[command(
    name = "card_reward_value_loop_driver",
    about = "Extract card reward value-loop examples from SessionTraceV1 artifacts"
)]
struct Args {
    #[arg(long = "trace", value_name = "PATH")]
    traces: Vec<PathBuf>,

    #[arg(long, value_name = "PATH")]
    out: Option<PathBuf>,

    #[arg(long)]
    json_lines: bool,

    #[arg(long)]
    summary: bool,

    #[arg(long)]
    calibration: bool,

    #[arg(long)]
    replay_calibration: bool,

    #[arg(long)]
    closed_loop: bool,

    #[arg(long)]
    route_risk_calibration: bool,

    #[arg(long)]
    promote_calibration: bool,

    #[arg(long)]
    runtime_calibration: bool,

    #[arg(long, value_name = "PATH")]
    calibration_path: Option<PathBuf>,

    #[arg(long, value_name = "PATH")]
    promotion_report: Option<PathBuf>,

    #[arg(long, value_name = "PATH")]
    raw_calibration_out: Option<PathBuf>,

    #[arg(long, value_name = "PATH")]
    route_risk_calibration_out: Option<PathBuf>,

    #[arg(long, value_name = "PATH")]
    closed_loop_report: Option<PathBuf>,

    #[arg(long)]
    approve_short_horizon_gate: bool,

    #[arg(long, default_value_t = 3)]
    min_distinct_seeds: usize,

    #[arg(long, default_value_t = 3)]
    min_bucket_outcomes: usize,

    #[arg(long, default_value_t = 0.65)]
    min_bucket_confidence: f32,

    #[arg(long, default_value_t = 0.35)]
    max_bucket_uncertainty: f32,
}

fn main() {
    let args = Args::parse();
    if let Err(err) = run(args) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<(), String> {
    if [
        args.summary,
        args.calibration,
        args.replay_calibration,
        args.closed_loop,
        args.route_risk_calibration,
        args.promote_calibration,
        args.runtime_calibration,
    ]
    .into_iter()
    .filter(|enabled| *enabled)
    .count()
        > 1
    {
        return Err(
            "use only one of --summary, --calibration, --replay-calibration, --closed-loop, --route-risk-calibration, --promote-calibration, or --runtime-calibration"
                .to_string(),
        );
    }

    if args.promote_calibration {
        let calibration_path = args
            .calibration_path
            .as_ref()
            .ok_or_else(|| "--promote-calibration requires --calibration-path".to_string())?;
        let calibration = load_card_reward_outcome_calibration(calibration_path)?;
        let (promoted, report) =
            promote_card_reward_outcome_calibration_v1(&calibration, &promotion_config(&args));
        if let Some(report_path) = args.promotion_report.as_ref() {
            write_payload(
                report_path,
                serde_json::to_string_pretty(&report).map_err(|err| err.to_string())?,
            )?;
        }
        return write_or_print(
            args.out.as_ref(),
            if args.json_lines {
                serde_json::to_string(&promoted).map_err(|err| err.to_string())?
            } else {
                serde_json::to_string_pretty(&promoted).map_err(|err| err.to_string())?
            },
        );
    }

    if args.traces.is_empty() {
        return Err("--trace is required unless --promote-calibration is used".to_string());
    }

    let mut examples = Vec::new();
    for path in &args.traces {
        let trace = load_session_trace_v1(path)?;
        examples.extend(extract_card_reward_value_loop_examples_v1(&trace)?);
    }

    if args.runtime_calibration {
        let pipeline =
            build_card_reward_runtime_calibration_pipeline_v1(&examples, &promotion_config(&args));
        if let Some(path) = args.raw_calibration_out.as_ref() {
            write_payload(
                path,
                serde_json::to_string_pretty(&pipeline.raw_calibration)
                    .map_err(|err| err.to_string())?,
            )?;
        }
        if let Some(path) = args.promotion_report.as_ref() {
            write_payload(
                path,
                serde_json::to_string_pretty(&pipeline.promotion_report)
                    .map_err(|err| err.to_string())?,
            )?;
        }
        if let Some(path) = args.route_risk_calibration_out.as_ref() {
            write_payload(
                path,
                serde_json::to_string_pretty(&pipeline.route_risk_calibration)
                    .map_err(|err| err.to_string())?,
            )?;
        }
        if let Some(path) = args.closed_loop_report.as_ref() {
            write_payload(
                path,
                serde_json::to_string_pretty(&pipeline.closed_loop_report)
                    .map_err(|err| err.to_string())?,
            )?;
        }
        return write_or_print(
            args.out.as_ref(),
            if args.json_lines {
                serde_json::to_string(&pipeline.promoted_calibration)
                    .map_err(|err| err.to_string())?
            } else {
                serde_json::to_string_pretty(&pipeline.promoted_calibration)
                    .map_err(|err| err.to_string())?
            },
        );
    }

    let payload = if args.summary {
        let summary = summarize_card_reward_value_loop_examples_v1(&examples);
        if args.json_lines {
            serde_json::to_string(&summary).map_err(|err| err.to_string())?
        } else {
            serde_json::to_string_pretty(&summary).map_err(|err| err.to_string())?
        }
    } else if args.calibration {
        let calibration = calibrate_card_reward_outcomes_v1(&examples);
        if args.json_lines {
            serde_json::to_string(&calibration).map_err(|err| err.to_string())?
        } else {
            serde_json::to_string_pretty(&calibration).map_err(|err| err.to_string())?
        }
    } else if args.replay_calibration {
        let calibration = match args.calibration_path.as_ref() {
            Some(path) => load_card_reward_outcome_calibration(path)?,
            None => calibrate_card_reward_outcomes_v1(&examples),
        };
        let replay = replay_card_reward_records_with_calibration_v1(&examples, &calibration);
        if args.json_lines {
            serde_json::to_string(&replay).map_err(|err| err.to_string())?
        } else {
            serde_json::to_string_pretty(&replay).map_err(|err| err.to_string())?
        }
    } else if args.closed_loop {
        let (calibration, calibration_source) = match args.calibration_path.as_ref() {
            Some(path) => (
                load_card_reward_outcome_calibration(path)?,
                path.display().to_string(),
            ),
            None => (
                calibrate_card_reward_outcomes_v1(&examples),
                "generated_from_input_examples".to_string(),
            ),
        };
        let report =
            build_card_reward_closed_loop_report_v1(&examples, &calibration, calibration_source);
        if args.json_lines {
            serde_json::to_string(&report).map_err(|err| err.to_string())?
        } else {
            serde_json::to_string_pretty(&report).map_err(|err| err.to_string())?
        }
    } else if args.route_risk_calibration {
        let calibration = calibrate_card_reward_route_risk_v1(&examples);
        if args.json_lines {
            serde_json::to_string(&calibration).map_err(|err| err.to_string())?
        } else {
            serde_json::to_string_pretty(&calibration).map_err(|err| err.to_string())?
        }
    } else if args.json_lines {
        examples
            .iter()
            .map(|example| serde_json::to_string(example).map_err(|err| err.to_string()))
            .collect::<Result<Vec<_>, _>>()?
            .join("\n")
    } else {
        serde_json::to_string_pretty(&examples).map_err(|err| err.to_string())?
    };

    write_or_print(args.out.as_ref(), payload)
}

fn promotion_config(args: &Args) -> CardRewardOutcomeCalibrationPromotionConfigV1 {
    CardRewardOutcomeCalibrationPromotionConfigV1 {
        approve_short_horizon_autopilot_gate: args.approve_short_horizon_gate,
        min_distinct_seeds: args.min_distinct_seeds,
        min_bucket_outcome_attached_count: args.min_bucket_outcomes,
        min_bucket_confidence: args.min_bucket_confidence,
        max_bucket_uncertainty: args.max_bucket_uncertainty,
        reject_hidden_simulator_state: true,
    }
}

fn write_or_print(out: Option<&PathBuf>, payload: String) -> Result<(), String> {
    if let Some(out) = out {
        write_payload(out, payload)
    } else {
        println!("{payload}");
        Ok(())
    }
}

fn write_payload(out: &PathBuf, payload: String) -> Result<(), String> {
    if let Some(parent) = out.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(out, payload).map_err(|err| err.to_string())
}

fn load_card_reward_outcome_calibration(
    path: &PathBuf,
) -> Result<CardRewardOutcomeCalibrationV1, String> {
    let payload = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&payload).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::eval::card_reward_value_loop::{
        CardRewardOutcomeCalibrationBucketV1, CardRewardOutcomeCalibrationGlobalV1,
        CardRewardOutcomeCalibrationPromotionReportV1, CardRewardOutcomeCalibrationProvenanceV1,
        CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_NAME,
        CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_VERSION,
    };

    #[test]
    fn closed_loop_is_mutually_exclusive_with_summary() {
        let mut args = empty_args();
        args.summary = true;
        args.closed_loop = true;

        let err = run(args).expect_err("closed-loop should be a single output mode");

        assert!(err.contains("--closed-loop"));
    }

    #[test]
    fn promote_calibration_requires_calibration_path() {
        let mut args = empty_args();
        args.promote_calibration = true;

        let err = run(args).expect_err("promotion should require an input calibration");

        assert!(err.contains("--calibration-path"));
    }

    #[test]
    fn runtime_calibration_is_mutually_exclusive_with_calibration_mode() {
        let mut args = empty_args();
        args.calibration = true;
        args.runtime_calibration = true;

        let err = run(args).expect_err("runtime calibration should be a single output mode");

        assert!(err.contains("--runtime-calibration"));
    }

    #[test]
    fn route_risk_calibration_is_mutually_exclusive_with_summary_mode() {
        let mut args = empty_args();
        args.summary = true;
        args.route_risk_calibration = true;

        let err = run(args).expect_err("route-risk calibration should be a single output mode");

        assert!(err.contains("--route-risk-calibration"));
    }

    #[test]
    fn promote_calibration_writes_runtime_calibration_and_report() {
        let input = unique_temp_path("promotion_input.calibration.json");
        let output = unique_temp_path("promotion_output.calibration.json");
        let report_path = unique_temp_path("promotion.report.json");
        fs::write(
            &input,
            serde_json::to_string_pretty(&calibration_fixture()).expect("fixture should serialize"),
        )
        .expect("fixture should write");
        let mut args = empty_args();
        args.promote_calibration = true;
        args.calibration_path = Some(input.clone());
        args.out = Some(output.clone());
        args.promotion_report = Some(report_path.clone());
        args.approve_short_horizon_gate = true;
        args.min_distinct_seeds = 1;
        args.min_bucket_outcomes = 1;

        run(args).expect("promotion should write artifacts");

        let promoted: CardRewardOutcomeCalibrationV1 = serde_json::from_str(
            &fs::read_to_string(&output).expect("promoted calibration should exist"),
        )
        .expect("promoted calibration should parse");
        assert!(promoted.provenance.short_horizon_autopilot_gate_approved);
        assert!(promoted.card_id_buckets[0].usable_for_autopilot_gate);
        let report: CardRewardOutcomeCalibrationPromotionReportV1 = serde_json::from_str(
            &fs::read_to_string(&report_path).expect("promotion report should exist"),
        )
        .expect("promotion report should parse");
        assert_eq!(report.promoted_bucket_count, 1);

        let _ = fs::remove_file(input);
        let _ = fs::remove_file(output);
        let _ = fs::remove_file(report_path);
    }

    fn empty_args() -> Args {
        Args {
            traces: vec![PathBuf::from("unused.trace.json")],
            out: None,
            json_lines: false,
            summary: false,
            calibration: false,
            replay_calibration: false,
            closed_loop: false,
            route_risk_calibration: false,
            promote_calibration: false,
            runtime_calibration: false,
            calibration_path: None,
            promotion_report: None,
            raw_calibration_out: None,
            route_risk_calibration_out: None,
            closed_loop_report: None,
            approve_short_horizon_gate: false,
            min_distinct_seeds: 3,
            min_bucket_outcomes: 3,
            min_bucket_confidence: 0.65,
            max_bucket_uncertainty: 0.35,
        }
    }

    fn calibration_fixture() -> CardRewardOutcomeCalibrationV1 {
        CardRewardOutcomeCalibrationV1 {
            schema_name: CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_NAME.to_string(),
            schema_version: CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_VERSION,
            label_role: "diagnostic_not_teacher_label".to_string(),
            trainable_as_action_label: false,
            policy_quality_claim: false,
            estimator_kind: "selected_outcome_card_id_prior_v1".to_string(),
            provenance: CardRewardOutcomeCalibrationProvenanceV1 {
                source_example_schema_name: "CardRewardValueLoopExampleV1".to_string(),
                source_example_schema_version: 1,
                source_trace_schema_names: vec!["SessionTraceV1".to_string()],
                source_trace_schema_versions: vec![14],
                source_run_count: 1,
                distinct_seed_count: Some(1),
                ruleset_version: Some("sts_simulator:test".to_string()),
                data_roles: vec!["BehaviorPolicyNotTeacher".to_string()],
                hidden_simulator_state_used: false,
                short_horizon_autopilot_gate_approved: false,
            },
            total_examples: 1,
            usable_outcome_examples: 1,
            missing_outcome_examples: 0,
            global: CardRewardOutcomeCalibrationGlobalV1 {
                selected_count: 1,
                outcome_attached_count: 1,
                mean_next_combat_hp_loss: Some(8.0),
            },
            card_id_buckets: vec![CardRewardOutcomeCalibrationBucketV1 {
                bucket_key: "card_id:TwinStrike".to_string(),
                card_id: "TwinStrike".to_string(),
                selected_count: 1,
                outcome_attached_count: 1,
                missing_outcome_count: 0,
                mean_next_combat_hp_loss: Some(4.0),
                hp_loss_bucket_counts: Vec::new(),
                upgraded_count: 0,
                removed_count: 0,
                confidence: 0.8,
                uncertainty: 0.2,
                usable_for_value_estimate: true,
                usable_for_autopilot_gate: false,
            }],
        }
    }

    fn unique_temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "sts_card_reward_value_loop_driver_{}_{}",
            std::process::id(),
            name
        ))
    }
}
