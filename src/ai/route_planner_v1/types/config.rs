use serde::{Deserialize, Serialize};

use super::trace::{RouteObjectiveV1, RouteSelectionModeV1};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RoutePlannerConfigV1 {
    pub objective: RouteObjectiveV1,
    pub selection_mode: RouteSelectionModeV1,
    pub path_budget: usize,
    pub easy_pool_floor_cutoff: i32,
    pub base_monster_hp_loss: f32,
    pub base_elite_hp_loss: f32,
    pub low_hp_ratio: f32,
    pub very_low_hp_ratio: f32,
    pub early_shop_good_gold: i32,
    pub wing_boots_charge_cost: f32,
}

impl Default for RoutePlannerConfigV1 {
    fn default() -> Self {
        Self {
            objective: RouteObjectiveV1::DataCollectionSurvivalV1,
            selection_mode: RouteSelectionModeV1::DeterministicArgmax,
            path_budget: 2_000,
            easy_pool_floor_cutoff: 3,
            base_monster_hp_loss: 7.0,
            base_elite_hp_loss: 24.0,
            low_hp_ratio: 0.45,
            very_low_hp_ratio: 0.28,
            early_shop_good_gold: 150,
            wing_boots_charge_cost: 2.0,
        }
    }
}
