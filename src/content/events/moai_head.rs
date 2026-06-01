use crate::content::relics::RelicId;
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

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
                    let source = DomainEventSource::Event(EventId::MoaiHead);
                    run_state.lose_max_hp_with_source(hp_loss, source);
                    run_state.heal_with_source(run_state.max_hp, source);
                    event_state.current_screen = 1;
                }
                1 => {
                    // Trade Golden Idol for 333 gold
                    if let Some(pos) = run_state
                        .relics
                        .iter()
                        .position(|r| r.id == RelicId::GoldenIdol)
                    {
                        run_state.remove_relic_at_with_source(
                            pos,
                            DomainEventSource::Event(EventId::MoaiHead),
                        );
                        run_state.change_gold_with_source(
                            333,
                            DomainEventSource::Event(EventId::MoaiHead),
                        );
                        event_state.current_screen = 1;
                    }
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
    use super::{get_choices, handle_choice};
    use crate::content::relics::{RelicId, RelicState};
    use crate::state::core::EngineState;
    use crate::state::events::{EventId, EventState};
    use crate::state::run::RunState;
    use crate::state::selection::{DomainEvent, DomainEventSource};

    fn moai_run(current_hp: i32, max_hp: i32) -> RunState {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.current_hp = current_hp;
        run_state.max_hp = max_hp;
        run_state.gold = 0;
        run_state.event_state = Some(EventState::new(EventId::MoaiHead));
        run_state.emitted_events.clear();
        run_state
    }

    #[test]
    fn disabled_trade_without_golden_idol_does_not_advance_or_grant_gold() {
        let mut run_state = moai_run(20, 80);
        let mut engine_state = EngineState::EventRoom;

        let choices = get_choices(&run_state, run_state.event_state.as_ref().unwrap());
        assert!(choices[1].disabled);

        handle_choice(&mut engine_state, &mut run_state, 1);

        assert_eq!(run_state.gold, 0);
        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 0);
        assert!(matches!(engine_state, EngineState::EventRoom));
        assert!(run_state.take_emitted_events().is_empty());
    }

    #[test]
    fn enter_loses_max_hp_then_heals_to_new_max_with_event_source() {
        let mut run_state = moai_run(20, 80);
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.max_hp, 70);
        assert_eq!(run_state.current_hp, 70);
        let events = run_state.take_emitted_events();
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::MaxHpChanged {
                delta: -10,
                current_hp: 20,
                max_hp: 70,
                source: DomainEventSource::Event(EventId::MoaiHead),
            }
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::HpChanged {
                delta: 50,
                current_hp: 70,
                max_hp: 70,
                source: DomainEventSource::Event(EventId::MoaiHead),
            }
        )));
    }

    #[test]
    fn enter_max_hp_loss_survives_mark_but_full_heal_is_blocked() {
        let mut run_state = moai_run(20, 80);
        run_state
            .relics
            .push(RelicState::new(RelicId::MarkOfTheBloom));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.max_hp, 70);
        assert_eq!(run_state.current_hp, 20);
        let events = run_state.take_emitted_events();
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::MaxHpChanged {
                delta: -10,
                current_hp: 20,
                max_hp: 70,
                source: DomainEventSource::Event(EventId::MoaiHead),
            }
        )));
        assert!(!events
            .iter()
            .any(|event| matches!(event, DomainEvent::HpChanged { .. })));
    }

    #[test]
    fn trade_removes_golden_idol_and_grants_gold_with_event_sources() {
        let mut run_state = moai_run(20, 80);
        run_state.relics.push(RelicState::new(RelicId::GoldenIdol));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 1);

        assert!(!run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::GoldenIdol));
        assert_eq!(run_state.gold, 333);
        let events = run_state.take_emitted_events();
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::RelicLost {
                relic_id: RelicId::GoldenIdol,
                source: DomainEventSource::Event(EventId::MoaiHead),
            }
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::GoldChanged {
                delta: 333,
                new_total: 333,
                source: DomainEventSource::Event(EventId::MoaiHead),
            }
        )));
    }
}
