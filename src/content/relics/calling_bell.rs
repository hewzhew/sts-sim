use crate::state::core::EngineState;
use crate::state::run::RunState;
use crate::content::cards::CardId;
use crate::content::relics::RelicTier;

pub fn on_equip(run_state: &mut RunState, _return_state: EngineState) -> Option<EngineState> {
    // Add Curse of the Bell directly to deck
    run_state.add_card_to_deck(CardId::CurseOfTheBell);

    let mut rs = crate::state::reward::RewardState::new();
    rs.items.push(crate::state::reward::RewardItem::Relic { relic_id: run_state.random_relic_by_tier(RelicTier::Common) });
    rs.items.push(crate::state::reward::RewardItem::Relic { relic_id: run_state.random_relic_by_tier(RelicTier::Uncommon) });
    rs.items.push(crate::state::reward::RewardItem::Relic { relic_id: run_state.random_relic_by_tier(RelicTier::Rare) });
    
    Some(EngineState::RewardScreen(rs))
}
