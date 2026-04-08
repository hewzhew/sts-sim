// Java: Bonfire (shrines) — "Bonfire Elementals"
// Screen 0 (INTRO): [Approach] → Screen 1
// Screen 1 (CHOOSE): [Offer card] → grid-select to sacrifice a card
// After grid-select returns to screen 2: reward based on offered card's rarity
//   (rarity stored in internal_state by Purge handler in run_loop.rs)
//   Curse → SpiritPoop relic (Circlet if already owned)
//   Basic → nothing
//   Common/Special → heal 5
//   Uncommon → heal to full
//   Rare → +10 maxHP + heal to full
// Screen 3 (COMPLETE): [Leave]

use crate::content::relics::{RelicId, RelicState};
use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => vec![EventChoiceMeta::new("[Approach] Investigate the bonfire.")],
        1 => {
            let has_removable = !run_state.master_deck.is_empty();
            if has_removable {
                vec![EventChoiceMeta::new(
                    "[Offer] Sacrifice a card to the spirits.",
                )]
            } else {
                vec![EventChoiceMeta::disabled(
                    "[Offer] No cards to sacrifice.",
                    "No purgeable cards",
                )]
            }
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, _choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            // Approach → go to card sacrifice screen
            event_state.current_screen = 1;
        }
        1 => {
            // Sacrifice a card via grid-select.
            // The Purge handler in run_loop.rs stores the removed card's rarity
            // in event_state.internal_state before removal.
            event_state.current_screen = 2;
            run_state.event_state = Some(event_state);
            *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                min_choices: 1,
                max_choices: 1,
                reason: RunPendingChoiceReason::Purge,
                return_state: Box::new(EngineState::EventRoom),
            });
            return;
        }
        2 => {
            // Returned from purge. Read rarity from internal_state
            // (set by Purge handler: 0=Curse, 1=Basic, 2=Common, 3=Special, 4=Uncommon, 5=Rare)
            let rarity = event_state.internal_state;
            match rarity {
                0 => {
                    // Curse → SpiritPoop relic (Circlet if already owned)
                    let relic_id = if run_state.relics.iter().any(|r| r.id == RelicId::SpiritPoop) {
                        RelicId::Circlet
                    } else {
                        RelicId::SpiritPoop
                    };
                    run_state.relics.push(RelicState::new(relic_id));
                }
                1 => {
                    // Basic → nothing
                }
                2 | 3 => {
                    // Common / Special → heal 5
                    run_state.current_hp = (run_state.current_hp + 5).min(run_state.max_hp);
                }
                4 => {
                    // Uncommon → heal to full
                    run_state.current_hp = run_state.max_hp;
                }
                5 => {
                    // Rare → +10 maxHP + heal to full
                    run_state.max_hp += 10;
                    run_state.current_hp = run_state.max_hp;
                }
                _ => {}
            }
            event_state.current_screen = 3;
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
