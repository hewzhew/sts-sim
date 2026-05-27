use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use sts_simulator::eval::run_control::{
    canonical_player_class, parse_run_control_command, render_run_control_state,
    AutoCombatCaptureConfig, RunControlCommand, RunControlConfig, RunControlSession,
    SessionTraceRecorder,
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

    #[arg(long)]
    auto_capture_combat: bool,

    #[arg(long)]
    auto_capture_combat_root: Option<PathBuf>,
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
        reward_automation: Default::default(),
        auto_capture: AutoCombatCaptureConfig {
            enabled: args.auto_capture_combat,
            root: args.auto_capture_combat_root.clone(),
        },
    });

    println!("{}", render_run_control_state(&session));
    if args.auto_capture_combat {
        let root = args
            .auto_capture_combat_root
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "tools/artifacts/benchmarks/seed<seed>_act<act>".to_string());
        println!("auto combat capture enabled: {root}");
    }
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
            .should_quit
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
    let mut ignore_blank_after_long_command = false;
    loop {
        print!("run-play> ");
        io::stdout().flush().map_err(|err| err.to_string())?;
        let mut line = String::new();
        let bytes = stdin.read_line(&mut line).map_err(|err| err.to_string())?;
        if bytes == 0 {
            break;
        }
        if ignore_blank_after_long_command && line.trim().is_empty() {
            println!(
                "ignored blank Enter after long-running automation; type a visible id or command explicitly"
            );
            continue;
        }
        match execute_line(session, &line, trace.as_deref_mut()) {
            Ok(result) if result.should_quit => break,
            Ok(result) => {
                ignore_blank_after_long_command = result.ignore_following_blank_enter;
            }
            Err(err) => {
                ignore_blank_after_long_command = false;
                println!("error: {err}");
                println!("{}", render_run_control_state(session));
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExecuteLineResult {
    should_quit: bool,
    ignore_following_blank_enter: bool,
}

fn execute_line(
    session: &mut RunControlSession,
    line: &str,
    mut trace: Option<&mut SessionTraceRecorder>,
) -> Result<ExecuteLineResult, String> {
    let trimmed = line.trim();
    let command = parse_run_control_command(line)?;
    if let Some(message) = long_command_progress_message(&command) {
        println!("{message}");
        io::stdout().flush().map_err(|err| err.to_string())?;
    }
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
                recorder.record_action_step(
                    pending,
                    session,
                    action_result,
                    &outcome.trace_annotations,
                )?;
            }
        } else {
            recorder.record_artifact_command(trimmed, session, &command)?;
        }
        if let Some(path) = outcome.search_evidence_path.as_ref() {
            recorder.record_search_evidence_artifact(trimmed, session, path)?;
        }
    }
    Ok(ExecuteLineResult {
        should_quit: outcome.should_quit,
        ignore_following_blank_enter: ignores_following_blank_enter(&command),
    })
}

fn long_command_progress_message(command: &RunControlCommand) -> Option<&'static str> {
    match command {
        RunControlCommand::AutoStep(_) => Some(
            "running advance-to-human-boundary; combat search may take a few seconds. Extra blank Enter presses after it finishes will be ignored.",
        ),
        RunControlCommand::SearchCombat(_) => Some(
            "running combat search; this may take a few seconds. Extra blank Enter presses after it finishes will be ignored.",
        ),
        _ => None,
    }
}

fn ignores_following_blank_enter(command: &RunControlCommand) -> bool {
    matches!(
        command,
        RunControlCommand::AutoStep(_) | RunControlCommand::SearchCombat(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::eval::run_control::{
        RunControlAutoStepOptions, RunControlSearchCombatOptions,
    };

    #[test]
    fn long_running_commands_show_progress_and_guard_blank_enter() {
        let auto = RunControlCommand::AutoStep(RunControlAutoStepOptions::default());
        let search = RunControlCommand::SearchCombat(RunControlSearchCombatOptions::default());
        let deck = RunControlCommand::Deck;

        assert!(long_command_progress_message(&auto).is_some());
        assert!(long_command_progress_message(&search).is_some());
        assert!(long_command_progress_message(&deck).is_none());

        assert!(ignores_following_blank_enter(&auto));
        assert!(ignores_following_blank_enter(&search));
        assert!(!ignores_following_blank_enter(&deck));
    }
}
