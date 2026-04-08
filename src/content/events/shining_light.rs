use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

fn get_damage(run_state: &RunState) -> i32 {
    if run_state.ascension_level >= 15 {
        (run_state.max_hp as f32 * 0.3).round() as i32
    } else {
        (run_state.max_hp as f32 * 0.2).round() as i32
    }
}

fn has_upgradable_cards(run_state: &RunState) -> bool {
    run_state.master_deck.iter().any(|c| {
        let def = crate::content::cards::get_card_definition(c.id);
        c.id == crate::content::cards::CardId::SearingBlow
            || (c.upgrades == 0
                && def.card_type != crate::content::cards::CardType::Status
                && def.card_type != crate::content::cards::CardType::Curse)
    })
}

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    if event_state.current_screen == 1 {
        return vec![EventChoiceMeta::new("[Leave]")];
    }

    let damage = get_damage(run_state);
    let mut choices = Vec::new();

    if has_upgradable_cards(run_state) {
        choices.push(EventChoiceMeta::new(format!(
            "[Enter the Light] Take {} damage. Upgrade 2 random cards.",
            damage
        )));
    } else {
        choices.push(EventChoiceMeta::disabled(
            "[Enter the Light] No upgradable cards.",
            "No upgradable cards in your deck.",
        ));
    }

    choices.push(EventChoiceMeta::new("[Leave]"));
    choices
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Enter Light: take damage + upgrade 2 random cards
                    let damage = get_damage(run_state);
                    run_state.current_hp = (run_state.current_hp - damage).max(0);
                    run_state.upgrade_random_cards(2);
                }
                _ => {
                    // Leave
                }
            }
            event_state.current_screen = 1;
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
