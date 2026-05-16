use crate::content::relics::RelicId;
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            vec![
                EventChoiceMeta::new(format!("[Pay] Lose all ({}) Gold.", run_state.gold)),
                EventChoiceMeta::new("[Fight] Engage the bandits!"),
            ]
        }
        1 | 2 | 3 => {
            // Multi-screen dialogue after paying
            vec![EventChoiceMeta::new("[Continue]")]
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Pay all gold
                    run_state
                        .set_gold_with_source(0, DomainEventSource::Event(EventId::MaskedBandits));
                    event_state.current_screen = 1;
                }
                _ => {
                    // Fight bandits
                    let gold = run_state.rng_pool.misc_rng.random_range(25, 35);
                    let mut rewards = crate::rewards::state::RewardState::new();
                    rewards
                        .items
                        .push(crate::rewards::state::RewardItem::Gold { amount: gold });

                    if run_state.relics.iter().any(|r| r.id == RelicId::RedMask) {
                        rewards
                            .items
                            .push(crate::rewards::state::RewardItem::Relic {
                                relic_id: RelicId::Circlet,
                            });
                    } else {
                        rewards
                            .items
                            .push(crate::rewards::state::RewardItem::Relic {
                                relic_id: RelicId::RedMask,
                            });
                    }

                    event_state.completed = true;
                    run_state.event_state = Some(event_state);

                    *engine_state =
                        EngineState::EventCombat(crate::state::core::EventCombatState {
                            rewards,
                            reward_allowed: true,
                            no_cards_in_rewards: false,
                            post_combat_return: crate::state::core::PostCombatReturn::MapNavigation,
                            encounter_key: "Masked Bandits",
                        });
                    return;
                }
            }
        }
        1 => {
            event_state.current_screen = 2;
        }
        2 => {
            event_state.current_screen = 3;
        }
        3 => {
            event_state.completed = true;
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

    #[test]
    fn pay_path_opens_map_after_java_dialog_sequence_without_extra_leave_click() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.gold = 123;
        run_state.event_state = Some(EventState::new(EventId::MaskedBandits));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);
        assert_eq!(run_state.gold, 0);
        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 1);

        handle_choice(&mut engine_state, &mut run_state, 0);
        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 2);

        handle_choice(&mut engine_state, &mut run_state, 0);
        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 3);

        handle_choice(&mut engine_state, &mut run_state, 0);
        assert!(run_state.event_state.as_ref().unwrap().completed);
    }

    #[test]
    fn fight_uses_java_event_encounter_key_and_event_rewards() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.event_state = Some(EventState::new(EventId::MaskedBandits));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 1);

        let EngineState::EventCombat(combat) = engine_state else {
            panic!("fight should enter EventCombat");
        };
        assert_eq!(combat.encounter_key, "Masked Bandits");
        assert!(combat.reward_allowed);
        assert!(combat
            .rewards
            .items
            .iter()
            .any(|item| matches!(item, crate::rewards::state::RewardItem::Gold { amount } if (25..=35).contains(amount))));
        assert!(combat.rewards.items.iter().any(|item| matches!(
            item,
            crate::rewards::state::RewardItem::Relic {
                relic_id: RelicId::RedMask
            }
        )));
    }
}
