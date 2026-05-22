use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use sts_simulator::eval::run_control::{
    canonical_player_class, parse_run_control_command, render_run_control_state, RunControlConfig,
    RunControlSession, SessionTraceRecorder,
};

#[derive(Parser, Debug)]
#[command(about = "Thin simulator run/play driver with exact combat capture support")]
struct Args {
    #[arg(long, default_value_t = 1)]
    seed: u64,

    #[arg(long, default_value_t = 0)]
    ascension: u8,

    #[arg(long, value_enum, default_value_t = CliPlayerClass::Ironclad)]
    class: CliPlayerClass,

    #[arg(long)]
    final_act: bool,

    #[arg(long)]
    script: Option<PathBuf>,

    #[arg(long)]
    trace: Option<PathBuf>,
}

#[derive(Clone, Debug, ValueEnum)]
enum CliPlayerClass {
    Ironclad,
    Silent,
    Defect,
    Watcher,
}

impl CliPlayerClass {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Ironclad => "ironclad",
            Self::Silent => "silent",
            Self::Defect => "defect",
            Self::Watcher => "watcher",
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let player_class = canonical_player_class(args.class.as_str())?;
    let mut session = RunControlSession::new(RunControlConfig {
        seed: args.seed,
        ascension_level: args.ascension,
        final_act: args.final_act,
        player_class,
    });

    println!("{}", render_run_control_state(&session));
    let mut trace = args
        .trace
        .as_ref()
        .map(|path| SessionTraceRecorder::new(path.clone(), &session));

    if let Some(script) = args.script.as_ref() {
        run_script(&mut session, script, trace.as_mut())?;
    } else {
        run_repl(&mut session, trace.as_mut())?;
    }
    Ok(())
}

fn run_script(
    session: &mut RunControlSession,
    script: &PathBuf,
    mut trace: Option<&mut SessionTraceRecorder>,
) -> Result<(), String> {
    let payload = fs::read_to_string(script).map_err(|err| err.to_string())?;
    for (line_number, line) in payload.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        println!("> {trimmed}");
        if execute_line(session, trimmed, trace.as_deref_mut())
            .map_err(|err| format!("{}:{}: {err}", script.display(), line_number + 1))?
        {
            break;
        }
    }
    Ok(())
}

fn run_repl(
    session: &mut RunControlSession,
    mut trace: Option<&mut SessionTraceRecorder>,
) -> Result<(), String> {
    let stdin = io::stdin();
    loop {
        print!("run-play> ");
        io::stdout().flush().map_err(|err| err.to_string())?;
        let mut line = String::new();
        let bytes = stdin.read_line(&mut line).map_err(|err| err.to_string())?;
        if bytes == 0 {
            break;
        }
        match execute_line(session, &line, trace.as_deref_mut()) {
            Ok(true) => break,
            Ok(false) => {}
            Err(err) => {
                println!("error: {err}");
                println!("{}", render_run_control_state(session));
            }
        }
    }
    Ok(())
}

fn execute_line(
    session: &mut RunControlSession,
    line: &str,
    mut trace: Option<&mut SessionTraceRecorder>,
) -> Result<bool, String> {
    let trimmed = line.trim();
    let command = parse_run_control_command(line)?;
    let pending_trace = trace
        .as_ref()
        .map(|_| SessionTraceRecorder::prepare_step(session, trimmed, &command));
    let outcome = session.apply_command(command.clone())?;
    if !outcome.message.is_empty() {
        println!("{}", outcome.message);
    }
    if let Some(recorder) = trace.as_deref_mut() {
        if let Some(action_result) = outcome.action_result.as_ref() {
            if let Some(pending) = pending_trace {
                recorder.record_action_step(pending, session, action_result)?;
            }
        } else {
            recorder.record_artifact_command(trimmed, session, &command)?;
        }
    }
    Ok(outcome.should_quit)
}
