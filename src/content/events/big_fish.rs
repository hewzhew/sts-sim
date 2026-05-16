use crate::content::cards::CardId;
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

const MAX_HP_AMT: i32 = 5;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    if event_state.current_screen == 1 {
        return vec![EventChoiceMeta::new("[Leave]")];
    }

    let heal_amt = run_state.max_hp / 3;
    vec![
        EventChoiceMeta::new(format!("[Banana] Heal {} HP.", heal_amt)),
        EventChoiceMeta::new(format!("[Donut] Gain {} Max HP.", MAX_HP_AMT)),
        EventChoiceMeta::new("[Box] Obtain a random Relic. Become Cursed - Regret."),
    ]
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    let heal_amt = run_state.max_hp / 3;
                    run_state
                        .heal_with_source(heal_amt, DomainEventSource::Event(EventId::BigFish));
                }
                1 => {
                    run_state.gain_max_hp_with_source(
                        MAX_HP_AMT,
                        MAX_HP_AMT,
                        DomainEventSource::Event(EventId::BigFish),
                    );
                }
                _ => {
                    // Box: Random relic + Regret curse
                    let relic_id = run_state.random_screenless_relic_reward();
                    if let Some(next_state) = run_state.obtain_relic_with_source(
                        relic_id,
                        EngineState::EventRoom,
                        DomainEventSource::Event(EventId::BigFish),
                    ) {
                        *engine_state = next_state;
                    }
                    super::obtain_event_card(run_state, EventId::BigFish, CardId::Regret);
                }
            }
            event_state.current_screen = 1;
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

    fn big_fish_run(current_hp: i32, max_hp: i32) -> RunState {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.current_hp = current_hp;
        run_state.max_hp = max_hp;
        run_state.event_state = Some(EventState::new(EventId::BigFish));
        run_state.emitted_events.clear();
        run_state
    }

    #[test]
    fn banana_uses_java_player_heal_semantics_and_event_source() {
        let mut run_state = big_fish_run(20, 81);
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.current_hp, 47);
        assert!(run_state.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::HpChanged {
                delta: 27,
                current_hp: 47,
                max_hp: 81,
                source: DomainEventSource::Event(EventId::BigFish),
            }
        )));
    }

    #[test]
    fn banana_heal_is_blocked_by_mark_of_the_bloom() {
        let mut run_state = big_fish_run(20, 81);
        run_state
            .relics
            .push(RelicState::new(RelicId::MarkOfTheBloom));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.current_hp, 20);
        assert!(!run_state
            .take_emitted_events()
            .iter()
            .any(|event| matches!(event, DomainEvent::HpChanged { .. })));
    }

    #[test]
    fn donut_increase_max_hp_uses_java_increase_then_heal_semantics() {
        let mut run_state = big_fish_run(20, 80);
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 1);

        assert_eq!(run_state.max_hp, 85);
        assert_eq!(run_state.current_hp, 25);
        let events = run_state.take_emitted_events();
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::HpChanged {
                delta: 5,
                current_hp: 25,
                max_hp: 85,
                source: DomainEventSource::Event(EventId::BigFish),
            }
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::MaxHpChanged {
                delta: 5,
                current_hp: 25,
                max_hp: 85,
                source: DomainEventSource::Event(EventId::BigFish),
            }
        )));
    }

    #[test]
    fn donut_max_hp_gain_survives_mark_but_attached_heal_is_blocked() {
        let mut run_state = big_fish_run(20, 80);
        run_state
            .relics
            .push(RelicState::new(RelicId::MarkOfTheBloom));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 1);

        assert_eq!(run_state.max_hp, 85);
        assert_eq!(run_state.current_hp, 20);
        let events = run_state.take_emitted_events();
        assert!(!events
            .iter()
            .any(|event| matches!(event, DomainEvent::HpChanged { .. })));
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::MaxHpChanged {
                delta: 5,
                current_hp: 20,
                max_hp: 85,
                source: DomainEventSource::Event(EventId::BigFish),
            }
        )));
    }
}
