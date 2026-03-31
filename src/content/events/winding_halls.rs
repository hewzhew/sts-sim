use crate::content::cards::CardId;
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => vec![EventChoiceMeta::new("[Proceed]")],
        1 => {
            let hp_loss_pct = if run_state.ascension_level >= 15 { 0.18 } else { 0.125 };
            let hp_loss = (run_state.max_hp as f32 * hp_loss_pct).round() as i32;
            let heal_pct = if run_state.ascension_level >= 15 { 0.20 } else { 0.25 };
            let heal_amt = (run_state.max_hp as f32 * heal_pct).round() as i32;
            let max_hp_loss = (run_state.max_hp as f32 * 0.05).round() as i32;
            vec![
                EventChoiceMeta::new(format!("[Embrace] Lose {} HP. Obtain 2 Madness.", hp_loss)),
                EventChoiceMeta::new(format!("[Retrace] Heal {} HP. Become Cursed - Writhe.", heal_amt)),
                EventChoiceMeta::new(format!("[Accept] Lose {} Max HP.", max_hp_loss)),
            ]
        },
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            event_state.current_screen = 1;
        },
        1 => {
            match choice_idx {
                0 => {
                    // Embrace Madness: take damage (DEFAULT type) + 2 Madness
                    let hp_loss_pct = if run_state.ascension_level >= 15 { 0.18 } else { 0.125 };
                    let mut hp_loss = (run_state.max_hp as f32 * hp_loss_pct).round() as i32;
                    // Tungsten Rod reduces DEFAULT damage by 1
                    if run_state.relics.iter().any(|r| r.id == crate::content::relics::RelicId::TungstenRod) {
                        hp_loss = (hp_loss - 1).max(0);
                    }
                    run_state.current_hp = (run_state.current_hp - hp_loss).max(0);
                    run_state.add_card_to_deck(CardId::Madness);
                    run_state.add_card_to_deck(CardId::Madness);
                    event_state.current_screen = 2;
                },
                1 => {
                    // Retrace: heal + Writhe
                    let heal_pct = if run_state.ascension_level >= 15 { 0.20 } else { 0.25 };
                    let heal_amt = (run_state.max_hp as f32 * heal_pct).round() as i32;
                    run_state.current_hp = (run_state.current_hp + heal_amt).min(run_state.max_hp);
                    run_state.add_card_to_deck(CardId::Writhe);
                    event_state.current_screen = 2;
                },
                _ => {
                    // Accept: lose Max HP
                    let max_hp_loss = (run_state.max_hp as f32 * 0.05).round() as i32;
                    run_state.max_hp = (run_state.max_hp - max_hp_loss).max(1);
                    if run_state.current_hp > run_state.max_hp {
                        run_state.current_hp = run_state.max_hp;
                    }
                    event_state.current_screen = 2;
                },
            }
        },
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
