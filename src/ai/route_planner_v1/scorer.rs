use crate::state::map::node::RoomType;

use super::risk::first_elite_is_underprepared;
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
    has_cursed_key: bool,
    config: &RoutePlannerConfigV1,
) -> RouteScoreTermsV1 {
    RouteScoreTermsV1 {
        card_reward: needs.need_card_rewards
            * (features.expected_card_rewards + path.max_early_pressure as f32 * 0.15),
        relic: needs.need_relics * (features.expected_relics + path.max_elites as f32 * 0.45),
        remove: needs.need_remove * needs.need_shop * shop_route_access_value(features, path),
        upgrade: needs.need_upgrade * (features.upgrade_access + path.max_fires as f32 * 0.10),
        heal: needs.need_heal * (features.heal_access + path.max_fires as f32 * 0.16),
        shop: needs.need_shop * shop_route_access_value(features, path),
        event: needs.need_event * (features.event_access + path.max_unknowns as f32 * 0.08),
        potion: needs.need_potion * features.expected_potion_gain,
        curse_debt: -expected_cursed_key_chest_debt(features, path, has_cursed_key)
            * config.cursed_key_treasure_curse_penalty
            * (1.0 + needs.need_remove + needs.avoid_damage * 0.5),
        hp_loss: -needs.avoid_damage * features.expected_hp_loss_p90 / 12.0,
        death_risk: -features.death_risk * (1.0 + needs.avoid_damage) * 5.0,
        flexibility: needs.value_flexibility * flexibility_value(path),
        elite_prep: first_elite_preparation_value(path, needs),
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

fn expected_cursed_key_chest_debt(
    features: &NodeFeaturesV1,
    path: &RoutePathSummaryV1,
    has_cursed_key: bool,
) -> f32 {
    if !has_cursed_key {
        return 0.0;
    }
    let visible_chests = path.max_treasures as f32;
    let current_unknown_treasure_risk = if features.is_question_mark {
        features.expected_curse_debt
    } else {
        0.0
    };
    visible_chests + current_unknown_treasure_risk
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
    if features.expected_curse_debt > 0.0 {
        cautions.push(format!(
            "Cursed Key chest curse debt: expected {:.2}",
            features.expected_curse_debt
        ));
    }
    if path.max_elites > path.min_elites {
        reasons.push("elite fights are optional on visible continuations".to_string());
    } else if path.min_elites > 0 {
        cautions.push("elite pressure is forced on visible continuations".to_string());
    }
    let first_elite = &path.first_elite;
    if first_elite.paths_with_first_elite > 0 {
        if first_elite.optional {
            reasons.push("first elite can be skipped on some continuations".to_string());
        }
        if first_elite.max_hallway_fights_before > 0 {
            reasons.push(format!(
                "first elite prep: {} hallway fight(s) before it",
                format_range(
                    first_elite.min_hallway_fights_before,
                    first_elite.max_hallway_fights_before
                )
            ));
        }
        if first_elite.can_bail_to_rest_before {
            reasons.push("rest bailout exists before first elite".to_string());
        }
        if first_elite.can_bail_to_shop_before {
            reasons.push("shop bailout exists before first elite".to_string());
        }
        if first_elite_is_underprepared(path) {
            cautions
                .push("first elite arrives before another hallway reward or bailout".to_string());
        }
    }
    if path.min_fires > 0 {
        reasons.push("rest site is guaranteed somewhere on the route".to_string());
    } else if path.max_fires > 0 {
        reasons.push("rest site exists on some visible continuations".to_string());
    } else {
        cautions.push("no visible rest site before boss".to_string());
    }
    if path.min_shops > 0 {
        reasons.push(format!(
            "guaranteed shop access: {}",
            format_range(path.min_shops, path.max_shops)
        ));
    } else if path.max_shops > 0 {
        reasons.push(format!("optional shop access: {}", path.max_shops));
    }
    if path.min_damage_rooms_before_recovery > 0 {
        cautions.push(format!(
            "forced damage before recovery: {} room(s)",
            format_range(
                path.min_damage_rooms_before_recovery,
                path.max_damage_rooms_before_recovery
            )
        ));
    } else if path.max_damage_rooms_before_recovery > 0 {
        reasons.push(format!(
            "some continuations can recover before damage; worst pre-recovery damage rooms={}",
            path.max_damage_rooms_before_recovery
        ));
    }
    if path.min_unknowns_before_recovery > 0 {
        cautions.push(format!(
            "forced ? before recovery: {}",
            format_range(
                path.min_unknowns_before_recovery,
                path.max_unknowns_before_recovery
            )
        ));
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

fn shop_route_access_value(features: &NodeFeaturesV1, path: &RoutePathSummaryV1) -> f32 {
    let guaranteed_shop_value = path.min_shops as f32 * 0.62;
    let optional_shop_value = path.max_shops.saturating_sub(path.min_shops) as f32 * 0.08;
    let early_shop_value = path
        .first_shop_floor
        .map(|floor| {
            if floor <= 3 {
                0.12
            } else if floor <= 6 {
                0.06
            } else {
                0.0
            }
        })
        .unwrap_or(0.0);

    features.shop_access + guaranteed_shop_value + optional_shop_value + early_shop_value
}

fn first_elite_preparation_value(path: &RoutePathSummaryV1, needs: &NeedVectorV1) -> f32 {
    let segment = &path.first_elite;
    if segment.paths_with_first_elite == 0 {
        return 0.0;
    }
    let hallway_prep = segment.max_hallway_fights_before.min(3) as f32 / 3.0;
    let bailout_prep = if segment.can_bail_to_rest_before {
        0.30
    } else {
        0.0
    } + if segment.can_bail_to_shop_before {
        0.18
    } else {
        0.0
    };
    let optionality = if segment.optional { 0.18 } else { 0.0 };
    let forced_prep_debt = if first_elite_is_underprepared(path) {
        -0.55
    } else {
        0.0
    };
    let prep_signal = hallway_prep * 0.75 + bailout_prep + optionality + forced_prep_debt;
    needs.need_relics * (0.50 + needs.can_take_elite * 0.50) * prep_signal
}

fn forced_path_penalty(path: &RoutePathSummaryV1, needs: &NeedVectorV1) -> f32 {
    if path.min_elites > 0 && needs.can_take_elite < 0.5 {
        -1.5
    } else {
        0.0
    }
}

fn format_range(min: usize, max: usize) -> String {
    if min == max {
        min.to_string()
    } else {
        format!("{min}-{max}")
    }
}
