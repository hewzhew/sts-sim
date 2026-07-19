use crate::ai::noncombat_strategy_v1::{
    RunStrategySnapshotV2, StrategyThreatCoverageGapV1, StrategyThreatSourceV1,
};
use crate::ai::route_window_facts::RouteWindowPath;
use crate::content::relics::RelicId;
use crate::state::map::node::RoomType;
use crate::state::RunState;

use super::super::types::{
    PathSurvivalEnvelopeV1, PathThreatExposureV1, RouteDecisionContextV1, RoutePathSummaryV1,
    RoutePathViabilityV1,
};

pub(in crate::ai::route_planner_v1) fn path_survival_envelope_v1(
    run_state: &RunState,
    path: &RouteWindowPath,
    path_summary: &RoutePathSummaryV1,
    family_summary: &RoutePathSummaryV1,
    viability: &RoutePathViabilityV1,
    context: &RouteDecisionContextV1,
    strategy: &RunStrategySnapshotV2,
) -> PathSurvivalEnvelopeV1 {
    let mut hallway_fights = 0usize;
    let mut elite_before_recovery = false;
    let mut boss_before_recovery = false;
    for node in &path.nodes {
        match node.room_type {
            Some(RoomType::RestRoom) => break,
            Some(RoomType::MonsterRoom) => hallway_fights = hallway_fights.saturating_add(1),
            Some(RoomType::MonsterRoomElite) => {
                elite_before_recovery = true;
                break;
            }
            Some(RoomType::MonsterRoomBoss) => {
                boss_before_recovery = true;
                break;
            }
            _ => {}
        }
    }

    let uncovered_threats = strategy
        .threat_coverage
        .gaps
        .iter()
        .filter(|gap| {
            (hallway_fights > 0 && gap.source == StrategyThreatSourceV1::ActHallwayPool)
                || (elite_before_recovery
                    && matches!(
                        gap.source,
                        StrategyThreatSourceV1::ActElitePool
                            | StrategyThreatSourceV1::ActEliteEncounter
                    ))
                || (boss_before_recovery && gap.source == StrategyThreatSourceV1::ActBoss)
        })
        .cloned()
        .collect::<Vec<StrategyThreatCoverageGapV1>>();
    let post_combat_heal = post_combat_heal(run_state);
    let conservative_hp_margin = context.hp as f32
        + post_combat_heal.saturating_mul(hallway_fights as i32) as f32
        - viability.cumulative_hp_loss_p90;
    let rest_escape_available = family_summary.paths_with_recovery_before_damage > 0
        || path_summary.first_elite.can_bail_to_rest_before;
    let shop_escape_available = path_summary.first_elite.can_bail_to_shop_before;
    let has_resource_buffer = context.potions.filled > 0
        || post_combat_heal > 0
        || rest_escape_available
        || shop_escape_available;
    let threat_exposure = if uncovered_threats.is_empty() {
        PathThreatExposureV1::Covered
    } else if has_resource_buffer {
        PathThreatExposureV1::ExposedWithBuffer
    } else {
        PathThreatExposureV1::ExposedWithoutBuffer
    };

    PathSurvivalEnvelopeV1 {
        current_hp: context.hp,
        post_combat_heal,
        potion_buffer_count: context.potions.filled,
        forced_damage_rooms_before_recovery: path_summary.min_damage_rooms_before_recovery,
        hallway_fights_before_recovery: hallway_fights,
        elite_before_recovery,
        boss_before_recovery,
        rest_escape_available,
        shop_escape_available,
        cumulative_hp_loss_p90: viability.cumulative_hp_loss_p90,
        conservative_hp_margin,
        threat_exposure,
        uncovered_threats,
    }
}

fn post_combat_heal(run_state: &RunState) -> i32 {
    if run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::MarkOfTheBloom)
    {
        return 0;
    }
    if run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::BlackBlood)
    {
        12
    } else if run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::BurningBlood)
    {
        6
    } else {
        0
    }
}
