use std::path::PathBuf;

use crate::state::core::{CampfireChoice, ClientInput};

#[derive(Clone, Debug, PartialEq)]
pub enum RunPlayCommand {
    Noop,
    Help,
    Quit,
    State,
    Actions,
    Capture {
        path: PathBuf,
        label: Option<String>,
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

pub fn parse_run_play_command(line: &str) -> Result<RunPlayCommand, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(RunPlayCommand::Noop);
    }

    let mut parts = trimmed.split_whitespace();
    let Some(command) = parts.next() else {
        return Ok(RunPlayCommand::Noop);
    };
    let rest = parts.collect::<Vec<_>>();

    match command.to_ascii_lowercase().as_str() {
        "?" | "help" => Ok(RunPlayCommand::Help),
        "q" | "quit" | "exit" => Ok(RunPlayCommand::Quit),
        "state" => Ok(RunPlayCommand::State),
        "actions" | "legal" => Ok(RunPlayCommand::Actions),
        "capture" | "save-capture" => parse_capture_command(&rest),
        "action" => Ok(RunPlayCommand::ActionIndex(parse_usize_arg(
            rest.first(),
            "action index",
        )?)),
        "play" => Ok(RunPlayCommand::PlayCard {
            card_index: parse_usize_arg(rest.first(), "hand card index")?,
            target_slot_or_id: parse_optional_usize_arg(rest.get(1), "target slot")?,
        }),
        "end" => Ok(RunPlayCommand::Input(ClientInput::EndTurn)),
        "potion" => Ok(RunPlayCommand::UsePotion {
            potion_index: parse_usize_arg(rest.first(), "potion slot")?,
            target_slot_or_id: parse_optional_usize_arg(rest.get(1), "target slot")?,
        }),
        "discard-potion" => Ok(RunPlayCommand::Input(ClientInput::DiscardPotion(
            parse_usize_arg(rest.first(), "potion slot")?,
        ))),
        "go" => Ok(RunPlayCommand::Input(ClientInput::SelectMapNode(
            parse_usize_arg(rest.first(), "map x")?,
        ))),
        "fly" => Ok(RunPlayCommand::Input(ClientInput::FlyToNode(
            parse_usize_arg(rest.first(), "map x")?,
            parse_usize_arg(rest.get(1), "map y")?,
        ))),
        "event" | "option" => Ok(RunPlayCommand::Input(ClientInput::EventChoice(
            parse_usize_arg(rest.first(), "event option index")?,
        ))),
        "claim" => Ok(RunPlayCommand::Input(ClientInput::ClaimReward(
            parse_usize_arg(rest.first(), "reward index")?,
        ))),
        "pick" | "card" | "select-card" => Ok(RunPlayCommand::Input(ClientInput::SelectCard(
            parse_usize_arg(rest.first(), "card option index")?,
        ))),
        "select" => Ok(RunPlayCommand::Input(ClientInput::SubmitDeckSelect(
            parse_usize_list(&rest, "deck index")?,
        ))),
        "hand-select" => Ok(RunPlayCommand::Input(ClientInput::SubmitHandSelect(
            parse_u32_list(&rest, "hand card uuid")?,
        ))),
        "grid-select" => Ok(RunPlayCommand::Input(ClientInput::SubmitGridSelect(
            parse_u32_list(&rest, "grid card uuid")?,
        ))),
        "choose" => Ok(RunPlayCommand::Input(ClientInput::SubmitDiscoverChoice(
            parse_usize_arg(rest.first(), "choice index")?,
        ))),
        "proceed" => Ok(RunPlayCommand::Input(ClientInput::Proceed)),
        "cancel" => Ok(RunPlayCommand::Input(ClientInput::Cancel)),
        "open" | "chest" => Ok(RunPlayCommand::Input(ClientInput::OpenChest)),
        "rest" => Ok(RunPlayCommand::Input(ClientInput::CampfireOption(
            CampfireChoice::Rest,
        ))),
        "smith" => Ok(RunPlayCommand::Input(ClientInput::CampfireOption(
            CampfireChoice::Smith(parse_usize_arg(rest.first(), "deck index")?),
        ))),
        "dig" => Ok(RunPlayCommand::Input(ClientInput::CampfireOption(
            CampfireChoice::Dig,
        ))),
        "lift" => Ok(RunPlayCommand::Input(ClientInput::CampfireOption(
            CampfireChoice::Lift,
        ))),
        "recall" => Ok(RunPlayCommand::Input(ClientInput::CampfireOption(
            CampfireChoice::Recall,
        ))),
        "toke" => Ok(RunPlayCommand::Input(ClientInput::CampfireOption(
            CampfireChoice::Toke(parse_usize_arg(rest.first(), "deck index")?),
        ))),
        "buy" => parse_buy_command(&rest),
        "purge" => Ok(RunPlayCommand::Input(ClientInput::PurgeCard(
            parse_usize_arg(rest.first(), "deck index")?,
        ))),
        "relic" => Ok(RunPlayCommand::Input(ClientInput::SubmitRelicChoice(
            parse_usize_arg(rest.first(), "boss relic index")?,
        ))),
        other => Err(format!("unknown run/play command '{other}'")),
    }
}

pub fn run_play_help() -> &'static str {
    "commands: state, actions, action <idx>, capture <path> [label], play <hand_idx> [target_slot], end, potion <slot> [target_slot], discard-potion <slot>, go <x>, fly <x> <y>, event <idx>, claim <idx>, pick <idx>, select <deck_idx...>, hand-select <uuid...>, grid-select <uuid...>, open, rest, smith <deck_idx>, buy card|relic|potion <idx>, purge <deck_idx>, relic <idx>, proceed, cancel, quit"
}

fn parse_capture_command(rest: &[&str]) -> Result<RunPlayCommand, String> {
    let path = rest
        .first()
        .ok_or_else(|| "capture requires an output path".to_string())?;
    let label = (!rest[1.min(rest.len())..].is_empty()).then(|| rest[1..].join(" "));
    Ok(RunPlayCommand::Capture {
        path: PathBuf::from(path),
        label,
    })
}

fn parse_buy_command(rest: &[&str]) -> Result<RunPlayCommand, String> {
    let kind = rest
        .first()
        .ok_or_else(|| "buy requires card|relic|potion".to_string())?
        .to_ascii_lowercase();
    let index = parse_usize_arg(rest.get(1), "shop index")?;
    match kind.as_str() {
        "card" => Ok(RunPlayCommand::Input(ClientInput::BuyCard(index))),
        "relic" => Ok(RunPlayCommand::Input(ClientInput::BuyRelic(index))),
        "potion" => Ok(RunPlayCommand::Input(ClientInput::BuyPotion(index))),
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
    fn run_play_parser_accepts_capture_label() {
        let parsed = parse_run_play_command("capture captures/jaw.json jaw worm start")
            .expect("capture command should parse");

        assert_eq!(
            parsed,
            RunPlayCommand::Capture {
                path: PathBuf::from("captures/jaw.json"),
                label: Some("jaw worm start".to_string()),
            }
        );
    }
}
