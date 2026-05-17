// Java: Bonfire (shrines) — "Bonfire Elementals" / "Bonfire Spirits"
// This event is a duplicate/variant of bonfire_elementals. Both correspond
// to Java's single Bonfire.java (ID: "Bonfire Elementals").
//
// Screen 0: [Approach] → Screen 1
// Screen 1: [Offer] → grid-select (Purge) → screen 2
// Screen 2: reward based on removed card's rarity (read from internal_state)
// Screen 3: [Leave]

use crate::content::relics::RelicId;
use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

pub fn get_choices(_run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => vec![EventChoiceMeta::new("[Approach]")],
        1 => {
            // Offer a card to the bonfire
            vec![EventChoiceMeta::new("[Offer] Select a card to offer.")]
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, _choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            event_state.current_screen = 1;
        }
        1 => {
            // Transition to RunPendingChoice::Purge to select a card.
            // The Purge handler stores the removed card's rarity in
            // event_state.internal_state before removal.
            event_state.current_screen = 2;
            run_state.event_state = Some(event_state);
            *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                min_choices: 1,
                max_choices: 1,
                reason: RunPendingChoiceReason::PurgeNonBottled,
                return_state: Box::new(EngineState::EventRoom),
            });
            return;
        }
        2 => {
            // Post-purge: apply rarity-based reward from internal_state
            // (set by Purge handler: 0=Curse, 1=Basic, 2=Common, 3=Special, 4=Uncommon, 5=Rare)
            let rarity = event_state.internal_state;
            let source = DomainEventSource::Event(EventId::BonfireSpirits);
            match rarity {
                0 => {
                    // Curse → SpiritPoop relic (Circlet if already owned)
                    let relic_id = if run_state.relics.iter().any(|r| r.id == RelicId::SpiritPoop) {
                        RelicId::Circlet
                    } else {
                        RelicId::SpiritPoop
                    };
                    if let Some(next_state) =
                        run_state.obtain_relic_with_source(relic_id, EngineState::EventRoom, source)
                    {
                        *engine_state = next_state;
                    }
                }
                1 => {
                    // Basic → nothing
                }
                2 | 3 => {
                    // Common / Special → heal 5
                    run_state.heal_with_source(5, source);
                }
                4 => {
                    // Uncommon → heal to full
                    run_state.heal_with_source(run_state.max_hp, source);
                }
                5 => {
                    // Rare → Java increaseMaxHp(10, false), then heal(maxHealth).
                    run_state.gain_max_hp_with_source(10, 10, source);
                    run_state.heal_with_source(run_state.max_hp, source);
                }
                _ => {}
            }
            event_state.current_screen = 3;
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}

#[cfg(test)]
mod tests {
    use super::handle_choice;
    use crate::content::relics::RelicId;
    use crate::state::core::EngineState;
    use crate::state::events::{EventId, EventState};
    use crate::state::run::RunState;
    use crate::state::selection::{DomainEvent, DomainEventSource};

    fn bonfire_run(rarity_state: i32) -> RunState {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        let mut event_state = EventState::new(EventId::BonfireSpirits);
        event_state.current_screen = 2;
        event_state.internal_state = rarity_state;
        run_state.event_state = Some(event_state);
        run_state.emitted_events.clear();
        run_state
    }

    #[test]
    fn common_offer_heals_with_spirits_event_source() {
        let mut run_state = bonfire_run(2);
        run_state.current_hp = 10;
        run_state.max_hp = 80;
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.current_hp, 15);
        assert!(run_state.take_emitted_events().iter().any(|event| {
            matches!(
                event,
                DomainEvent::HpChanged {
                    delta: 5,
                    current_hp: 15,
                    max_hp: 80,
                    source: DomainEventSource::Event(EventId::BonfireSpirits)
                }
            )
        }));
    }

    #[test]
    fn curse_offer_obtains_spirit_poop_with_spirits_event_source() {
        let mut run_state = bonfire_run(0);
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert!(run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::SpiritPoop));
        assert!(run_state.take_emitted_events().iter().any(|event| {
            matches!(
                event,
                DomainEvent::RelicObtained {
                    relic_id: RelicId::SpiritPoop,
                    source: DomainEventSource::Event(EventId::BonfireSpirits)
                }
            )
        }));
    }
}
