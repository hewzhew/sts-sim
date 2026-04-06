use crate::state::core::EngineState;
use crate::state::run::RunState;

pub fn on_equip(run_state: &mut RunState, _return_state: EngineState) -> Option<EngineState> {
    let mut rs = crate::state::reward::RewardState::new();
    for _ in 0..5 {
        let cards = run_state.generate_card_reward(3);
        rs.items.push(crate::state::RewardItem::Card { cards });
    }
    Some(EngineState::RewardScreen(rs))
}
