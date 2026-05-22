use std::path::PathBuf;

use crate::ai::combat_search_v2::CombatSearchV2PotionPolicy;
use crate::state::core::{CampfireChoice, ClientInput};

use super::reward_auto::{parse_on_off, parse_reward_automation_target, RewardAutomationTarget};

#[derive(Clone, Debug, PartialEq)]
pub enum RunControlCommand {
    Noop,
    DefaultCandidate,
    Candidate(String),
    Help,
    Quit,
    Main,
    Deck,
    Map,
    Relics,
    Potions,
    Draw,
    Discard,
    Exhaust,
    Inspect(String),
    SaveDecisionCase {
        path: Option<PathBuf>,
    },
    Details,
    Raw,
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
    SearchCombat(RunControlSearchCombatOptions),
    AutoStep(RunControlAutoStepOptions),
    RewardAutomationStatus,
    SetRewardAutomation {
        target: RewardAutomationTarget,
        enabled: bool,
    },
    CardIndex(usize),
    RelicIndex(usize),
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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunControlSearchCombatOptions {
    pub max_nodes: Option<usize>,
    pub max_actions_per_line: Option<usize>,
    pub max_engine_steps_per_action: Option<usize>,
    pub wall_ms: Option<u64>,
    pub potion_policy: Option<CombatSearchV2PotionPolicy>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunControlAutoStepOptions {
    pub search: RunControlSearchCombatOptions,
    pub max_operations: Option<usize>,
}

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
        "map" => Ok(RunControlCommand::Map),
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
        "skip" => Ok(RunControlCommand::Candidate("skip".to_string())),
        "leave" => Ok(RunControlCommand::Candidate("leave".to_string())),
        "d" | "details" => Ok(RunControlCommand::Details),
        "r" | "raw" => Ok(RunControlCommand::Raw),
        "actions" | "legal" => Ok(RunControlCommand::Actions),
        "capture" | "save-capture" => parse_capture_command(&rest),
        "capture-case" => parse_capture_case_command(&rest),
        "save-baseline" => parse_save_baseline_command(&rest),
        "save-baseline-case" => parse_save_baseline_case_command(&rest),
        "bench-add" => parse_bench_add_command(&rest),
        "sc" | "search-combat" | "solve-combat" | "auto-combat" => {
            parse_search_combat_command(&rest)
        }
        "n" | "next" | "advance" | "advance-to-human-boundary" | "auto-step" | "autostep" => {
            parse_auto_step_command(&rest)
        }
        "auto-reward" => parse_auto_reward_command(&rest),
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
        "card" => Ok(RunControlCommand::CardIndex(parse_usize_arg(
            rest.first(),
            "card index",
        )?)),
        "pick" | "select-card" => Ok(RunControlCommand::Input(ClientInput::SelectCard(
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

pub fn run_control_help() -> &'static str {
    "\
Help:
  Core:
    main/state, deck, map, relics, potions, inspect <id>, case [path], d/details, r/raw, quit
    n/next = advance to next human choice; <id> chooses a visible option
    Enter chooses the single visible option when safe

  Combat:
    play <hand_idx> [target_slot], end, potion <slot> [target_slot], discard-potion <slot>
    draw, discard, exhaust, actions, action <idx>
    sc/search-combat [max_nodes=N] [wall_ms=N] [potion=never|all]

  Map/Event/Reward:
    go <x>, fly <x> <y>, event <idx>, claim <idx>, pick <idx>, select <deck_idx...>
    hand-select <uuid...>, grid-select <uuid...>, choose <idx>, open, relic <idx>

  Shop/Campfire:
    buy card|relic|potion <idx>, purge <deck_idx>, rest, smith <deck_idx>, dig, lift, recall, toke <deck_idx>

  Combat Capture / Benchmark:
    capture <path> [label]
    capture-case <benchmark_dir> <case_id> [label]
    save-baseline <path> [case_id]
    save-baseline-case <benchmark_dir> <case_id>
    bench-add <benchmark_dir> <case_id>

  Automation:
    n/next/advance-to-human-boundary [max_nodes=N] [wall_ms=N] [potion=never|all] [max_ops=N]
    auto-reward
    auto-reward gold|potion|all on|off"
}

pub fn run_control_short_hint() -> &'static str {
    "main | n=advance | deck | map | relics | potions | inspect <id> | auto-reward | details | raw | help"
}

fn is_candidate_id(command: &str) -> bool {
    command.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
        && command.chars().any(|ch| ch.is_ascii_digit())
}

fn is_structured_candidate_id(command: &str) -> bool {
    let Some((prefix, suffix)) = command.split_once('-') else {
        return false;
    };
    matches!(prefix, "card" | "relic" | "potion" | "smith")
        && !suffix.is_empty()
        && suffix.chars().all(|ch| ch.is_ascii_digit())
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

fn parse_search_combat_command(rest: &[&str]) -> Result<RunControlCommand, String> {
    Ok(RunControlCommand::SearchCombat(
        parse_search_combat_options(rest)?,
    ))
}

fn parse_search_combat_options(rest: &[&str]) -> Result<RunControlSearchCombatOptions, String> {
    let mut options = RunControlSearchCombatOptions::default();
    for token in rest {
        let (key, value) = token
            .split_once('=')
            .ok_or_else(|| format!("search-combat option must be key=value, got '{token}'"))?;
        match key.to_ascii_lowercase().as_str() {
            "max_nodes" | "nodes" => {
                options.max_nodes = Some(parse_usize_value(value, "max_nodes")?);
            }
            "max_actions" | "max_actions_per_line" => {
                options.max_actions_per_line =
                    Some(parse_usize_value(value, "max_actions_per_line")?);
            }
            "max_steps" | "max_engine_steps_per_action" => {
                options.max_engine_steps_per_action =
                    Some(parse_usize_value(value, "max_engine_steps_per_action")?);
            }
            "wall_ms" | "ms" => {
                options.wall_ms = Some(parse_u64_value(value, "wall_ms")?);
            }
            "potion" | "potion_policy" => {
                options.potion_policy = Some(parse_potion_policy(value)?);
            }
            other => return Err(format!("unknown search-combat option '{other}'")),
        }
    }
    Ok(options)
}

fn parse_auto_step_command(rest: &[&str]) -> Result<RunControlCommand, String> {
    let mut options = RunControlAutoStepOptions::default();
    let mut search_tokens = Vec::new();
    for token in rest {
        let Some((key, value)) = token.split_once('=') else {
            return Err(format!("advance option must be key=value, got '{token}'"));
        };
        match key.to_ascii_lowercase().as_str() {
            "max_ops" | "max_operations" | "max_steps" => {
                options.max_operations = Some(parse_usize_value(value, "max_ops")?);
            }
            _ => search_tokens.push(*token),
        }
    }
    options.search = parse_search_combat_options(&search_tokens)?;
    Ok(RunControlCommand::AutoStep(options))
}

fn parse_auto_reward_command(rest: &[&str]) -> Result<RunControlCommand, String> {
    match rest {
        [] | ["status"] => Ok(RunControlCommand::RewardAutomationStatus),
        [target, enabled] => Ok(RunControlCommand::SetRewardAutomation {
            target: parse_reward_automation_target(target)?,
            enabled: parse_on_off(enabled)?,
        }),
        _ => Err("auto-reward expects no args or: auto-reward gold|potion|all on|off".to_string()),
    }
}

fn parse_potion_policy(value: &str) -> Result<CombatSearchV2PotionPolicy, String> {
    match value.to_ascii_lowercase().as_str() {
        "never" => Ok(CombatSearchV2PotionPolicy::Never),
        "all" | "all_legal_potion_actions" => Ok(CombatSearchV2PotionPolicy::All),
        _ => Err(format!(
            "invalid potion policy '{value}', expected never|all"
        )),
    }
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

fn parse_usize_value(value: &str, name: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("invalid {name} '{value}'"))
}

fn parse_u64_value(value: &str, name: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("invalid {name} '{value}'"))
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

    #[test]
    fn run_control_parser_accepts_search_combat_options() {
        assert_eq!(
            parse_run_control_command("search-combat max_nodes=123 wall_ms=50 potion=all")
                .expect("search-combat should parse"),
            RunControlCommand::SearchCombat(RunControlSearchCombatOptions {
                max_nodes: Some(123),
                max_actions_per_line: None,
                max_engine_steps_per_action: None,
                wall_ms: Some(50),
                potion_policy: Some(CombatSearchV2PotionPolicy::All),
            })
        );
        assert_eq!(
            parse_run_control_command("sc").expect("sc should parse"),
            RunControlCommand::SearchCombat(RunControlSearchCombatOptions::default())
        );
    }

    #[test]
    fn run_control_parser_accepts_auto_step_options() {
        assert_eq!(
            parse_run_control_command("n").expect("n should parse"),
            RunControlCommand::AutoStep(RunControlAutoStepOptions::default())
        );
        assert_eq!(
            parse_run_control_command("advance-to-human-boundary")
                .expect("long advance command should parse"),
            RunControlCommand::AutoStep(RunControlAutoStepOptions::default())
        );
        assert_eq!(
            parse_run_control_command("auto-step max_nodes=123 wall_ms=50 max_ops=9")
                .expect("auto-step should parse"),
            RunControlCommand::AutoStep(RunControlAutoStepOptions {
                search: RunControlSearchCombatOptions {
                    max_nodes: Some(123),
                    max_actions_per_line: None,
                    max_engine_steps_per_action: None,
                    wall_ms: Some(50),
                    potion_policy: None,
                },
                max_operations: Some(9),
            })
        );
    }

    #[test]
    fn run_control_parser_accepts_auto_reward_settings() {
        assert_eq!(
            parse_run_control_command("auto-reward").expect("auto-reward should parse"),
            RunControlCommand::RewardAutomationStatus
        );
        assert_eq!(
            parse_run_control_command("auto-reward potion off")
                .expect("auto-reward setting should parse"),
            RunControlCommand::SetRewardAutomation {
                target: RewardAutomationTarget::Potion,
                enabled: false,
            }
        );
    }

    #[test]
    fn run_control_parser_accepts_visible_non_numeric_ids() {
        assert_eq!(
            parse_run_control_command("card-2").expect("shop card id should parse"),
            RunControlCommand::Candidate("card-2".to_string())
        );
        assert_eq!(
            parse_run_control_command("relic-1").expect("shop relic id should parse"),
            RunControlCommand::Candidate("relic-1".to_string())
        );
        assert_eq!(
            parse_run_control_command("potion-0").expect("shop potion id should parse"),
            RunControlCommand::Candidate("potion-0".to_string())
        );
        assert_eq!(
            parse_run_control_command("smith-8").expect("campfire smith id should parse"),
            RunControlCommand::Candidate("smith-8".to_string())
        );
        assert_eq!(
            parse_run_control_command("leave").expect("leave id should parse"),
            RunControlCommand::Candidate("leave".to_string())
        );
        assert_eq!(
            parse_run_control_command("purge").expect("purge candidate should parse"),
            RunControlCommand::Candidate("purge".to_string())
        );
    }

    #[test]
    fn run_control_parser_accepts_contextual_shop_words() {
        assert_eq!(
            parse_run_control_command("card 2").expect("card index should parse"),
            RunControlCommand::CardIndex(2)
        );
        assert_eq!(
            parse_run_control_command("relic 1").expect("relic index should parse"),
            RunControlCommand::RelicIndex(1)
        );
    }

    #[test]
    fn run_control_parser_accepts_view_commands() {
        assert_eq!(
            parse_run_control_command("h").expect("h should parse"),
            RunControlCommand::Help
        );
        assert_eq!(
            parse_run_control_command("").expect("enter should parse"),
            RunControlCommand::DefaultCandidate
        );
        assert_eq!(
            parse_run_control_command("0").expect("candidate id should parse"),
            RunControlCommand::Candidate("0".to_string())
        );
        assert_eq!(
            parse_run_control_command("deck").expect("deck should parse"),
            RunControlCommand::Deck
        );
        assert_eq!(
            parse_run_control_command("d").expect("d should parse"),
            RunControlCommand::Details
        );
        assert_eq!(
            parse_run_control_command("raw").expect("raw should parse"),
            RunControlCommand::Raw
        );
        assert_eq!(
            parse_run_control_command("case").expect("case should parse"),
            RunControlCommand::SaveDecisionCase { path: None }
        );
    }
}
