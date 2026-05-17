// Java: AccursedBlacksmith (shrines)
// Screen 0: [Forge] Upgrade a card | [Rummage] Gain Pain curse + WarpedTongs relic | [Leave]
// Screen 1: [Leave]
//
// Forge uses gridSelectScreen for player to choose which card to upgrade.

use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    if event_state.current_screen == 1 {
        return vec![EventChoiceMeta::new("[Leave]")];
    }

    let has_upgradable = run_state
        .master_deck
        .iter()
        .any(crate::state::core::master_deck_card_can_upgrade);

    let mut choices = vec![];
    if has_upgradable {
        choices.push(EventChoiceMeta::new("[Forge] Upgrade a card."));
    } else {
        choices.push(EventChoiceMeta::disabled(
            "[Forge] Upgrade a card.",
            "No upgradable cards",
        ));
    }
    choices.push(EventChoiceMeta::new(
        "[Rummage] Obtain Pain and Warped Tongs.",
    ));
    choices.push(EventChoiceMeta::new("[Leave]"));
    choices
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    if event_state.completed {
        run_state.event_state = Some(event_state);
        return;
    }

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Forge: upgrade a card via grid-select (Java: gridSelectScreen.open(getUpgradableCards(), 1, ...))
                    if run_state
                        .master_deck
                        .iter()
                        .any(crate::state::core::master_deck_card_can_upgrade)
                    {
                        *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                            min_choices: 1,
                            max_choices: 1,
                            reason: RunPendingChoiceReason::Upgrade,
                            return_state: Box::new(EngineState::EventRoom),
                        });
                        event_state.current_screen = 1;
                    }
                }
                1 => {
                    // Rummage: obtain Pain curse + WarpedTongs relic
                    super::obtain_event_card(
                        run_state,
                        EventId::AccursedBlacksmith,
                        crate::content::cards::CardId::Pain,
                    );
                    let _ = run_state.obtain_relic_with_source(
                        crate::content::relics::RelicId::WarpedTongs,
                        EngineState::EventRoom,
                        DomainEventSource::Event(EventId::AccursedBlacksmith),
                    );
                    event_state.current_screen = 1;
                }
                _ => {
                    // Leave
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::state::selection::DomainEvent;

    fn blacksmith_run() -> RunState {
        let mut run_state = RunState::new(1, 0, true, "Ironclad");
        run_state.event_state = Some(EventState {
            id: EventId::AccursedBlacksmith,
            current_screen: 0,
            internal_state: 0,
            completed: false,
            combat_pending: false,
            extra_data: Vec::new(),
        });
        run_state
    }

    #[test]
    fn forge_opens_upgrade_pending_choice_like_grid_select() {
        let mut run_state = blacksmith_run();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert!(matches!(
            engine_state,
            EngineState::RunPendingChoice(ref pending)
                if pending.reason == RunPendingChoiceReason::Upgrade
                    && pending.min_choices == 1
                    && pending.max_choices == 1
        ));
        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 1);
    }

    #[test]
    fn disabled_forge_does_not_open_empty_upgrade_selection() {
        let mut run_state = blacksmith_run();
        run_state.master_deck.clear();
        run_state
            .master_deck
            .push(crate::runtime::combat::CombatCard::new(CardId::Injury, 100));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 0);
        assert!(matches!(engine_state, EngineState::EventRoom));
        assert!(run_state.take_emitted_events().is_empty());
    }

    #[test]
    fn rummage_uses_event_sources_for_pain_and_warped_tongs() {
        let mut run_state = blacksmith_run();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 1);

        assert!(run_state
            .master_deck
            .iter()
            .any(|card| card.id == CardId::Pain));
        assert!(run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::WarpedTongs));
        let events = run_state.take_emitted_events();
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::CardObtained {
                card,
                source: DomainEventSource::Event(EventId::AccursedBlacksmith),
            } if card.id == CardId::Pain
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::RelicObtained {
                relic_id: RelicId::WarpedTongs,
                source: DomainEventSource::Event(EventId::AccursedBlacksmith),
            }
        )));
    }

    #[test]
    fn rummage_pain_can_be_blocked_by_omamori_without_blocking_warped_tongs() {
        let mut run_state = blacksmith_run();
        run_state.relics.push(RelicState::new(RelicId::Omamori));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 1);

        assert!(!run_state
            .master_deck
            .iter()
            .any(|card| card.id == CardId::Pain));
        assert!(run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::WarpedTongs));
        let omamori = run_state
            .relics
            .iter()
            .find(|relic| relic.id == RelicId::Omamori)
            .expect("Omamori should remain after blocking Pain");
        assert_eq!(omamori.counter, 1);
    }
}
