use std::cmp::Ordering;
use std::collections::BTreeSet;

use serde::Serialize;

use crate::ai::noncombat_strategy_v1::{
    build_run_strategy_snapshot_from_run_state_v2, StrategyPackageIdV2, StrategyPlanSupportV1,
};
use crate::ai::route_window_facts::{
    build_route_path_family_from_target, RouteWindowCoverageKind, RouteWindowFactsConfig,
    RouteWindowPath, RouteWindowPathFamily,
};
use crate::ai::strategy::boss_encounter_readiness::{
    boss_encounter_readiness_v1, BossEncounterPreparationBandV1, BossEncounterReadinessV1,
};
use crate::content::relics::RelicId;
use crate::state::core::EngineState;
use crate::state::map::node::RoomType;

use super::{
    exact_run_policy_decision_v1, positive_ranked_run_policy_prior_v1, DecisionCandidateKey,
    ExactRunPolicyActionSuccessorV1, ExactRunPolicyDecisionV1, RunControlSession,
    RunPolicyCandidateV1, RunPolicyPriorV1,
};

const ROUTE_PATH_BUDGET_V1: usize = 2_000;
const RECENT_COMBAT_HIGH_ATTRITION_MAX_HP_DENOMINATOR_V1: i32 = 6;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutePolicyBandV1 {
    PreservePendingRewards,
    ForcedBoss,
    CriticalRecovery,
    RecoveryOption,
    LiquidityConversion,
    EliteGrowth,
    FlexibleGrowth,
    Ordinary,
    ForcedPressure,
    AbandonUnclaimableRewards,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutePolicyArrivalV1 {
    Combat,
    Event,
    Campfire,
    Shop,
    Treasure,
    BossRelic,
    Map,
    Other,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoutePolicyContextV1 {
    pub act: u8,
    pub ascension: u8,
    pub current_hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub critical_recovery: bool,
    pub recovery_pressure: bool,
    pub shop_conversion_support: StrategyPlanSupportV1,
    pub recent_combat_hp_loss: Option<i32>,
    pub boss_encounter_readiness: BossEncounterReadinessV1,
    pub pending_rewards_only_unclaimable_potions: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoutePolicyPathEvidenceV1 {
    pub coverage: Option<RouteWindowCoverageKind>,
    pub observed_path_count: usize,
    pub min_damage_rooms_before_recovery: usize,
    pub max_damage_rooms_before_recovery: usize,
    pub paths_with_recovery: usize,
    pub paths_with_recovery_before_damage: usize,
    pub min_elites: usize,
    pub max_elites: usize,
    pub min_campfires: usize,
    pub max_campfires: usize,
    pub min_shops: usize,
    pub max_shops: usize,
    pub min_treasures: usize,
    pub max_treasures: usize,
    pub min_unknowns: usize,
    pub max_unknowns: usize,
}

impl RoutePolicyPathEvidenceV1 {
    fn complete(&self) -> bool {
        self.coverage == Some(RouteWindowCoverageKind::CompleteWithinHorizon)
    }

    fn every_path_recovers_before_damage(&self) -> bool {
        self.complete()
            && self.observed_path_count > 0
            && self.paths_with_recovery_before_damage == self.observed_path_count
    }

    fn some_path_recovers_before_damage(&self) -> bool {
        self.paths_with_recovery_before_damage > 0
    }

    fn optional_elite(&self) -> bool {
        self.min_elites == 0 && self.max_elites > 0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RoutePolicyActionV1 {
    Select {
        x: i32,
        y: i32,
        room_type: Option<RoomType>,
        uses_wing_boots: bool,
        has_emerald_key: bool,
        actual_wing_boots_spent: i32,
        arrival: RoutePolicyArrivalV1,
        path: RoutePolicyPathEvidenceV1,
    },
    CancelToPendingRewards,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoutePolicyActionEvidenceV1 {
    pub candidate_id: String,
    pub candidate_key: DecisionCandidateKey,
    pub action: RoutePolicyActionV1,
    pub band: RoutePolicyBandV1,
    surface_index: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoutePolicyAuditCandidateV1 {
    pub owner_rank: usize,
    pub candidate_id: String,
    pub label: String,
    pub candidate_key: DecisionCandidateKey,
    pub action: RoutePolicyActionV1,
    pub band: RoutePolicyBandV1,
    pub surface_index: usize,
    pub prior_probability: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExactRoutePolicyAuditV1 {
    pub context: RoutePolicyContextV1,
    pub candidates: Vec<RoutePolicyAuditCandidateV1>,
}

#[derive(Clone, Debug)]
pub struct ExactRoutePolicyDecisionV1 {
    pub exact: ExactRunPolicyDecisionV1,
    pub context: RoutePolicyContextV1,
    pub evidence: Vec<RoutePolicyActionEvidenceV1>,
    pub prior: RunPolicyPriorV1,
}

impl ExactRoutePolicyDecisionV1 {
    pub fn audit(
        &self,
        legal: &[RunPolicyCandidateV1<'_>],
    ) -> Result<ExactRoutePolicyAuditV1, String> {
        let candidates = self
            .evidence
            .iter()
            .enumerate()
            .map(|(owner_rank, evidence)| {
                let legal_candidate = legal
                    .iter()
                    .find(|candidate| candidate.candidate_id == evidence.candidate_id)
                    .ok_or_else(|| {
                        format!(
                            "route policy audit could not find legal candidate '{}'",
                            evidence.candidate_id
                        )
                    })?;
                let prior_probability = self
                    .prior
                    .entries
                    .iter()
                    .find(|entry| entry.candidate_id == evidence.candidate_id)
                    .map(|entry| entry.probability)
                    .ok_or_else(|| {
                        format!(
                            "route policy audit could not find prior for candidate '{}'",
                            evidence.candidate_id
                        )
                    })?;
                Ok(RoutePolicyAuditCandidateV1 {
                    owner_rank,
                    candidate_id: evidence.candidate_id.clone(),
                    label: legal_candidate.label.to_string(),
                    candidate_key: evidence.candidate_key.clone(),
                    action: evidence.action.clone(),
                    band: evidence.band,
                    surface_index: evidence.surface_index,
                    prior_probability,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(ExactRoutePolicyAuditV1 {
            context: self.context.clone(),
            candidates,
        })
    }
}

pub fn exact_route_policy_audit_v1(
    session: &RunControlSession,
    legal: &[RunPolicyCandidateV1<'_>],
) -> Result<ExactRoutePolicyAuditV1, String> {
    exact_route_policy_decision_v1(session, legal)?.audit(legal)
}

pub fn exact_route_policy_prior_v1(
    session: &RunControlSession,
    legal: &[RunPolicyCandidateV1<'_>],
) -> Result<RunPolicyPriorV1, String> {
    Ok(exact_route_policy_decision_v1(session, legal)?.prior)
}

pub fn exact_route_policy_decision_v1(
    session: &RunControlSession,
    legal: &[RunPolicyCandidateV1<'_>],
) -> Result<ExactRoutePolicyDecisionV1, String> {
    if !session.engine_state.is_map_surface() {
        return Err("exact route policy requires a map boundary".to_string());
    }

    let exact = exact_run_policy_decision_v1(session)?;
    validate_same_candidate_surface(&exact, legal)?;
    let context = route_policy_context_v1(session);
    let mut evidence = exact
        .actions
        .iter()
        .enumerate()
        .map(|(surface_index, successor)| {
            route_action_evidence_v1(session, successor, surface_index, &context)
        })
        .collect::<Result<Vec<_>, _>>()?;
    evidence.sort_by(|left, right| compare_route_evidence(left, right, &context));
    let prior = positive_ranked_run_policy_prior_v1(
        legal,
        evidence
            .iter()
            .map(|candidate| candidate.candidate_id.clone()),
    )?;

    Ok(ExactRoutePolicyDecisionV1 {
        exact,
        context,
        evidence,
        prior,
    })
}

fn validate_same_candidate_surface(
    exact: &ExactRunPolicyDecisionV1,
    legal: &[RunPolicyCandidateV1<'_>],
) -> Result<(), String> {
    let exact_ids = exact
        .actions
        .iter()
        .filter(|action| action.candidate_key.as_ref().is_some_and(is_route_key))
        .map(|action| action.candidate_id.as_str())
        .collect::<BTreeSet<_>>();
    let legal_ids = legal
        .iter()
        .map(|candidate| candidate.candidate_id)
        .collect::<BTreeSet<_>>();
    if exact_ids != legal_ids || exact_ids.len() != legal.len() {
        return Err(format!(
            "route policy surface differs from exact typed surface: exact={} policy={}",
            exact_ids.len(),
            legal.len()
        ));
    }
    Ok(())
}

fn is_route_key(key: &DecisionCandidateKey) -> bool {
    matches!(
        key,
        DecisionCandidateKey::RouteSelect { .. } | DecisionCandidateKey::RouteCancel
    )
}

fn route_policy_context_v1(session: &RunControlSession) -> RoutePolicyContextV1 {
    let current_hp = session.run_state.current_hp.max(0);
    let max_hp = session.run_state.max_hp.max(0);
    let strategy = build_run_strategy_snapshot_from_run_state_v2(&session.run_state);
    RoutePolicyContextV1 {
        act: session.run_state.act_num,
        ascension: session.run_state.ascension_level,
        current_hp,
        max_hp,
        gold: session.run_state.gold,
        critical_recovery: max_hp > 0 && current_hp.saturating_mul(3) <= max_hp,
        recovery_pressure: max_hp > 0 && current_hp.saturating_mul(2) <= max_hp,
        shop_conversion_support: strategy.support(StrategyPackageIdV2::GoldPlan),
        recent_combat_hp_loss: session
            .recent_combat_attrition()
            .map(|attrition| attrition.raw_hp_loss),
        boss_encounter_readiness: boss_encounter_readiness_v1(&session.run_state),
        pending_rewards_only_unclaimable_potions:
            map_overlay_pending_rewards_only_unclaimable_potions(session),
    }
}

fn map_overlay_pending_rewards_only_unclaimable_potions(session: &RunControlSession) -> bool {
    let EngineState::MapOverlay { return_state } = &session.engine_state else {
        return false;
    };
    let mut returned = session.clone();
    returned.engine_state = return_state.as_ref().clone();
    super::reward_surface_has_only_unclaimable_potions(&returned)
}

fn route_action_evidence_v1(
    parent: &RunControlSession,
    successor: &ExactRunPolicyActionSuccessorV1,
    surface_index: usize,
    context: &RoutePolicyContextV1,
) -> Result<RoutePolicyActionEvidenceV1, String> {
    let candidate_key = successor.candidate_key.clone().ok_or_else(|| {
        format!(
            "route candidate '{}' has no typed key",
            successor.candidate_id
        )
    })?;
    let action = match candidate_key {
        DecisionCandidateKey::RouteSelect {
            x,
            y,
            room_type,
            uses_wing_boots,
            has_emerald_key,
        } => {
            let child = &successor.exact.session;
            if child.run_state.map.current_x != x || child.run_state.map.current_y != y {
                return Err(format!(
                    "route candidate '{}' selected ({x},{y}) but exact successor arrived at ({},{})",
                    successor.candidate_id,
                    child.run_state.map.current_x,
                    child.run_state.map.current_y
                ));
            }
            let before_charges = wing_boots_charges(parent);
            let after_charges = wing_boots_charges(child);
            let actual_wing_boots_spent = before_charges.saturating_sub(after_charges);
            if uses_wing_boots && actual_wing_boots_spent <= 0 {
                return Err(format!(
                    "route candidate '{}' claims a Wing Boots jump without consuming a charge",
                    successor.candidate_id
                ));
            }
            RoutePolicyActionV1::Select {
                x,
                y,
                room_type,
                uses_wing_boots,
                has_emerald_key,
                actual_wing_boots_spent,
                arrival: route_arrival_v1(&child.engine_state),
                path: route_path_evidence_v1(parent, x, y),
            }
        }
        DecisionCandidateKey::RouteCancel => RoutePolicyActionV1::CancelToPendingRewards,
        ref other => {
            return Err(format!(
                "exact route policy received non-route candidate key {other:?}"
            ))
        }
    };
    let band = route_policy_band_v1(&action, context);
    Ok(RoutePolicyActionEvidenceV1 {
        candidate_id: successor.candidate_id.clone(),
        candidate_key,
        action,
        band,
        surface_index,
    })
}

fn wing_boots_charges(session: &RunControlSession) -> i32 {
    session
        .run_state
        .relics
        .iter()
        .find(|relic| relic.id == RelicId::WingBoots && !relic.used_up)
        .map(|relic| relic.counter.max(0))
        .unwrap_or_default()
}

fn route_arrival_v1(state: &EngineState) -> RoutePolicyArrivalV1 {
    match state {
        EngineState::CombatStart(_)
        | EngineState::CombatProcessing
        | EngineState::CombatPlayerTurn
        | EngineState::PendingChoice(_) => RoutePolicyArrivalV1::Combat,
        EngineState::EventRoom => RoutePolicyArrivalV1::Event,
        EngineState::Campfire => RoutePolicyArrivalV1::Campfire,
        EngineState::Shop(_) => RoutePolicyArrivalV1::Shop,
        EngineState::TreasureRoom(_) => RoutePolicyArrivalV1::Treasure,
        EngineState::BossRelicSelect(_) => RoutePolicyArrivalV1::BossRelic,
        EngineState::MapNavigation | EngineState::MapOverlay { .. } => RoutePolicyArrivalV1::Map,
        _ => RoutePolicyArrivalV1::Other,
    }
}

fn route_path_evidence_v1(
    session: &RunControlSession,
    x: i32,
    y: i32,
) -> RoutePolicyPathEvidenceV1 {
    let horizon_nodes = 16_usize.saturating_sub(y.max(0) as usize).max(1);
    let family = build_route_path_family_from_target(
        &session.run_state,
        x,
        y,
        RouteWindowFactsConfig {
            horizon_nodes,
            path_budget: ROUTE_PATH_BUDGET_V1,
        },
    );
    summarize_path_family(&family)
}

#[derive(Clone, Copy, Debug, Default)]
struct PathStats {
    damage_before_recovery: usize,
    recovered: bool,
    recovery_before_damage: bool,
    elites: usize,
    campfires: usize,
    shops: usize,
    treasures: usize,
    unknowns: usize,
}

fn summarize_path_family(family: &RouteWindowPathFamily) -> RoutePolicyPathEvidenceV1 {
    let stats = family.paths.iter().map(summarize_path).collect::<Vec<_>>();
    if stats.is_empty() {
        return RoutePolicyPathEvidenceV1 {
            coverage: Some(family.coverage.kind),
            ..Default::default()
        };
    }
    let min = |read: fn(&PathStats) -> usize| stats.iter().map(read).min().unwrap_or_default();
    let max = |read: fn(&PathStats) -> usize| stats.iter().map(read).max().unwrap_or_default();
    RoutePolicyPathEvidenceV1 {
        coverage: Some(family.coverage.kind),
        observed_path_count: stats.len(),
        min_damage_rooms_before_recovery: min(|item| item.damage_before_recovery),
        max_damage_rooms_before_recovery: max(|item| item.damage_before_recovery),
        paths_with_recovery: stats.iter().filter(|item| item.recovered).count(),
        paths_with_recovery_before_damage: stats
            .iter()
            .filter(|item| item.recovery_before_damage)
            .count(),
        min_elites: min(|item| item.elites),
        max_elites: max(|item| item.elites),
        min_campfires: min(|item| item.campfires),
        max_campfires: max(|item| item.campfires),
        min_shops: min(|item| item.shops),
        max_shops: max(|item| item.shops),
        min_treasures: min(|item| item.treasures),
        max_treasures: max(|item| item.treasures),
        min_unknowns: min(|item| item.unknowns),
        max_unknowns: max(|item| item.unknowns),
    }
}

fn summarize_path(path: &RouteWindowPath) -> PathStats {
    let mut stats = PathStats::default();
    for node in &path.nodes {
        match node.room_type {
            Some(
                RoomType::MonsterRoom | RoomType::MonsterRoomElite | RoomType::MonsterRoomBoss,
            ) => {
                if !stats.recovered {
                    stats.damage_before_recovery = stats.damage_before_recovery.saturating_add(1);
                }
                if node.room_type == Some(RoomType::MonsterRoomElite) {
                    stats.elites = stats.elites.saturating_add(1);
                }
            }
            Some(RoomType::RestRoom) => {
                stats.campfires = stats.campfires.saturating_add(1);
                if !stats.recovered {
                    stats.recovery_before_damage = stats.damage_before_recovery == 0;
                    stats.recovered = true;
                }
            }
            Some(RoomType::ShopRoom) => {
                stats.shops = stats.shops.saturating_add(1);
            }
            Some(RoomType::TreasureRoom) => {
                stats.treasures = stats.treasures.saturating_add(1);
            }
            Some(RoomType::EventRoom) => {
                stats.unknowns = stats.unknowns.saturating_add(1);
            }
            _ => {}
        }
    }
    stats
}

fn route_policy_band_v1(
    action: &RoutePolicyActionV1,
    context: &RoutePolicyContextV1,
) -> RoutePolicyBandV1 {
    let RoutePolicyActionV1::Select {
        room_type, path, ..
    } = action
    else {
        return if context.pending_rewards_only_unclaimable_potions {
            RoutePolicyBandV1::AbandonUnclaimableRewards
        } else {
            RoutePolicyBandV1::PreservePendingRewards
        };
    };
    if *room_type == Some(RoomType::MonsterRoomBoss) {
        return RoutePolicyBandV1::ForcedBoss;
    }
    if context.critical_recovery && path.every_path_recovers_before_damage() {
        return RoutePolicyBandV1::CriticalRecovery;
    }
    if context.recovery_pressure && path.some_path_recovers_before_damage() {
        return RoutePolicyBandV1::RecoveryOption;
    }
    if *room_type == Some(RoomType::ShopRoom)
        && path.min_elites == 0
        && shop_conversion_is_supported(context)
    {
        return RoutePolicyBandV1::LiquidityConversion;
    }
    if *room_type == Some(RoomType::MonsterRoomElite)
        && path.min_elites == 1
        && a0_act1_elite_growth_is_supported(context)
    {
        return RoutePolicyBandV1::EliteGrowth;
    }
    if path.min_elites == 0
        && (path.optional_elite()
            || path.max_campfires > path.min_campfires
            || path.max_shops > path.min_shops)
    {
        return RoutePolicyBandV1::FlexibleGrowth;
    }
    if context.critical_recovery && path.min_damage_rooms_before_recovery > 0 {
        return RoutePolicyBandV1::ForcedPressure;
    }
    RoutePolicyBandV1::Ordinary
}

fn a0_act1_elite_growth_is_supported(context: &RoutePolicyContextV1) -> bool {
    context.act == 1
        && context.ascension == 0
        && context.max_hp > 0
        && context.current_hp.saturating_mul(4) >= context.max_hp.saturating_mul(3)
        && !recent_combat_attrition_is_high(context)
        && !context
            .boss_encounter_readiness
            .preparation
            .requires_resource_preservation()
}

fn recent_combat_attrition_is_high(context: &RoutePolicyContextV1) -> bool {
    context.max_hp > 0
        && context.recent_combat_hp_loss.is_some_and(|loss| {
            loss.max(0)
                .saturating_mul(RECENT_COMBAT_HIGH_ATTRITION_MAX_HP_DENOMINATOR_V1)
                >= context.max_hp
        })
}

fn shop_conversion_is_supported(context: &RoutePolicyContextV1) -> bool {
    matches!(
        context.shop_conversion_support,
        StrategyPlanSupportV1::Plausible | StrategyPlanSupportV1::Strong
    )
}

fn compare_route_evidence(
    left: &RoutePolicyActionEvidenceV1,
    right: &RoutePolicyActionEvidenceV1,
    context: &RoutePolicyContextV1,
) -> Ordering {
    left.band
        .cmp(&right.band)
        .then_with(|| compare_route_actions(&left.action, &right.action, context))
        .then_with(|| left.surface_index.cmp(&right.surface_index))
}

fn compare_route_actions(
    left: &RoutePolicyActionV1,
    right: &RoutePolicyActionV1,
    context: &RoutePolicyContextV1,
) -> Ordering {
    let (
        RoutePolicyActionV1::Select {
            uses_wing_boots: left_wing,
            path: left_path,
            ..
        },
        RoutePolicyActionV1::Select {
            uses_wing_boots: right_wing,
            path: right_path,
            ..
        },
    ) = (left, right)
    else {
        return Ordering::Equal;
    };

    let survival = left_path
        .min_damage_rooms_before_recovery
        .cmp(&right_path.min_damage_rooms_before_recovery)
        .then_with(|| {
            left_path
                .max_damage_rooms_before_recovery
                .cmp(&right_path.max_damage_rooms_before_recovery)
        })
        .then_with(|| {
            right_path
                .paths_with_recovery_before_damage
                .cmp(&left_path.paths_with_recovery_before_damage)
        });
    if context.recovery_pressure && survival != Ordering::Equal {
        return survival;
    }

    let boss_preparation = if matches!(
        context.boss_encounter_readiness.preparation,
        BossEncounterPreparationBandV1::Exposed | BossEncounterPreparationBandV1::PotionBacked
    ) {
        right_path
            .max_campfires
            .cmp(&left_path.max_campfires)
            .then_with(|| right_path.min_campfires.cmp(&left_path.min_campfires))
            .then_with(|| left_path.min_elites.cmp(&right_path.min_elites))
            .then_with(|| left_path.max_elites.cmp(&right_path.max_elites))
    } else {
        Ordering::Equal
    };
    if boss_preparation != Ordering::Equal {
        return boss_preparation;
    }

    let shop = if shop_conversion_is_supported(context) {
        right_path
            .min_shops
            .cmp(&left_path.min_shops)
            .then_with(|| right_path.max_shops.cmp(&left_path.max_shops))
    } else {
        Ordering::Equal
    };
    shop.then_with(|| left_path.min_elites.cmp(&right_path.min_elites))
        .then_with(|| right_path.max_elites.cmp(&left_path.max_elites))
        .then_with(|| right_path.max_campfires.cmp(&left_path.max_campfires))
        .then_with(|| right_path.min_campfires.cmp(&left_path.min_campfires))
        .then_with(|| right_path.max_treasures.cmp(&left_path.max_treasures))
        .then_with(|| {
            right_path
                .observed_path_count
                .cmp(&left_path.observed_path_count)
        })
        .then_with(|| left_wing.cmp(right_wing))
        .then_with(|| right_path.complete().cmp(&left_path.complete()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::run_control::{build_decision_surface, RunControlConfig};
    use crate::state::map::node::{MapEdge, MapRoomNode};
    use crate::state::map::state::MapState;

    fn policy_candidates<'a>(
        surface: &'a super::super::DecisionSurface,
    ) -> Vec<RunPolicyCandidateV1<'a>> {
        surface
            .view
            .candidates
            .iter()
            .filter_map(|candidate| {
                candidate
                    .action
                    .executable_action_ref()
                    .map(|action| RunPolicyCandidateV1 {
                        candidate_id: &candidate.id,
                        label: &candidate.label,
                        action,
                    })
            })
            .collect()
    }

    fn node(x: i32, y: i32, room_type: RoomType) -> MapRoomNode {
        let mut node = MapRoomNode::new(x, y);
        node.class = Some(room_type);
        node
    }

    fn route_action(
        room_type: RoomType,
        arrival: RoutePolicyArrivalV1,
        path: RoutePolicyPathEvidenceV1,
    ) -> RoutePolicyActionV1 {
        RoutePolicyActionV1::Select {
            x: 0,
            y: 0,
            room_type: Some(room_type),
            uses_wing_boots: false,
            has_emerald_key: false,
            actual_wing_boots_spent: 0,
            arrival,
            path,
        }
    }

    fn two_route_session() -> RunControlSession {
        let mut combat = node(0, 0, RoomType::MonsterRoom);
        combat.edges.insert(MapEdge::new(0, 0, 0, 1));
        let mut rest = node(1, 0, RoomType::RestRoom);
        rest.edges.insert(MapEdge::new(1, 0, 1, 1));
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.event_state = None;
        session.run_state.map = MapState::new(vec![
            vec![combat, rest],
            vec![
                node(0, 1, RoomType::RestRoom),
                node(1, 1, RoomType::MonsterRoom),
            ],
        ]);
        session.run_state.current_hp = 10;
        session.run_state.max_hp = 80;
        session.engine_state = EngineState::MapNavigation;
        session
    }

    fn shop_or_hallway_session(gold: i32) -> RunControlSession {
        let mut shop = node(0, 0, RoomType::ShopRoom);
        shop.edges.insert(MapEdge::new(0, 0, 0, 1));
        let mut hallway = node(1, 0, RoomType::MonsterRoom);
        hallway.edges.insert(MapEdge::new(1, 0, 1, 1));
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.event_state = None;
        session.run_state.map = MapState::new(vec![
            vec![shop, hallway],
            vec![
                node(0, 1, RoomType::MonsterRoom),
                node(1, 1, RoomType::MonsterRoom),
            ],
        ]);
        session.run_state.current_hp = 80;
        session.run_state.max_hp = 80;
        session.run_state.gold = gold;
        session.engine_state = EngineState::MapNavigation;
        session
    }

    #[test]
    fn every_exact_route_action_has_a_typed_key_and_positive_support() {
        let session = two_route_session();
        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let decision =
            exact_route_policy_decision_v1(&session, &legal).expect("exact route decision");

        assert_eq!(decision.evidence.len(), legal.len());
        assert_eq!(decision.prior.entries.len(), legal.len());
        assert!(decision
            .evidence
            .iter()
            .all(|entry| is_route_key(&entry.candidate_key)));
        assert!(decision
            .prior
            .entries
            .iter()
            .all(|entry| entry.probability.is_finite() && entry.probability > 0.0));

        let audit = decision.audit(&legal).expect("typed route audit");
        assert_eq!(audit.candidates.len(), legal.len());
        assert_eq!(
            audit
                .candidates
                .iter()
                .map(|candidate| candidate.owner_rank)
                .collect::<Vec<_>>(),
            (0..legal.len()).collect::<Vec<_>>()
        );
        assert!(audit
            .candidates
            .iter()
            .all(|candidate| candidate.prior_probability.is_finite()
                && candidate.prior_probability > 0.0));
        serde_json::to_value(audit).expect("serialize typed route audit");
    }

    #[test]
    fn critical_hp_prefers_a_proven_recovery_before_damage() {
        let session = two_route_session();
        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let decision =
            exact_route_policy_decision_v1(&session, &legal).expect("exact route decision");

        let first = decision.evidence.first().expect("preferred route");
        assert!(matches!(
            first.action,
            RoutePolicyActionV1::Select {
                room_type: Some(RoomType::RestRoom),
                ..
            }
        ));
        assert_eq!(first.band, RoutePolicyBandV1::CriticalRecovery);
    }

    #[test]
    fn mandatory_elite_is_not_relabelled_as_flexible_by_later_room_variation() {
        let action = RoutePolicyActionV1::Select {
            x: 1,
            y: 1,
            room_type: Some(RoomType::MonsterRoomElite),
            uses_wing_boots: false,
            has_emerald_key: false,
            actual_wing_boots_spent: 0,
            arrival: RoutePolicyArrivalV1::Combat,
            path: RoutePolicyPathEvidenceV1 {
                coverage: Some(RouteWindowCoverageKind::CompleteWithinHorizon),
                observed_path_count: 2,
                min_elites: 1,
                max_elites: 1,
                min_campfires: 1,
                max_campfires: 2,
                ..RoutePolicyPathEvidenceV1::default()
            },
        };

        assert_eq!(
            route_policy_band_v1(
                &action,
                &RoutePolicyContextV1 {
                    act: 2,
                    ascension: 0,
                    current_hp: 70,
                    max_hp: 80,
                    gold: 0,
                    critical_recovery: false,
                    recovery_pressure: false,
                    shop_conversion_support: StrategyPlanSupportV1::Blocked,
                    recent_combat_hp_loss: None,
                    boss_encounter_readiness: BossEncounterReadinessV1::default(),
                    pending_rewards_only_unclaimable_potions: false,
                }
            ),
            RoutePolicyBandV1::Ordinary
        );
    }

    #[test]
    fn healthy_a0_act1_forced_double_elite_does_not_outrank_optional_growth() {
        let context = RoutePolicyContextV1 {
            act: 1,
            ascension: 0,
            current_hp: 84,
            max_hp: 85,
            gold: 37,
            critical_recovery: false,
            recovery_pressure: false,
            shop_conversion_support: StrategyPlanSupportV1::Blocked,
            recent_combat_hp_loss: None,
            boss_encounter_readiness: BossEncounterReadinessV1::default(),
            pending_rewards_only_unclaimable_potions: false,
        };
        let elite = route_action(
            RoomType::MonsterRoomElite,
            RoutePolicyArrivalV1::Combat,
            RoutePolicyPathEvidenceV1 {
                min_elites: 2,
                max_elites: 2,
                min_campfires: 1,
                max_campfires: 1,
                ..RoutePolicyPathEvidenceV1::default()
            },
        );
        let event = route_action(
            RoomType::EventRoom,
            RoutePolicyArrivalV1::Event,
            RoutePolicyPathEvidenceV1 {
                min_elites: 0,
                max_elites: 1,
                min_campfires: 1,
                max_campfires: 2,
                ..RoutePolicyPathEvidenceV1::default()
            },
        );

        let elite_band = route_policy_band_v1(&elite, &context);
        let event_band = route_policy_band_v1(&event, &context);
        assert_eq!(elite_band, RoutePolicyBandV1::Ordinary);
        assert_eq!(event_band, RoutePolicyBandV1::FlexibleGrowth);
        assert!(event_band < elite_band);
    }

    #[test]
    fn healthy_a0_act1_single_forced_elite_keeps_growth_priority() {
        let action = route_action(
            RoomType::MonsterRoomElite,
            RoutePolicyArrivalV1::Combat,
            RoutePolicyPathEvidenceV1 {
                min_elites: 1,
                max_elites: 2,
                min_campfires: 1,
                max_campfires: 2,
                ..RoutePolicyPathEvidenceV1::default()
            },
        );
        let context = RoutePolicyContextV1 {
            act: 1,
            ascension: 0,
            current_hp: 84,
            max_hp: 85,
            gold: 37,
            critical_recovery: false,
            recovery_pressure: false,
            shop_conversion_support: StrategyPlanSupportV1::Blocked,
            recent_combat_hp_loss: None,
            boss_encounter_readiness: BossEncounterReadinessV1::default(),
            pending_rewards_only_unclaimable_potions: false,
        };

        assert_eq!(
            route_policy_band_v1(&action, &context),
            RoutePolicyBandV1::EliteGrowth
        );
    }

    #[test]
    fn a0_act1_direct_elite_growth_requires_three_quarters_hp() {
        let action = route_action(
            RoomType::MonsterRoomElite,
            RoutePolicyArrivalV1::Combat,
            RoutePolicyPathEvidenceV1 {
                min_elites: 1,
                max_elites: 1,
                ..RoutePolicyPathEvidenceV1::default()
            },
        );
        let context = RoutePolicyContextV1 {
            act: 1,
            ascension: 0,
            current_hp: 63,
            max_hp: 85,
            gold: 0,
            critical_recovery: false,
            recovery_pressure: false,
            shop_conversion_support: StrategyPlanSupportV1::Blocked,
            recent_combat_hp_loss: None,
            boss_encounter_readiness: BossEncounterReadinessV1::default(),
            pending_rewards_only_unclaimable_potions: false,
        };

        assert_eq!(
            route_policy_band_v1(&action, &context),
            RoutePolicyBandV1::Ordinary
        );
    }

    #[test]
    fn potion_backed_slime_boss_plan_does_not_fund_direct_elite_growth() {
        let action = route_action(
            RoomType::MonsterRoomElite,
            RoutePolicyArrivalV1::Combat,
            RoutePolicyPathEvidenceV1 {
                min_elites: 1,
                max_elites: 1,
                min_campfires: 1,
                max_campfires: 1,
                ..RoutePolicyPathEvidenceV1::default()
            },
        );
        let context = RoutePolicyContextV1 {
            act: 1,
            ascension: 0,
            current_hp: 65,
            max_hp: 85,
            gold: 41,
            critical_recovery: false,
            recovery_pressure: false,
            shop_conversion_support: StrategyPlanSupportV1::Blocked,
            recent_combat_hp_loss: Some(7),
            boss_encounter_readiness: BossEncounterReadinessV1 {
                boss: Some(crate::content::monsters::factory::EncounterId::SlimeBoss),
                preparation: BossEncounterPreparationBandV1::PotionBacked,
                ..BossEncounterReadinessV1::default()
            },
            pending_rewards_only_unclaimable_potions: false,
        };

        assert_eq!(
            route_policy_band_v1(&action, &context),
            RoutePolicyBandV1::Ordinary
        );
    }

    #[test]
    fn established_slime_boss_plan_keeps_the_bounded_elite_growth_prior() {
        let action = route_action(
            RoomType::MonsterRoomElite,
            RoutePolicyArrivalV1::Combat,
            RoutePolicyPathEvidenceV1 {
                min_elites: 1,
                max_elites: 1,
                ..RoutePolicyPathEvidenceV1::default()
            },
        );
        let context = RoutePolicyContextV1 {
            act: 1,
            ascension: 0,
            current_hp: 65,
            max_hp: 85,
            gold: 41,
            critical_recovery: false,
            recovery_pressure: false,
            shop_conversion_support: StrategyPlanSupportV1::Blocked,
            recent_combat_hp_loss: Some(7),
            boss_encounter_readiness: BossEncounterReadinessV1 {
                boss: Some(crate::content::monsters::factory::EncounterId::SlimeBoss),
                preparation: BossEncounterPreparationBandV1::Established,
                ..BossEncounterReadinessV1::default()
            },
            pending_rewards_only_unclaimable_potions: false,
        };

        assert_eq!(
            route_policy_band_v1(&action, &context),
            RoutePolicyBandV1::EliteGrowth
        );
    }

    #[test]
    fn exposed_boss_plan_prefers_campfire_scope_over_optional_elite_count() {
        let monster = route_action(
            RoomType::MonsterRoom,
            RoutePolicyArrivalV1::Combat,
            RoutePolicyPathEvidenceV1 {
                min_elites: 0,
                max_elites: 2,
                min_campfires: 1,
                max_campfires: 2,
                ..RoutePolicyPathEvidenceV1::default()
            },
        );
        let campfire = route_action(
            RoomType::RestRoom,
            RoutePolicyArrivalV1::Campfire,
            RoutePolicyPathEvidenceV1 {
                min_elites: 0,
                max_elites: 1,
                min_campfires: 3,
                max_campfires: 3,
                ..RoutePolicyPathEvidenceV1::default()
            },
        );
        let context = RoutePolicyContextV1 {
            act: 1,
            ascension: 0,
            current_hp: 73,
            max_hp: 85,
            gold: 29,
            critical_recovery: false,
            recovery_pressure: false,
            shop_conversion_support: StrategyPlanSupportV1::Blocked,
            recent_combat_hp_loss: Some(10),
            boss_encounter_readiness: BossEncounterReadinessV1 {
                boss: Some(crate::content::monsters::factory::EncounterId::SlimeBoss),
                preparation: BossEncounterPreparationBandV1::Exposed,
                ..BossEncounterReadinessV1::default()
            },
            pending_rewards_only_unclaimable_potions: false,
        };

        assert_eq!(
            compare_route_actions(&campfire, &monster, &context),
            Ordering::Less
        );
    }

    #[test]
    fn guaranteed_campfire_growth_precedes_raw_path_count() {
        let monster = route_action(
            RoomType::MonsterRoom,
            RoutePolicyArrivalV1::Combat,
            RoutePolicyPathEvidenceV1 {
                observed_path_count: 21,
                min_damage_rooms_before_recovery: 2,
                max_damage_rooms_before_recovery: 8,
                min_elites: 0,
                max_elites: 2,
                min_campfires: 1,
                max_campfires: 3,
                min_treasures: 1,
                max_treasures: 1,
                ..RoutePolicyPathEvidenceV1::default()
            },
        );
        let event = route_action(
            RoomType::EventRoom,
            RoutePolicyArrivalV1::Event,
            RoutePolicyPathEvidenceV1 {
                observed_path_count: 13,
                min_damage_rooms_before_recovery: 1,
                max_damage_rooms_before_recovery: 1,
                min_elites: 0,
                max_elites: 2,
                min_campfires: 2,
                max_campfires: 3,
                min_treasures: 1,
                max_treasures: 1,
                ..RoutePolicyPathEvidenceV1::default()
            },
        );
        let context = RoutePolicyContextV1 {
            act: 2,
            ascension: 0,
            current_hp: 66,
            max_hp: 85,
            gold: 195,
            critical_recovery: false,
            recovery_pressure: false,
            shop_conversion_support: StrategyPlanSupportV1::Strong,
            recent_combat_hp_loss: Some(5),
            boss_encounter_readiness: BossEncounterReadinessV1::default(),
            pending_rewards_only_unclaimable_potions: false,
        };

        assert_eq!(
            route_policy_band_v1(&monster, &context),
            RoutePolicyBandV1::FlexibleGrowth
        );
        assert_eq!(
            route_policy_band_v1(&event, &context),
            RoutePolicyBandV1::FlexibleGrowth
        );
        assert_eq!(
            compare_route_actions(&event, &monster, &context),
            Ordering::Less,
            "a guaranteed campfire is strategic growth, while raw path count is only representation cardinality"
        );
    }

    #[test]
    fn high_recent_raw_attrition_blocks_the_elite_growth_label() {
        let action = route_action(
            RoomType::MonsterRoomElite,
            RoutePolicyArrivalV1::Combat,
            RoutePolicyPathEvidenceV1 {
                min_elites: 1,
                max_elites: 1,
                ..RoutePolicyPathEvidenceV1::default()
            },
        );
        let context = RoutePolicyContextV1 {
            act: 1,
            ascension: 0,
            current_hp: 84,
            max_hp: 85,
            gold: 0,
            critical_recovery: false,
            recovery_pressure: false,
            shop_conversion_support: StrategyPlanSupportV1::Blocked,
            recent_combat_hp_loss: Some(15),
            boss_encounter_readiness: BossEncounterReadinessV1::default(),
            pending_rewards_only_unclaimable_potions: false,
        };

        assert_eq!(
            route_policy_band_v1(&action, &context),
            RoutePolicyBandV1::Ordinary
        );
    }

    #[test]
    fn full_potion_belt_does_not_loop_back_to_an_unclaimable_reward() {
        use crate::content::potions::{Potion, PotionId};
        use crate::state::rewards::{RewardItem, RewardState};

        let mut session = two_route_session();
        session.run_state.potions = vec![
            Some(Potion::new(PotionId::WeakenPotion, 1)),
            Some(Potion::new(PotionId::FearPotion, 2)),
            Some(Potion::new(PotionId::DexterityPotion, 3)),
        ];
        let mut reward = RewardState::new();
        reward.items.push(RewardItem::Potion {
            potion_id: PotionId::FirePotion,
        });
        session.engine_state = EngineState::map_overlay(EngineState::RewardScreen(reward));
        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);

        let decision =
            exact_route_policy_decision_v1(&session, &legal).expect("exact route decision");

        assert!(matches!(
            decision.evidence.first().map(|evidence| &evidence.action),
            Some(RoutePolicyActionV1::Select { .. })
        ));
        assert_eq!(
            decision.evidence.last().map(|evidence| evidence.band),
            Some(RoutePolicyBandV1::AbandonUnclaimableRewards)
        );
    }

    #[test]
    fn funded_shop_is_a_real_liquidity_conversion_not_ordinary_path_noise() {
        let session = shop_or_hallway_session(237);
        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let decision =
            exact_route_policy_decision_v1(&session, &legal).expect("exact route decision");

        assert_eq!(
            decision.context.shop_conversion_support,
            StrategyPlanSupportV1::Strong
        );
        assert!(matches!(
            decision.evidence.first(),
            Some(RoutePolicyActionEvidenceV1 {
                action: RoutePolicyActionV1::Select {
                    room_type: Some(RoomType::ShopRoom),
                    ..
                },
                band: RoutePolicyBandV1::LiquidityConversion,
                ..
            })
        ));
    }

    #[test]
    fn funded_shop_with_a_forced_elite_does_not_outrank_a_flexible_safe_route() {
        let context = RoutePolicyContextV1 {
            act: 2,
            ascension: 0,
            current_hp: 72,
            max_hp: 90,
            gold: 129,
            critical_recovery: false,
            recovery_pressure: false,
            shop_conversion_support: StrategyPlanSupportV1::Plausible,
            recent_combat_hp_loss: None,
            boss_encounter_readiness: BossEncounterReadinessV1::default(),
            pending_rewards_only_unclaimable_potions: false,
        };
        let shop = route_action(
            RoomType::ShopRoom,
            RoutePolicyArrivalV1::Shop,
            RoutePolicyPathEvidenceV1 {
                coverage: Some(RouteWindowCoverageKind::CompleteWithinHorizon),
                observed_path_count: 1,
                min_elites: 1,
                max_elites: 1,
                min_shops: 1,
                max_shops: 1,
                ..RoutePolicyPathEvidenceV1::default()
            },
        );
        let flexible = route_action(
            RoomType::EventRoom,
            RoutePolicyArrivalV1::Event,
            RoutePolicyPathEvidenceV1 {
                coverage: Some(RouteWindowCoverageKind::CompleteWithinHorizon),
                observed_path_count: 2,
                min_elites: 0,
                max_elites: 1,
                min_campfires: 1,
                max_campfires: 2,
                ..RoutePolicyPathEvidenceV1::default()
            },
        );

        let shop_band = route_policy_band_v1(&shop, &context);
        let flexible_band = route_policy_band_v1(&flexible, &context);

        assert_eq!(shop_band, RoutePolicyBandV1::Ordinary);
        assert_eq!(flexible_band, RoutePolicyBandV1::FlexibleGrowth);
        assert!(flexible_band < shop_band);
    }

    #[test]
    fn shop_is_not_reported_as_guaranteed_recovery() {
        let stats = summarize_path(&RouteWindowPath {
            nodes: vec![
                crate::ai::route_window_facts::RouteWindowNode {
                    x: 0,
                    y: 0,
                    room_type: Some(RoomType::ShopRoom),
                },
                crate::ai::route_window_facts::RouteWindowNode {
                    x: 0,
                    y: 1,
                    room_type: Some(RoomType::MonsterRoomElite),
                },
            ],
        });

        assert_eq!(stats.shops, 1);
        assert_eq!(stats.elites, 1);
        assert_eq!(stats.damage_before_recovery, 1);
        assert!(!stats.recovered);
        assert!(!stats.recovery_before_damage);
    }

    #[test]
    fn unfunded_shop_does_not_receive_liquidity_conversion_priority() {
        let session = shop_or_hallway_session(0);
        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let decision =
            exact_route_policy_decision_v1(&session, &legal).expect("exact route decision");
        let shop = decision
            .evidence
            .iter()
            .find(|candidate| {
                matches!(
                    candidate.action,
                    RoutePolicyActionV1::Select {
                        room_type: Some(RoomType::ShopRoom),
                        ..
                    }
                )
            })
            .expect("shop route");

        assert_eq!(
            decision.context.shop_conversion_support,
            StrategyPlanSupportV1::Blocked
        );
        assert_ne!(shop.band, RoutePolicyBandV1::LiquidityConversion);
    }
}
