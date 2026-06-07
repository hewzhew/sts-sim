use std::fs;
use std::io::{self, Write};
use std::path::Path;

use sts_simulator::eval::run_control::{
    mark_current_boundary, parse_run_control_command, render_bookmarks, render_run_control_state,
    validate_bookmark_name, RunControlCommand, RunControlSession, RunPlayBookmarkV1,
    SessionTraceRecorder,
};

pub(crate) fn run_script(
    session: &mut RunControlSession,
    script: &Path,
    bookmark_registry_path: &Path,
    mut trace: Option<&mut SessionTraceRecorder>,
) -> Result<(), String> {
    let payload = fs::read_to_string(script).map_err(|err| err.to_string())?;
    for (line_number, line) in payload.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        println!("> {trimmed}");
        if execute_line(
            session,
            trimmed,
            bookmark_registry_path,
            trace.as_deref_mut(),
        )
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
    bookmark_registry_path: &Path,
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
        match execute_line(session, &line, bookmark_registry_path, trace.as_deref_mut()) {
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
    bookmark_registry_path: &Path,
    mut trace: Option<&mut SessionTraceRecorder>,
) -> Result<ExecuteLineResult, String> {
    let trimmed = line.trim();
    if let Some(command) = parse_terminal_bookmark_command(trimmed)? {
        return execute_bookmark_command(session, bookmark_registry_path, trace, command);
    }
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
            recorder.record_boundary_annotations(trimmed, session, &outcome.trace_annotations)?;
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

enum TerminalBookmarkCommand {
    Mark(String),
    List,
}

fn parse_terminal_bookmark_command(raw: &str) -> Result<Option<TerminalBookmarkCommand>, String> {
    let mut parts = raw.split_whitespace();
    let Some(command) = parts.next() else {
        return Ok(None);
    };
    match command.to_ascii_lowercase().as_str() {
        "mark" | "bookmark" => {
            let name = parts.next().ok_or_else(|| {
                "mark requires a name, for example: mark before_reward".to_string()
            })?;
            if parts.next().is_some() {
                return Err("mark accepts exactly one name".to_string());
            }
            validate_bookmark_name(name)?;
            Ok(Some(TerminalBookmarkCommand::Mark(name.to_string())))
        }
        "marks" | "bookmarks" => {
            if parts.next().is_some() {
                return Err("marks does not accept arguments".to_string());
            }
            Ok(Some(TerminalBookmarkCommand::List))
        }
        _ => Ok(None),
    }
}

fn execute_bookmark_command(
    session: &RunControlSession,
    bookmark_registry_path: &Path,
    trace: Option<&mut SessionTraceRecorder>,
    command: TerminalBookmarkCommand,
) -> Result<ExecuteLineResult, String> {
    match command {
        TerminalBookmarkCommand::List => {
            println!("{}", render_bookmarks(bookmark_registry_path)?);
        }
        TerminalBookmarkCommand::Mark(name) => {
            let recorder = trace.ok_or_else(|| {
                "mark requires trace recording; start run_play_driver with --record, --trace, --continue-trace, or --goto"
                    .to_string()
            })?;
            let bookmark = mark_current_boundary(
                bookmark_registry_path,
                &name,
                recorder.path(),
                recorder.step_count(),
                session,
            )?;
            println!("{}", render_mark_saved(&bookmark));
        }
    }
    Ok(ExecuteLineResult {
        should_quit: false,
        ignore_following_blank_enter: false,
    })
}

fn render_mark_saved(bookmark: &RunPlayBookmarkV1) -> String {
    format!(
        "saved bookmark `{}` at {} | Act {} Floor {} | HP {}/{} | replay_steps={}\nresume later: --goto {}",
        bookmark.name,
        bookmark.screen_title,
        bookmark.act,
        bookmark.floor,
        bookmark.hp,
        bookmark.max_hp,
        bookmark.replay_steps,
        bookmark.name
    )
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
    use std::time::{SystemTime, UNIX_EPOCH};
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

    #[test]
    fn bookmark_commands_save_and_list_trace_position() {
        let dir = unique_temp_dir("terminal_bookmark");
        let registry_path = dir.join("bookmarks.json");
        let trace_path = dir.join("trace.json");
        let mut session = RunControlSession::new(Default::default());
        let mut recorder = SessionTraceRecorder::new(trace_path, &session);
        execute_line(&mut session, "", &registry_path, Some(&mut recorder))
            .expect("default candidate should record through terminal");

        execute_line(
            &mut session,
            "mark before_reward",
            &registry_path,
            Some(&mut recorder),
        )
        .expect("mark should save bookmark");

        let rendered = render_bookmarks(&registry_path).expect("bookmarks should render");
        assert!(rendered.contains("before_reward"));
        assert!(rendered.contains("replay_steps=1"));
        assert!(rendered.contains("goto: --goto before_reward"));

        execute_line(&mut session, "marks", &registry_path, Some(&mut recorder))
            .expect("marks should print");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn terminal_records_noncombat_stop_boundary_without_action_step() {
        let dir = unique_temp_dir("terminal_noncombat_boundary");
        let registry_path = dir.join("bookmarks.json");
        let trace_path = dir.join("trace.json");
        let mut session = RunControlSession::new(Default::default());
        let mut shop = sts_simulator::state::shop::ShopState::new();
        shop.cards.push(sts_simulator::state::shop::ShopCard {
            card_id: sts_simulator::content::cards::CardId::Armaments,
            upgrades: 0,
            price: 49,
            can_buy: true,
            blocked_reason: None,
        });
        session.engine_state = sts_simulator::state::core::EngineState::Shop(shop);
        let mut recorder = SessionTraceRecorder::new(trace_path, &session);

        execute_line(&mut session, "n", &registry_path, Some(&mut recorder))
            .expect("auto-step stop should record boundary through terminal");

        assert!(recorder.trace().steps.is_empty());
        assert_eq!(recorder.trace().boundary_records.len(), 1);
        assert_eq!(recorder.trace().boundary_records[0].screen_title, "Shop");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn mark_requires_trace_recording() {
        let dir = unique_temp_dir("terminal_bookmark_no_trace");
        let registry_path = dir.join("bookmarks.json");
        let mut session = RunControlSession::new(Default::default());

        let err = execute_line(&mut session, "mark missing_trace", &registry_path, None)
            .expect_err("mark without trace should fail");

        assert!(err.contains("--record"));

        let _ = fs::remove_dir_all(dir);
    }

    fn unique_temp_dir(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{label}_{}_{}", std::process::id(), nanos))
    }
}
