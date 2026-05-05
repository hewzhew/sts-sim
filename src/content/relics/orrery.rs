use crate::state::core::EngineState;
use crate::state::run::RunState;

pub fn on_equip(run_state: &mut RunState, _return_state: EngineState) -> Option<EngineState> {
    let mut rs = crate::rewards::state::RewardState::new();
    for _ in 0..5 {
        let cards = crate::rewards::generator::generate_card_reward(
            run_state,
            crate::rewards::generator::adjusted_card_reward_choice_count(run_state, 3),
            false,
        );
        rs.items
            .push(crate::rewards::state::RewardItem::Card { cards });
    }
    Some(EngineState::RewardScreen(rs))
}

#[cfg(test)]
mod tests {
    use super::on_equip;
    use crate::content::relics::{RelicId, RelicState};
    use crate::state::core::EngineState;
    use crate::state::run::RunState;

    #[test]
    fn orrery_card_rewards_respect_question_card() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();
        run_state
            .relics
            .push(RelicState::new(RelicId::QuestionCard));

        let next_state = on_equip(&mut run_state, EngineState::MapNavigation)
            .expect("orrery should open reward screen");

        match next_state {
            EngineState::RewardScreen(reward_state) => {
                assert_eq!(reward_state.items.len(), 5);
                for item in reward_state.items {
                    match item {
                        crate::rewards::state::RewardItem::Card { cards } => {
                            assert_eq!(cards.len(), 4);
                        }
                        other => panic!("expected card reward, got {other:?}"),
                    }
                }
            }
            other => panic!("expected reward screen, got {other:?}"),
        }
    }
}
