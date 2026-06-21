use serde::{Deserialize, Serialize};

use crate::state::map::node::RoomType;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MapRouteTargetV1 {
    pub x: i32,
    pub y: i32,
    pub room_type: Option<RoomType>,
    pub has_emerald_key: bool,
    pub move_kind: RouteMoveKindV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum RouteMoveKindV1 {
    NormalEdge,
    WingBootsJump,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RoutePathSummaryV1 {
    pub path_count: usize,
    #[serde(default, skip_serializing_if = "is_false")]
    pub path_budget_exhausted: bool,
    pub min_early_pressure: usize,
    pub max_early_pressure: usize,
    pub min_elites: usize,
    pub max_elites: usize,
    pub min_shops: usize,
    pub max_shops: usize,
    pub min_fires: usize,
    pub max_fires: usize,
    pub min_unknowns: usize,
    pub max_unknowns: usize,
    pub min_treasures: usize,
    pub max_treasures: usize,
    pub first_shop_floor: Option<i32>,
    pub first_fire_floor: Option<i32>,
    #[serde(default)]
    pub min_damage_rooms_before_recovery: usize,
    #[serde(default)]
    pub max_damage_rooms_before_recovery: usize,
    #[serde(default)]
    pub min_unknowns_before_recovery: usize,
    #[serde(default)]
    pub max_unknowns_before_recovery: usize,
    #[serde(default)]
    pub paths_with_recovery_before_damage: usize,
    pub first_elite: RouteFirstEliteSegmentV1,
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RouteFirstEliteSegmentV1 {
    pub paths_with_first_elite: usize,
    pub forced: bool,
    pub optional: bool,
    pub min_hallway_fights_before: usize,
    pub max_hallway_fights_before: usize,
    pub min_unknowns_before: usize,
    pub max_unknowns_before: usize,
    pub min_fires_before: usize,
    pub max_fires_before: usize,
    pub min_shops_before: usize,
    pub max_shops_before: usize,
    pub can_bail_to_rest_before: bool,
    pub can_bail_to_shop_before: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NodeFeaturesV1 {
    pub node_type: Option<RoomType>,
    pub expected_card_rewards: f32,
    pub expected_relics: f32,
    pub expected_gold_gain: f32,
    pub expected_potion_gain: f32,
    #[serde(default)]
    pub expected_curse_debt: f32,
    pub shop_access: f32,
    pub remove_access: f32,
    pub upgrade_access: f32,
    pub heal_access: f32,
    pub event_access: f32,
    pub expected_hp_loss_mean: f32,
    pub expected_hp_loss_p90: f32,
    pub death_risk: f32,
    pub is_elite: bool,
    pub is_burning_elite: bool,
    pub is_rest: bool,
    pub is_shop: bool,
    pub is_question_mark: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum RouteSafetyFlagV1 {
    Ok,
    RiskyButAllowed,
    RejectUnlessNoAlternative,
}
