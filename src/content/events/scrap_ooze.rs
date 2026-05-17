use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

// internal_state encodes: lower 16 bits = relic_chance, upper 16 bits = current_dmg

fn decode_state(state: i32) -> (i32, i32) {
    let chance = state & 0xFFFF;
    let dmg = (state >> 16) & 0xFFFF;
    (chance, dmg)
}

fn encode_state(chance: i32, dmg: i32) -> i32 {
    (dmg << 16) | (chance & 0xFFFF)
}

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    if event_state.current_screen == 1 {
        return vec![EventChoiceMeta::new("[Leave]")];
    }

    let (chance, dmg) = if event_state.internal_state == 0 {
        // Initial state
        let base_dmg = if run_state.ascension_level >= 15 {
            5
        } else {
            3
        };
        (25, base_dmg)
    } else {
        decode_state(event_state.internal_state)
    };

    vec![
        EventChoiceMeta::new(format!(
            "[Reach In] Take {} damage. {}% chance to obtain a Relic.",
            dmg, chance
        )),
        EventChoiceMeta::new("[Leave]"),
    ]
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Reach In
                    let (mut chance, mut dmg) = if event_state.internal_state == 0 {
                        let base_dmg = if run_state.ascension_level >= 15 {
                            5
                        } else {
                            3
                        };
                        (25, base_dmg)
                    } else {
                        decode_state(event_state.internal_state)
                    };

                    // Java uses DamageInfo(null, dmg): no block, no Torii owner hook,
                    // but Tungsten Rod still applies on HP loss.
                    super::apply_player_default_damage(
                        run_state,
                        dmg,
                        super::EventDamageOwner::None,
                        DomainEventSource::Event(EventId::ScrapOoze),
                    );

                    // Roll for relic
                    let roll = run_state.rng_pool.misc_rng.random_range(0, 99);
                    if roll >= 99 - chance {
                        // Success! Get a relic
                        let relic_id = run_state.random_screenless_relic_reward();
                        if let Some(next_state) = run_state.obtain_relic_with_source(
                            relic_id,
                            EngineState::EventRoom,
                            DomainEventSource::Event(EventId::ScrapOoze),
                        ) {
                            *_engine_state = next_state;
                        }
                        event_state.current_screen = 1;
                    } else {
                        // Fail: escalate
                        chance += 10;
                        dmg += 1;
                        event_state.internal_state = encode_state(chance, dmg);
                        // Stay on screen 0
                    }
                }
                _ => {
                    // Flee
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
    use crate::state::core::EngineState;
    use crate::state::events::{EventId, EventState};
    use crate::state::run::RunState;
    use crate::state::selection::{DomainEvent, DomainEventSource};

    #[test]
    fn reach_in_default_null_damage_ignores_torii_but_applies_tungsten() {
        let mut run_state = RunState::new(1, 0, true, "Ironclad");
        run_state.current_hp = 30;
        run_state.max_hp = 80;
        run_state.relics.push(RelicState::new(RelicId::Torii));
        run_state.relics.push(RelicState::new(RelicId::TungstenRod));
        run_state.event_state = Some(EventState::new(EventId::ScrapOoze));
        run_state.emitted_events.clear();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(
            run_state.current_hp, 28,
            "Java DamageInfo(null, 3) skips Torii but Tungsten reduces HP loss by 1"
        );
        assert!(run_state.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::HpChanged {
                delta: -2,
                current_hp: 28,
                max_hp: 80,
                source: DomainEventSource::Event(EventId::ScrapOoze),
            }
        )));
    }
}
