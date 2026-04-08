use crate::content::relics::RelicId;
use crate::rewards::state::{RewardItem, RewardState};
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            // Intro: Read or Ignore
            vec![
                EventChoiceMeta::new("[Read] Begin reading the tome."),
                EventChoiceMeta::new("[Leave]"),
            ]
        }
        1 => vec![EventChoiceMeta::new("[Continue] Take 1 damage.")],
        2 => vec![EventChoiceMeta::new("[Continue] Take 2 damage.")],
        3 => vec![EventChoiceMeta::new("[Continue] Take 3 damage.")],
        4 => {
            let final_dmg = if run_state.ascension_level >= 15 {
                15
            } else {
                10
            };
            vec![
                EventChoiceMeta::new(format!(
                    "[Take the Book] Take {} damage. Obtain a Book relic.",
                    final_dmg
                )),
                EventChoiceMeta::new("[Stop Reading] Take 3 damage."),
            ]
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => match choice_idx {
            0 => {
                event_state.current_screen = 1;
            }
            _ => {
                event_state.completed = true;
            }
        },
        1 => {
            run_state.current_hp = (run_state.current_hp - 1).max(0);
            event_state.current_screen = 2;
        }
        2 => {
            run_state.current_hp = (run_state.current_hp - 2).max(0);
            event_state.current_screen = 3;
        }
        3 => {
            run_state.current_hp = (run_state.current_hp - 3).max(0);
            event_state.current_screen = 4;
        }
        4 => {
            match choice_idx {
                0 => {
                    // Take the book
                    let final_dmg = if run_state.ascension_level >= 15 {
                        15
                    } else {
                        10
                    };
                    run_state.current_hp = (run_state.current_hp - final_dmg).max(0);
                    // Random book relic (Java randomBook)
                    let book_relics = [
                        RelicId::Necronomicon,
                        RelicId::Enchiridion,
                        RelicId::NilrysCodex,
                    ];
                    let available: Vec<RelicId> = book_relics
                        .iter()
                        .copied()
                        .filter(|r| !run_state.relics.iter().any(|owned| owned.id == *r))
                        .collect();
                    let relic_id = if available.is_empty() {
                        RelicId::Circlet
                    } else {
                        let idx = run_state
                            .rng_pool
                            .misc_rng
                            .random_range(0, available.len() as i32 - 1)
                            as usize;
                        available[idx]
                    };
                    // Java: addRelicToRewards(r) + combatRewardScreen.open()
                    let mut rewards = RewardState::new();
                    rewards.items.push(RewardItem::Relic { relic_id });
                    event_state.current_screen = 5;
                    run_state.event_state = Some(event_state);
                    *engine_state = EngineState::RewardScreen(rewards);
                    return;
                }
                _ => {
                    // Stop reading: 3 damage
                    run_state.current_hp = (run_state.current_hp - 3).max(0);
                    event_state.current_screen = 5;
                }
            }
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
