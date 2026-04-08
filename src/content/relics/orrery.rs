use crate::state::core::EngineState;
use crate::state::run::RunState;

pub fn on_equip(run_state: &mut RunState, _return_state: EngineState) -> Option<EngineState> {
    let mut rs = crate::rewards::state::RewardState::new();
    for _ in 0..5 {
        let cards = crate::rewards::generator::generate_card_reward(run_state, 3, false);
        rs.items
            .push(crate::rewards::state::RewardItem::Card { cards });
    }
    Some(EngineState::RewardScreen(rs))
}
