use crate::state::core::{EngineState, EventCombatState, PostCombatReturn};
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(_run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            vec![
                EventChoiceMeta::new("[Open] Fight the guardians for a rare Relic!"),
                EventChoiceMeta::new("[Leave]"),
            ]
        }
        1 => {
            // Confirm fight
            vec![EventChoiceMeta::new("[Fight]")]
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
                    // Open — advance to confirm screen
                    event_state.current_screen = 1;
                }
                _ => {
                    // Java first moves to END text, then opens the map on the
                    // next click.
                    event_state.current_screen = 2;
                }
            }
        }
        1 => {
            // Fight! Set up rewards and enter event combat.
            // Java: miscRng.random(45, 55) for gold, returnRandomScreenlessRelic(RARE) for relic
            let gold = run_state.rng_pool.misc_rng.random_range(45, 55);
            let mut rewards = crate::rewards::state::RewardState::new();
            rewards
                .items
                .push(crate::rewards::state::RewardItem::Gold { amount: gold });

            let relic_id =
                run_state.random_screenless_relic(crate::content::relics::RelicTier::Rare);
            rewards
                .items
                .push(crate::rewards::state::RewardItem::Relic { relic_id });

            event_state.completed = true;
            run_state.event_state = Some(event_state);

            // Transition to event combat
            *engine_state = EngineState::EventCombat(EventCombatState {
                rewards,
                reward_allowed: true,
                no_cards_in_rewards: false,
                elite_trigger: false,
                post_combat_return: PostCombatReturn::MapNavigation,
                encounter_key: "2 Orb Walkers",
            });
            return;
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
    fn leave_path_preserves_java_end_screen_before_map() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.event_state = Some(EventState::new(
            crate::state::events::EventId::MysteriousSphere,
        ));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 1);

        let event_state = run_state.event_state.as_ref().unwrap();
        assert_eq!(event_state.current_screen, 2);
        assert!(!event_state.completed);
        assert!(matches!(engine_state, EngineState::EventRoom));

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert!(run_state.event_state.as_ref().unwrap().completed);
    }

    #[test]
    fn fight_path_generates_java_event_rewards_before_event_combat() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.event_state = Some(EventState::new(
            crate::state::events::EventId::MysteriousSphere,
        ));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);
        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 1);

        handle_choice(&mut engine_state, &mut run_state, 0);

        let EngineState::EventCombat(combat) = engine_state else {
            panic!("confirmed Mysterious Sphere fight should enter EventCombat");
        };
        assert_eq!(combat.encounter_key, "2 Orb Walkers");
        assert!(combat.reward_allowed);
        assert!(combat
            .rewards
            .items
            .iter()
            .any(|item| matches!(item, crate::rewards::state::RewardItem::Gold { amount } if (45..=55).contains(amount))));
        assert!(combat
            .rewards
            .items
            .iter()
            .any(|item| matches!(item, crate::rewards::state::RewardItem::Relic { .. })));
    }
}
