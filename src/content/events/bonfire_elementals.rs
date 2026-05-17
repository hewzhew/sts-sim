// Java: Bonfire (shrines) — "Bonfire Elementals"
// Screen 0 (INTRO): [Approach] → Screen 1
// Screen 1 (CHOOSE): [Offer card] → grid-select to sacrifice a card
// After grid-select returns to screen 2: reward based on offered card's rarity
//   (rarity stored in internal_state by Purge handler in run_loop.rs)
//   Curse → SpiritPoop relic (Circlet if already owned)
//   Basic → nothing
//   Common/Special → heal 5
//   Uncommon → heal to full
//   Rare → +10 maxHP + heal to full
// Screen 3 (COMPLETE): [Leave]

use crate::content::relics::RelicId;
use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

pub fn get_choices(_run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => vec![EventChoiceMeta::new("[Approach] Investigate the bonfire.")],
        1 => vec![EventChoiceMeta::new(
            "[Offer] Sacrifice a card to the spirits.",
        )],
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, _choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            // Approach → go to card sacrifice screen
            event_state.current_screen = 1;
        }
        1 => {
            if crate::state::core::has_non_bottled_purgeable_master_deck_card(run_state) {
                event_state.current_screen = 2;
                // Sacrifice a card via grid-select.
                // The Purge handler in run_loop.rs stores the removed card's rarity
                // in event_state.internal_state before removal.
                run_state.event_state = Some(event_state);
                *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                    min_choices: 1,
                    max_choices: 1,
                    reason: RunPendingChoiceReason::PurgeNonBottled,
                    return_state: Box::new(EngineState::EventRoom),
                });
                return;
            } else {
                event_state.current_screen = 3;
            }
        }
        2 => {
            // Returned from purge. Read rarity from internal_state
            // (set by Purge handler: 0=Curse, 1=Basic, 2=Common, 3=Special, 4=Uncommon, 5=Rare)
            let rarity = event_state.internal_state;
            let source = DomainEventSource::Event(EventId::BonfireElementals);
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
    use super::{get_choices, handle_choice};
    use crate::content::cards::CardId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::state::core::EngineState;
    use crate::state::events::{EventId, EventState};
    use crate::state::run::RunState;
    use crate::state::selection::{DomainEvent, DomainEventSource};

    fn bonfire_run(rarity_state: i32) -> RunState {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        let mut event_state = EventState::new(EventId::BonfireElementals);
        event_state.current_screen = 2;
        event_state.internal_state = rarity_state;
        run_state.event_state = Some(event_state);
        run_state.emitted_events.clear();
        run_state
    }

    #[test]
    fn offer_without_purgeable_card_advances_without_pending_like_java() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.master_deck = vec![crate::runtime::combat::CombatCard::new(
            CardId::AscendersBane,
            11,
        )];
        let mut event_state = EventState::new(EventId::BonfireElementals);
        event_state.current_screen = 1;
        run_state.event_state = Some(event_state);
        run_state.emitted_events.clear();
        let mut engine_state = EngineState::EventRoom;

        let choices = get_choices(&run_state, run_state.event_state.as_ref().unwrap());
        assert!(
            !choices[0].disabled,
            "Java Bonfire keeps Offer clickable and handles the empty group in buttonEffect"
        );

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 3);
        assert!(matches!(engine_state, EngineState::EventRoom));
        assert!(run_state.take_emitted_events().is_empty());

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert!(!run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::SpiritPoop));
        assert!(run_state.event_state.as_ref().unwrap().completed);
    }

    #[test]
    fn common_offer_heals_with_event_source() {
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
                    source: DomainEventSource::Event(EventId::BonfireElementals)
                }
            )
        }));
    }

    #[test]
    fn rare_offer_matches_java_max_hp_then_full_heal_sequence() {
        let mut run_state = bonfire_run(5);
        run_state.current_hp = 30;
        run_state.max_hp = 80;
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.max_hp, 90);
        assert_eq!(run_state.current_hp, 90);
        let events = run_state.take_emitted_events();
        assert!(events.iter().any(|event| {
            matches!(
                event,
                DomainEvent::MaxHpChanged {
                    delta: 10,
                    current_hp: 40,
                    max_hp: 90,
                    source: DomainEventSource::Event(EventId::BonfireElementals)
                }
            )
        }));
        assert!(events.iter().any(|event| {
            matches!(
                event,
                DomainEvent::HpChanged {
                    delta: 50,
                    current_hp: 90,
                    max_hp: 90,
                    source: DomainEventSource::Event(EventId::BonfireElementals)
                }
            )
        }));
    }

    #[test]
    fn heal_rewards_obey_mark_of_the_bloom() {
        let mut run_state = bonfire_run(4);
        run_state.current_hp = 10;
        run_state.max_hp = 80;
        run_state
            .relics
            .push(RelicState::new(RelicId::MarkOfTheBloom));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.current_hp, 10);
        assert!(!run_state
            .take_emitted_events()
            .iter()
            .any(|event| matches!(event, DomainEvent::HpChanged { .. })));
    }

    #[test]
    fn curse_offer_obtains_spirit_poop_with_event_source() {
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
                    source: DomainEventSource::Event(EventId::BonfireElementals)
                }
            )
        }));
    }
}
