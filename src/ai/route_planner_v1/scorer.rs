use crate::state::map::node::RoomType;

use super::types::{
    NeedVectorV1, NodeFeaturesV1, RouteMoveKindV1, RoutePathSummaryV1, RoutePlannerConfigV1,
    RouteSafetyFlagV1, RouteScoreTermsV1,
};

pub(super) fn score_route_candidate(
    features: &NodeFeaturesV1,
    path: &RoutePathSummaryV1,
    needs: &NeedVectorV1,
    move_kind: RouteMoveKindV1,
    emerald_key_taken: bool,
    config: &RoutePlannerConfigV1,
) -> RouteScoreTermsV1 {
    RouteScoreTermsV1 {
        card_reward: needs.need_card_rewards
            * (features.expected_card_rewards + path.max_early_pressure as f32 * 0.15),
        relic: needs.need_relics * (features.expected_relics + path.max_elites as f32 * 0.45),
        remove: needs.need_remove * (features.remove_access + path.max_shops as f32 * 0.12),
        upgrade: needs.need_upgrade * (features.upgrade_access + path.max_fires as f32 * 0.10),
        heal: needs.need_heal * (features.heal_access + path.max_fires as f32 * 0.16),
        shop: needs.need_shop * (features.shop_access + path.max_shops as f32 * 0.10),
        event: needs.need_event * (features.event_access + path.max_unknowns as f32 * 0.08),
        potion: needs.need_potion * features.expected_potion_gain,
        hp_loss: -needs.avoid_damage * features.expected_hp_loss_p90 / 12.0,
        death_risk: -features.death_risk * (1.0 + needs.avoid_damage) * 5.0,
        flexibility: needs.value_flexibility * flexibility_value(path),
        wing_boots_cost: if move_kind == RouteMoveKindV1::WingBootsJump {
            -config.wing_boots_charge_cost
        } else {
            0.0
        },
        forced_path_penalty: forced_path_penalty(path, needs),
        burning_elite_key_value: if features.is_burning_elite && !emerald_key_taken {
            0.75
        } else {
            0.0
        },
    }
}

pub(super) fn safety_flag(
    features: &NodeFeaturesV1,
    path: &RoutePathSummaryV1,
    needs: &NeedVectorV1,
) -> RouteSafetyFlagV1 {
    let forced_elite = path.min_elites > 0 || features.is_elite;
    let no_rest_bailout = path.max_fires == 0 && !features.is_rest;
    if forced_elite && no_rest_bailout && needs.can_take_elite < 0.45 {
        return RouteSafetyFlagV1::RejectUnlessNoAlternative;
    }
    if features.death_risk > 0.35 || needs.avoid_damage > 0.65 && expected_damage_room(features) {
        return RouteSafetyFlagV1::RiskyButAllowed;
    }
    RouteSafetyFlagV1::Ok
}

pub(super) fn route_reasons(
    features: &NodeFeaturesV1,
    path: &RoutePathSummaryV1,
    safety: RouteSafetyFlagV1,
) -> (Vec<String>, Vec<String>) {
    let mut reasons = Vec::new();
    let mut cautions = Vec::new();
    match features.node_type {
        Some(RoomType::MonsterRoom) => {
            reasons.push("immediate card reward/gold source".to_string())
        }
        Some(RoomType::MonsterRoomElite) => reasons.push("immediate relic route".to_string()),
        Some(RoomType::RestRoom) => reasons.push("immediate rest/smith access".to_string()),
        Some(RoomType::ShopRoom) => reasons.push("immediate shop/remove access".to_string()),
        Some(RoomType::EventRoom) => {
            reasons.push("question mark evaluated as mixed outcomes".to_string())
        }
        Some(RoomType::TreasureRoom) => reasons.push("immediate relic without combat".to_string()),
        _ => {}
    }
    if path.max_elites > path.min_elites {
        reasons.push("elite fights are optional on visible continuations".to_string());
    } else if path.min_elites > 0 {
        cautions.push("elite pressure is forced on visible continuations".to_string());
    }
    if path.min_fires > 0 {
        reasons.push("rest site is guaranteed somewhere on the route".to_string());
    } else if path.max_fires > 0 {
        reasons.push("rest site exists on some visible continuations".to_string());
    } else {
        cautions.push("no visible rest site before boss".to_string());
    }
    if path.max_shops > 0 {
        reasons.push(format!("shop access exists: {}", path.max_shops));
    }
    if path.path_count > 1 {
        reasons.push(format!(
            "keeps {} visible continuations open",
            path.path_count
        ));
    }
    match safety {
        RouteSafetyFlagV1::Ok => {}
        RouteSafetyFlagV1::RiskyButAllowed => {
            cautions.push("risk gate: risky but allowed".to_string())
        }
        RouteSafetyFlagV1::RejectUnlessNoAlternative => {
            cautions.push("risk gate: reject unless no safer alternative exists".to_string())
        }
    }
    (reasons, cautions)
}

fn flexibility_value(path: &RoutePathSummaryV1) -> f32 {
    let branches = (path.path_count as f32).ln_1p().min(4.0) / 4.0;
    let room_variety = usize::from(path.max_fires > 0)
        + usize::from(path.max_shops > 0)
        + usize::from(path.max_unknowns > 0)
        + usize::from(path.max_elites > 0);
    branches + room_variety as f32 * 0.12
}

fn forced_path_penalty(path: &RoutePathSummaryV1, needs: &NeedVectorV1) -> f32 {
    if path.min_elites > 0 && needs.can_take_elite < 0.5 {
        -1.5
    } else {
        0.0
    }
}

fn expected_damage_room(features: &NodeFeaturesV1) -> bool {
    matches!(
        features.node_type,
        Some(RoomType::MonsterRoom | RoomType::MonsterRoomElite | RoomType::MonsterRoomBoss)
    ) || features.expected_hp_loss_p90 > 0.0
}
