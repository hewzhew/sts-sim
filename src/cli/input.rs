use crate::state::core::{EngineState, ClientInput, CampfireChoice};
use crate::combat::CombatState;

pub fn parse_input(line: &str, es: &EngineState, _cs: &Option<CombatState>) -> Option<ClientInput> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() { return None; }

    match parts[0] {
        "play" | "p" => {
            let idx: usize = parts.get(1)?.parse().ok()?;
            let target = parts.get(2).and_then(|s| s.parse::<usize>().ok()).and_then(|t_idx| {
                _cs.as_ref().and_then(|combat| {
                    combat.monsters.get(t_idx).map(|m| m.id)
                })
            });
            Some(ClientInput::PlayCard { card_index: idx, target })
        },
        "end" | "e" => Some(ClientInput::EndTurn),
        "potion" => {
            let slot: usize = parts.get(1)?.parse().ok()?;
            let target = parts.get(2).and_then(|s| s.parse::<usize>().ok());
            Some(ClientInput::UsePotion { potion_index: slot, target })
        },
        "go" => {
            let x: usize = parts.get(1)?.parse().ok()?;
            Some(ClientInput::SelectMapNode(x))
        },
        "rest" => Some(ClientInput::CampfireOption(CampfireChoice::Rest)),
        "smith" => {
            let idx: usize = parts.get(1)?.parse().ok()?;
            Some(ClientInput::CampfireOption(CampfireChoice::Smith(idx)))
        },
        "claim" => {
            let idx: usize = parts.get(1)?.parse().ok()?;
            Some(ClientInput::ClaimReward(idx))
        },
        "pick" => {
            let idx: usize = parts.get(1)?.parse().ok()?;
            Some(ClientInput::SelectCard(idx))
        },
        "proceed" | "leave" | "skip" => Some(ClientInput::Proceed),
        "relic" => {
            let idx: usize = parts.get(1)?.parse().ok()?;
            Some(ClientInput::SubmitRelicChoice(idx))
        },
        "cancel" => Some(ClientInput::Cancel),
        "select" => {
            let indices: Vec<usize> = parts[1..].iter()
                .filter_map(|s| s.parse().ok())
                .collect();
            Some(ClientInput::SubmitDeckSelect(indices))
        },
        "choose" => {
            let indices: Vec<usize> = parts[1..].iter()
                .filter_map(|s| s.parse().ok())
                .collect();
            match es {
                EngineState::PendingChoice(crate::state::core::PendingChoice::HandSelect { .. }) => {
                    if let Some(cs) = _cs {
                        let uuids: Vec<u32> = indices.iter()
                            .filter_map(|&i| cs.hand.get(i).map(|c| c.uuid))
                            .collect();
                        Some(ClientInput::SubmitHandSelect(uuids))
                    } else { None }
                },
                _ => None,
            }
        },
        "buy" => {
            match parts.get(1)? {
                &"card" => {
                    let idx: usize = parts.get(2)?.parse().ok()?;
                    Some(ClientInput::BuyCard(idx))
                },
                &"relic" => {
                    let idx: usize = parts.get(2)?.parse().ok()?;
                    Some(ClientInput::BuyRelic(idx))
                },
                &"potion" => {
                    let idx: usize = parts.get(2)?.parse().ok()?;
                    Some(ClientInput::BuyPotion(idx))
                },
                _ => None,
            }
        },
        "purge" => {
            let idx: usize = parts.get(1)?.parse().ok()?;
            Some(ClientInput::PurgeCard(idx))
        },
        // Numeric input — context-dependent
        _ => {
            if let Ok(idx) = parts[0].parse::<usize>() {
                match es {
                    EngineState::EventRoom => Some(ClientInput::EventChoice(idx)),
                    EngineState::MapNavigation => Some(ClientInput::SelectMapNode(idx)),
                    _ => None,
                }
            } else {
                None
            }
        }
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
