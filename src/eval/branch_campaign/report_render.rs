use std::collections::BTreeMap;

use super::branch_display::{
    compact_campaign_choice_label_metadata_v1, render_campaign_branch_state,
    render_campaign_discard_example_v1, render_compact_choice_path,
};
use super::intervention::{
    campaign_strategy_next_step_v1, render_campaign_intervention_details_v2,
};
use super::lineage::{
    campaign_boss_relic_lineage_counts_v1, campaign_branch_boss_relic_lineage_key_v1,
    render_string_counts_v1,
};
use super::performance::{
    aggregate_campaign_combat_performance_v1, format_seconds_from_us_1dp_v1,
    render_campaign_combat_performance_v1,
};
use super::retry::{BranchCampaignCombatRetryLedgerV1, BOSS_GATE_RETRY_ATTEMPTS_PER_GATE};
use super::strategic_signals::{
    campaign_strategic_signals_for_render_v1, render_campaign_strategic_concern_v1,
    render_campaign_strategic_signals_v1,
};
use super::{
    BranchCampaignBranchV1, BranchCampaignReportV1, BranchCampaignStrategyRequestV1,
    UNSPENT_GOLD_PRESSURE_THRESHOLD,
};

pub fn render_branch_campaign_compact_v1(
    report: &BranchCampaignReportV1,
    branch_examples: usize,
) -> String {
    render_branch_campaign_compact_with_detail_v1(
        report,
        branch_examples,
        BranchCampaignReportDetailV1::Human,
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BranchCampaignReportDetailV1 {
    Human,
    Diagnose,
    Perf,
}

pub fn render_branch_campaign_compact_with_detail_v1(
    report: &BranchCampaignReportV1,
    branch_examples: usize,
    detail: BranchCampaignReportDetailV1,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "{} seed={} ascension=A{} domain={} role={} class={} rounds={} stop={}",
        report.schema_name,
        report.seed,
        report.run_domain.ascension_level,
        report.run_domain.label,
        report.run_domain.role,
        report.run_domain.player_class,
        report.rounds_completed,
        report.stop_reason
    ));
    if !report.run_prelude.is_empty() {
        lines.push(format!(
            "Replay root: id={} coordinate={} prefix_commands={}",
            report.run_prelude.replay_root_id,
            report.run_prelude.branch_command_coordinate,
            report.run_prelude.prefix_commands.len()
        ));
    }
    lines.push(format!(
        "Active {} | Frozen {} | Dead {} | Abandoned {} | Victories {} | Stuck {} | Discarded {}",
        report.active.len(),
        report.frozen.len(),
        report.dead.len(),
        report.abandoned.len(),
        report.victories.len(),
        report.stuck.len(),
        report.discarded_count
    ));
    if let Some(round) = report.rounds.last() {
        let mut last_round = format!(
            "Last round: started={} produced={} branch_points={} active_after={}",
            round.started_active,
            round.produced_branches,
            round.explored_branch_points,
            round.active_after
        );
        if round.frozen_added > 0 {
            last_round.push_str(&format!(" frozen+={}", round.frozen_added));
        }
        if round.discarded_added > 0 {
            last_round.push_str(&format!(" discarded+={}", round.discarded_added));
        }
        if round.combat_budget_retries > 0 {
            last_round.push_str(&format!(" combat_retries={}", round.combat_budget_retries));
        }
        let limits = render_round_limits_v1(round.branch_limit_hit, round.wall_limit_hit);
        if !limits.is_empty() {
            last_round.push_str(&format!(" limits=[{limits}]"));
        }
        lines.push(last_round);
    }
    let total_round_elapsed_ms: u64 = report
        .rounds
        .iter()
        .map(|round| round.elapsed_wall_ms)
        .sum();
    let parent_elapsed_wall_ms_sum: u64 = report
        .rounds
        .iter()
        .map(|round| round.parent_elapsed_wall_ms_sum)
        .sum();
    let parent_elapsed_wall_ms_max = report
        .rounds
        .iter()
        .map(|round| round.parent_elapsed_wall_ms_max)
        .max()
        .unwrap_or_default();
    let combat_retry_elapsed_wall_ms_sum: u64 = report
        .rounds
        .iter()
        .map(|round| round.combat_retry_elapsed_wall_ms_sum)
        .sum();
    let combat_retry_elapsed_wall_ms_max = report
        .rounds
        .iter()
        .map(|round| round.combat_retry_elapsed_wall_ms_max)
        .max()
        .unwrap_or_default();
    if detail == BranchCampaignReportDetailV1::Perf
        && (total_round_elapsed_ms > 0 || parent_elapsed_wall_ms_sum > 0)
    {
        lines.push(format!(
            "Timing: rounds={} parent_sum={} parent_max={} combat_retry_sum={} combat_retry_max={}",
            format_seconds_1dp_v1(total_round_elapsed_ms),
            format_seconds_1dp_v1(parent_elapsed_wall_ms_sum),
            format_seconds_1dp_v1(parent_elapsed_wall_ms_max),
            format_seconds_1dp_v1(combat_retry_elapsed_wall_ms_sum),
            format_seconds_1dp_v1(combat_retry_elapsed_wall_ms_max),
        ));
    }
    if detail == BranchCampaignReportDetailV1::Perf && !report.state_store.is_empty() {
        lines.push(format!(
            "State store: sessions={} nodes={} linked={} coords=decision:{}/{} route:{}/{} replay=exact:{} ancestor:{} miss:{} suffix=sum:{} max:{} cache=pruned:{} anchors:{} lookups={}/{} inserts={} retains={}",
            report.state_store.sessions,
            report.state_store.nodes,
            report.state_store.linked_nodes,
            report.state_store.decision_coordinate_nodes,
            report.state_store.decision_coordinate_sessions,
            report.state_store.route_decision_coordinate_nodes,
            report.state_store.route_decision_coordinate_sessions,
            report.state_store.replay_exact_hits,
            report.state_store.replay_ancestor_hits,
            report.state_store.replay_misses,
            report.state_store.replay_suffix_commands_sum,
            report.state_store.replay_suffix_commands_max,
            report.state_store.sessions_pruned,
            report.state_store.anchor_sessions_kept,
            report.state_store.lookup_hits,
            report.state_store.lookup_misses,
            report.state_store.inserts,
            report.state_store.retains,
        ));
    }
    let combat_performance = aggregate_campaign_combat_performance_v1(report);
    if detail == BranchCampaignReportDetailV1::Perf && combat_performance.samples > 0 {
        lines.push(render_campaign_combat_performance_v1(&combat_performance));
        if let Some(example) = combat_performance.slowest.first() {
            lines.push(format!(
                "  slowest: A{}F{} turn={} {} {} {} bucket={} status={}",
                example.act,
                example.floor,
                example.turn,
                example.combat_kind,
                example.enemies,
                format_seconds_from_us_1dp_v1(example.total_us),
                example.dominant_bucket,
                example.coverage_status
            ));
        }
    }
    if detail != BranchCampaignReportDetailV1::Human && !report.combat_retry_ledger.is_empty() {
        lines.push(format!(
            "Combat retry ledger: boss_gate={}",
            render_combat_retry_ledger_v1(&report.combat_retry_ledger)
        ));
    }
    if detail != BranchCampaignReportDetailV1::Human
        && report.discarded_count > 0
        && !report.discarded_examples.is_empty()
    {
        lines.push(format!(
            "Branch pressure: discarded={} examples=[{}]",
            report.discarded_count,
            render_branch_pressure_examples_v1(&report.discarded_examples)
        ));
    }
    if detail != BranchCampaignReportDetailV1::Human && !report.abandoned.is_empty() {
        lines.push(format!(
            "Abandoned examples: count={} reasons=[{}] examples=[{}]",
            report.abandoned.len(),
            render_campaign_branch_stop_reasons_v1(&report.abandoned, 3),
            render_campaign_branch_examples_v1(&report.abandoned, 3)
        ));
    }
    if let Some(final_boss_failures) =
        render_campaign_final_boss_failure_summary_v1(report, branch_examples)
    {
        lines.push(final_boss_failures);
    }
    if let Some(boss_relic_coverage) = render_campaign_boss_relic_coverage_v1(report) {
        lines.push(boss_relic_coverage);
    }
    if report.route_evidence.decisions > 0 || report.route_evidence.candidate_pools > 0 {
        if detail != BranchCampaignReportDetailV1::Human {
            lines.push(format!(
                "Route evidence: decisions={} candidate_pools={} pool_candidates={} pool_safety=ok:{} risky:{} rejected:{} complete_pools={} first_elite optional={} forced={} none={} avg_elite_prep={} underprepared={} bailouts=rest:{} shop:{}",
                report.route_evidence.decisions,
                report.route_evidence.candidate_pools,
                report.route_evidence.candidate_pool_candidates,
                report.route_evidence.candidate_pool_ok,
                report.route_evidence.candidate_pool_risky,
                report.route_evidence.candidate_pool_rejected,
                report.route_evidence.complete_candidate_pools,
                report.route_evidence.first_elite_optional,
                report.route_evidence.first_elite_forced,
                report.route_evidence.first_elite_none,
                format_bp(report.route_evidence.avg_elite_prep_bp),
                report.route_evidence.underprepared_first_elite,
                report.route_evidence.rest_bailout,
                report.route_evidence.shop_bailout,
            ));
            if let Some(example) = report.route_evidence.examples.first() {
                lines.push(format!(
                    "  example: {} | first_elite={} elite_prep={}",
                    example.target,
                    example.first_elite,
                    format_bp(example.elite_prep_bp)
                ));
            }
        }
        if report.route_evidence.underprepared_first_elite > 0 {
            let label = if detail == BranchCampaignReportDetailV1::Human {
                "Warning route"
            } else {
                "Route concern"
            };
            lines.push(format!(
                "{label}: forced_first_elite_underprepared={}/{} rest_bailout={} shop_bailout={}",
                report.route_evidence.underprepared_first_elite,
                report.route_evidence.decisions,
                report.route_evidence.rest_bailout,
                report.route_evidence.shop_bailout,
            ));
            if detail != BranchCampaignReportDetailV1::Human {
                if let Some(example) = report.route_evidence.underprepared_examples.first() {
                    lines.push(format!(
                        "  concern example: {} | first_elite={} elite_prep={}",
                        example.target,
                        example.first_elite,
                        format_bp(example.elite_prep_bp)
                    ));
                }
            }
        }
    }
    if let Some(pressure) = campaign_unspent_gold_pressure_v1(report) {
        lines.push(format!(
            "Resource concern: high_unspent_gold_near_boss={} max_gold={} causes=[{}]",
            pressure.count, pressure.max_gold, pressure.cause_counts
        ));
        if detail != BranchCampaignReportDetailV1::Human {
            lines.push(format!("  resource example: {}", pressure.example));
        }
    }
    if let Some(pressure) = campaign_boss_mechanic_pressure_v1(report) {
        lines.push(format!(
            "Boss pressure: bosses=[{}] signals=[{}]",
            pressure.boss_counts, pressure.signal_counts
        ));
        if detail != BranchCampaignReportDetailV1::Human {
            lines.push(format!("  boss example: {}", pressure.example));
        }
    }
    if let Some(combat_lab) = render_campaign_combat_lab_probe_summary_v1(report) {
        lines.extend(combat_lab);
    }
    let strategic_signals = campaign_strategic_signals_for_render_v1(report);
    if detail != BranchCampaignReportDetailV1::Human {
        if let Some(strategic) = render_campaign_strategic_signals_v1(&strategic_signals) {
            lines.push(strategic);
        }
    }
    if let Some(concern) = render_campaign_strategic_concern_v1(&strategic_signals) {
        lines.push(concern);
    }
    if detail != BranchCampaignReportDetailV1::Human {
        if let Some(choice_coverage) = render_campaign_choice_coverage_v1(report) {
            lines.push(choice_coverage);
        }
    }
    if let Some(victory_lines) = render_campaign_victory_quality_lines_v1(report) {
        lines.push(String::new());
        lines.extend(victory_lines);
    }
    if report.stop_reason == "max_rounds"
        && (!report.active.is_empty() || !report.frozen.is_empty())
    {
        lines.push(
            "Next: budget ended; continue with an explicit source, e.g. .\\tools\\campaign.ps1 -From <source> -Continue, or add -Rounds N for a small fixed continuation"
                .to_string(),
        );
    }
    let render_strategy_requests = report.victories.is_empty()
        && !report.strategy_requests.is_empty()
        && (campaign_report_stop_needs_immediate_intervention_v1(report)
            || report.active.is_empty());
    if render_strategy_requests {
        lines.push(String::new());
        if campaign_report_stop_needs_immediate_intervention_v1(report) {
            lines.push("Needs intervention:".to_string());
        } else {
            lines.push("Deferred strategy notes:".to_string());
        }
        for request in report.strategy_requests.iter().take(4) {
            lines.push(format!(
                "  {} | {} | branches={}",
                request.kind, request.boundary_title, request.branch_count
            ));
            if let Some(reason) = request.stop_reasons.first() {
                lines.push(format!("    stop: {reason}"));
            }
            if let Some(example) = request.examples.first() {
                lines.push(format!("    example: {example}"));
            }
            lines.extend(render_campaign_strategy_context_v1(request));
            lines.push(format!("    needed: {}", request.suggested_action));
            if let Some(next_step) = campaign_strategy_next_step_v1(&request.kind) {
                lines.push(format!("    next: {next_step}"));
            }
            lines.extend(render_campaign_intervention_details_v2(report, request));
        }
    }
    if !report.active.is_empty() {
        lines.push(String::new());
        lines.push("Top active:".to_string());
        let shown = report
            .active
            .iter()
            .take(render_branch_examples_for_detail_v1(
                branch_examples,
                detail,
            ))
            .collect::<Vec<_>>();
        let baseline = shown.first().copied();
        for (index, branch) in shown.into_iter().enumerate() {
            lines.push(format!(
                "  {}. {} | {} | choices: {}{}{}",
                index + 1,
                render_campaign_branch_state(branch),
                branch.frontier_title,
                render_compact_choice_path(&branch.choice_labels),
                render_campaign_continuation_origin_suffix_v1(branch),
                render_campaign_branch_diff_suffix_v1(branch, baseline, index)
            ));
        }
    }
    if !report.frozen.is_empty() {
        lines.push(String::new());
        lines.push("Frozen examples:".to_string());
        let shown = report
            .frozen
            .iter()
            .take(render_branch_examples_for_detail_v1(
                branch_examples,
                detail,
            ))
            .collect::<Vec<_>>();
        let baseline = shown.first().copied();
        for (index, branch) in shown.into_iter().enumerate() {
            lines.push(format!(
                "  {}. {} | {} | choices: {}{}{}",
                index + 1,
                render_campaign_branch_state(branch),
                branch.frontier_title,
                render_compact_choice_path(&branch.choice_labels),
                render_campaign_continuation_origin_suffix_v1(branch),
                render_campaign_branch_diff_suffix_v1(branch, baseline, index)
            ));
        }
    }
    lines.join("\n")
}

fn render_campaign_continuation_origin_suffix_v1(branch: &BranchCampaignBranchV1) -> String {
    let Some(origin) = &branch.continuation_origin else {
        return String::new();
    };
    let route_suffix = origin
        .route_origin
        .as_ref()
        .map(render_campaign_route_continuation_origin_v1)
        .unwrap_or_default();
    let lane_suffix = origin
        .target_lane
        .as_ref()
        .map(render_campaign_continuation_target_lane_v1)
        .unwrap_or_default();
    let source_suffix = if origin.target_origin_source.is_empty() {
        String::new()
    } else {
        format!(" source={}", origin.target_origin_source)
    };
    format!(
        " | origin={}:{}:{}{}{}{}",
        origin.kind,
        origin.event_type,
        compact_campaign_choice_label_metadata_v1(&origin.label),
        source_suffix,
        lane_suffix,
        route_suffix
    )
}

fn render_campaign_continuation_target_lane_v1(
    lane: &crate::eval::branch_campaign::BranchCampaignContinuationTargetLaneV1,
) -> String {
    format!(
        " lane={}:{}:{}:{}",
        lane.bucket,
        render_campaign_continuation_admission_status_v1(lane.admission_status),
        render_campaign_continuation_disposition_v1(lane.disposition),
        lane.semantic_lane
    )
}

fn render_campaign_continuation_admission_status_v1(
    status: crate::eval::campaign_journal::CampaignJournalCandidateAdmissionStatusV1,
) -> &'static str {
    match status {
        crate::eval::campaign_journal::CampaignJournalCandidateAdmissionStatusV1::Unknown => {
            "unknown"
        }
        crate::eval::campaign_journal::CampaignJournalCandidateAdmissionStatusV1::Scheduled => {
            "scheduled"
        }
        crate::eval::campaign_journal::CampaignJournalCandidateAdmissionStatusV1::Deferred => {
            "deferred"
        }
        crate::eval::campaign_journal::CampaignJournalCandidateAdmissionStatusV1::Rejected => {
            "rejected"
        }
    }
}

fn render_campaign_continuation_disposition_v1(
    disposition: crate::eval::campaign_journal::CampaignJournalCandidateDispositionV1,
) -> &'static str {
    match disposition {
        crate::eval::campaign_journal::CampaignJournalCandidateDispositionV1::Kept => "kept",
        crate::eval::campaign_journal::CampaignJournalCandidateDispositionV1::Pruned => "pruned",
    }
}

fn render_campaign_route_continuation_origin_v1(
    origin: &crate::eval::branch_campaign::BranchCampaignRouteContinuationOriginV1,
) -> String {
    let path = origin
        .path
        .as_ref()
        .map(|path| {
            format!(
                " paths={}/{} elites={}-{} fires={}-{} shops={}-{}",
                origin.observed_path_count,
                origin.path_budget,
                path.min_elites,
                path.max_elites,
                path.min_fires,
                path.max_fires,
                path.min_shops,
                path.max_shops,
            )
        })
        .unwrap_or_else(|| {
            format!(
                " paths={}/{}",
                origin.observed_path_count, origin.path_budget
            )
        });
    let first_elite = origin
        .first_elite
        .as_ref()
        .map(render_campaign_route_first_elite_origin_v1)
        .unwrap_or_default();
    format!(
        " route=x{}y{} coverage={}{}{}",
        origin.target_x, origin.target_y, origin.projection_coverage, path, first_elite
    )
}

fn render_campaign_route_first_elite_origin_v1(
    first_elite: &crate::eval::branch_campaign::BranchCampaignRouteFirstEliteContinuationOriginV1,
) -> String {
    let status = if first_elite.forced {
        "forced"
    } else if first_elite.optional {
        "optional"
    } else {
        "none"
    };
    format!(
        " first_elite={} hallways={}-{} rest_bailout={} shop_bailout={}",
        status,
        first_elite.min_hallway_fights_before,
        first_elite.max_hallway_fights_before,
        first_elite.can_bail_to_rest_before,
        first_elite.can_bail_to_shop_before
    )
}

fn render_round_limits_v1(branch_limit_hit: bool, wall_limit_hit: bool) -> String {
    let mut limits = Vec::new();
    if branch_limit_hit {
        limits.push("branch");
    }
    if wall_limit_hit {
        limits.push("wall");
    }
    limits.join(",")
}

fn render_branch_examples_for_detail_v1(
    branch_examples: usize,
    detail: BranchCampaignReportDetailV1,
) -> usize {
    match detail {
        BranchCampaignReportDetailV1::Human => branch_examples.min(2),
        BranchCampaignReportDetailV1::Diagnose | BranchCampaignReportDetailV1::Perf => {
            branch_examples
        }
    }
}

fn render_campaign_final_boss_failure_summary_v1(
    report: &BranchCampaignReportV1,
    branch_examples: usize,
) -> Option<String> {
    let failures = report
        .abandoned
        .iter()
        .filter(|branch| {
            branch.frontier_title == "Combat"
                && branch
                    .summary
                    .as_ref()
                    .is_some_and(|summary| summary.act == 3 && summary.floor >= 48)
        })
        .collect::<Vec<_>>();
    if failures.is_empty() {
        return None;
    }

    let mut boss_counts = BTreeMap::<String, usize>::new();
    let mut hp_min = i32::MAX;
    let mut hp_max = i32::MIN;
    let mut deck_min = usize::MAX;
    let mut deck_max = usize::MIN;
    for branch in &failures {
        if let Some(summary) = branch.summary.as_ref() {
            let boss = if summary.boss.is_empty() {
                "unknown".to_string()
            } else {
                summary.boss.clone()
            };
            *boss_counts.entry(boss).or_default() += 1;
            hp_min = hp_min.min(summary.hp);
            hp_max = hp_max.max(summary.hp);
            deck_min = deck_min.min(summary.deck_count);
            deck_max = deck_max.max(summary.deck_count);
        }
    }

    let bosses = boss_counts
        .into_iter()
        .map(|(boss, count)| format!("{boss}={count}"))
        .collect::<Vec<_>>()
        .join(" ");
    let examples = failures
        .iter()
        .take(branch_examples.max(1).min(3))
        .map(|branch| render_campaign_branch_state(branch))
        .collect::<Vec<_>>()
        .join(" | ");

    Some(format!(
        "Final boss failures: abandoned={} bosses=[{}] hp={}..{} deck={}..{} examples=[{}]",
        failures.len(),
        bosses,
        hp_min,
        hp_max,
        deck_min,
        deck_max,
        examples
    ))
}

fn render_campaign_boss_relic_coverage_v1(report: &BranchCampaignReportV1) -> Option<String> {
    let active = campaign_boss_relic_lineage_counts_v1(&report.active);
    let frozen = campaign_boss_relic_lineage_counts_v1(&report.frozen);
    let abandoned = campaign_boss_relic_lineage_counts_v1(&report.abandoned);
    if active.is_empty() && frozen.is_empty() && abandoned.is_empty() {
        return None;
    }

    let mut furthest = BTreeMap::<String, (u8, i32)>::new();
    for branch in report
        .active
        .iter()
        .chain(report.frozen.iter())
        .chain(report.abandoned.iter())
        .chain(report.victories.iter())
        .chain(report.dead.iter())
        .chain(report.stuck.iter())
    {
        let Some(lineage) = campaign_branch_boss_relic_lineage_key_v1(branch) else {
            continue;
        };
        let Some(summary) = branch.summary.as_ref() else {
            continue;
        };
        let progress = (summary.act, summary.floor);
        furthest
            .entry(lineage)
            .and_modify(|existing| {
                if progress > *existing {
                    *existing = progress;
                }
            })
            .or_insert(progress);
    }

    Some(format!(
        "Boss relic coverage: active=[{}] frozen=[{}] abandoned=[{}] furthest=[{}]",
        render_string_counts_v1(&active),
        render_string_counts_v1(&frozen),
        render_string_counts_v1(&abandoned),
        furthest
            .into_iter()
            .map(|(lineage, (act, floor))| format!("{lineage}=A{act}F{floor}"))
            .collect::<Vec<_>>()
            .join(" ")
    ))
}

fn format_seconds_1dp_v1(ms: u64) -> String {
    format!("{:.1}s", ms as f64 / 1000.0)
}

fn format_bp(value: i32) -> String {
    format!("{:.2}", f64::from(value) / 100.0)
}

fn render_combat_retry_ledger_v1(ledger: &BranchCampaignCombatRetryLedgerV1) -> String {
    ledger
        .boss_gate_attempts
        .iter()
        .map(|entry| {
            format!(
                "A{}F{}={}/{}",
                entry.act, entry.floor, entry.attempts, BOSS_GATE_RETRY_ATTEMPTS_PER_GATE
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

struct CampaignUnspentGoldPressureV1 {
    count: usize,
    max_gold: i32,
    cause_counts: String,
    example: String,
}

struct CampaignBossMechanicPressureV1 {
    boss_counts: String,
    signal_counts: String,
    example: String,
}

fn campaign_boss_mechanic_pressure_v1(
    report: &BranchCampaignReportV1,
) -> Option<CampaignBossMechanicPressureV1> {
    let branches = report
        .active
        .iter()
        .chain(report.frozen.iter())
        .chain(report.victories.iter())
        .chain(report.abandoned.iter())
        .chain(report.stuck.iter())
        .chain(report.dead.iter())
        .filter(|branch| branch_has_boss_mechanic_pressure_v1(branch))
        .collect::<Vec<_>>();
    if branches.is_empty() {
        return None;
    }

    let mut boss_counts = BTreeMap::<String, usize>::new();
    let mut signal_counts = BTreeMap::<String, usize>::new();
    for branch in &branches {
        let Some(summary) = branch.summary.as_ref() else {
            continue;
        };
        *boss_counts.entry(summary.boss.clone()).or_default() += 1;
        for signal in &summary.boss_pressure {
            *signal_counts.entry(signal.clone()).or_default() += 1;
        }
    }

    let example = branches
        .iter()
        .max_by(|left, right| {
            boss_mechanic_pressure_key_v1(left).cmp(&boss_mechanic_pressure_key_v1(right))
        })
        .map(|branch| {
            let summary = branch
                .summary
                .as_ref()
                .expect("filtered branch has summary");
            format!(
                "A{}F{} HP {}/{} deck {} boss={} | {}",
                summary.act,
                summary.floor,
                summary.hp,
                summary.max_hp,
                summary.deck_count,
                summary.boss,
                summary.boss_pressure.join(" ")
            )
        })
        .unwrap_or_default();

    Some(CampaignBossMechanicPressureV1 {
        boss_counts: render_string_count_map_v1(&boss_counts, usize::MAX),
        signal_counts: render_string_count_map_v1(&signal_counts, 8),
        example,
    })
}

fn branch_has_boss_mechanic_pressure_v1(branch: &BranchCampaignBranchV1) -> bool {
    let Some(summary) = branch.summary.as_ref() else {
        return false;
    };
    !summary.boss.is_empty()
        && !summary.boss_pressure.is_empty()
        && summary.floor >= boss_approach_floor_v1(summary.act)
}

fn render_campaign_combat_lab_probe_summary_v1(
    report: &BranchCampaignReportV1,
) -> Option<Vec<String>> {
    let probes = report
        .active
        .iter()
        .chain(report.frozen.iter())
        .chain(report.victories.iter())
        .chain(report.abandoned.iter())
        .chain(report.stuck.iter())
        .chain(report.dead.iter())
        .flat_map(|branch| branch.combat_lab_probes.iter())
        .collect::<Vec<_>>();
    if probes.is_empty() {
        return None;
    }

    let mut kind_counts = BTreeMap::<String, usize>::new();
    let mut result_counts = BTreeMap::<String, usize>::new();
    for probe in &probes {
        *kind_counts.entry(probe.kind.clone()).or_default() += 1;
        *result_counts.entry(probe.result.clone()).or_default() += 1;
    }
    let example = probes
        .iter()
        .find(|probe| probe.kind == "current_act_boss_preview")
        .or_else(|| probes.first())
        .expect("non-empty probe list");

    let mut lines = vec![
        format!(
            "Combat lab probes: {} {}",
            render_string_count_map_v1(&kind_counts, 6),
            render_string_count_map_v1(&result_counts, 6)
        ),
        format!(
            "  probe example: boss={} source={} boundary={} result={}",
            example.boss.as_deref().unwrap_or("unknown"),
            example.source,
            example.boundary,
            example.result
        ),
    ];
    if !example.diagnosis.is_empty() {
        lines.push(format!(
            "  probe diagnosis: {}/{} confidence={} signals={}",
            example.diagnosis.outcome_class,
            example.diagnosis.search_reason,
            example.diagnosis.confidence,
            render_probe_signal_list_v1(&example.diagnosis.signals)
        ));
    }
    Some(lines)
}

fn render_probe_signal_list_v1(signals: &[String]) -> String {
    if signals.is_empty() {
        "-".to_string()
    } else {
        signals.join(",")
    }
}

fn boss_mechanic_pressure_key_v1(branch: &BranchCampaignBranchV1) -> (i32, i32, i32) {
    branch
        .summary
        .as_ref()
        .map(|summary| {
            (
                summary.floor,
                summary.boss_pressure.len() as i32,
                summary.hp,
            )
        })
        .unwrap_or((0, 0, 0))
}

fn render_string_count_map_v1(counts: &BTreeMap<String, usize>, limit: usize) -> String {
    counts
        .iter()
        .take(limit)
        .map(|(label, count)| format!("{label}={count}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn campaign_unspent_gold_pressure_v1(
    report: &BranchCampaignReportV1,
) -> Option<CampaignUnspentGoldPressureV1> {
    let pressured = report
        .active
        .iter()
        .chain(report.frozen.iter())
        .filter(|branch| branch_has_unspent_gold_pressure_v1(branch))
        .collect::<Vec<_>>();
    if pressured.is_empty() {
        return None;
    }
    let max_gold = pressured
        .iter()
        .filter_map(|branch| branch.summary.as_ref().map(|summary| summary.gold))
        .max()
        .unwrap_or(0);
    let cause_counts = render_unspent_gold_cause_counts_v1(&pressured);
    let example = pressured
        .iter()
        .max_by(|left, right| {
            unspent_gold_pressure_key_v1(left).cmp(&unspent_gold_pressure_key_v1(right))
        })
        .map(|branch| {
            let summary = branch
                .summary
                .as_ref()
                .expect("filtered branch has summary");
            format!(
                "A{}F{} gold {} cause={} | {}",
                summary.act,
                summary.floor,
                summary.gold,
                branch_unspent_gold_pressure_cause_v1(branch),
                render_compact_choice_path(&branch.choice_labels)
            )
        })
        .unwrap_or_default();
    Some(CampaignUnspentGoldPressureV1 {
        count: pressured.len(),
        max_gold,
        cause_counts,
        example,
    })
}

fn branch_has_unspent_gold_pressure_v1(branch: &BranchCampaignBranchV1) -> bool {
    let Some(summary) = branch.summary.as_ref() else {
        return false;
    };
    summary.gold >= UNSPENT_GOLD_PRESSURE_THRESHOLD
        && summary.floor >= boss_approach_floor_v1(summary.act)
}

fn unspent_gold_pressure_key_v1(branch: &BranchCampaignBranchV1) -> (i32, i32) {
    branch
        .summary
        .as_ref()
        .map(|summary| (summary.gold, summary.floor))
        .unwrap_or((0, 0))
}

fn branch_unspent_gold_pressure_cause_v1(branch: &BranchCampaignBranchV1) -> &'static str {
    let has_buy = branch
        .choice_labels
        .iter()
        .any(|label| is_campaign_shop_buy_label_v1(label));
    if has_buy {
        return "purchase_seen_gold_still_high";
    }
    let has_shop_leave = branch
        .choice_labels
        .iter()
        .any(|label| is_campaign_shop_leave_label_v1(label));
    if has_shop_leave {
        return "shop_leave_without_purchase";
    }
    let has_shop_signal = branch
        .choice_labels
        .iter()
        .any(|label| label.to_ascii_lowercase().contains("shop"));
    if has_shop_signal {
        return "shop_seen_without_purchase";
    }
    "no_shop_action_seen"
}

fn is_campaign_shop_buy_label_v1(label: &str) -> bool {
    let normalized = label.trim().to_ascii_lowercase();
    normalized.starts_with("buy ") || normalized.contains("| buy ")
}

fn is_campaign_shop_leave_label_v1(label: &str) -> bool {
    let normalized = label.to_ascii_lowercase();
    normalized.contains("leave shop")
        || normalized.contains("auto leave shop")
        || normalized.contains("decline selected shop purchase portfolio")
}

fn render_unspent_gold_cause_counts_v1(branches: &[&BranchCampaignBranchV1]) -> String {
    let mut counts = BTreeMap::<&'static str, usize>::new();
    for branch in branches {
        *counts
            .entry(branch_unspent_gold_pressure_cause_v1(branch))
            .or_default() += 1;
    }
    [
        "no_shop_action_seen",
        "shop_leave_without_purchase",
        "purchase_seen_gold_still_high",
        "shop_seen_without_purchase",
    ]
    .into_iter()
    .filter_map(|cause| counts.get(cause).map(|count| format!("{cause}={count}")))
    .collect::<Vec<_>>()
    .join(" ")
}

pub(super) fn boss_approach_floor_v1(act: u8) -> i32 {
    match act {
        1 => 10,
        2 => 24,
        3 => 40,
        _ => i32::MAX,
    }
}

fn render_branch_pressure_examples_v1(examples: &[String]) -> String {
    unique_limited_strings(
        examples
            .iter()
            .map(|example| truncate_branch_pressure_example_v1(example)),
        3,
    )
    .join(" | ")
}

fn render_campaign_branch_examples_v1(
    branches: &[BranchCampaignBranchV1],
    max_examples: usize,
) -> String {
    unique_limited_strings(
        branches
            .iter()
            .map(render_campaign_discard_example_v1)
            .map(|example| truncate_branch_pressure_example_v1(&example)),
        max_examples,
    )
    .join(" | ")
}

fn render_campaign_branch_stop_reasons_v1(
    branches: &[BranchCampaignBranchV1],
    max_examples: usize,
) -> String {
    unique_limited_strings(
        branches
            .iter()
            .map(|branch| branch.stop_reason.trim())
            .filter(|reason| !reason.is_empty())
            .map(truncate_branch_pressure_example_v1),
        max_examples,
    )
    .join(" | ")
}

fn render_campaign_choice_coverage_v1(report: &BranchCampaignReportV1) -> Option<String> {
    if report.active.is_empty() && report.frozen.is_empty() {
        return None;
    }
    let active_first = render_campaign_choice_count_summary_v1(
        report
            .active
            .iter()
            .filter_map(campaign_branch_first_choice_v1),
    );
    let active_latest = render_campaign_choice_count_summary_v1(
        report
            .active
            .iter()
            .filter_map(campaign_branch_latest_choice_v1),
    );
    let frozen_first = render_campaign_choice_count_summary_v1(
        report
            .frozen
            .iter()
            .filter_map(campaign_branch_first_choice_v1),
    );
    let frozen_latest = render_campaign_choice_count_summary_v1(
        report
            .frozen
            .iter()
            .filter_map(campaign_branch_latest_choice_v1),
    );
    Some(format!(
        "Choice coverage: active_first=[{}] active_latest=[{}] frozen_first=[{}] frozen_latest=[{}]",
        active_first, active_latest, frozen_first, frozen_latest
    ))
}

fn render_campaign_choice_count_summary_v1<'a>(choices: impl Iterator<Item = &'a str>) -> String {
    let mut counts = BTreeMap::<String, usize>::new();
    for choice in choices {
        let compact_choice = compact_campaign_choice_label_metadata_v1(choice);
        let choice = compact_choice.trim();
        if choice.is_empty() {
            continue;
        }
        *counts.entry(choice.to_string()).or_default() += 1;
    }
    if counts.is_empty() {
        return "-".to_string();
    }
    let mut entries = counts.into_iter().collect::<Vec<_>>();
    entries.sort_by(|(left_label, left_count), (right_label, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_label.cmp(right_label))
    });
    entries
        .into_iter()
        .take(3)
        .map(|(label, count)| format!("{}={count}", truncate_campaign_diff_label_v1(&label)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn campaign_branch_first_choice_v1(branch: &BranchCampaignBranchV1) -> Option<&str> {
    branch.choice_labels.first().map(String::as_str)
}

fn campaign_branch_latest_choice_v1(branch: &BranchCampaignBranchV1) -> Option<&str> {
    branch.choice_labels.last().map(String::as_str)
}

fn render_campaign_branch_diff_suffix_v1(
    branch: &BranchCampaignBranchV1,
    baseline: Option<&BranchCampaignBranchV1>,
    index: usize,
) -> String {
    if index == 0 {
        return String::new();
    }
    let Some(baseline) = baseline else {
        return String::new();
    };
    let mut parts = Vec::new();
    if let Some(choice_diff) = render_campaign_choice_diff_v1(branch, baseline) {
        parts.push(format!("choices {choice_diff}"));
    }
    if let (Some(summary), Some(base_summary)) =
        (branch.summary.as_ref(), baseline.summary.as_ref())
    {
        if summary.formation_stage != base_summary.formation_stage {
            parts.push(format!(
                "stage {}->{}",
                base_summary.formation_stage, summary.formation_stage
            ));
        }
        if let Some(diff) = render_string_set_diff_v1(
            &summary.formation_strengths,
            &base_summary.formation_strengths,
        ) {
            parts.push(format!("strengths {diff}"));
        }
        if let Some(diff) =
            render_string_set_diff_v1(&summary.formation_needs, &base_summary.formation_needs)
        {
            parts.push(format!("needs {diff}"));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" | diff: {}", parts.join("; "))
    }
}

fn render_campaign_choice_diff_v1(
    branch: &BranchCampaignBranchV1,
    baseline: &BranchCampaignBranchV1,
) -> Option<String> {
    let mut additions = Vec::new();
    let max_len = branch.choice_labels.len().max(baseline.choice_labels.len());
    for index in 0..max_len {
        let current = branch.choice_labels.get(index);
        let base = baseline.choice_labels.get(index);
        if current == base {
            continue;
        }
        if let Some(label) = current {
            let label = compact_campaign_choice_label_metadata_v1(label);
            additions.push(format!("+{}", truncate_campaign_diff_label_v1(&label)));
        }
        if additions.len() >= 3 {
            break;
        }
    }
    if additions.is_empty() {
        None
    } else {
        Some(additions.join(" "))
    }
}

fn render_string_set_diff_v1(current: &[String], baseline: &[String]) -> Option<String> {
    let mut added = current
        .iter()
        .filter(|value| !baseline.contains(value))
        .cloned()
        .collect::<Vec<_>>();
    let mut removed = baseline
        .iter()
        .filter(|value| !current.contains(value))
        .cloned()
        .collect::<Vec<_>>();
    added.sort();
    removed.sort();
    let mut parts = Vec::new();
    parts.extend(added.into_iter().take(3).map(|value| format!("+{value}")));
    parts.extend(removed.into_iter().take(3).map(|value| format!("-{value}")));
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn truncate_campaign_diff_label_v1(value: &str) -> String {
    const MAX_CHARS: usize = 48;
    if value.chars().count() <= MAX_CHARS {
        return value.to_string();
    }
    let prefix = value
        .chars()
        .take(MAX_CHARS.saturating_sub(3))
        .collect::<String>();
    format!("{prefix}...")
}

fn render_campaign_victory_quality_lines_v1(
    report: &BranchCampaignReportV1,
) -> Option<Vec<String>> {
    let first = report.victories.first()?;
    let best = report
        .victories
        .iter()
        .max_by(|left, right| {
            victory_quality_key_v1(left)
                .cmp(&victory_quality_key_v1(right))
                .then_with(|| left.branch_id.cmp(&right.branch_id).reverse())
        })
        .unwrap_or(first);

    let mut lines = Vec::new();
    if report.victories.len() == 1 || first.branch_id == best.branch_id {
        lines.push(render_campaign_victory_line_v1("Victory", first));
    } else {
        lines.push(render_campaign_victory_line_v1("First victory", first));
        lines.push(render_campaign_victory_line_v1("Best victory", best));
    }
    Some(lines)
}

fn render_campaign_victory_line_v1(label: &str, branch: &BranchCampaignBranchV1) -> String {
    format!(
        "{label}: {} | choices: {}",
        render_campaign_branch_state(branch),
        render_compact_choice_path(&branch.choice_labels)
    )
}

fn victory_quality_key_v1(branch: &BranchCampaignBranchV1) -> (i32, i32, i32) {
    branch
        .summary
        .as_ref()
        .map(|summary| {
            let hp_percent = if summary.max_hp > 0 {
                (summary.hp.max(0) * 1000) / summary.max_hp
            } else {
                1000
            };
            (hp_percent, summary.hp, summary.gold)
        })
        .unwrap_or((0, 0, 0))
}

pub(super) fn unique_limited_strings<I>(items: I, limit: usize) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut result = Vec::new();
    for item in items {
        if result.len() >= limit {
            break;
        }
        if !result.contains(&item) {
            result.push(item);
        }
    }
    result
}

fn truncate_branch_pressure_example_v1(value: &str) -> String {
    const MAX_CHARS: usize = 96;
    let parts = value
        .split(" -> ")
        .map(compact_campaign_choice_label_metadata_v1)
        .collect::<Vec<_>>();
    let compressed = if parts.len() > 4 {
        format!(
            "{} -> {} -> ... -> {}",
            parts[0],
            parts[1],
            parts.last().map(String::as_str).unwrap_or_default()
        )
    } else {
        parts.join(" -> ")
    };
    if compressed.chars().count() <= MAX_CHARS {
        return compressed;
    }
    let prefix = compressed
        .chars()
        .take(MAX_CHARS.saturating_sub(3))
        .collect::<String>();
    format!("{prefix}...")
}

fn render_campaign_strategy_context_v1(request: &BranchCampaignStrategyRequestV1) -> Vec<String> {
    let mut lines = Vec::new();
    if request.act > 0 || request.floor > 0 {
        lines.push(format!("    context: A{}F{}", request.act, request.floor));
    }
    if let Some(offer) = &request.next_card_reward_offer {
        if !offer.is_empty() {
            lines.push(format!("    next reward offer: {}", offer.join(" | ")));
        }
    }
    for detail in request.boundary_details.iter().take(3) {
        lines.push(format!("    detail: {detail}"));
    }
    lines
}

fn campaign_report_stop_needs_immediate_intervention_v1(report: &BranchCampaignReportV1) -> bool {
    report.stop_reason == "needs_intervention"
        || (matches!(
            report.stop_reason.as_str(),
            "stuck" | "no_active_branch" | "no_progress"
        ) && report.active.is_empty()
            && report.frozen.is_empty())
}
