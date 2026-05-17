use crate::content::relics::RelicId;
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => vec![EventChoiceMeta::new("[Proceed]")],
        1 => {
            let gold_reward = if run_state.ascension_level >= 15 {
                50
            } else {
                75
            };
            let damage = (run_state.max_hp / 10).max(1);
            vec![
                EventChoiceMeta::new(format!(
                    "[Touch] Lose {} HP. Gain {} Gold.",
                    damage, gold_reward
                )),
                EventChoiceMeta::new("[Trade] Obtain a face Relic."),
                EventChoiceMeta::new("[Leave]"),
            ]
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            event_state.current_screen = 1;
        }
        1 => {
            match choice_idx {
                0 => {
                    // Touch: damage + gold
                    // Java: DamageInfo(null, damage) — DEFAULT damage type (not HP_LOSS)
                    // DEFAULT damage can be reduced by Tungsten Rod (-1)
                    let gold_reward = if run_state.ascension_level >= 15 {
                        50
                    } else {
                        75
                    };
                    let mut damage = (run_state.max_hp / 10).max(1);
                    // Apply Tungsten Rod if present (reduces non-HP_LOSS damage by 1)
                    if run_state
                        .relics
                        .iter()
                        .any(|r| r.id == RelicId::TungstenRod)
                    {
                        damage = (damage - 1).max(0);
                    }
                    run_state.change_gold_with_source(
                        gold_reward,
                        DomainEventSource::Event(EventId::FaceTrader),
                    );
                    run_state.change_hp_with_source(
                        -damage,
                        DomainEventSource::Event(EventId::FaceTrader),
                    );
                    event_state.current_screen = 2;
                }
                1 => {
                    // Trade: get a face relic
                    // Java: Collections.shuffle(ids, new Random(miscRng.randomLong()))
                    let face_relics = [
                        RelicId::CultistMask,
                        RelicId::FaceOfCleric,
                        RelicId::GremlinMask,
                        RelicId::NlothsMask,
                        RelicId::SsserpentHead,
                    ];
                    let mut available: Vec<RelicId> = face_relics
                        .iter()
                        .copied()
                        .filter(|r| !run_state.relics.iter().any(|owned| owned.id == *r))
                        .collect();

                    let relic_id = if available.is_empty() {
                        // Consume randomLong for seed parity even with no available relics
                        let _seed = run_state.rng_pool.misc_rng.random_long();
                        RelicId::Circlet
                    } else {
                        crate::runtime::rng::shuffle_with_random_long(
                            &mut available,
                            &mut run_state.rng_pool.misc_rng,
                        );
                        available[0]
                    };
                    if let Some(next_state) = run_state.obtain_relic_with_source(
                        relic_id,
                        EngineState::EventRoom,
                        DomainEventSource::Event(EventId::FaceTrader),
                    ) {
                        *engine_state = next_state;
                    }
                    event_state.current_screen = 2;
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

#[cfg(test)]
mod tests {
    use super::handle_choice;
    use crate::content::relics::{RelicId, RelicState};
    use crate::state::core::EngineState;
    use crate::state::events::{EventId, EventState};
    use crate::state::run::RunState;
    use crate::state::selection::{DomainEvent, DomainEventSource};

    fn face_trader_main_run() -> RunState {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.current_hp = 20;
        run_state.max_hp = 80;
        run_state.gold = 0;
        let mut event_state = EventState::new(EventId::FaceTrader);
        event_state.current_screen = 1;
        run_state.event_state = Some(event_state);
        run_state.emitted_events.clear();
        run_state
    }

    #[test]
    fn touch_uses_event_hp_and_gold_sources() {
        let mut run_state = face_trader_main_run();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.current_hp, 12);
        assert_eq!(run_state.gold, 75);
        let events = run_state.take_emitted_events();
        assert!(matches!(
            events.as_slice(),
            [
                DomainEvent::GoldChanged {
                    delta: 75,
                    new_total: 75,
                    source: DomainEventSource::Event(EventId::FaceTrader),
                },
                DomainEvent::HpChanged {
                    delta: -8,
                    current_hp: 12,
                    max_hp: 80,
                    source: DomainEventSource::Event(EventId::FaceTrader),
                },
            ]
        ));
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::HpChanged {
                delta: -8,
                current_hp: 12,
                max_hp: 80,
                source: DomainEventSource::Event(EventId::FaceTrader),
            }
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::GoldChanged {
                delta: 75,
                new_total: 75,
                source: DomainEventSource::Event(EventId::FaceTrader),
            }
        )));
    }

    #[test]
    fn touch_damage_respects_tungsten_rod() {
        let mut run_state = face_trader_main_run();
        run_state.relics.push(RelicState::new(RelicId::TungstenRod));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.current_hp, 13);
        assert!(run_state.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::HpChanged {
                delta: -7,
                source: DomainEventSource::Event(EventId::FaceTrader),
                ..
            }
        )));
    }

    #[test]
    fn trade_obtains_face_relic_through_event_source_pipeline() {
        let mut run_state = face_trader_main_run();
        run_state.relics.retain(|relic| {
            !matches!(
                relic.id,
                RelicId::CultistMask
                    | RelicId::FaceOfCleric
                    | RelicId::GremlinMask
                    | RelicId::NlothsMask
                    | RelicId::SsserpentHead
            )
        });
        run_state.rng_pool.misc_rng = crate::runtime::rng::StsRng::new(1);
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 1);

        let obtained = run_state.relics.last().unwrap().id;
        assert!(matches!(
            obtained,
            RelicId::CultistMask
                | RelicId::FaceOfCleric
                | RelicId::GremlinMask
                | RelicId::NlothsMask
                | RelicId::SsserpentHead
        ));
        assert!(run_state.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::RelicObtained {
                relic_id,
                source: DomainEventSource::Event(EventId::FaceTrader),
            } if *relic_id == obtained
        )));
    }

    #[test]
    fn trade_grants_circlet_when_all_face_relics_are_owned() {
        let mut run_state = face_trader_main_run();
        run_state.relics.extend([
            RelicState::new(RelicId::CultistMask),
            RelicState::new(RelicId::FaceOfCleric),
            RelicState::new(RelicId::GremlinMask),
            RelicState::new(RelicId::NlothsMask),
            RelicState::new(RelicId::SsserpentHead),
        ]);
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 1);

        assert_eq!(run_state.relics.last().unwrap().id, RelicId::Circlet);
        assert!(run_state.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::RelicObtained {
                relic_id: RelicId::Circlet,
                source: DomainEventSource::Event(EventId::FaceTrader),
            }
        )));
    }
}
