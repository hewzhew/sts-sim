use std::fs;
use std::path::PathBuf;

use clap::Parser;

use sts_simulator::eval::card_reward_value_loop::extract_card_reward_value_loop_examples_v1;
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
}

fn main() {
    let args = Args::parse();
    if let Err(err) = run(args) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<(), String> {
    let mut examples = Vec::new();
    for path in &args.traces {
        let trace = load_session_trace_v1(path)?;
        examples.extend(extract_card_reward_value_loop_examples_v1(&trace)?);
    }

    let payload = if args.json_lines {
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
