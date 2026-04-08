use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

const DAMAGE: i32 = 7;
const MIN_GOLD: i32 = 50;
const MAX_GOLD: i32 = 80;
const REQUIRED_DAMAGE: i32 = 10;

fn has_high_damage_card(run_state: &RunState) -> bool {
    run_state.master_deck.iter().any(|c| {
        let def = crate::content::cards::get_card_definition(c.id);
        def.base_damage >= REQUIRED_DAMAGE
    })
}

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    if event_state.current_screen >= 1 {
        return vec![EventChoiceMeta::new("[Leave]")];
    }

    let can_attack = has_high_damage_card(run_state);
    let mut choices = vec![EventChoiceMeta::new(format!(
        "[Remove a card] Take {} damage. Remove a card from your deck.",
        DAMAGE
    ))];

    if can_attack {
        choices.push(EventChoiceMeta::new(format!(
            "[Attack] Gain {}-{} Gold.",
            MIN_GOLD, MAX_GOLD
        )));
    } else {
        choices.push(EventChoiceMeta::disabled(
            format!(
                "[Attack] Requires an Attack card with ≥{} damage.",
                REQUIRED_DAMAGE
            ),
            "No qualifying attack card.",
        ));
    }

    choices.push(EventChoiceMeta::new("[Leave]"));
    choices
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Remove card: take damage, then purge
                    run_state.current_hp = (run_state.current_hp - DAMAGE).max(0);
                    event_state.current_screen = 1;
                    run_state.event_state = Some(event_state);
                    *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                        reason: RunPendingChoiceReason::Purge,
                        min_choices: 1,
                        max_choices: 1,
                        return_state: Box::new(EngineState::EventRoom),
                    });
                    return;
                }
                1 => {
                    // Attack: gain gold
                    if has_high_damage_card(run_state) {
                        let gold = run_state.rng_pool.misc_rng.random_range(MIN_GOLD, MAX_GOLD);
                        run_state.gold += gold;
                    }
                    event_state.current_screen = 1;
                }
                _ => {
                    // Leave
                    event_state.current_screen = 1;
                }
            }
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
