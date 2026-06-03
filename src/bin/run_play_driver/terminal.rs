use std::fs;
use std::io::{self, Write};
use std::path::Path;

use sts_simulator::eval::run_control::{
    parse_run_control_command, render_run_control_state, RunControlCommand, RunControlSession,
    SessionTraceRecorder,
};

pub(crate) fn run_script(
    session: &mut RunControlSession,
    script: &Path,
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

pub(crate) fn run_repl(
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
        RunControlCommand::AutoRun(_) => Some(
            "running auto-run with route planner; combat search may take a few seconds. Extra blank Enter presses after it finishes will be ignored.",
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
        RunControlCommand::AutoStep(_)
            | RunControlCommand::AutoRun(_)
            | RunControlCommand::SearchCombat(_)
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
        let auto_run = RunControlCommand::AutoRun(RunControlAutoStepOptions::default());
        let search = RunControlCommand::SearchCombat(RunControlSearchCombatOptions::default());
        let deck = RunControlCommand::Deck;

        assert!(long_command_progress_message(&auto).is_some());
        assert!(long_command_progress_message(&auto_run).is_some());
        assert!(long_command_progress_message(&search).is_some());
        assert!(long_command_progress_message(&deck).is_none());

        assert!(ignores_following_blank_enter(&auto));
        assert!(ignores_following_blank_enter(&auto_run));
        assert!(ignores_following_blank_enter(&search));
        assert!(!ignores_following_blank_enter(&deck));
    }
}
