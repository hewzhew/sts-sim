use serde::{Deserialize, Serialize};

use crate::content::cards::{CardId, CardType};
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;
use crate::state::map::node::RoomType;

pub const ROUTE_DECISION_TRACE_SCHEMA_NAME: &str = "RouteDecisionTraceV1";
pub const ROUTE_DECISION_TRACE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RouteDecisionTraceV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub objective: RouteObjectiveV1,
    pub selection_mode: RouteSelectionModeV1,
    pub label_role: String,
    pub context: RouteDecisionContextV1,
    pub candidates: Vec<RouteCandidateTraceV1>,
    pub selected_index: Option<usize>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RouteObjectiveV1 {
    DataCollectionSurvivalV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RouteSelectionModeV1 {
    DeterministicArgmax,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RouteDecisionContextV1 {
    pub act: u8,
    pub floor: i32,
    pub ascension: u8,
    pub class: String,
    pub boss: Option<String>,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub deck: DeckRouteSummaryV1,
    pub relics: RouteRelicSummaryV1,
    pub potions: PotionRouteSummaryV1,
    pub current_x: i32,
    pub current_y: i32,
    pub legal_next_nodes: Vec<MapRouteTargetV1>,
    pub counters: RouteCountersV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeckRouteSummaryV1 {
    pub deck_size: usize,
    pub starter_strikes: u8,
    pub starter_defends: u8,
    pub curses: u8,
    pub attacks: u8,
    pub skills: u8,
    pub powers: u8,
    pub frontload_damage_score: i32,
    pub block_score: i32,
    pub aoe_score: i32,
    pub scaling_score: i32,
    pub draw_score: i32,
    pub energy_score: i32,
    pub key_upgrades_available: u8,
    pub important_cards_unupgraded: u8,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RouteRelicSummaryV1 {
    pub relic_count: usize,
    pub relics: Vec<RelicId>,
    pub wing_boots_charges: u8,
    pub has_juzu_bracelet: bool,
    pub has_tiny_chest: bool,
    pub has_preserved_insect: bool,
    pub has_peace_pipe: bool,
    pub has_shovel: bool,
    pub has_girya: bool,
    pub has_smiling_mask: bool,
    pub has_membership_card: bool,
    pub has_courier: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PotionRouteSummaryV1 {
    pub slots: usize,
    pub filled: usize,
    pub potions: Vec<PotionId>,
    pub has_elite_potion_signal: bool,
    pub has_defensive_potion_signal: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RouteCountersV1 {
    pub unknown_belief: UnknownRoomBeliefV1,
    pub wing_boots_charges: u8,
    pub emerald_key_taken: bool,
    pub ruby_key_taken: bool,
    pub sapphire_key_taken: bool,
    pub normal_fights_remaining_scheduled: usize,
    pub elite_fights_remaining_scheduled: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct UnknownRoomBeliefV1 {
    pub monster_chance: f32,
    pub shop_chance: f32,
    pub treasure_chance: f32,
    pub event_chance: f32,
    pub elite_chance: f32,
    pub has_juzu_bracelet: bool,
    pub has_tiny_chest: bool,
    pub deadly_events: bool,
}

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
pub struct RouteCandidateTraceV1 {
    pub target: MapRouteTargetV1,
    pub features: NodeFeaturesV1,
    pub path_summary: RoutePathSummaryV1,
    pub needs: NeedVectorV1,
    pub score_terms: RouteScoreTermsV1,
    pub total_score: f32,
    pub safety: RouteSafetyFlagV1,
    pub reasons: Vec<String>,
    pub cautions: Vec<String>,
    pub suggested_command: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RoutePathSummaryV1 {
    pub path_count: usize,
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
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NodeFeaturesV1 {
    pub node_type: Option<RoomType>,
    pub expected_card_rewards: f32,
    pub expected_relics: f32,
    pub expected_gold_gain: f32,
    pub expected_potion_gain: f32,
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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NeedVectorV1 {
    pub need_card_rewards: f32,
    pub need_relics: f32,
    pub need_remove: f32,
    pub need_upgrade: f32,
    pub need_heal: f32,
    pub need_shop: f32,
    pub need_event: f32,
    pub need_potion: f32,
    pub can_take_elite: f32,
    pub avoid_damage: f32,
    pub value_flexibility: f32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RouteScoreTermsV1 {
    pub card_reward: f32,
    pub relic: f32,
    pub remove: f32,
    pub upgrade: f32,
    pub heal: f32,
    pub shop: f32,
    pub event: f32,
    pub potion: f32,
    pub hp_loss: f32,
    pub death_risk: f32,
    pub flexibility: f32,
    pub wing_boots_cost: f32,
    pub forced_path_penalty: f32,
    pub burning_elite_key_value: f32,
}

impl RouteScoreTermsV1 {
    pub fn total(&self) -> f32 {
        self.card_reward
            + self.relic
            + self.remove
            + self.upgrade
            + self.heal
            + self.shop
            + self.event
            + self.potion
            + self.hp_loss
            + self.death_risk
            + self.flexibility
            + self.wing_boots_cost
            + self.forced_path_penalty
            + self.burning_elite_key_value
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum RouteSafetyFlagV1 {
    Ok,
    RiskyButAllowed,
    RejectUnlessNoAlternative,
}

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

impl DeckRouteSummaryV1 {
    pub(crate) fn observes_card(&mut self, card_id: CardId, card_type: CardType, upgrades: u8) {
        match card_type {
            CardType::Attack => self.attacks += 1,
            CardType::Skill => self.skills += 1,
            CardType::Power => self.powers += 1,
            CardType::Curse => self.curses += 1,
            CardType::Status => {}
        }
        match card_id {
            CardId::Strike | CardId::StrikeG | CardId::StrikeB | CardId::StrikeP => {
                self.starter_strikes += 1
            }
            CardId::Defend | CardId::DefendG | CardId::DefendB | CardId::DefendP => {
                self.starter_defends += 1
            }
            _ => {}
        }
        if upgrades == 0
            && matches!(
                card_id,
                CardId::Bash | CardId::Neutralize | CardId::Eruption
            )
        {
            self.important_cards_unupgraded += 1;
        }
    }
}
