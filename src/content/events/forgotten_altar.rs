use crate::content::cards::CardId;
use crate::content::relics::{RelicId, RelicState};
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            let has_idol = run_state.relics.iter().any(|r| r.id == RelicId::GoldenIdol);
            let hp_loss_pct = if run_state.ascension_level >= 15 {
                0.35
            } else {
                0.25
            };
            let hp_loss = (run_state.max_hp as f32 * hp_loss_pct).round() as i32;
            let mut choices = vec![];
            if has_idol {
                choices.push(EventChoiceMeta::new(
                    "[Offer] Trade Golden Idol for Bloody Idol.",
                ));
            } else {
                choices.push(EventChoiceMeta::disabled(
                    "[Offer] Requires Golden Idol.",
                    "No Golden Idol",
                ));
            }
            choices.push(EventChoiceMeta::new(format!(
                "[Pray] Gain 5 Max HP. Lose {} HP.",
                hp_loss
            )));
            choices.push(EventChoiceMeta::new("[Desecrate] Become Cursed - Decay."));
            choices
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
                    // Trade Golden Idol for Bloody Idol
                    if let Some(pos) = run_state
                        .relics
                        .iter()
                        .position(|r| r.id == RelicId::GoldenIdol)
                    {
                        run_state.relics.remove(pos);
                        run_state.relics.push(RelicState::new(RelicId::BloodyIdol));
                    }
                    event_state.current_screen = 1;
                }
                1 => {
                    // +5 Max HP, lose HP
                    let hp_loss_pct = if run_state.ascension_level >= 15 {
                        0.35
                    } else {
                        0.25
                    };
                    let mut hp_loss = (run_state.max_hp as f32 * hp_loss_pct).round() as i32;
                    // DEFAULT damage type — Tungsten Rod reduces by 1
                    if run_state
                        .relics
                        .iter()
                        .any(|r| r.id == crate::content::relics::RelicId::TungstenRod)
                    {
                        hp_loss = (hp_loss - 1).max(0);
                    }
                    run_state.max_hp += 5;
                    run_state.current_hp = (run_state.current_hp - hp_loss).max(0);
                    event_state.current_screen = 1;
                }
                _ => {
                    // Desecrate: Decay curse
                    run_state.add_card_to_deck(CardId::Decay);
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
