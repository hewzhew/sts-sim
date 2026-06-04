use std::fs;
use std::path::PathBuf;

use clap::Parser;

use sts_simulator::eval::card_reward_value_loop::{
    calibrate_card_reward_outcomes_v1, extract_card_reward_value_loop_examples_v1,
    replay_card_reward_records_with_calibration_v1, summarize_card_reward_value_loop_examples_v1,
    CardRewardOutcomeCalibrationV1,
};
use sts_simulator::eval::run_control::load_session_trace_v1;

#[derive(Debug, Parser)]
#[command(
    name = "card_reward_value_loop_driver",
    about = "Extract card reward value-loop examples from SessionTraceV1 artifacts"
)]
struct Args {
    #[arg(long = "trace", value_name = "PATH", required = true)]
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

    #[arg(long, value_name = "PATH")]
    calibration_path: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();
    if let Err(err) = run(args) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<(), String> {
    if [args.summary, args.calibration, args.replay_calibration]
        .into_iter()
        .filter(|enabled| *enabled)
        .count()
        > 1
    {
        return Err(
            "use only one of --summary, --calibration, or --replay-calibration".to_string(),
        );
    }

    let mut examples = Vec::new();
    for path in &args.traces {
        let trace = load_session_trace_v1(path)?;
        examples.extend(extract_card_reward_value_loop_examples_v1(&trace)?);
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
    } else if args.json_lines {
        examples
            .iter()
            .map(|example| serde_json::to_string(example).map_err(|err| err.to_string()))
            .collect::<Result<Vec<_>, _>>()?
            .join("\n")
    } else {
        serde_json::to_string_pretty(&examples).map_err(|err| err.to_string())?
    };

    if let Some(out) = args.out {
        if let Some(parent) = out.parent().filter(|parent| !parent.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        fs::write(out, payload).map_err(|err| err.to_string())?;
    } else {
        println!("{payload}");
    }
    Ok(())
}

fn load_card_reward_outcome_calibration(
    path: &PathBuf,
) -> Result<CardRewardOutcomeCalibrationV1, String> {
    let payload = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&payload).map_err(|err| err.to_string())
}
