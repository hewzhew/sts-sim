use crate::rewards::state::{RewardItem, RewardState};
use crate::state::core::EngineState;
use crate::state::run::RunState;

pub fn on_equip(run_state: &mut RunState, return_state: EngineState) -> Option<EngineState> {
    let mut reward_state = match return_state {
        EngineState::RewardScreen(reward_state) => reward_state,
        _ => RewardState::new(),
    };

    let potion_class = run_state.potion_class();
    for _ in 0..5 {
        let potion_id = crate::content::potions::random_potion_any(
            &mut run_state.rng_pool.potion_rng,
            potion_class,
        );
        reward_state.items.push(RewardItem::Potion { potion_id });
    }

    if let Some(index) = reward_state
        .items
        .iter()
        .position(|item| matches!(item, RewardItem::Card { .. }))
    {
        reward_state.items.remove(index);
    }

    Some(EngineState::RewardScreen(reward_state))
}
