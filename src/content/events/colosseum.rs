// Java: Colosseum (city)
// Screen 0 (INTRO): [Proceed] → fight description
// Screen 1 (FIGHT): [Fight!] → combat with "Colosseum Slavers" (no rewards, rewardAllowed=false)
// After first combat → reopen() → Screen 2 (POST_COMBAT):
//   [Flee] → leave (openMap)
//   [Fight] → combat with "Colosseum Nobs" (rewards: RARE relic, UNCOMMON relic, 100g)
// Screen 3 (LEAVE): [Leave] → openMap

use crate::content::monsters::factory::EncounterId;
use crate::rewards::state::{RewardItem, RewardState};
use crate::state::core::{CombatStartRequest, EngineState, PostCombatReturn};
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(_run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            // Introduction
            vec![EventChoiceMeta::new("[Proceed]")]
        }
        1 => {
            // Ready to fight
            vec![EventChoiceMeta::new("[Fight!]")]
        }
        2 => {
            // Post-first-combat: choose to fight Nobs or flee
            vec![
                EventChoiceMeta::new("[Flee] Leave the Colosseum."),
                EventChoiceMeta::new("[Fight] Challenge the Nobs for riches!"),
            ]
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            // Intro → ready to fight
            event_state.current_screen = 1;
        }
        1 => {
            // First fight: Colosseum Slavers (no rewards)
            // Java: rewardAllowed = false, enterCombatFromImage()
            // After combat, return to EventRoom → screen 2 (POST_COMBAT)
            event_state.current_screen = 2;
            run_state.event_state = Some(event_state);
            *engine_state = EngineState::CombatStart(CombatStartRequest::event(
                EncounterId::ColosseumSlavers,
                RewardState::new(),
                false,
                true,
                false,
                PostCombatReturn::EventRoom,
            ));
            return;
        }
        2 => {
            match choice_idx {
                0 => {
                    // Flee — leave the Colosseum
                    event_state.completed = true;
                }
                _ => {
                    // Fight Nobs: set up rewards BEFORE combat (matches Java)
                    // Java: addRelicToRewards(RARE), addRelicToRewards(UNCOMMON), addGoldToRewards(100)
                    let mut rewards = RewardState::new();

                    let rare_relic =
                        run_state.random_screenless_relic(crate::content::relics::RelicTier::Rare);
                    rewards.items.push(RewardItem::Relic {
                        relic_id: rare_relic,
                    });

                    let uncommon_relic = run_state
                        .random_screenless_relic(crate::content::relics::RelicTier::Uncommon);
                    rewards.items.push(RewardItem::Relic {
                        relic_id: uncommon_relic,
                    });

                    // Gold reward
                    rewards.items.push(RewardItem::Gold { amount: 100 });

                    event_state.current_screen = 3;
                    event_state.completed = true;
                    run_state.event_state = Some(event_state);
                    *engine_state = EngineState::CombatStart(CombatStartRequest::event(
                        EncounterId::ColosseumNobs,
                        rewards,
                        true,
                        false,
                        true,
                        PostCombatReturn::MapNavigation,
                    ));
                    return;
                }
            }
        }
        _ => {
            // Post-second-combat leave
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::core::CombatContext;

    #[test]
    fn first_fight_returns_to_event_room_without_rewards_or_elite_trigger() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.event_state = Some(crate::state::events::EventState::new(
            crate::state::events::EventId::Colosseum,
        ));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);
        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 1);

        handle_choice(&mut engine_state, &mut run_state, 0);

        let EngineState::CombatStart(request) = engine_state else {
            panic!("first Colosseum fight should request CombatStart");
        };
        assert_eq!(request.encounter_id, EncounterId::ColosseumSlavers);
        let CombatContext::Event(combat) = request.context else {
            panic!("first Colosseum fight should carry event combat context");
        };
        assert!(!combat.reward_allowed);
        assert!(combat.no_cards_in_rewards);
        assert!(!combat.elite_trigger);
        assert!(matches!(
            combat.post_combat_return,
            PostCombatReturn::EventRoom
        ));
    }

    #[test]
    fn second_fight_preserves_java_elite_trigger_without_normal_elite_rewards() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.event_state = Some(crate::state::events::EventState::new(
            crate::state::events::EventId::Colosseum,
        ));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);
        handle_choice(&mut engine_state, &mut run_state, 0);
        run_state.event_state.as_mut().unwrap().current_screen = 2;
        engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 1);

        let EngineState::CombatStart(request) = engine_state else {
            panic!("second Colosseum fight should request CombatStart");
        };
        assert_eq!(request.encounter_id, EncounterId::ColosseumNobs);
        let CombatContext::Event(combat) = request.context else {
            panic!("second Colosseum fight should carry event combat context");
        };
        assert!(combat.reward_allowed);
        assert!(combat.elite_trigger);
        assert_eq!(combat.rewards.items.len(), 3);
        assert!(combat
            .rewards
            .items
            .iter()
            .any(|item| matches!(item, RewardItem::Gold { amount: 100 })));
        assert!(combat
            .rewards
            .items
            .iter()
            .any(|item| matches!(item, RewardItem::Relic { .. })));
    }
}
