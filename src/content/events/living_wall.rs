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

    let has_upgradable = run_state
        .master_deck
        .iter()
        .any(crate::state::core::master_deck_card_can_upgrade);
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
        let has_non_bottled_purgeable =
            crate::state::core::has_non_bottled_purgeable_master_deck_card(run_state);
        let has_upgradable = run_state
            .master_deck
            .iter()
            .any(crate::state::core::master_deck_card_can_upgrade);
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
            if choice_idx >= 2 && !has_upgradable {
                return;
            }

            if !has_non_bottled_purgeable {
                event_state.current_screen = 1;
                return;
            }

            let reason = match choice_idx {
                0 => RunPendingChoiceReason::PurgeNonBottled, // [Forget]
                1 => RunPendingChoiceReason::TransformNonBottled, // [Change]
                _ => RunPendingChoiceReason::Upgrade,         // [Grow], it's button index 2
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;

    fn living_wall_run() -> RunState {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.event_state = Some(EventState::new(crate::state::events::EventId::LivingWall));
        run_state.emitted_events.clear();
        run_state
    }

    #[test]
    fn disabled_grow_does_not_open_empty_upgrade_selection() {
        let mut run_state = living_wall_run();
        run_state.master_deck.clear();
        run_state
            .master_deck
            .push(crate::runtime::combat::CombatCard::new(CardId::Injury, 100));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 2);

        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 0);
        assert!(matches!(engine_state, EngineState::EventRoom));
    }

    #[test]
    fn grow_keeps_java_non_bottled_purgeable_guard_before_upgrade_prompt() {
        let mut run_state = living_wall_run();
        run_state.master_deck.clear();
        let strike = crate::runtime::combat::CombatCard::new(CardId::Strike, 100);
        let mut bottle = crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::BottledFlame,
        );
        bottle.amount = strike.uuid as i32;
        run_state.relics.push(bottle);
        run_state.master_deck.push(strike);
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 2);

        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 1);
        assert!(
            matches!(engine_state, EngineState::EventRoom),
            "Java checks getGroupWithoutBottledCards(getPurgeableCards()) before opening the Grow upgrade grid"
        );
    }
}
