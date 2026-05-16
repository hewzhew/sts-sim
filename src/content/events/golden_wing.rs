use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

const DAMAGE: i32 = 7;
const MIN_GOLD: i32 = 50;
const MAX_GOLD: i32 = 80;
const REQUIRED_DAMAGE: i32 = 10;

fn has_high_damage_card(run_state: &RunState) -> bool {
    run_state.master_deck.iter().any(|c| {
        let def = crate::content::cards::get_card_definition(c.id);
        def.base_damage >= REQUIRED_DAMAGE
    })
}

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    if event_state.current_screen >= 1 {
        return vec![EventChoiceMeta::new("[Leave]")];
    }

    let can_attack = has_high_damage_card(run_state);
    let mut choices = vec![EventChoiceMeta::new(format!(
        "[Remove a card] Take {} damage. Remove a card from your deck.",
        DAMAGE
    ))];

    if can_attack {
        choices.push(EventChoiceMeta::new(format!(
            "[Attack] Gain {}-{} Gold.",
            MIN_GOLD, MAX_GOLD
        )));
    } else {
        choices.push(EventChoiceMeta::disabled(
            format!(
                "[Attack] Requires an Attack card with ≥{} damage.",
                REQUIRED_DAMAGE
            ),
            "No qualifying attack card.",
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
                    // Remove card: take damage, then purge
                    let damage = if run_state
                        .relics
                        .iter()
                        .any(|r| r.id == crate::content::relics::RelicId::TungstenRod)
                    {
                        (DAMAGE - 1).max(0)
                    } else {
                        DAMAGE
                    };
                    run_state.change_hp_with_source(
                        -damage,
                        DomainEventSource::Event(EventId::GoldenWing),
                    );
                    event_state.current_screen = 1;
                    run_state.event_state = Some(event_state);
                    *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                        reason: RunPendingChoiceReason::PurgeNonBottled,
                        min_choices: 1,
                        max_choices: 1,
                        return_state: Box::new(EngineState::EventRoom),
                    });
                    return;
                }
                1 => {
                    // Attack: gain gold
                    if has_high_damage_card(run_state) {
                        let gold = run_state.rng_pool.misc_rng.random_range(MIN_GOLD, MAX_GOLD);
                        run_state.change_gold_with_source(
                            gold,
                            DomainEventSource::Event(EventId::GoldenWing),
                        );
                    }
                    event_state.current_screen = 1;
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
    use super::handle_choice;
    use crate::content::relics::{RelicId, RelicState};
    use crate::state::core::{EngineState, RunPendingChoiceReason};
    use crate::state::events::{EventId, EventState};
    use crate::state::run::RunState;
    use crate::state::selection::{DomainEvent, DomainEventSource};

    fn golden_wing_run() -> RunState {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.current_hp = 20;
        run_state.max_hp = 80;
        run_state.event_state = Some(EventState::new(EventId::GoldenWing));
        run_state.emitted_events.clear();
        run_state
    }

    #[test]
    fn remove_path_damage_uses_event_source_before_purge_selection() {
        let mut run_state = golden_wing_run();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.current_hp, 13);
        assert!(run_state.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::HpChanged {
                delta: -7,
                current_hp: 13,
                max_hp: 80,
                source: DomainEventSource::Event(EventId::GoldenWing),
            }
        )));
        assert!(matches!(
            engine_state,
            EngineState::RunPendingChoice(ref pending)
                if pending.reason == RunPendingChoiceReason::PurgeNonBottled
        ));
    }

    #[test]
    fn remove_path_damage_respects_tungsten_rod_like_java_player_damage() {
        let mut run_state = golden_wing_run();
        run_state.relics.push(RelicState::new(RelicId::TungstenRod));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.current_hp, 14);
        assert!(run_state.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::HpChanged {
                delta: -6,
                current_hp: 14,
                max_hp: 80,
                source: DomainEventSource::Event(EventId::GoldenWing),
            }
        )));
    }
}
