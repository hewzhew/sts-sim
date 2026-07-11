use super::super::action_effects::state_sustained_mitigation_score;
use super::super::card_pile_value::{
    choker_capacity, hand_value, next_draw_value, CardPileValueV1, ChokerCapacityV1,
};
use super::super::phase_profile::{combat_search_phase_profile, CombatSearchPhaseProfileV1};
use super::super::value_facts::living_enemy_count;
use crate::runtime::combat::CombatState;
use crate::state::core::EngineState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::ai::combat_search_v2::value) struct CombatSearchCoreValueFactsV1 {
    pub(in crate::ai::combat_search_v2::value) living_enemy_count: usize,
    pub(in crate::ai::combat_search_v2::value) phase_profile: CombatSearchPhaseProfileV1,
    pub(in crate::ai::combat_search_v2::value) sustained_mitigation: i32,
    pub(in crate::ai::combat_search_v2::value) hand: CardPileValueV1,
    pub(in crate::ai::combat_search_v2::value) choker_capacity: ChokerCapacityV1,
    pub(in crate::ai::combat_search_v2::value) next_draw: CardPileValueV1,
}

pub(in crate::ai::combat_search_v2::value) fn combat_search_core_value_facts(
    engine: &EngineState,
    combat: &CombatState,
) -> CombatSearchCoreValueFactsV1 {
    CombatSearchCoreValueFactsV1 {
        living_enemy_count: living_enemy_count(combat),
        phase_profile: combat_search_phase_profile(engine, combat),
        sustained_mitigation: state_sustained_mitigation_score(combat),
        hand: hand_value(combat),
        choker_capacity: choker_capacity(combat),
        next_draw: next_draw_value(combat),
    }
}
