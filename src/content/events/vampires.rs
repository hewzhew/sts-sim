use crate::content::cards::{CardId, CardTag};
use crate::content::relics::RelicId;
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;
use crate::combat::CombatCard;

fn get_hp_loss(run_state: &RunState) -> i32 {
    let mut loss = (run_state.max_hp as f32 * 0.3).ceil() as i32;
    if loss >= run_state.max_hp {
        loss = run_state.max_hp - 1;
    }
    loss
}

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    if event_state.current_screen == 1 {
        return vec![EventChoiceMeta::new("[Leave]")];
    }
    
    let hp_loss = get_hp_loss(run_state);
    let mut choices = vec![
        EventChoiceMeta::new(format!("[Accept] Lose {} Max HP. Replace all Strikes with 5 Bites.", hp_loss)),
    ];

    let has_vial = run_state.relics.iter().any(|r| r.id == RelicId::BloodVial);
    if has_vial {
        choices.push(EventChoiceMeta::new("[Give Vial] Lose Blood Vial. Replace all Strikes with 5 Bites."));
    } else {
        choices.push(EventChoiceMeta::disabled(
            "[Give Vial] Lose Blood Vial. Replace all Strikes with 5 Bites.",
            "Requires Blood Vial",
        ));
    }

    choices.push(EventChoiceMeta::new("[Refuse] Leave."));
    choices
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => { // Accept: Max HP loss
                    let hp_loss = get_hp_loss(run_state);
                    run_state.max_hp -= hp_loss;
                    if run_state.current_hp > run_state.max_hp {
                        run_state.current_hp = run_state.max_hp;
                    }
                    replace_attacks(run_state);
                    event_state.current_screen = 1;
                },
                1 => { // Give Vial -> Requires BloodVial
                    if let Some(pos) = run_state.relics.iter().position(|r| r.id == RelicId::BloodVial) {
                        run_state.relics.remove(pos);
                    }
                    replace_attacks(run_state);
                    event_state.current_screen = 1;
                },
                _ => { // Refuse
                    event_state.completed = true;
                }
            }
        },
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}

fn replace_attacks(run_state: &mut RunState) {
    // Retain only cards that do NOT have the StarterStrike tag
    run_state.master_deck.retain(|card| {
        let def = crate::content::cards::get_card_definition(card.id);
        !def.tags.contains(&CardTag::StarterStrike)
    });

    // Add 5 Bites with proper UUID generation for internal engine identity
    let starting_uuid = run_state.master_deck.len() as u32 + 1000;
    for i in 0..5 {
        run_state.master_deck.push(CombatCard::new(CardId::Bite, starting_uuid + i));
    }
}
