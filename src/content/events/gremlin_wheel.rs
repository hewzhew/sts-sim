// Java: GremlinWheelGame (shrines) — "Wheel of Change"
// Spin wheel → result 0-5:
//   0 = Gain gold (100/200/300 by act)
//   1 = Obtain random relic
//   2 = Heal to full HP
//   3 = Obtain Decay curse
//   4 = Remove a card (grid-select purge)
//   5 = Lose HP (10% max, 15% at A15+)
// Screen 0: [Spin] → spin result stored in internal_state
// Screen 1: [Leave] after result applied

use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => vec![EventChoiceMeta::new("[Spin] Spin the wheel!")],
        1 => {
            // Show result description
            let desc = match event_state.internal_state {
                0 => {
                    let gold = match run_state.act_num {
                        1 => 100,
                        2 => 200,
                        _ => 300,
                    };
                    format!("You gained {} Gold!", gold)
                }
                1 => "You obtained a random relic!".to_string(),
                2 => "You healed to full HP!".to_string(),
                3 => "A dark curse falls upon you... Gained Decay.".to_string(),
                4 => "The spirits consume a card...".to_string(),
                _ => {
                    let pct = if run_state.ascension_level >= 15 {
                        15
                    } else {
                        10
                    };
                    format!("The wheel damages you! Lost {}% max HP.", pct)
                }
            };
            vec![EventChoiceMeta::new(format!("{} [Leave]", desc))]
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, _choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    if event_state.completed {
        run_state.event_state = Some(event_state);
        return;
    }

    match event_state.current_screen {
        0 => {
            // Spin: roll 0-5 using miscRng (Java: miscRng.random(0, 5))
            let result = run_state.rng_pool.misc_rng.random_range(0, 5) as i32;

            // Apply result immediately
            match result {
                0 => {
                    // Gold
                    let gold = match run_state.act_num {
                        1 => 100,
                        2 => 200,
                        _ => 300,
                    };
                    run_state.change_gold_with_source(
                        gold,
                        DomainEventSource::Event(EventId::GremlinWheelGame),
                    );
                }
                1 => {
                    // Random relic — Java: addRelicToRewards(r) + combatRewardScreen.open()
                    let relic = run_state.random_screenless_relic_reward();
                    let mut rewards = crate::rewards::state::RewardState::new();
                    rewards
                        .items
                        .push(crate::rewards::state::RewardItem::Relic { relic_id: relic });
                    event_state.internal_state = result;
                    event_state.current_screen = 1;
                    run_state.event_state = Some(event_state);
                    *engine_state = EngineState::RewardScreen(rewards);
                    return;
                }
                2 => {
                    // Java: player.heal(player.maxHealth)
                    run_state.heal_with_source(
                        run_state.max_hp,
                        DomainEventSource::Event(EventId::GremlinWheelGame),
                    );
                }
                3 => {
                    // Obtain Decay curse
                    super::obtain_event_card(
                        run_state,
                        crate::state::events::EventId::GremlinWheelGame,
                        crate::content::cards::CardId::Decay,
                    );
                }
                4 => {
                    // Remove a card (Java: grid-select purge)
                    if crate::state::core::has_non_bottled_purgeable_master_deck_card(run_state) {
                        *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                            min_choices: 1,
                            max_choices: 1,
                            reason: RunPendingChoiceReason::PurgeNonBottled,
                            return_state: Box::new(EngineState::EventRoom),
                        });
                    }
                }
                _ => {
                    // Java: player.damage(DamageInfo(null, damage, HP_LOSS)).
                    // HP_LOSS bypasses block, but AbstractPlayer.damage still applies
                    // onLoseHpLast, so Tungsten Rod reduces this by 1.
                    let pct = if run_state.ascension_level >= 15 {
                        0.15f32
                    } else {
                        0.10f32
                    };
                    let mut damage = (run_state.max_hp as f32 * pct) as i32;
                    if damage > 0
                        && run_state
                            .relics
                            .iter()
                            .any(|r| r.id == crate::content::relics::RelicId::TungstenRod)
                    {
                        damage -= 1;
                    }
                    run_state.change_hp_with_source(
                        -damage,
                        DomainEventSource::Event(EventId::GremlinWheelGame),
                    );
                }
            }

            event_state.internal_state = result;
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
    use crate::runtime::rng::StsRng;
    use crate::state::core::EngineState;
    use crate::state::events::{EventId, EventState};
    use crate::state::run::RunState;
    use crate::state::selection::{DomainEvent, DomainEventSource};

    fn seed_for_wheel_result(result: i32) -> u64 {
        (1..10_000)
            .find(|seed| {
                let mut rng = StsRng::new(*seed);
                rng.random_range(0, 5) == result
            })
            .expect("test seed for Gremlin Wheel result")
    }

    fn wheel_run(current_hp: i32, max_hp: i32, ascension_level: u8) -> RunState {
        let mut run_state = RunState::new(1, ascension_level, false, "Ironclad");
        run_state.current_hp = current_hp;
        run_state.max_hp = max_hp;
        run_state.event_state = Some(EventState::new(EventId::GremlinWheelGame));
        run_state.emitted_events.clear();
        run_state
    }

    #[test]
    fn full_heal_uses_java_heal_source_and_respects_mark_of_the_bloom() {
        let mut run_state = wheel_run(20, 80, 0);
        run_state.rng_pool.misc_rng = StsRng::new(seed_for_wheel_result(2));
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
    fn full_heal_emits_event_source_without_mark() {
        let mut run_state = wheel_run(20, 80, 0);
        run_state.rng_pool.misc_rng = StsRng::new(seed_for_wheel_result(2));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.current_hp, 80);
        let events = run_state.take_emitted_events();
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::HpChanged {
                delta: 60,
                current_hp: 80,
                max_hp: 80,
                source: DomainEventSource::Event(EventId::GremlinWheelGame),
            }
        )));
    }

    #[test]
    fn hp_loss_result_uses_source_and_can_reduce_hp_to_zero() {
        let mut run_state = wheel_run(5, 80, 0);
        run_state.rng_pool.misc_rng = StsRng::new(seed_for_wheel_result(5));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.current_hp, 0);
        let events = run_state.take_emitted_events();
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::HpChanged {
                delta: -5,
                current_hp: 0,
                max_hp: 80,
                source: DomainEventSource::Event(EventId::GremlinWheelGame),
            }
        )));
    }

    #[test]
    fn hp_loss_result_applies_tungsten_rod_on_lose_hp_last() {
        let mut run_state = wheel_run(20, 80, 0);
        run_state.rng_pool.misc_rng = StsRng::new(seed_for_wheel_result(5));
        run_state.relics.push(RelicState::new(RelicId::TungstenRod));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.current_hp, 13);
        let events = run_state.take_emitted_events();
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::HpChanged {
                delta: -7,
                current_hp: 13,
                max_hp: 80,
                source: DomainEventSource::Event(EventId::GremlinWheelGame),
            }
        )));
    }
}
