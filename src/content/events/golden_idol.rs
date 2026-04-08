// Java: GoldenIdolEvent (exordium)
// Screen 0: [Take] → obtain Golden Idol relic + enter trap screen | [Leave]
// Screen 1 (trap): [Run] Obtain Injury curse | [Fight] Take damage | [Lose Max HP]
// Screen 2: [Leave]
//
// Java constructor calculates damage and maxHpLoss based on A15:
//   damage = (int)(maxHP * 0.25f) or 0.35f at A15+
//   maxHpLoss = (int)(maxHP * 0.08f) or 0.10f at A15+, min 1

use crate::content::cards::CardId;
use crate::content::relics::{RelicId, RelicState};
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

fn calc_damage(run_state: &RunState) -> i32 {
    if run_state.ascension_level >= 15 {
        (run_state.max_hp as f32 * 0.35) as i32
    } else {
        (run_state.max_hp as f32 * 0.25) as i32
    }
}

fn calc_max_hp_loss(run_state: &RunState) -> i32 {
    let loss = if run_state.ascension_level >= 15 {
        (run_state.max_hp as f32 * 0.10) as i32
    } else {
        (run_state.max_hp as f32 * 0.08) as i32
    };
    loss.max(1)
}

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            vec![
                EventChoiceMeta::new("[Take] Obtain Golden Idol."),
                EventChoiceMeta::new("[Leave]"),
            ]
        }
        1 => {
            // Trap triggered — pick your punishment
            let damage = calc_damage(run_state);
            let max_hp_loss = calc_max_hp_loss(run_state);
            vec![
                EventChoiceMeta::new("[Run] Obtain Injury curse."),
                EventChoiceMeta::new(format!("[Fight] Take {} damage.", damage)),
                EventChoiceMeta::new(format!("[Lose Max HP] Lose {} Max HP.", max_hp_loss)),
            ]
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Take: obtain Golden Idol, advance to trap screen
                    let relic_id = if run_state.relics.iter().any(|r| r.id == RelicId::GoldenIdol) {
                        RelicId::Circlet
                    } else {
                        RelicId::GoldenIdol
                    };
                    run_state.relics.push(RelicState::new(relic_id));
                    event_state.current_screen = 1;
                }
                _ => {
                    // Leave — don't take the idol
                    event_state.completed = true;
                }
            }
        }
        1 => {
            // Trap punishment
            match choice_idx {
                0 => {
                    // Run: obtain Injury curse
                    run_state.add_card_to_deck(CardId::Injury);
                }
                1 => {
                    // Fight: take damage (DEFAULT type — Tungsten Rod reduces by 1)
                    let mut damage = calc_damage(run_state);
                    if run_state
                        .relics
                        .iter()
                        .any(|r| r.id == RelicId::TungstenRod)
                    {
                        damage = (damage - 1).max(0);
                    }
                    run_state.current_hp = (run_state.current_hp - damage).max(0);
                }
                _ => {
                    // Lose Max HP
                    let max_hp_loss = calc_max_hp_loss(run_state);
                    run_state.max_hp = (run_state.max_hp - max_hp_loss).max(1);
                    if run_state.current_hp > run_state.max_hp {
                        run_state.current_hp = run_state.max_hp;
                    }
                }
            }
            event_state.current_screen = 2;
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
