use crate::rewards::state::{RewardItem, RewardState};
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    if event_state.current_screen == 1 {
        return vec![EventChoiceMeta::new("[Leave]")];
    }

    let potion_count = if run_state.ascension_level >= 15 {
        2
    } else {
        3
    };
    vec![EventChoiceMeta::new(format!(
        "[Take] Obtain {} random Potions.",
        potion_count
    ))]
}

pub fn handle_choice(
    engine_state: &mut EngineState,
    run_state: &mut RunState,
    _choice_idx: usize,
) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            // Take potions
            let potion_count = if run_state.ascension_level >= 15 {
                2
            } else {
                3
            };
            // Java adds potion RewardItems and opens the combat reward screen.
            let mut rewards = RewardState::new();
            for _ in 0..potion_count {
                let pid = run_state.random_potion();
                rewards.items.push(RewardItem::Potion { potion_id: pid });
            }
            event_state.current_screen = 1;
            run_state.event_state = Some(event_state);
            *engine_state = EngineState::RewardScreen(rewards);
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
    use super::handle_choice;
    use crate::rewards::state::RewardItem;
    use crate::state::core::EngineState;
    use crate::state::events::{EventId, EventState};
    use crate::state::run::RunState;

    fn lab_run(ascension_level: u8) -> RunState {
        let mut run_state = RunState::new(1, ascension_level, false, "Ironclad");
        run_state.event_state = Some(EventState::new(EventId::Lab));
        run_state.emitted_events.clear();
        run_state
    }

    #[test]
    fn lab_opens_three_potion_rewards_without_directly_filling_inventory() {
        let mut run_state = lab_run(0);
        let starting_potions = run_state.potions.clone();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.potions, starting_potions);
        assert!(run_state.take_emitted_events().is_empty());
        match engine_state {
            EngineState::RewardScreen(rewards) => {
                assert_eq!(rewards.items.len(), 3);
                assert!(rewards
                    .items
                    .iter()
                    .all(|item| matches!(item, RewardItem::Potion { .. })));
            }
            other => panic!("expected reward screen, got {other:?}"),
        }
    }

    #[test]
    fn lab_ascension_fifteen_opens_two_potion_rewards() {
        let mut run_state = lab_run(15);
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        match engine_state {
            EngineState::RewardScreen(rewards) => {
                assert_eq!(rewards.items.len(), 2);
                assert!(rewards
                    .items
                    .iter()
                    .all(|item| matches!(item, RewardItem::Potion { .. })));
            }
            other => panic!("expected reward screen, got {other:?}"),
        }
    }
}
