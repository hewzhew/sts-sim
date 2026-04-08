use crate::content::relics::{RelicId, RelicState};
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

// internal_state encodes: lower 16 bits = relic_chance, upper 16 bits = current_dmg

fn decode_state(state: i32) -> (i32, i32) {
    let chance = state & 0xFFFF;
    let dmg = (state >> 16) & 0xFFFF;
    (chance, dmg)
}

fn encode_state(chance: i32, dmg: i32) -> i32 {
    (dmg << 16) | (chance & 0xFFFF)
}

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    if event_state.current_screen == 1 {
        return vec![EventChoiceMeta::new("[Leave]")];
    }

    let (chance, dmg) = if event_state.internal_state == 0 {
        // Initial state
        let base_dmg = if run_state.ascension_level >= 15 {
            5
        } else {
            3
        };
        (25, base_dmg)
    } else {
        decode_state(event_state.internal_state)
    };

    vec![
        EventChoiceMeta::new(format!(
            "[Reach In] Take {} damage. {}% chance to obtain a Relic.",
            dmg, chance
        )),
        EventChoiceMeta::new("[Leave]"),
    ]
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Reach In
                    let (mut chance, mut dmg) = if event_state.internal_state == 0 {
                        let base_dmg = if run_state.ascension_level >= 15 {
                            5
                        } else {
                            3
                        };
                        (25, base_dmg)
                    } else {
                        decode_state(event_state.internal_state)
                    };

                    // Take damage (DEFAULT type — Tungsten Rod reduces by 1)
                    let mut actual_dmg = dmg;
                    if run_state
                        .relics
                        .iter()
                        .any(|r| r.id == RelicId::TungstenRod)
                    {
                        actual_dmg = (actual_dmg - 1).max(0);
                    }
                    run_state.current_hp = (run_state.current_hp - actual_dmg).max(0);

                    // Roll for relic
                    let roll = run_state.rng_pool.misc_rng.random_range(0, 99);
                    if roll >= 99 - chance {
                        // Success! Get a relic
                        let relic_id = run_state.random_relic();
                        run_state.relics.push(RelicState::new(relic_id));
                        event_state.current_screen = 1;
                    } else {
                        // Fail: escalate
                        chance += 10;
                        dmg += 1;
                        event_state.internal_state = encode_state(chance, dmg);
                        // Stay on screen 0
                    }
                }
                _ => {
                    // Flee
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
