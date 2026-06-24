use std::path::PathBuf;

use crate::state::core::{CampfireChoice, ClientInput};
use crate::state::selection::{SelectionResolution, SelectionScope};

use super::options::{
    parse_auto_reward_command, parse_auto_run_command, parse_auto_step_command, parse_buy_command,
    parse_optional_usize_arg, parse_route_auto_step_command, parse_search_combat_options,
    parse_search_defaults_command, parse_u32_list, parse_usize_arg, parse_usize_list,
};
use super::RunControlCommand;

pub fn parse_run_control_command(line: &str) -> Result<RunControlCommand, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(RunControlCommand::DefaultCandidate);
    }
    if trimmed.starts_with('#') {
        return Ok(RunControlCommand::Noop);
    }

    let mut parts = trimmed.split_whitespace();
    let Some(command) = parts.next() else {
        return Ok(RunControlCommand::Noop);
    };
    let rest = parts.collect::<Vec<_>>();
    if is_candidate_id(command) || is_structured_candidate_id(command) {
        return Ok(RunControlCommand::Candidate(command.to_string()));
    }

    match command.to_ascii_lowercase().as_str() {
        "?" | "h" | "help" => Ok(RunControlCommand::Help),
        "q" | "quit" | "exit" => Ok(RunControlCommand::Quit),
        "main" | "state" => Ok(RunControlCommand::Main),
        "deck" => Ok(RunControlCommand::Deck),
        "map" if is_full_map_arg(rest.first()) => Ok(RunControlCommand::MapFull),
        "map" => Ok(RunControlCommand::Map),
        "ms" | "map-summary" | "route-summary" | "routes" => Ok(RunControlCommand::MapSummary),
        "mf" | "map-full" | "full-map" => Ok(RunControlCommand::MapFull),
        "bd" | "boundary" | "boundary-record" | "noncombat-boundary" => {
            Ok(RunControlCommand::BoundaryRecord)
        }
        "rs" | "route" | "route-suggest" | "route-suggestion" => {
            Ok(RunControlCommand::RouteSuggest)
        }
        "rg" | "route-go" | "route-next" => Ok(RunControlCommand::RouteGo),
        "relics" | "relic-list" => Ok(RunControlCommand::Relics),
        "potions" | "potion-list" => Ok(RunControlCommand::Potions),
        "draw" | "draw-pile" => Ok(RunControlCommand::Draw),
        "discard" | "discard-pile" => Ok(RunControlCommand::Discard),
        "exhaust" | "exhaust-pile" => Ok(RunControlCommand::Exhaust),
        "inspect" => Ok(RunControlCommand::Inspect(
            rest.first()
                .ok_or_else(|| "inspect requires a visible id".to_string())?
                .to_string(),
        )),
        "case" | "save-case" => Ok(RunControlCommand::SaveDecisionCase {
            path: rest.first().map(PathBuf::from),
        }),
        "back" => Ok(RunControlCommand::Candidate("back".to_string())),
        "bowl" | "singing-bowl" => Ok(RunControlCommand::Candidate("bowl".to_string())),
        "skip" => Ok(RunControlCommand::Candidate("skip".to_string())),
        "leave" => Ok(RunControlCommand::Candidate("leave".to_string())),
        "rewards" | "reward-overlay" | "pending-rewards" => {
            Ok(RunControlCommand::Candidate("rewards".to_string()))
        }
        "d" | "details" => Ok(RunControlCommand::Details),
        "r" | "raw" => Ok(RunControlCommand::Raw),
        "actions" | "legal" => Ok(RunControlCommand::Actions),
        "capture" | "save-capture" => parse_capture_command(&rest),
        "capture-case" => parse_capture_case_command(&rest),
        "cap" | "capture-combat" => parse_default_capture_case_command(&rest),
        "save-baseline" => parse_save_baseline_command(&rest),
        "save-baseline-case" => parse_save_baseline_case_command(&rest),
        "b" | "baseline" | "save-baseline-last" | "baseline-last" => {
            parse_save_baseline_last_command(&rest)
        }
        "bench-add" => parse_bench_add_command(&rest),
        "sd" | "search-defaults" | "search-default" | "search-config" => {
            parse_search_defaults_command(&rest)
        }
        "sc" | "search-combat" | "solve-combat" | "auto-combat" => {
            parse_search_combat_command(&rest)
        }
        "n" | "next" | "advance" | "advance-to-human-boundary" | "auto-step" | "autostep" => {
            parse_auto_step_command(&rest)
        }
        "nr" | "next-route" | "advance-route" => parse_route_auto_step_command(&rest),
        "ar" | "auto-run" | "autorun" | "run-auto" => parse_auto_run_command(&rest),
        "auto-reward" => parse_auto_reward_command(&rest),
        "branch-skip-card-reward" => Ok(RunControlCommand::BranchSkipCardReward(parse_usize_arg(
            rest.first(),
            "card reward item index",
        )?)),
        "action" => Ok(RunControlCommand::ActionIndex(parse_usize_arg(
            rest.first(),
            "action index",
        )?)),
        "play" => Ok(RunControlCommand::PlayCard {
            card_index: parse_usize_arg(rest.first(), "hand card index")?,
            target_slot_or_id: parse_optional_usize_arg(rest.get(1), "target slot")?,
        }),
        "end" => Ok(RunControlCommand::Input(ClientInput::EndTurn)),
        "potion" => Ok(RunControlCommand::UsePotion {
            potion_index: parse_usize_arg(rest.first(), "potion slot")?,
            target_slot_or_id: parse_optional_usize_arg(rest.get(1), "target slot")?,
        }),
        "discard-potion" => Ok(RunControlCommand::Input(ClientInput::DiscardPotion(
            parse_usize_arg(rest.first(), "potion slot")?,
        ))),
        "go" => Ok(RunControlCommand::Input(ClientInput::SelectMapNode(
            parse_usize_arg(rest.first(), "map x")?,
        ))),
        "fly" => Ok(RunControlCommand::Input(ClientInput::FlyToNode(
            parse_usize_arg(rest.first(), "map x")?,
            parse_usize_arg(rest.get(1), "map y")?,
        ))),
        "event" | "option" => Ok(RunControlCommand::Input(ClientInput::EventChoice(
            parse_usize_arg(rest.first(), "event option index")?,
        ))),
        "event-select" | "event-selection" => parse_event_select_command(&rest),
        "claim" => Ok(RunControlCommand::Input(ClientInput::ClaimReward(
            parse_usize_arg(rest.first(), "reward index")?,
        ))),
        "card" => Ok(RunControlCommand::CardIndex(parse_usize_arg(
            rest.first(),
            "card index",
        )?)),
        "rp" | "record-pick" | "recorded-pick" => Ok(RunControlCommand::RecordedCardRewardPick(
            parse_usize_arg(rest.first(), "card reward index")?,
        )),
        "pick" | "select-card" => Ok(RunControlCommand::Input(ClientInput::SelectCard(
            parse_usize_arg(rest.first(), "card option index")?,
        ))),
        "select" => Ok(RunControlCommand::SelectionIndices(parse_usize_list(
            &rest,
            "selection index",
        )?)),
        "hand-select" => Ok(RunControlCommand::Input(ClientInput::SubmitSelection(
            SelectionResolution::card_uuids(
                SelectionScope::Hand,
                parse_u32_list(&rest, "hand card uuid")?,
            ),
        ))),
        "grid-select" => Ok(RunControlCommand::Input(ClientInput::SubmitSelection(
            SelectionResolution::card_uuids(
                SelectionScope::Grid,
                parse_u32_list(&rest, "grid card uuid")?,
            ),
        ))),
        "choose" => Ok(RunControlCommand::Input(ClientInput::SubmitDiscoverChoice(
            parse_usize_arg(rest.first(), "choice index")?,
        ))),
        "proceed" => Ok(RunControlCommand::Input(ClientInput::Proceed)),
        "cancel" => Ok(RunControlCommand::Input(ClientInput::Cancel)),
        "open" | "chest" => Ok(RunControlCommand::Input(ClientInput::OpenChest)),
        "rest" => Ok(RunControlCommand::Input(ClientInput::CampfireOption(
            CampfireChoice::Rest,
        ))),
        "smith" => Ok(RunControlCommand::Input(ClientInput::CampfireOption(
            CampfireChoice::Smith(parse_usize_arg(rest.first(), "deck index")?),
        ))),
        "dig" => Ok(RunControlCommand::Input(ClientInput::CampfireOption(
            CampfireChoice::Dig,
        ))),
        "lift" => Ok(RunControlCommand::Input(ClientInput::CampfireOption(
            CampfireChoice::Lift,
        ))),
        "recall" => Ok(RunControlCommand::Input(ClientInput::CampfireOption(
            CampfireChoice::Recall,
        ))),
        "toke" => Ok(RunControlCommand::Input(ClientInput::CampfireOption(
            CampfireChoice::Toke(parse_usize_arg(rest.first(), "deck index")?),
        ))),
        "buy" => parse_buy_command(&rest),
        "purge" if rest.is_empty() => Ok(RunControlCommand::Candidate("purge".to_string())),
        "purge" => Ok(RunControlCommand::Input(ClientInput::PurgeCard(
            parse_usize_arg(rest.first(), "deck index")?,
        ))),
        "relic" => Ok(RunControlCommand::RelicIndex(parse_usize_arg(
            rest.first(),
            "relic index",
        )?)),
        other => Err(format!("unknown run/play command '{other}'")),
    }
}

fn parse_event_select_command(rest: &[&str]) -> Result<RunControlCommand, String> {
    let _ = rest;
    Err("event-select is retired; use `event <idx>` and then `select <deck_idx...>`".to_string())
}

fn is_candidate_id(command: &str) -> bool {
    command.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
        && command.chars().any(|ch| ch.is_ascii_digit())
}

fn is_structured_candidate_id(command: &str) -> bool {
    let Some((prefix, suffix)) = command.split_once('-') else {
        return false;
    };
    matches!(
        prefix.to_ascii_lowercase().as_str(),
        "card" | "relic" | "potion" | "smith"
    ) && !suffix.is_empty()
        && suffix.chars().all(|ch| ch.is_ascii_digit())
}

fn is_full_map_arg(value: Option<&&str>) -> bool {
    value.is_some_and(|arg| {
        matches!(
            arg.to_ascii_lowercase().as_str(),
            "full" | "f" | "all" | "grid"
        )
    })
}

fn parse_capture_command(rest: &[&str]) -> Result<RunControlCommand, String> {
    let path = rest
        .first()
        .ok_or_else(|| "capture requires an output path".to_string())?;
    let label = (!rest[1.min(rest.len())..].is_empty()).then(|| rest[1..].join(" "));
    Ok(RunControlCommand::Capture {
        path: PathBuf::from(path),
        label,
    })
}

fn parse_capture_case_command(rest: &[&str]) -> Result<RunControlCommand, String> {
    let root = rest
        .first()
        .ok_or_else(|| "capture-case requires a benchmark_dir".to_string())?;
    let case_id = rest
        .get(1)
        .ok_or_else(|| "capture-case requires a case_id".to_string())?;
    let label = (!rest[2.min(rest.len())..].is_empty()).then(|| rest[2..].join(" "));
    Ok(RunControlCommand::CaptureCase {
        root: PathBuf::from(root),
        case_id: case_id.to_string(),
        label,
    })
}

fn parse_default_capture_case_command(rest: &[&str]) -> Result<RunControlCommand, String> {
    let case_id = rest
        .first()
        .ok_or_else(|| "cap requires a case_id, for example: cap first_combat".to_string())?;
    let label = (!rest[1.min(rest.len())..].is_empty()).then(|| rest[1..].join(" "));
    Ok(RunControlCommand::CaptureCaseDefault {
        case_id: case_id.to_string(),
        label,
    })
}

fn parse_save_baseline_command(rest: &[&str]) -> Result<RunControlCommand, String> {
    let path = rest
        .first()
        .ok_or_else(|| "save-baseline requires an output path".to_string())?;
    Ok(RunControlCommand::SaveBaseline {
        path: PathBuf::from(path),
        case_id: rest.get(1).map(|value| value.to_string()),
    })
}

fn parse_save_baseline_case_command(rest: &[&str]) -> Result<RunControlCommand, String> {
    let root = rest
        .first()
        .ok_or_else(|| "save-baseline-case requires a benchmark_dir".to_string())?;
    let case_id = rest
        .get(1)
        .ok_or_else(|| "save-baseline-case requires a case_id".to_string())?;
    Ok(RunControlCommand::SaveBaselineCase {
        root: PathBuf::from(root),
        case_id: case_id.to_string(),
    })
}

fn parse_save_baseline_last_command(rest: &[&str]) -> Result<RunControlCommand, String> {
    if !rest.is_empty() {
        return Err(
            "baseline uses the last capture-case; use save-baseline-case <benchmark_dir> <case_id> to override"
                .to_string(),
        );
    }
    Ok(RunControlCommand::SaveBaselineForLastCaptureCase)
}

fn parse_bench_add_command(rest: &[&str]) -> Result<RunControlCommand, String> {
    let root = rest
        .first()
        .ok_or_else(|| "bench-add requires a benchmark_dir".to_string())?;
    let case_id = rest
        .get(1)
        .ok_or_else(|| "bench-add requires a case_id".to_string())?;
    Ok(RunControlCommand::RegisterBenchmarkCase {
        root: PathBuf::from(root),
        case_id: case_id.to_string(),
    })
}

fn parse_search_combat_command(rest: &[&str]) -> Result<RunControlCommand, String> {
    Ok(RunControlCommand::SearchCombat(
        parse_search_combat_options(rest)?,
    ))
}
