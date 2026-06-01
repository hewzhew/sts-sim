use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

const GOLD_GAIN: i32 = 75;
const DAMAGE: i32 = 11;

pub fn get_choices(_run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    if event_state.current_screen == 1 {
        return vec![EventChoiceMeta::new("[Leave]")];
    }

    // Gold loss stored in internal_state (rolled at init time, matching Java)
    let gold_loss = event_state.internal_state;
    vec![
        EventChoiceMeta::new(format!(
            "[Gather Gold] Gain {} Gold. Take {} damage.",
            GOLD_GAIN, DAMAGE
        )),
        EventChoiceMeta::new(format!("[Leave] Lose {} Gold.", gold_loss)),
    ]
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Java: damage first via DamageInfo(player, damage), then gain gold.
                    let source = DomainEventSource::Event(EventId::WorldOfGoop);
                    super::apply_player_default_damage(
                        run_state,
                        DAMAGE,
                        super::EventDamageOwner::Player,
                        source,
                    );
                    run_state.change_gold_with_source(GOLD_GAIN, source);
                }
                _ => {
                    // Leave: lose pre-rolled gold amount
                    let gold_loss = event_state.internal_state;
                    let actual_loss = gold_loss.min(run_state.gold);
                    run_state.change_gold_with_source(
                        -actual_loss,
                        DomainEventSource::Event(EventId::WorldOfGoop),
                    );
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

/// Initialize GoopPuddle state.
/// Java: goldLoss is a constructor field — miscRng.random(35,75) or random(20,50) at init time.
/// internal_state = goldLoss amount
pub fn init_goop_puddle_state(run_state: &mut RunState) -> i32 {
    let gold_loss = if run_state.ascension_level >= 15 {
        run_state.rng_pool.misc_rng.random_range(35, 75)
    } else {
        run_state.rng_pool.misc_rng.random_range(20, 50)
    };
    gold_loss.min(run_state.gold)
}

#[cfg(test)]
mod tests {
    use super::{handle_choice, init_goop_puddle_state};
    use crate::content::relics::{RelicId, RelicState};
    use crate::state::core::EngineState;
    use crate::state::events::{EventId, EventState};
    use crate::state::run::RunState;
    use crate::state::selection::{DomainEvent, DomainEventSource};

    fn goop_run(current_hp: i32, gold: i32) -> RunState {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.current_hp = current_hp;
        run_state.gold = gold;
        run_state.event_state = Some(EventState::new(EventId::WorldOfGoop));
        run_state.emitted_events.clear();
        run_state
    }

    #[test]
    fn init_clamps_leave_gold_loss_to_current_gold_like_java_constructor() {
        let mut run_state = goop_run(80, 3);

        let loss = init_goop_puddle_state(&mut run_state);

        assert_eq!(loss, 3);
    }

    #[test]
    fn gather_gold_applies_java_damage_before_gold_gain() {
        let mut run_state = goop_run(30, 0);
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.current_hp, 19);
        assert_eq!(run_state.gold, 75);
        assert!(matches!(
            run_state.take_emitted_events().as_slice(),
            [
                DomainEvent::HpChanged {
                    delta: -11,
                    current_hp: 19,
                    source: DomainEventSource::Event(EventId::WorldOfGoop),
                    ..
                },
                DomainEvent::GoldChanged {
                    delta: 75,
                    new_total: 75,
                    source: DomainEventSource::Event(EventId::WorldOfGoop),
                },
            ]
        ));
    }

    #[test]
    fn gather_gold_default_damage_applies_tungsten_rod() {
        let mut run_state = goop_run(30, 0);
        run_state.relics.push(RelicState::new(RelicId::TungstenRod));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.current_hp, 20);
        assert!(run_state.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::HpChanged {
                delta: -10,
                current_hp: 20,
                source: DomainEventSource::Event(EventId::WorldOfGoop),
                ..
            }
        )));
    }
}
