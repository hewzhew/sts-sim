use crate::state::map::node::RoomType;

use super::types::{
    NeedVectorV1, NodeFeaturesV1, RoutePathSummaryV1, RoutePathViabilityV1, RouteSafetyFlagV1,
};

pub(super) fn safety_flag(
    features: &NodeFeaturesV1,
    path: &RoutePathSummaryV1,
    needs: &NeedVectorV1,
    viability: &RoutePathViabilityV1,
    max_hp: i32,
) -> RouteSafetyFlagV1 {
    if !viability.survives_projected_segment {
        return RouteSafetyFlagV1::RejectUnlessNoAlternative;
    }
    let forced_elite = path.min_elites > 0 || features.is_elite;
    let no_pre_elite_bailout = !path.first_elite.can_bail_to_rest_before
        && !features.is_rest
        && viability.elite_included_before_recovery;
    if first_elite_is_underprepared(path) && needs.can_take_elite < 0.45 {
        return RouteSafetyFlagV1::RejectUnlessNoAlternative;
    }
    if forced_elite && no_pre_elite_bailout && needs.can_take_elite < 0.45 {
        return RouteSafetyFlagV1::RejectUnlessNoAlternative;
    }
    if very_low_hp_forced_damage_before_recovery(path, needs) {
        return RouteSafetyFlagV1::RejectUnlessNoAlternative;
    }
    if projected_survival_reserve_is_thin(viability, max_hp) {
        return RouteSafetyFlagV1::RiskyButAllowed;
    }
    if first_elite_is_underprepared(path) && needs.can_take_elite < 0.65 {
        return RouteSafetyFlagV1::RiskyButAllowed;
    }
    if low_hp_damage_before_recovery(path, needs) {
        return RouteSafetyFlagV1::RiskyButAllowed;
    }
    if features.death_risk > 0.35 || needs.avoid_damage > 0.65 && expected_damage_room(features) {
        return RouteSafetyFlagV1::RiskyButAllowed;
    }
    RouteSafetyFlagV1::Ok
}

fn projected_survival_reserve_is_thin(viability: &RoutePathViabilityV1, max_hp: i32) -> bool {
    max_hp > 0 && viability.projected_hp_after_segment <= max_hp as f32 * 0.25
}

pub(super) fn first_elite_is_underprepared(path: &RoutePathSummaryV1) -> bool {
    let segment = &path.first_elite;
    segment.forced
        && segment.max_hallway_fights_before < 2
        && !segment.can_bail_to_rest_before
        && !segment.can_bail_to_shop_before
}

fn very_low_hp_forced_damage_before_recovery(
    path: &RoutePathSummaryV1,
    needs: &NeedVectorV1,
) -> bool {
    needs.need_heal >= 0.95 && path.min_damage_rooms_before_recovery > 0
}

fn low_hp_damage_before_recovery(path: &RoutePathSummaryV1, needs: &NeedVectorV1) -> bool {
    (needs.need_heal >= 0.75 && path.min_damage_rooms_before_recovery > 0)
        || needs.need_heal >= 0.95
            && path.min_unknowns_before_recovery > 0
            && path.paths_with_recovery_before_damage == 0
}

fn expected_damage_room(features: &NodeFeaturesV1) -> bool {
    matches!(
        features.node_type,
        Some(RoomType::MonsterRoom | RoomType::MonsterRoomElite | RoomType::MonsterRoomBoss)
    ) || features.expected_hp_loss_p90 > 0.0
}
