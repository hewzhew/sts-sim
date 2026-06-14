use crate::state::map::node::RoomType;

use super::super::types::{
    MapRouteTargetV1, NodeFeaturesV1, RoutePlannerConfigV1, UnknownRoomBeliefV1,
};

pub(in crate::ai::route_planner_v1) fn node_features(
    target: &MapRouteTargetV1,
    belief: &UnknownRoomBeliefV1,
    hp: i32,
    max_hp: i32,
    has_empty_potion_slot: bool,
    has_cursed_key: bool,
    config: &RoutePlannerConfigV1,
) -> NodeFeaturesV1 {
    let hp_ratio = if max_hp > 0 {
        hp as f32 / max_hp as f32
    } else {
        0.0
    };
    let mut features = base_node_features(target.room_type);
    if target.has_emerald_key && features.is_elite {
        features.is_burning_elite = true;
    }
    if target.room_type == Some(RoomType::EventRoom) {
        apply_unknown_room_belief(&mut features, belief, config);
    }
    if has_cursed_key {
        apply_cursed_key_chest_debt(&mut features, target.room_type, belief);
    }
    if !has_empty_potion_slot {
        features.expected_potion_gain = 0.0;
    }
    if hp_ratio < config.low_hp_ratio {
        features.death_risk += features.expected_hp_loss_p90 / max_hp.max(1) as f32;
    }
    features.death_risk = features.death_risk.clamp(0.0, 1.0);
    features
}

fn apply_cursed_key_chest_debt(
    features: &mut NodeFeaturesV1,
    room_type: Option<RoomType>,
    belief: &UnknownRoomBeliefV1,
) {
    match room_type {
        Some(RoomType::TreasureRoom) => features.expected_curse_debt += 1.0,
        Some(RoomType::EventRoom) => features.expected_curse_debt += belief.treasure_chance,
        _ => {}
    }
}

fn base_node_features(room_type: Option<RoomType>) -> NodeFeaturesV1 {
    match room_type {
        Some(RoomType::MonsterRoom) => NodeFeaturesV1 {
            node_type: room_type,
            expected_card_rewards: 1.0,
            expected_gold_gain: 15.0,
            expected_potion_gain: 0.35,
            expected_hp_loss_mean: 7.0,
            expected_hp_loss_p90: 14.0,
            death_risk: 0.05,
            ..empty_features()
        },
        Some(RoomType::MonsterRoomElite) => NodeFeaturesV1 {
            node_type: room_type,
            expected_card_rewards: 1.0,
            expected_relics: 1.0,
            expected_gold_gain: 30.0,
            expected_potion_gain: 0.25,
            expected_hp_loss_mean: 24.0,
            expected_hp_loss_p90: 40.0,
            death_risk: 0.25,
            is_elite: true,
            ..empty_features()
        },
        Some(RoomType::RestRoom) => NodeFeaturesV1 {
            node_type: room_type,
            upgrade_access: 1.0,
            heal_access: 1.0,
            is_rest: true,
            ..empty_features()
        },
        Some(RoomType::ShopRoom) => NodeFeaturesV1 {
            node_type: room_type,
            shop_access: 1.0,
            remove_access: 1.0,
            is_shop: true,
            ..empty_features()
        },
        Some(RoomType::TreasureRoom) => NodeFeaturesV1 {
            node_type: room_type,
            expected_relics: 1.0,
            ..empty_features()
        },
        Some(RoomType::EventRoom) => NodeFeaturesV1 {
            node_type: room_type,
            is_question_mark: true,
            ..empty_features()
        },
        Some(RoomType::MonsterRoomBoss) => NodeFeaturesV1 {
            node_type: room_type,
            expected_relics: 1.0,
            expected_hp_loss_mean: 35.0,
            expected_hp_loss_p90: 60.0,
            death_risk: 0.45,
            ..empty_features()
        },
        _ => NodeFeaturesV1 {
            node_type: room_type,
            ..empty_features()
        },
    }
}

fn apply_unknown_room_belief(
    features: &mut NodeFeaturesV1,
    belief: &UnknownRoomBeliefV1,
    config: &RoutePlannerConfigV1,
) {
    features.expected_card_rewards = belief.monster_chance;
    features.expected_relics = belief.treasure_chance + belief.elite_chance;
    features.expected_gold_gain = belief.monster_chance * 15.0;
    features.expected_potion_gain = belief.monster_chance * 0.35;
    features.shop_access = belief.shop_chance;
    features.remove_access = belief.shop_chance;
    features.event_access = belief.event_chance;
    features.expected_hp_loss_mean = belief.monster_chance * config.base_monster_hp_loss
        + belief.elite_chance * config.base_elite_hp_loss;
    features.expected_hp_loss_p90 = features.expected_hp_loss_mean * 1.8;
    features.death_risk = belief.monster_chance * 0.05 + belief.elite_chance * 0.25;
}

fn empty_features() -> NodeFeaturesV1 {
    NodeFeaturesV1 {
        node_type: None,
        expected_card_rewards: 0.0,
        expected_relics: 0.0,
        expected_gold_gain: 0.0,
        expected_potion_gain: 0.0,
        expected_curse_debt: 0.0,
        shop_access: 0.0,
        remove_access: 0.0,
        upgrade_access: 0.0,
        heal_access: 0.0,
        event_access: 0.0,
        expected_hp_loss_mean: 0.0,
        expected_hp_loss_p90: 0.0,
        death_risk: 0.0,
        is_elite: false,
        is_burning_elite: false,
        is_rest: false,
        is_shop: false,
        is_question_mark: false,
    }
}
