use crate::content::cards::CardId;
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => vec![EventChoiceMeta::new("[Explore]")],
        1 => {
            let gold_gain = if run_state.ascension_level >= 15 {
                50
            } else {
                99
            };
            vec![
                EventChoiceMeta::new(format!("[Steal] Gain {} Gold.", gold_gain)),
                EventChoiceMeta::new("[Join] Take 6 damage. Obtain Ritual Dagger."),
            ]
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            event_state.current_screen = 1;
        }
        1 => {
            match choice_idx {
                0 => {
                    // Steal gold
                    let gold_gain = if run_state.ascension_level >= 15 {
                        50
                    } else {
                        99
                    };
                    run_state.gold += gold_gain;
                    event_state.current_screen = 2;
                }
                _ => {
                    // Join cult: 6 damage (DEFAULT type — Tungsten Rod reduces by 1) + Ritual Dagger
                    let mut dmg = 6;
                    if run_state
                        .relics
                        .iter()
                        .any(|r| r.id == crate::content::relics::RelicId::TungstenRod)
                    {
                        dmg -= 1;
                    }
                    run_state.current_hp = (run_state.current_hp - dmg).max(0);
                    run_state.add_card_to_deck(CardId::RitualDagger);
                    event_state.current_screen = 2;
                }
            }
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
