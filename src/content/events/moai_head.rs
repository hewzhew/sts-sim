use crate::content::relics::RelicId;
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            let hp_loss_pct = if run_state.ascension_level >= 15 {
                0.18
            } else {
                0.125
            };
            let hp_loss = (run_state.max_hp as f32 * hp_loss_pct).round() as i32;
            let has_idol = run_state.relics.iter().any(|r| r.id == RelicId::GoldenIdol);
            let mut choices = vec![EventChoiceMeta::new(format!(
                "[Enter] Lose {} Max HP. Heal to full.",
                hp_loss
            ))];
            if has_idol {
                choices.push(EventChoiceMeta::new(
                    "[Trade] Give Golden Idol. Gain 333 Gold.",
                ));
            } else {
                choices.push(EventChoiceMeta::disabled(
                    "[Trade] Requires Golden Idol.",
                    "No Golden Idol",
                ));
            }
            choices.push(EventChoiceMeta::new("[Leave]"));
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
                    // Enter: lose max HP, heal to full
                    let hp_loss_pct = if run_state.ascension_level >= 15 {
                        0.18
                    } else {
                        0.125
                    };
                    let hp_loss = (run_state.max_hp as f32 * hp_loss_pct).round() as i32;
                    run_state.max_hp = (run_state.max_hp - hp_loss).max(1);
                    run_state.current_hp = run_state.max_hp;
                    event_state.current_screen = 1;
                }
                1 => {
                    // Trade Golden Idol for 333 gold
                    if let Some(pos) = run_state
                        .relics
                        .iter()
                        .position(|r| r.id == RelicId::GoldenIdol)
                    {
                        run_state.relics.remove(pos);
                        run_state.gold += 333;
                    }
                    event_state.current_screen = 1;
                }
                _ => {
                    event_state.completed = true;
                }
            }
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
