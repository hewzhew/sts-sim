// Java: Bonfire (shrines) — "Bonfire Elementals" / "Bonfire Spirits"
// This event is a duplicate/variant of bonfire_elementals. Both correspond
// to Java's single Bonfire.java (ID: "Bonfire Elementals").
//
// Screen 0: [Approach] → Screen 1
// Screen 1: [Offer] → grid-select (Purge) → screen 2
// Screen 2: reward based on removed card's rarity (read from internal_state)
// Screen 3: [Leave]

use crate::content::relics::{RelicId, RelicState};
use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(_run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => vec![EventChoiceMeta::new("[Approach]")],
        1 => {
            // Offer a card to the bonfire
            vec![EventChoiceMeta::new("[Offer] Select a card to offer.")]
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, _choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            event_state.current_screen = 1;
        }
        1 => {
            // Transition to RunPendingChoice::Purge to select a card.
            // The Purge handler stores the removed card's rarity in
            // event_state.internal_state before removal.
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
            // Post-purge: apply rarity-based reward from internal_state
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
