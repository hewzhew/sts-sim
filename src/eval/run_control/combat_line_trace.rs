use crate::ai::combat_search_v2::{
    has_external_payoff_opportunity, CombatSearchV2Report, SearchTerminalLabel,
};
use crate::content::monsters::EnemyId;
use crate::content::powers::{store, PowerId};
use crate::sim::combat::{CombatPosition, CombatTerminal};
use crate::state::core::{EngineState, RunResult};

use super::combat_candidate_line::CombatCandidateLine;
use super::session::RunControlSession;
use super::trace_annotation::{
    CombatAutomationMonsterStateV1, CombatAutomationStepStateV1, CombatSearchPerformanceSnapshotV1,
    CombatSearchTerminalLineSummary, RunControlTraceAnnotationV1,
};
use super::transition_report::RunApplyStatus;

#[derive(Clone, Copy)]
pub(super) struct CombatCandidateLinePerformance {
    pub(super) nodes_expanded: u64,
    pub(super) nodes_generated: u64,
    pub(super) total_us: u64,
}

pub(super) fn combat_automation_step_state_v1(
    session: &RunControlSession,
) -> Option<CombatAutomationStepStateV1> {
    let combat = &session.active_combat.as_ref()?.combat_state;
    Some(CombatAutomationStepStateV1 {
        player_hp: combat.entities.player.current_hp,
        player_max_hp: combat.entities.player.max_hp,
        player_block: combat.entities.player.block,
        energy: combat.turn.energy,
        cards_played_this_turn: combat.turn.counters.cards_played_this_turn,
        early_end_turn_pending: combat.turn.counters.early_end_turn_pending,
        monsters: combat
            .entities
            .monsters
            .iter()
            .map(|monster| CombatAutomationMonsterStateV1 {
                id: monster.id,
                label: EnemyId::from_id(monster.monster_type)
                    .map(|enemy| enemy.get_name().to_string())
                    .unwrap_or_else(|| format!("monster#{}", monster.monster_type)),
                hp: monster.current_hp,
                max_hp: monster.max_hp,
                block: monster.block,
                alive: monster.is_alive_for_action(),
                time_warp: store::power_amount(combat, monster.id, PowerId::TimeWarp),
                strength: store::power_amount(combat, monster.id, PowerId::Strength),
            })
            .collect(),
    })
}

pub(super) fn combat_search_performance_trace_annotation(
    source: impl Into<String>,
    session: &RunControlSession,
    start: &CombatPosition,
    report: &CombatSearchV2Report,
) -> RunControlTraceAnnotationV1 {
    RunControlTraceAnnotationV1::CombatSearchPerformance {
        snapshot: combat_search_performance_snapshot(source.into(), session, start, report),
    }
}

pub(super) fn combat_line_performance_trace_annotation(
    source: impl Into<String>,
    session: &RunControlSession,
    start: &CombatPosition,
    report: &CombatSearchV2Report,
    selected_line: &CombatCandidateLine,
    line_performance: Option<CombatCandidateLinePerformance>,
) -> RunControlTraceAnnotationV1 {
    let Some(performance) = line_performance else {
        return combat_search_performance_trace_annotation(source, session, start, report);
    };
    let mut snapshot = combat_search_performance_snapshot(source.into(), session, start, report);
    let line_summary = combat_candidate_line_summary(selected_line);
    snapshot.coverage_status = "CompleteLineSolverApplied".to_string();
    snapshot.complete_trajectory_found = true;
    snapshot.complete_win_found = selected_line.terminal == CombatTerminal::Win;
    snapshot.best_complete = Some(line_summary.clone());
    snapshot.best_win = (selected_line.terminal == CombatTerminal::Win).then_some(line_summary);
    snapshot.best_hp_loss =
        (selected_line.terminal == CombatTerminal::Win).then_some(selected_line.hp_loss);
    snapshot.nodes_expanded = performance.nodes_expanded;
    snapshot.nodes_generated = performance.nodes_generated;
    snapshot.terminal_wins = u64::from(selected_line.terminal == CombatTerminal::Win);
    snapshot.total_us = performance.total_us;
    RunControlTraceAnnotationV1::CombatSearchPerformance { snapshot }
}

fn combat_search_performance_snapshot(
    source: String,
    session: &RunControlSession,
    start: &CombatPosition,
    report: &CombatSearchV2Report,
) -> CombatSearchPerformanceSnapshotV1 {
    let combat = &start.combat;
    CombatSearchPerformanceSnapshotV1 {
        source,
        act: session.run_state.act_num,
        floor: session.run_state.floor_num,
        turn: combat.turn.turn_count,
        combat_kind: combat_kind_label(combat),
        enemies: combat_enemy_names(combat),
        boss: session
            .run_state
            .boss_key
            .map(|boss| format!("{boss:?}"))
            .unwrap_or_else(|| "unknown".to_string()),
        external_payoff_opportunity: has_external_payoff_opportunity(combat),
        coverage_status: format!("{:?}", report.outcome.coverage_status),
        complete_trajectory_found: report.outcome.complete_trajectory_found,
        complete_win_found: report.outcome.complete_win_found,
        best_complete: report
            .best_complete_trajectory
            .as_ref()
            .map(combat_search_line_summary),
        best_win: report
            .best_win_trajectory
            .as_ref()
            .map(combat_search_line_summary),
        best_hp_loss: report
            .best_win_trajectory
            .as_ref()
            .map(|trajectory| trajectory.hp_loss),
        nodes_expanded: report.stats.nodes_expanded,
        nodes_generated: report.stats.nodes_generated,
        terminal_wins: report.stats.terminal_wins,
        total_us: micros_to_u64(report.performance.total_elapsed_us),
        unattributed_us: micros_to_u64(report.performance.unattributed_elapsed_us),
        rollout_calls: report.performance.rollout_estimate_calls,
        root_rollout_calls: report.performance.root_rollout_estimate_calls,
        child_rollout_calls: report.performance.child_rollout_estimate_calls,
        deferred_child_rollout_calls: report.performance.deferred_child_rollout_estimate_calls,
        turn_plan_seed_rollout_calls: report.performance.turn_plan_seed_rollout_estimate_calls,
        deferred_child_rollout_nodes: report.performance.deferred_child_rollout_nodes,
        deferred_child_rollout_requeues: report.performance.deferred_child_rollout_requeues,
        rollout_cache_hits: report.rollout.cache_hits,
        rollout_cache_queries: report.rollout.cache_queries,
        rollout_cache_misses: report.rollout.cache_misses,
        rollout_cache_inserts: report.rollout.cache_inserts,
        rollout_budget_skips: report.rollout.budget_skips,
        rollout_max_evaluation_budget_skips: report.rollout.max_evaluation_budget_skips,
        rollout_deadline_budget_skips: report.rollout.deadline_budget_skips,
        rollout_truncated: report.rollout.truncated_rollouts,
        rollout_terminal_wins: report.rollout.terminal_wins,
        rollout_cache_lookup_us: micros_to_u64(report.rollout.performance.cache_lookup_us),
        rollout_policy_dispatch_us: micros_to_u64(report.rollout.performance.policy_dispatch_us),
        rollout_no_potion_iterations: report.rollout.performance.no_potion_iterations,
        rollout_no_potion_phase_profile_us: micros_to_u64(
            report.rollout.performance.no_potion_phase_profile_us,
        ),
        rollout_no_potion_legal_actions_us: micros_to_u64(
            report.rollout.performance.no_potion_legal_actions_us,
        ),
        rollout_no_potion_choose_action_us: micros_to_u64(
            report.rollout.performance.no_potion_choose_action_us,
        ),
        rollout_no_potion_choose_ordering_us: micros_to_u64(
            report.rollout.performance.no_potion_choose_ordering_us,
        ),
        rollout_no_potion_probe_us: micros_to_u64(report.rollout.performance.no_potion_probe_us),
        rollout_no_potion_probe_score_calls: report.rollout.performance.no_potion_probe_score_calls,
        rollout_no_potion_probe_actions_evaluated: report
            .rollout
            .performance
            .no_potion_probe_actions_evaluated,
        rollout_no_potion_probe_step_reuses: report.rollout.performance.no_potion_probe_step_reuses,
        rollout_no_potion_probe_engine_step_us: micros_to_u64(
            report.rollout.performance.no_potion_probe_engine_step_us,
        ),
        rollout_no_potion_probe_phase_profile_us: micros_to_u64(
            report.rollout.performance.no_potion_probe_phase_profile_us,
        ),
        rollout_no_potion_probe_action_facts_us: micros_to_u64(
            report.rollout.performance.no_potion_probe_action_facts_us,
        ),
        rollout_no_potion_engine_step_us: micros_to_u64(
            report.rollout.performance.no_potion_engine_step_us,
        ),
        rollout_no_potion_child_build_us: micros_to_u64(
            report.rollout.performance.no_potion_child_build_us,
        ),
        terminal_child_rollout_skips: report.performance.terminal_child_rollout_skips,
        terminal_turn_plan_seed_rollout_skips: report
            .performance
            .terminal_turn_plan_seed_rollout_skips,
        turn_local_dominance_rollout_skips: report.performance.turn_local_dominance_rollout_skips,
        rollout_us: micros_to_u64(report.performance.rollout_estimate_elapsed_us),
        expansion_us: micros_to_u64(report.performance.expansion_elapsed_us),
        child_bookkeeping_us: micros_to_u64(report.performance.child_bookkeeping_elapsed_us),
        engine_step_us: micros_to_u64(report.performance.engine_step_elapsed_us),
        pre_expand_us: micros_to_u64(report.performance.pre_expand_elapsed_us),
        frontier_pop_us: micros_to_u64(report.performance.frontier_pop_elapsed_us),
        turn_plan_seed_us: micros_to_u64(report.performance.turn_plan_frontier_seed_elapsed_us),
        shadow_audit_us: micros_to_u64(report.performance.shadow_audit_elapsed_us),
        root_turn_plan_diag_us: micros_to_u64(
            report.performance.root_turn_plan_diagnostics_elapsed_us,
        ),
    }
}

fn combat_kind_label(combat: &crate::runtime::combat::CombatState) -> String {
    if combat.meta.is_boss_fight {
        "boss".to_string()
    } else if combat.meta.is_elite_fight {
        "elite".to_string()
    } else {
        "hallway".to_string()
    }
}

fn combat_enemy_names(combat: &crate::runtime::combat::CombatState) -> Vec<String> {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.current_hp > 0 && !monster.is_escaped)
        .map(|monster| {
            EnemyId::from_id(monster.monster_type)
                .map(|enemy| enemy.get_name().to_string())
                .unwrap_or_else(|| format!("monster#{}", monster.monster_type))
        })
        .collect()
}

fn combat_search_line_summary(
    trajectory: &crate::ai::combat_search_v2::CombatSearchV2TrajectoryReport,
) -> CombatSearchTerminalLineSummary {
    CombatSearchTerminalLineSummary {
        terminal: trajectory.terminal,
        final_hp: trajectory.final_hp,
        hp_loss: trajectory.hp_loss,
        turns: trajectory.turns,
        cards_played: trajectory.cards_played,
        potions_used: trajectory.potions_used,
        potions_discarded: trajectory.potions_discarded,
        action_count: trajectory.actions.len(),
    }
}

fn combat_candidate_line_summary(line: &CombatCandidateLine) -> CombatSearchTerminalLineSummary {
    CombatSearchTerminalLineSummary {
        terminal: match line.terminal {
            CombatTerminal::Win => SearchTerminalLabel::Win,
            CombatTerminal::Loss => SearchTerminalLabel::Loss,
            CombatTerminal::Unresolved => SearchTerminalLabel::Unresolved,
        },
        final_hp: line.final_hp,
        hp_loss: line.hp_loss,
        turns: line.turns,
        cards_played: line.cards_played,
        potions_used: line.potions_used,
        potions_discarded: line.potions_discarded,
        action_count: line.actions.len(),
    }
}

pub(super) fn current_run_apply_status(session: &RunControlSession) -> RunApplyStatus {
    match session.engine_state {
        EngineState::GameOver(RunResult::Victory) => RunApplyStatus::Victory,
        EngineState::GameOver(RunResult::Defeat) => RunApplyStatus::Defeat,
        _ => RunApplyStatus::Running,
    }
}

pub(super) fn millis_to_micros_u64(value: u128) -> u64 {
    micros_to_u64(value.saturating_mul(1_000))
}

fn micros_to_u64(value: u128) -> u64 {
    value.min(u128::from(u64::MAX)) as u64
}
