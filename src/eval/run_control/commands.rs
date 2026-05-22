use std::path::PathBuf;

use crate::state::core::{CampfireChoice, ClientInput};

#[derive(Clone, Debug, PartialEq)]
pub enum RunControlCommand {
    Noop,
    Help,
    Quit,
    State,
    Actions,
    Capture {
        path: PathBuf,
        label: Option<String>,
    },
    CaptureCase {
        root: PathBuf,
        case_id: String,
        label: Option<String>,
    },
    SaveBaseline {
        path: PathBuf,
        case_id: Option<String>,
    },
    SaveBaselineCase {
        root: PathBuf,
        case_id: String,
    },
    RegisterBenchmarkCase {
        root: PathBuf,
        case_id: String,
    },
    ActionIndex(usize),
    PlayCard {
        card_index: usize,
        target_slot_or_id: Option<usize>,
    },
    UsePotion {
        potion_index: usize,
        target_slot_or_id: Option<usize>,
    },
    Input(ClientInput),
}

pub fn parse_run_control_command(line: &str) -> Result<RunControlCommand, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(RunControlCommand::Noop);
    }

    let mut parts = trimmed.split_whitespace();
    let Some(command) = parts.next() else {
        return Ok(RunControlCommand::Noop);
    };
    let rest = parts.collect::<Vec<_>>();

    match command.to_ascii_lowercase().as_str() {
        "?" | "help" => Ok(RunControlCommand::Help),
        "q" | "quit" | "exit" => Ok(RunControlCommand::Quit),
        "state" => Ok(RunControlCommand::State),
        "actions" | "legal" => Ok(RunControlCommand::Actions),
        "capture" | "save-capture" => parse_capture_command(&rest),
        "capture-case" => parse_capture_case_command(&rest),
        "save-baseline" => parse_save_baseline_command(&rest),
        "save-baseline-case" => parse_save_baseline_case_command(&rest),
        "bench-add" => parse_bench_add_command(&rest),
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
        "claim" => Ok(RunControlCommand::Input(ClientInput::ClaimReward(
            parse_usize_arg(rest.first(), "reward index")?,
        ))),
        "pick" | "card" | "select-card" => Ok(RunControlCommand::Input(ClientInput::SelectCard(
            parse_usize_arg(rest.first(), "card option index")?,
        ))),
        "select" => Ok(RunControlCommand::Input(ClientInput::SubmitDeckSelect(
            parse_usize_list(&rest, "deck index")?,
        ))),
        "hand-select" => Ok(RunControlCommand::Input(ClientInput::SubmitHandSelect(
            parse_u32_list(&rest, "hand card uuid")?,
        ))),
        "grid-select" => Ok(RunControlCommand::Input(ClientInput::SubmitGridSelect(
            parse_u32_list(&rest, "grid card uuid")?,
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
        "purge" => Ok(RunControlCommand::Input(ClientInput::PurgeCard(
            parse_usize_arg(rest.first(), "deck index")?,
        ))),
        "relic" => Ok(RunControlCommand::Input(ClientInput::SubmitRelicChoice(
            parse_usize_arg(rest.first(), "boss relic index")?,
        ))),
        other => Err(format!("unknown run/play command '{other}'")),
    }
}

pub fn run_control_help() -> &'static str {
    "commands: state, actions, action <idx>, capture <path> [label], capture-case <benchmark_dir> <case_id> [label], save-baseline <path> [case_id], save-baseline-case <benchmark_dir> <case_id>, bench-add <benchmark_dir> <case_id>, play <hand_idx> [target_slot], end, potion <slot> [target_slot], discard-potion <slot>, go <x>, fly <x> <y>, event <idx>, claim <idx>, pick <idx>, select <deck_idx...>, hand-select <uuid...>, grid-select <uuid...>, open, rest, smith <deck_idx>, buy card|relic|potion <idx>, purge <deck_idx>, relic <idx>, proceed, cancel, quit"
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

fn parse_buy_command(rest: &[&str]) -> Result<RunControlCommand, String> {
    let kind = rest
        .first()
        .ok_or_else(|| "buy requires card|relic|potion".to_string())?
        .to_ascii_lowercase();
    let index = parse_usize_arg(rest.get(1), "shop index")?;
    match kind.as_str() {
        "card" => Ok(RunControlCommand::Input(ClientInput::BuyCard(index))),
        "relic" => Ok(RunControlCommand::Input(ClientInput::BuyRelic(index))),
        "potion" => Ok(RunControlCommand::Input(ClientInput::BuyPotion(index))),
        _ => Err("buy requires card|relic|potion".to_string()),
    }
}

fn parse_usize_arg(value: Option<&&str>, name: &str) -> Result<usize, String> {
    value
        .ok_or_else(|| format!("missing {name}"))?
        .parse::<usize>()
        .map_err(|_| format!("invalid {name} '{}'", value.unwrap()))
}

fn parse_optional_usize_arg(value: Option<&&str>, name: &str) -> Result<Option<usize>, String> {
    value
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| format!("invalid {name} '{value}'"))
        })
        .transpose()
}

fn parse_usize_list(values: &[&str], name: &str) -> Result<Vec<usize>, String> {
    values
        .iter()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| format!("invalid {name} '{value}'"))
        })
        .collect()
}

fn parse_u32_list(values: &[&str], name: &str) -> Result<Vec<u32>, String> {
    values
        .iter()
        .map(|value| {
            value
                .parse::<u32>()
                .map_err(|_| format!("invalid {name} '{value}'"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_control_parser_accepts_capture_label() {
        let parsed = parse_run_control_command("capture captures/jaw.json jaw worm start")
            .expect("capture command should parse");

        assert_eq!(
            parsed,
            RunControlCommand::Capture {
                path: PathBuf::from("captures/jaw.json"),
                label: Some("jaw worm start".to_string()),
            }
        );
    }

    #[test]
    fn run_control_parser_accepts_case_artifact_commands() {
        assert_eq!(
            parse_run_control_command("capture-case data/bench case_a first fight")
                .expect("capture-case should parse"),
            RunControlCommand::CaptureCase {
                root: PathBuf::from("data/bench"),
                case_id: "case_a".to_string(),
                label: Some("first fight".to_string()),
            }
        );
        assert_eq!(
            parse_run_control_command("save-baseline-case data/bench case_a")
                .expect("save-baseline-case should parse"),
            RunControlCommand::SaveBaselineCase {
                root: PathBuf::from("data/bench"),
                case_id: "case_a".to_string(),
            }
        );
        assert_eq!(
            parse_run_control_command("bench-add data/bench case_a")
                .expect("bench-add should parse"),
            RunControlCommand::RegisterBenchmarkCase {
                root: PathBuf::from("data/bench"),
                case_id: "case_a".to_string(),
            }
        );
    }
}
