use crate::eval::run_control::CombatSearchPerformanceSnapshotV1;
use serde::{Deserialize, Serialize};

use super::BranchCampaignReportV1;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCombatPerformanceSummaryV1 {
    pub samples: usize,
    pub total_us: u64,
    pub unattributed_us: u64,
    pub rollout_us: u64,
    pub expansion_us: u64,
    pub child_bookkeeping_us: u64,
    pub engine_step_us: u64,
    pub pre_expand_us: u64,
    pub frontier_pop_us: u64,
    pub turn_plan_seed_us: u64,
    pub shadow_audit_us: u64,
    pub root_turn_plan_diag_us: u64,
    pub nodes_expanded: u64,
    pub nodes_generated: u64,
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
    pub external_payoff_samples: usize,
    pub boss_samples: usize,
    pub slowest: Vec<BranchCampaignCombatPerformanceExampleV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCombatPerformanceExampleV1 {
    pub total_us: u64,
    pub act: u8,
    pub floor: i32,
    pub turn: u32,
    pub combat_kind: String,
    pub enemies: String,
    pub coverage_status: String,
    pub dominant_bucket: String,
}

pub(super) fn format_seconds_from_us_1dp_v1(us: u64) -> String {
    format!("{:.1}s", us as f64 / 1_000_000.0)
}

pub(super) fn aggregate_campaign_combat_performance_v1(
    report: &BranchCampaignReportV1,
) -> BranchCampaignCombatPerformanceSummaryV1 {
    let mut summary = BranchCampaignCombatPerformanceSummaryV1::default();
    for round in &report.rounds {
        merge_combat_performance_summary_v1(&mut summary, &round.combat_performance);
    }
    summary
}

pub(super) fn render_campaign_combat_performance_v1(
    summary: &BranchCampaignCombatPerformanceSummaryV1,
) -> String {
    format!(
        "Combat perf: samples={} total={} dominant={} rollout={}% expansion={}% child={}% engine={}% rollout_calls={} root_calls={} child_calls={} deferred_child_calls={} seed_calls={} deferred_nodes={} deferred_requeues={} cache=hits/queries/misses/inserts:{}/{}/{}/{} budget_skips={}(max={} deadline={}) terminal_skips={} seed_terminal_skips={} dominance_skips={} rollout_inner=iters:{} policy_total:{}% phase:{}% legal:{}% choose:{}% order:{}% probe:{}% probe_calls:{} probe_eval:{} probe_reuse:{} probe_engine:{}% probe_phase:{}% probe_facts:{}% engine:{}% build:{}% external_payoff={} boss={}",
        summary.samples,
        format_seconds_from_us_1dp_v1(summary.total_us),
        dominant_combat_performance_bucket_for_summary_v1(summary),
        performance_percent_v1(summary.rollout_us, summary.total_us),
        performance_percent_v1(summary.expansion_us, summary.total_us),
        performance_percent_v1(summary.child_bookkeeping_us, summary.total_us),
        performance_percent_v1(summary.engine_step_us, summary.total_us),
        summary.rollout_calls,
        summary.root_rollout_calls,
        summary.child_rollout_calls,
        summary.deferred_child_rollout_calls,
        summary.turn_plan_seed_rollout_calls,
        summary.deferred_child_rollout_nodes,
        summary.deferred_child_rollout_requeues,
        summary.rollout_cache_hits,
        summary.rollout_cache_queries,
        summary.rollout_cache_misses,
        summary.rollout_cache_inserts,
        summary.rollout_budget_skips,
        summary.rollout_max_evaluation_budget_skips,
        summary.rollout_deadline_budget_skips,
        summary.terminal_child_rollout_skips,
        summary.terminal_turn_plan_seed_rollout_skips,
        summary.turn_local_dominance_rollout_skips,
        summary.rollout_no_potion_iterations,
        performance_percent_v1(summary.rollout_policy_dispatch_us, summary.total_us),
        performance_percent_v1(summary.rollout_no_potion_phase_profile_us, summary.total_us),
        performance_percent_v1(summary.rollout_no_potion_legal_actions_us, summary.total_us),
        performance_percent_v1(summary.rollout_no_potion_choose_action_us, summary.total_us),
        performance_percent_v1(summary.rollout_no_potion_choose_ordering_us, summary.total_us),
        performance_percent_v1(summary.rollout_no_potion_probe_us, summary.total_us),
        summary.rollout_no_potion_probe_score_calls,
        summary.rollout_no_potion_probe_actions_evaluated,
        summary.rollout_no_potion_probe_step_reuses,
        performance_percent_v1(
            summary.rollout_no_potion_probe_engine_step_us,
            summary.total_us
        ),
        performance_percent_v1(
            summary.rollout_no_potion_probe_phase_profile_us,
            summary.total_us
        ),
        performance_percent_v1(
            summary.rollout_no_potion_probe_action_facts_us,
            summary.total_us
        ),
        performance_percent_v1(summary.rollout_no_potion_engine_step_us, summary.total_us),
        performance_percent_v1(summary.rollout_no_potion_child_build_us, summary.total_us),
        summary.external_payoff_samples,
        summary.boss_samples,
    )
}

fn performance_percent_v1(part: u64, total: u64) -> u64 {
    if total == 0 {
        return 0;
    }
    part.saturating_mul(100) / total
}

pub(super) fn add_combat_performance_samples_v1(
    summary: &mut BranchCampaignCombatPerformanceSummaryV1,
    samples: &[CombatSearchPerformanceSnapshotV1],
) {
    for sample in samples {
        add_combat_performance_sample_v1(summary, sample);
    }
}

fn add_combat_performance_sample_v1(
    summary: &mut BranchCampaignCombatPerformanceSummaryV1,
    sample: &CombatSearchPerformanceSnapshotV1,
) {
    summary.samples = summary.samples.saturating_add(1);
    summary.total_us = summary.total_us.saturating_add(sample.total_us);
    summary.unattributed_us = summary
        .unattributed_us
        .saturating_add(sample.unattributed_us);
    summary.rollout_us = summary.rollout_us.saturating_add(sample.rollout_us);
    summary.expansion_us = summary.expansion_us.saturating_add(sample.expansion_us);
    summary.child_bookkeeping_us = summary
        .child_bookkeeping_us
        .saturating_add(sample.child_bookkeeping_us);
    summary.engine_step_us = summary.engine_step_us.saturating_add(sample.engine_step_us);
    summary.pre_expand_us = summary.pre_expand_us.saturating_add(sample.pre_expand_us);
    summary.frontier_pop_us = summary
        .frontier_pop_us
        .saturating_add(sample.frontier_pop_us);
    summary.turn_plan_seed_us = summary
        .turn_plan_seed_us
        .saturating_add(sample.turn_plan_seed_us);
    summary.shadow_audit_us = summary
        .shadow_audit_us
        .saturating_add(sample.shadow_audit_us);
    summary.root_turn_plan_diag_us = summary
        .root_turn_plan_diag_us
        .saturating_add(sample.root_turn_plan_diag_us);
    summary.nodes_expanded = summary.nodes_expanded.saturating_add(sample.nodes_expanded);
    summary.nodes_generated = summary
        .nodes_generated
        .saturating_add(sample.nodes_generated);
    summary.rollout_calls = summary.rollout_calls.saturating_add(sample.rollout_calls);
    summary.root_rollout_calls = summary
        .root_rollout_calls
        .saturating_add(sample.root_rollout_calls);
    summary.child_rollout_calls = summary
        .child_rollout_calls
        .saturating_add(sample.child_rollout_calls);
    summary.deferred_child_rollout_calls = summary
        .deferred_child_rollout_calls
        .saturating_add(sample.deferred_child_rollout_calls);
    summary.turn_plan_seed_rollout_calls = summary
        .turn_plan_seed_rollout_calls
        .saturating_add(sample.turn_plan_seed_rollout_calls);
    summary.deferred_child_rollout_nodes = summary
        .deferred_child_rollout_nodes
        .saturating_add(sample.deferred_child_rollout_nodes);
    summary.deferred_child_rollout_requeues = summary
        .deferred_child_rollout_requeues
        .saturating_add(sample.deferred_child_rollout_requeues);
    summary.rollout_cache_hits = summary
        .rollout_cache_hits
        .saturating_add(sample.rollout_cache_hits);
    summary.rollout_cache_queries = summary
        .rollout_cache_queries
        .saturating_add(sample.rollout_cache_queries);
    summary.rollout_cache_misses = summary
        .rollout_cache_misses
        .saturating_add(sample.rollout_cache_misses);
    summary.rollout_cache_inserts = summary
        .rollout_cache_inserts
        .saturating_add(sample.rollout_cache_inserts);
    summary.rollout_budget_skips = summary
        .rollout_budget_skips
        .saturating_add(sample.rollout_budget_skips);
    summary.rollout_max_evaluation_budget_skips = summary
        .rollout_max_evaluation_budget_skips
        .saturating_add(sample.rollout_max_evaluation_budget_skips);
    summary.rollout_deadline_budget_skips = summary
        .rollout_deadline_budget_skips
        .saturating_add(sample.rollout_deadline_budget_skips);
    summary.rollout_truncated = summary
        .rollout_truncated
        .saturating_add(sample.rollout_truncated);
    summary.rollout_terminal_wins = summary
        .rollout_terminal_wins
        .saturating_add(sample.rollout_terminal_wins);
    summary.rollout_cache_lookup_us = summary
        .rollout_cache_lookup_us
        .saturating_add(sample.rollout_cache_lookup_us);
    summary.rollout_policy_dispatch_us = summary
        .rollout_policy_dispatch_us
        .saturating_add(sample.rollout_policy_dispatch_us);
    summary.rollout_no_potion_iterations = summary
        .rollout_no_potion_iterations
        .saturating_add(sample.rollout_no_potion_iterations);
    summary.rollout_no_potion_phase_profile_us = summary
        .rollout_no_potion_phase_profile_us
        .saturating_add(sample.rollout_no_potion_phase_profile_us);
    summary.rollout_no_potion_legal_actions_us = summary
        .rollout_no_potion_legal_actions_us
        .saturating_add(sample.rollout_no_potion_legal_actions_us);
    summary.rollout_no_potion_choose_action_us = summary
        .rollout_no_potion_choose_action_us
        .saturating_add(sample.rollout_no_potion_choose_action_us);
    summary.rollout_no_potion_choose_ordering_us = summary
        .rollout_no_potion_choose_ordering_us
        .saturating_add(sample.rollout_no_potion_choose_ordering_us);
    summary.rollout_no_potion_probe_us = summary
        .rollout_no_potion_probe_us
        .saturating_add(sample.rollout_no_potion_probe_us);
    summary.rollout_no_potion_probe_score_calls = summary
        .rollout_no_potion_probe_score_calls
        .saturating_add(sample.rollout_no_potion_probe_score_calls);
    summary.rollout_no_potion_probe_actions_evaluated = summary
        .rollout_no_potion_probe_actions_evaluated
        .saturating_add(sample.rollout_no_potion_probe_actions_evaluated);
    summary.rollout_no_potion_probe_step_reuses = summary
        .rollout_no_potion_probe_step_reuses
        .saturating_add(sample.rollout_no_potion_probe_step_reuses);
    summary.rollout_no_potion_probe_engine_step_us = summary
        .rollout_no_potion_probe_engine_step_us
        .saturating_add(sample.rollout_no_potion_probe_engine_step_us);
    summary.rollout_no_potion_probe_phase_profile_us = summary
        .rollout_no_potion_probe_phase_profile_us
        .saturating_add(sample.rollout_no_potion_probe_phase_profile_us);
    summary.rollout_no_potion_probe_action_facts_us = summary
        .rollout_no_potion_probe_action_facts_us
        .saturating_add(sample.rollout_no_potion_probe_action_facts_us);
    summary.rollout_no_potion_engine_step_us = summary
        .rollout_no_potion_engine_step_us
        .saturating_add(sample.rollout_no_potion_engine_step_us);
    summary.rollout_no_potion_child_build_us = summary
        .rollout_no_potion_child_build_us
        .saturating_add(sample.rollout_no_potion_child_build_us);
    summary.terminal_child_rollout_skips = summary
        .terminal_child_rollout_skips
        .saturating_add(sample.terminal_child_rollout_skips);
    summary.terminal_turn_plan_seed_rollout_skips = summary
        .terminal_turn_plan_seed_rollout_skips
        .saturating_add(sample.terminal_turn_plan_seed_rollout_skips);
    summary.turn_local_dominance_rollout_skips = summary
        .turn_local_dominance_rollout_skips
        .saturating_add(sample.turn_local_dominance_rollout_skips);
    if sample.external_payoff_opportunity {
        summary.external_payoff_samples = summary.external_payoff_samples.saturating_add(1);
    }
    if sample.combat_kind == "boss" {
        summary.boss_samples = summary.boss_samples.saturating_add(1);
    }
    push_combat_performance_example_v1(
        &mut summary.slowest,
        BranchCampaignCombatPerformanceExampleV1 {
            total_us: sample.total_us,
            act: sample.act,
            floor: sample.floor,
            turn: sample.turn,
            combat_kind: sample.combat_kind.clone(),
            enemies: sample.enemies.join(" + "),
            coverage_status: sample.coverage_status.clone(),
            dominant_bucket: dominant_combat_performance_bucket_for_sample_v1(sample).to_string(),
        },
    );
}

fn merge_combat_performance_summary_v1(
    target: &mut BranchCampaignCombatPerformanceSummaryV1,
    incoming: &BranchCampaignCombatPerformanceSummaryV1,
) {
    target.samples = target.samples.saturating_add(incoming.samples);
    target.total_us = target.total_us.saturating_add(incoming.total_us);
    target.unattributed_us = target
        .unattributed_us
        .saturating_add(incoming.unattributed_us);
    target.rollout_us = target.rollout_us.saturating_add(incoming.rollout_us);
    target.expansion_us = target.expansion_us.saturating_add(incoming.expansion_us);
    target.child_bookkeeping_us = target
        .child_bookkeeping_us
        .saturating_add(incoming.child_bookkeeping_us);
    target.engine_step_us = target
        .engine_step_us
        .saturating_add(incoming.engine_step_us);
    target.pre_expand_us = target.pre_expand_us.saturating_add(incoming.pre_expand_us);
    target.frontier_pop_us = target
        .frontier_pop_us
        .saturating_add(incoming.frontier_pop_us);
    target.turn_plan_seed_us = target
        .turn_plan_seed_us
        .saturating_add(incoming.turn_plan_seed_us);
    target.shadow_audit_us = target
        .shadow_audit_us
        .saturating_add(incoming.shadow_audit_us);
    target.root_turn_plan_diag_us = target
        .root_turn_plan_diag_us
        .saturating_add(incoming.root_turn_plan_diag_us);
    target.nodes_expanded = target
        .nodes_expanded
        .saturating_add(incoming.nodes_expanded);
    target.nodes_generated = target
        .nodes_generated
        .saturating_add(incoming.nodes_generated);
    target.rollout_calls = target.rollout_calls.saturating_add(incoming.rollout_calls);
    target.root_rollout_calls = target
        .root_rollout_calls
        .saturating_add(incoming.root_rollout_calls);
    target.child_rollout_calls = target
        .child_rollout_calls
        .saturating_add(incoming.child_rollout_calls);
    target.deferred_child_rollout_calls = target
        .deferred_child_rollout_calls
        .saturating_add(incoming.deferred_child_rollout_calls);
    target.turn_plan_seed_rollout_calls = target
        .turn_plan_seed_rollout_calls
        .saturating_add(incoming.turn_plan_seed_rollout_calls);
    target.deferred_child_rollout_nodes = target
        .deferred_child_rollout_nodes
        .saturating_add(incoming.deferred_child_rollout_nodes);
    target.deferred_child_rollout_requeues = target
        .deferred_child_rollout_requeues
        .saturating_add(incoming.deferred_child_rollout_requeues);
    target.rollout_cache_hits = target
        .rollout_cache_hits
        .saturating_add(incoming.rollout_cache_hits);
    target.rollout_cache_queries = target
        .rollout_cache_queries
        .saturating_add(incoming.rollout_cache_queries);
    target.rollout_cache_misses = target
        .rollout_cache_misses
        .saturating_add(incoming.rollout_cache_misses);
    target.rollout_cache_inserts = target
        .rollout_cache_inserts
        .saturating_add(incoming.rollout_cache_inserts);
    target.rollout_budget_skips = target
        .rollout_budget_skips
        .saturating_add(incoming.rollout_budget_skips);
    target.rollout_max_evaluation_budget_skips = target
        .rollout_max_evaluation_budget_skips
        .saturating_add(incoming.rollout_max_evaluation_budget_skips);
    target.rollout_deadline_budget_skips = target
        .rollout_deadline_budget_skips
        .saturating_add(incoming.rollout_deadline_budget_skips);
    target.rollout_truncated = target
        .rollout_truncated
        .saturating_add(incoming.rollout_truncated);
    target.rollout_terminal_wins = target
        .rollout_terminal_wins
        .saturating_add(incoming.rollout_terminal_wins);
    target.rollout_cache_lookup_us = target
        .rollout_cache_lookup_us
        .saturating_add(incoming.rollout_cache_lookup_us);
    target.rollout_policy_dispatch_us = target
        .rollout_policy_dispatch_us
        .saturating_add(incoming.rollout_policy_dispatch_us);
    target.rollout_no_potion_iterations = target
        .rollout_no_potion_iterations
        .saturating_add(incoming.rollout_no_potion_iterations);
    target.rollout_no_potion_phase_profile_us = target
        .rollout_no_potion_phase_profile_us
        .saturating_add(incoming.rollout_no_potion_phase_profile_us);
    target.rollout_no_potion_legal_actions_us = target
        .rollout_no_potion_legal_actions_us
        .saturating_add(incoming.rollout_no_potion_legal_actions_us);
    target.rollout_no_potion_choose_action_us = target
        .rollout_no_potion_choose_action_us
        .saturating_add(incoming.rollout_no_potion_choose_action_us);
    target.rollout_no_potion_choose_ordering_us = target
        .rollout_no_potion_choose_ordering_us
        .saturating_add(incoming.rollout_no_potion_choose_ordering_us);
    target.rollout_no_potion_probe_us = target
        .rollout_no_potion_probe_us
        .saturating_add(incoming.rollout_no_potion_probe_us);
    target.rollout_no_potion_probe_score_calls = target
        .rollout_no_potion_probe_score_calls
        .saturating_add(incoming.rollout_no_potion_probe_score_calls);
    target.rollout_no_potion_probe_actions_evaluated = target
        .rollout_no_potion_probe_actions_evaluated
        .saturating_add(incoming.rollout_no_potion_probe_actions_evaluated);
    target.rollout_no_potion_probe_step_reuses = target
        .rollout_no_potion_probe_step_reuses
        .saturating_add(incoming.rollout_no_potion_probe_step_reuses);
    target.rollout_no_potion_probe_engine_step_us = target
        .rollout_no_potion_probe_engine_step_us
        .saturating_add(incoming.rollout_no_potion_probe_engine_step_us);
    target.rollout_no_potion_probe_phase_profile_us = target
        .rollout_no_potion_probe_phase_profile_us
        .saturating_add(incoming.rollout_no_potion_probe_phase_profile_us);
    target.rollout_no_potion_probe_action_facts_us = target
        .rollout_no_potion_probe_action_facts_us
        .saturating_add(incoming.rollout_no_potion_probe_action_facts_us);
    target.rollout_no_potion_engine_step_us = target
        .rollout_no_potion_engine_step_us
        .saturating_add(incoming.rollout_no_potion_engine_step_us);
    target.rollout_no_potion_child_build_us = target
        .rollout_no_potion_child_build_us
        .saturating_add(incoming.rollout_no_potion_child_build_us);
    target.terminal_child_rollout_skips = target
        .terminal_child_rollout_skips
        .saturating_add(incoming.terminal_child_rollout_skips);
    target.terminal_turn_plan_seed_rollout_skips = target
        .terminal_turn_plan_seed_rollout_skips
        .saturating_add(incoming.terminal_turn_plan_seed_rollout_skips);
    target.turn_local_dominance_rollout_skips = target
        .turn_local_dominance_rollout_skips
        .saturating_add(incoming.turn_local_dominance_rollout_skips);
    target.external_payoff_samples = target
        .external_payoff_samples
        .saturating_add(incoming.external_payoff_samples);
    target.boss_samples = target.boss_samples.saturating_add(incoming.boss_samples);
    for example in &incoming.slowest {
        push_combat_performance_example_v1(&mut target.slowest, example.clone());
    }
}

fn push_combat_performance_example_v1(
    examples: &mut Vec<BranchCampaignCombatPerformanceExampleV1>,
    example: BranchCampaignCombatPerformanceExampleV1,
) {
    examples.push(example);
    examples.sort_by(|left, right| right.total_us.cmp(&left.total_us));
    examples.truncate(4);
}

fn dominant_combat_performance_bucket_for_sample_v1(
    sample: &CombatSearchPerformanceSnapshotV1,
) -> &'static str {
    dominant_combat_performance_bucket_v1(&[
        ("rollout", sample.rollout_us),
        ("expansion", sample.expansion_us),
        ("child", sample.child_bookkeeping_us),
        ("engine", sample.engine_step_us),
        ("pre_expand", sample.pre_expand_us),
        ("unattributed", sample.unattributed_us),
    ])
}

fn dominant_combat_performance_bucket_for_summary_v1(
    summary: &BranchCampaignCombatPerformanceSummaryV1,
) -> &'static str {
    dominant_combat_performance_bucket_v1(&[
        ("rollout", summary.rollout_us),
        ("expansion", summary.expansion_us),
        ("child", summary.child_bookkeeping_us),
        ("engine", summary.engine_step_us),
        ("pre_expand", summary.pre_expand_us),
        ("unattributed", summary.unattributed_us),
    ])
}

fn dominant_combat_performance_bucket_v1(buckets: &[(&'static str, u64)]) -> &'static str {
    buckets
        .iter()
        .max_by_key(|(_, value)| *value)
        .map(|(label, _)| *label)
        .unwrap_or("unknown")
}
