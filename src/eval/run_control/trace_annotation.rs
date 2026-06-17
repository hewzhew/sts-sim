use serde::{Deserialize, Serialize};

use crate::ai::card_reward_policy_v1::PublicRewardDecisionPacketV1;
use crate::ai::noncombat_decision_v1::{
    render_noncombat_decision_record_validation_errors, validate_noncombat_decision_record_v1,
    NonCombatDecisionRecordV1,
};
use crate::state::core::ClientInput;

use super::transition_report::CardSnapshot;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatAutomationActionV1 {
    pub step_index: usize,
    pub action_key: String,
    pub input: ClientInput,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub drawn_cards: Vec<CardSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub combat_after: Option<CombatAutomationStepStateV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatAutomationStepStateV1 {
    pub player_hp: i32,
    pub player_max_hp: i32,
    pub player_block: i32,
    pub energy: u8,
    pub cards_played_this_turn: u8,
    pub early_end_turn_pending: bool,
    pub monsters: Vec<CombatAutomationMonsterStateV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatAutomationMonsterStateV1 {
    pub id: usize,
    pub label: String,
    pub hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub alive: bool,
    pub time_warp: i32,
    pub strength: i32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatSearchPerformanceSnapshotV1 {
    pub source: String,
    pub act: u8,
    pub floor: i32,
    pub turn: u32,
    pub combat_kind: String,
    pub enemies: Vec<String>,
    pub boss: String,
    pub external_payoff_opportunity: bool,
    pub coverage_status: String,
    pub complete_trajectory_found: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_hp_loss: Option<i32>,
    pub nodes_expanded: u64,
    pub nodes_generated: u64,
    pub terminal_wins: u64,
    pub total_us: u64,
    pub unattributed_us: u64,
    pub rollout_calls: u64,
    pub root_rollout_calls: u64,
    pub child_rollout_calls: u64,
    pub deferred_child_rollout_calls: u64,
    pub turn_plan_seed_rollout_calls: u64,
    pub deferred_child_rollout_nodes: u64,
    pub deferred_child_rollout_requeues: u64,
    pub rollout_cache_hits: u64,
    pub rollout_cache_queries: u64,
    pub rollout_cache_misses: u64,
    pub rollout_cache_inserts: u64,
    pub rollout_budget_skips: u64,
    pub rollout_max_evaluation_budget_skips: u64,
    pub rollout_deadline_budget_skips: u64,
    pub rollout_truncated: u64,
    pub rollout_terminal_wins: u64,
    pub rollout_cache_lookup_us: u64,
    pub rollout_policy_dispatch_us: u64,
    pub rollout_no_potion_iterations: u64,
    pub rollout_no_potion_phase_profile_us: u64,
    pub rollout_no_potion_legal_actions_us: u64,
    pub rollout_no_potion_choose_action_us: u64,
    pub rollout_no_potion_choose_ordering_us: u64,
    pub rollout_no_potion_probe_us: u64,
    pub rollout_no_potion_probe_score_calls: u64,
    pub rollout_no_potion_probe_actions_evaluated: u64,
    pub rollout_no_potion_probe_step_reuses: u64,
    pub rollout_no_potion_probe_engine_step_us: u64,
    pub rollout_no_potion_probe_phase_profile_us: u64,
    pub rollout_no_potion_probe_action_facts_us: u64,
    pub rollout_no_potion_engine_step_us: u64,
    pub rollout_no_potion_child_build_us: u64,
    pub terminal_child_rollout_skips: u64,
    pub terminal_turn_plan_seed_rollout_skips: u64,
    pub turn_local_dominance_rollout_skips: u64,
    pub rollout_us: u64,
    pub expansion_us: u64,
    pub child_bookkeeping_us: u64,
    pub engine_step_us: u64,
    pub pre_expand_us: u64,
    pub frontier_pop_us: u64,
    pub turn_plan_seed_us: u64,
    pub shadow_audit_us: u64,
    pub root_turn_plan_diag_us: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoutePlannerCandidateSummaryV1 {
    pub rank: usize,
    pub target_x: i32,
    pub target_y: i32,
    pub room_type: String,
    pub move_kind: String,
    pub safety: String,
    pub score: f32,
    pub command: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoutePlannerFirstEliteEvidenceV1 {
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoutePlannerSelectionEvidenceV1 {
    pub elite_prep_bp: i32,
    pub first_elite: RoutePlannerFirstEliteEvidenceV1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RunControlTraceAnnotationV1 {
    RoutePlannerSelection {
        summary: String,
        selected_index: Option<usize>,
        candidate_count: usize,
        target_x: i32,
        target_y: i32,
        room_type: String,
        move_kind: String,
        safety: String,
        score: f32,
        command: String,
        top_candidates: Vec<RoutePlannerCandidateSummaryV1>,
        label_role: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        route_evidence: Option<RoutePlannerSelectionEvidenceV1>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        noncombat_record: Option<NonCombatDecisionRecordV1>,
    },
    NonCombatPolicyDecision {
        record: NonCombatDecisionRecordV1,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        card_reward_packet: Option<PublicRewardDecisionPacketV1>,
    },
    NonCombatHumanBoundary {
        record: NonCombatDecisionRecordV1,
    },
    AutoCombatCapture {
        case_id: String,
        capture_path: String,
        benchmark_manifest_path: String,
        label_role: String,
    },
    CombatAutomationTrajectory {
        source: String,
        action_count: usize,
        actions: Vec<CombatAutomationActionV1>,
        label_role: String,
    },
    CombatSearchPerformance {
        snapshot: CombatSearchPerformanceSnapshotV1,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CombatAutomationTrajectoryRefV1<'a> {
    pub source: &'a str,
    pub action_count: usize,
    pub actions: &'a [CombatAutomationActionV1],
    pub label_role: &'a str,
}

impl RunControlTraceAnnotationV1 {
    pub fn as_combat_automation_trajectory_v1(
        &self,
    ) -> Option<CombatAutomationTrajectoryRefV1<'_>> {
        let RunControlTraceAnnotationV1::CombatAutomationTrajectory {
            source,
            action_count,
            actions,
            label_role,
        } = self
        else {
            return None;
        };
        Some(CombatAutomationTrajectoryRefV1 {
            source,
            action_count: *action_count,
            actions,
            label_role,
        })
    }
}

pub(in crate::eval::run_control) fn validate_run_control_trace_annotations_v1(
    annotations: &[RunControlTraceAnnotationV1],
) -> Result<(), String> {
    for (idx, annotation) in annotations.iter().enumerate() {
        validate_run_control_trace_annotation_v1(idx, annotation)?;
    }
    Ok(())
}

fn validate_run_control_trace_annotation_v1(
    idx: usize,
    annotation: &RunControlTraceAnnotationV1,
) -> Result<(), String> {
    match annotation {
        RunControlTraceAnnotationV1::RoutePlannerSelection {
            noncombat_record: Some(record),
            ..
        } => validate_noncombat_record_annotation(idx, "route_planner_selection", record),
        RunControlTraceAnnotationV1::NonCombatPolicyDecision { record, .. } => {
            validate_noncombat_record_annotation(idx, "noncombat_policy_decision", record)
        }
        RunControlTraceAnnotationV1::NonCombatHumanBoundary { record } => {
            validate_noncombat_record_annotation(idx, "noncombat_human_boundary", record)
        }
        RunControlTraceAnnotationV1::RoutePlannerSelection {
            noncombat_record: None,
            ..
        }
        | RunControlTraceAnnotationV1::AutoCombatCapture { .. }
        | RunControlTraceAnnotationV1::CombatAutomationTrajectory { .. }
        | RunControlTraceAnnotationV1::CombatSearchPerformance { .. } => Ok(()),
    }
}

fn validate_noncombat_record_annotation(
    idx: usize,
    kind: &str,
    record: &NonCombatDecisionRecordV1,
) -> Result<(), String> {
    validate_noncombat_decision_record_v1(record).map_err(|errors| {
        format!(
            "annotation[{idx}] {kind} contains invalid NonCombatDecisionRecordV1: {}",
            render_noncombat_decision_record_validation_errors(&errors)
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combat_automation_trajectory_accessor_exposes_recorded_actions() {
        let action = CombatAutomationActionV1 {
            step_index: 3,
            action_key: "combat/end_turn".to_string(),
            input: ClientInput::EndTurn,
            drawn_cards: Vec::new(),
            combat_after: None,
        };
        let annotation = RunControlTraceAnnotationV1::CombatAutomationTrajectory {
            source: "search_combat".to_string(),
            action_count: 1,
            actions: vec![action],
            label_role: "simulator_generated_not_teacher_label".to_string(),
        };

        let trajectory = annotation
            .as_combat_automation_trajectory_v1()
            .expect("combat automation annotation should expose a trajectory view");

        assert_eq!(trajectory.source, "search_combat");
        assert_eq!(trajectory.action_count, 1);
        assert_eq!(trajectory.actions[0].step_index, 3);
        assert_eq!(
            trajectory.label_role,
            "simulator_generated_not_teacher_label"
        );
    }
}
