use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

fn has_upgradable_cards(run_state: &RunState) -> bool {
    run_state
        .master_deck
        .iter()
        .any(crate::state::core::master_deck_card_can_upgrade)
}

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    if event_state.current_screen == 1 {
        return vec![EventChoiceMeta::new("[Leave]")];
    }

    let mut choices = Vec::new();

    if has_upgradable_cards(run_state) {
        choices.push(EventChoiceMeta::new("[Pray] Upgrade a card."));
    } else {
        choices.push(EventChoiceMeta::disabled(
            "[Pray] Upgrade a card.",
            "No upgradable cards.",
        ));
    }

    choices.push(EventChoiceMeta::new("[Leave]"));
    choices
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    if has_upgradable_cards(run_state) {
                        event_state.current_screen = 1;
                        run_state.event_state = Some(event_state);
                        *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                            reason: RunPendingChoiceReason::Upgrade,
                            min_choices: 1,
                            max_choices: 1,
                            return_state: Box::new(EngineState::EventRoom),
                        });
                        return;
                    }
                }
                _ => {
                    // Leave
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

#[cfg(test)]
mod tests {
    use super::{get_choices, handle_choice};
    use crate::content::cards::CardId;
    use crate::runtime::combat::CombatCard;
    use crate::state::core::{EngineState, RunPendingChoiceReason};
    use crate::state::events::{EventId, EventState};
    use crate::state::run::RunState;

    fn shrine_run(deck: Vec<CombatCard>) -> RunState {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.master_deck = deck;
        run_state.event_state = Some(EventState::new(EventId::UpgradeShrine));
        run_state.emitted_events.clear();
        run_state
    }

    #[test]
    fn disabled_pray_does_not_open_empty_upgrade_selection() {
        let mut run_state = shrine_run(vec![CombatCard::new(CardId::Injury, 11)]);
        let mut engine_state = EngineState::EventRoom;

        let choices = get_choices(&run_state, run_state.event_state.as_ref().unwrap());
        assert!(choices[0].disabled);

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 0);
        assert!(matches!(engine_state, EngineState::EventRoom));
        assert!(run_state.take_emitted_events().is_empty());
    }

    #[test]
    fn searing_blow_remains_upgradeable_after_prior_upgrades() {
        let mut searing = CombatCard::new(CardId::SearingBlow, 12);
        searing.upgrades = 3;
        let mut run_state = shrine_run(vec![searing]);
        let mut engine_state = EngineState::EventRoom;

        let choices = get_choices(&run_state, run_state.event_state.as_ref().unwrap());
        assert!(!choices[0].disabled);

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert!(matches!(
            engine_state,
            EngineState::RunPendingChoice(ref pending)
                if pending.reason == RunPendingChoiceReason::Upgrade
        ));
        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 1);
    }
}
