use crate::combat::CombatState;
use crate::state::core::{CampfireChoice, ClientInput, EngineState};
use crate::state::run::RunState;
use crate::state::selection::{SelectionResolution, SelectionScope, SelectionTargetRef};

pub fn parse_input(
    line: &str,
    es: &EngineState,
    rs: &RunState,
    _cs: &Option<CombatState>,
) -> Option<ClientInput> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    match parts[0] {
        "play" | "p" => {
            let idx: usize = parts.get(1)?.parse().ok()?;
            let target = parts
                .get(2)
                .and_then(|s| s.parse::<usize>().ok())
                .and_then(|t_idx| {
                    _cs.as_ref()
                        .and_then(|combat| combat.entities.monsters.get(t_idx).map(|m| m.id))
                });
            Some(ClientInput::PlayCard {
                card_index: idx,
                target,
            })
        }
        "end" | "e" => Some(ClientInput::EndTurn),
        "potion" => {
            let slot: usize = parts.get(1)?.parse().ok()?;
            let target = parts
                .get(2)
                .and_then(|s| s.parse::<usize>().ok())
                .and_then(|t_idx| {
                    _cs.as_ref()
                        .and_then(|combat| combat.entities.monsters.get(t_idx).map(|m| m.id))
                });
            Some(ClientInput::UsePotion {
                potion_index: slot,
                target,
            })
        }
        "go" => {
            let x: usize = parts.get(1)?.parse().ok()?;
            Some(ClientInput::SelectMapNode(
                crate::cli::display::normalize_map_choice_x(rs, x),
            ))
        }
        "rest" => Some(ClientInput::CampfireOption(CampfireChoice::Rest)),
        "smith" => {
            let idx: usize = parts.get(1)?.parse().ok()?;
            Some(ClientInput::CampfireOption(CampfireChoice::Smith(idx)))
        }
        "claim" => {
            let idx: usize = parts.get(1)?.parse().ok()?;
            Some(ClientInput::ClaimReward(idx))
        }
        "pick" => {
            let idx: usize = parts.get(1)?.parse().ok()?;
            Some(ClientInput::SelectCard(idx))
        }
        "proceed" | "leave" | "skip" => Some(ClientInput::Proceed),
        "relic" => {
            let idx: usize = parts.get(1)?.parse().ok()?;
            Some(ClientInput::SubmitRelicChoice(idx))
        }
        "cancel" => Some(ClientInput::Cancel),
        "select" => {
            let indices: Vec<usize> = parts[1..].iter().filter_map(|s| s.parse().ok()).collect();
            match es {
                EngineState::RunPendingChoice(_) => {
                    Some(ClientInput::SubmitSelection(SelectionResolution {
                        scope: SelectionScope::Deck,
                        selected: indices
                            .into_iter()
                            .filter_map(|idx| rs.master_deck.get(idx))
                            .map(|card| SelectionTargetRef::CardUuid(card.uuid))
                            .collect(),
                    }))
                }
                _ => Some(ClientInput::SubmitDeckSelect(indices)),
            }
        }
        "choose" => {
            let indices: Vec<usize> = parts[1..].iter().filter_map(|s| s.parse().ok()).collect();
            match es {
                EngineState::PendingChoice(crate::state::core::PendingChoice::HandSelect {
                    candidate_uuids,
                    ..
                }) => Some(ClientInput::SubmitSelection(SelectionResolution {
                    scope: SelectionScope::Hand,
                    selected: indices
                        .iter()
                        .filter_map(|&i| candidate_uuids.get(i).copied())
                        .map(SelectionTargetRef::CardUuid)
                        .collect(),
                })),
                EngineState::PendingChoice(crate::state::core::PendingChoice::GridSelect {
                    candidate_uuids,
                    ..
                }) => Some(ClientInput::SubmitSelection(SelectionResolution {
                    scope: SelectionScope::Grid,
                    selected: indices
                        .iter()
                        .filter_map(|&i| candidate_uuids.get(i).copied())
                        .map(SelectionTargetRef::CardUuid)
                        .collect(),
                })),
                _ => None,
            }
        }
        "buy" => match parts.get(1)? {
            &"card" => {
                let idx: usize = parts.get(2)?.parse().ok()?;
                Some(ClientInput::BuyCard(idx))
            }
            &"relic" => {
                let idx: usize = parts.get(2)?.parse().ok()?;
                Some(ClientInput::BuyRelic(idx))
            }
            &"potion" => {
                let idx: usize = parts.get(2)?.parse().ok()?;
                Some(ClientInput::BuyPotion(idx))
            }
            _ => None,
        },
        "purge" => {
            let idx: usize = parts.get(1)?.parse().ok()?;
            Some(ClientInput::PurgeCard(idx))
        }
        // Numeric input — context-dependent
        _ => {
            if let Ok(idx) = parts[0].parse::<usize>() {
                match es {
                    EngineState::EventRoom => Some(ClientInput::EventChoice(idx)),
                    EngineState::MapNavigation => Some(ClientInput::SelectMapNode(
                        crate::cli::display::normalize_map_choice_x(rs, idx),
                    )),
                    _ => None,
                }
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_input;
    use crate::state::core::{ClientInput, EngineState};
    use crate::state::run::RunState;

    #[test]
    fn boss_map_input_normalizes_to_single_choice() {
        let mut run = RunState::new(41, 0, false, "Ironclad");
        run.map.current_y = 14;
        run.map.current_x = 0;

        assert_eq!(
            parse_input("go 6", &EngineState::MapNavigation, &run, &None),
            Some(ClientInput::SelectMapNode(0))
        );
        assert_eq!(
            parse_input("6", &EngineState::MapNavigation, &run, &None),
            Some(ClientInput::SelectMapNode(0))
        );
    }
}

pub fn print_help() {
    println!("Commands:");
    println!("  COMBAT:     play <idx> [target]  |  end  |  potion <slot> [target]");
    println!("  MAP:        go <x>  |  <number>");
    println!("  EVENT:      <number> to choose option");
    println!("  REWARD:     claim <idx>  |  pick <idx>  |  proceed");
    println!("  B_RELIC:    relic <idx>  |  skip");
    println!("  CAMPFIRE:   rest  |  smith <deck_idx>");
    println!("  SHOP:       buy card/relic/potion <idx>  |  purge <deck_idx>  |  leave");
    println!("  DECK SEL:   select <idx1> <idx2> ...  |  cancel");
    println!("  PENDING:    choose <idx1> <idx2> ...  |  cancel");
    println!("  INSPECT:    relics  |  potions  |  draw  |  discard  |  exhaust  |  state");
    println!("  BOT STEP:   a  |  step (bot acts once)");
    println!("  MODE:       auto  |  auto run  |  manual  |  skip  |  fast");
    println!("  OTHER:      help  |  quit");
    println!();
    println!("Modes:");
    println!("  auto       — bot plays everything automatically");
    println!("  auto run   — bot plays + quiet mode (minimal output)");
    println!("  manual     — you control everything (default)");
    println!("  skip       — bot finishes current combat, then returns to manual");
    println!("  fast       — toggle quiet mode (suppress per-card combat output)");
}
