use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

/// Returns the choices for the Living Wall event: [Forget, Change, Grow]
pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    if event_state.current_screen == 1 {
        return vec![EventChoiceMeta::new("[Leave]")];
    }
    
    let mut choices = vec![
        EventChoiceMeta::new("[Forget] Remove a card from your deck."),
        EventChoiceMeta::new("[Change] Transform a card in your deck."),
    ];

    let has_upgradable = run_state.master_deck.iter().any(|c| c.upgrades == 0);
    if has_upgradable {
        choices.push(EventChoiceMeta::new("[Grow] Upgrade a card in your deck."));
    } else {
        choices.push(EventChoiceMeta::disabled(
            "[Grow] Upgrade a card in your deck.",
            "Requires an upgradable card in your deck.",
        ));
    }

    choices
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    if let EngineState::EventRoom = engine_state {
        let event_state = if let Some(es) = &mut run_state.event_state {
            es
        } else {
            return;
        };

        if event_state.completed {
            return;
        }
        
        // This event only has 1 interactive screen (screen 0) where you pick one path, then screen 1 is just 'Leave'
        if event_state.current_screen == 0 {
            let reason = match choice_idx {
                0 => RunPendingChoiceReason::Purge, // [Forget]
                1 => RunPendingChoiceReason::Transform, // [Change]
                _ => RunPendingChoiceReason::Upgrade, // [Grow], it's button index 2
            };

            event_state.current_screen = 1; // Advance to post-choice 'Leave' screen
            *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                reason,
                min_choices: 1,
                max_choices: 1,
                return_state: Box::new(EngineState::EventRoom),
            });
        } else {
            // "Leave" button pressed on post-choice screen
            event_state.completed = true;
        }
    }
}
