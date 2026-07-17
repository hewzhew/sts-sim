use sts_simulator::eval::run_control::{
    combat_search_trace_summaries, CombatSearchTraceSummary, RunControlHpLossLimit,
    RunControlSession, RunControlTraceAnnotationV1, RunProgressOutcome, RunProgressStepV1,
};

use super::accepted_high_loss_diagnostic::{accepted_high_loss_diagnostic, capture_active_combat};
use super::combat_search_report::{
    combat_search_session_report, CombatSearchQuantumReport, CombatSearchSessionReport,
    CombatSearchSessionReportInput,
};
use super::combat_search_session_output::CombatSearchSessionOutput;
use super::combat_search_session_plan::{
    canonical_combat_search_session_plan, CombatSearchSessionPlan,
};
use super::combat_search_session_result::{combat_search_result, CombatSearchSessionResult};
use super::combat_search_survival::owner_audit_hp_loss_limit;
use super::combat_search_trace_actions::complete_search_action_keys;
use super::{boundary_router, Args, BranchStatus};

pub(super) fn run_combat_search_session_step(
    session: &mut RunControlSession,
    args: Args,
) -> Result<CombatSearchSessionResult, String> {
    let plan = canonical_combat_search_session_plan(session, args);
    if plan.should_checkpoint_before_search(args) {
        let status = awaiting_auto_boundary(
            "Combat",
            "checkpoint before canonical combat search session".to_string(),
        );
        let report = session_report(&plan, status.clone(), Vec::new(), None, false, "checkpoint");
        return Ok(combat_search_result(
            status,
            Some(report),
            CombatSearchSessionOutput::default(),
        ));
    }

    let combat_capture = capture_active_combat(session)?;
    let owner_hp_loss_limit = match owner_audit_hp_loss_limit(session) {
        RunControlHpLossLimit::Limit(limit) => Some(limit),
        RunControlHpLossLimit::Unlimited => None,
    };
    let outcome = match session.apply_combat_search(plan.search.clone()) {
        Ok(outcome) => outcome,
        Err(error) => {
            let status = BranchStatus::AdvanceFailed(error);
            let report = session_report(
                &plan,
                status.clone(),
                Vec::new(),
                None,
                false,
                "search_error",
            );
            return Ok(combat_search_result(
                status,
                Some(report),
                CombatSearchSessionOutput::default(),
            ));
        }
    };
    let status = search_status(session, &outcome);
    let action_keys = complete_search_action_keys(&outcome.trace_annotations);
    let applied_steps = committed_progress_steps(&outcome);
    let applied = !applied_steps.is_empty();
    let facts = candidate_facts(session, &outcome.trace_annotations, owner_hp_loss_limit);
    let decision = session_decision(applied, facts.as_ref());

    let mut output = CombatSearchSessionOutput::default();
    output.progress_steps = applied_steps;
    output.combat_search =
        combat_search_summaries(&outcome, &plan, facts.as_ref(), applied, decision);
    if let Some(diagnostic) = combat_capture.and_then(|capture| {
        accepted_high_loss_diagnostic(
            capture,
            "canonical_combat_session",
            &outcome.trace_annotations,
            applied,
            owner_hp_loss_limit,
        )
    }) {
        output.accepted_high_loss_diagnostics.push(diagnostic);
    }

    let report = (!applied).then(|| {
        session_report(
            &plan,
            status.clone(),
            action_keys,
            facts.as_ref(),
            applied,
            decision,
        )
    });
    Ok(combat_search_result(status, report, output))
}

#[derive(Clone, Copy)]
struct SearchCandidateFacts {
    tier: SearchCandidateTier,
    combat_final_hp: i32,
    run_hp: i32,
    potions_used: u32,
    turns: u32,
}

#[derive(Clone, Copy)]
enum SearchCandidateTier {
    RelaxedCompleteWin,
    ReserveCompliantCompleteWin,
}

impl SearchCandidateTier {
    fn as_str(self) -> &'static str {
        match self {
            Self::RelaxedCompleteWin => "relaxed_complete_win",
            Self::ReserveCompliantCompleteWin => "reserve_compliant_complete_win",
        }
    }
}

fn candidate_facts(
    session: &RunControlSession,
    annotations: &[RunControlTraceAnnotationV1],
    owner_hp_loss_limit: Option<u32>,
) -> Option<SearchCandidateFacts> {
    let best_win =
        combat_search_trace_summaries(annotations).find_map(|summary| summary.best_win)?;
    let tier = if owner_hp_loss_limit.is_some_and(|limit| best_win.hp_loss.max(0) as u32 > limit) {
        SearchCandidateTier::RelaxedCompleteWin
    } else {
        SearchCandidateTier::ReserveCompliantCompleteWin
    };
    Some(SearchCandidateFacts {
        tier,
        combat_final_hp: best_win.final_hp,
        run_hp: session.visible_player_hp().0,
        potions_used: best_win.potions_used,
        turns: best_win.turns,
    })
}

fn session_decision(applied: bool, facts: Option<&SearchCandidateFacts>) -> &'static str {
    match (applied, facts.map(|facts| facts.tier)) {
        (true, Some(SearchCandidateTier::ReserveCompliantCompleteWin)) => {
            "accepted_reserve_compliant_candidate"
        }
        (true, Some(SearchCandidateTier::RelaxedCompleteWin)) => "accepted_relaxed_candidate",
        (true, None) => "applied_direct_survival_action",
        (false, Some(_)) => "candidate_rejected_by_typed_acceptance",
        (false, None) => "no_accepted_candidate",
    }
}

fn committed_progress_steps(outcome: &RunProgressOutcome) -> Vec<RunProgressStepV1> {
    outcome
        .progress_steps
        .iter()
        .filter(|step| !matches!(step, RunProgressStepV1::Stop(_)))
        .cloned()
        .collect()
}

fn combat_search_summaries(
    outcome: &RunProgressOutcome,
    plan: &CombatSearchSessionPlan,
    facts: Option<&SearchCandidateFacts>,
    applied: bool,
    decision: &'static str,
) -> Vec<CombatSearchTraceSummary> {
    let mut summaries =
        combat_search_trace_summaries(&outcome.trace_annotations).collect::<Vec<_>>();
    for summary in &mut summaries {
        summary.lane = None;
        summary.profile_id = Some("canonical_combat_session".to_string());
        summary.profile_max_nodes = Some(plan.total_nodes);
        summary.profile_wall_ms = Some(plan.total_wall_ms);
        summary.profile_potion_policy = Some(potion_policy_label(plan.potion_policy).to_string());
        summary.profile_max_potions_used = plan.max_potions_used;
        summary.profile_internal_no_win_rescue_enabled = Some(false);
        summary.engine_fingerprint = Some(plan.semantics_fingerprint.clone());
        summary.portfolio_candidate_tier = facts.map(|facts| facts.tier.as_str().to_string());
        summary.portfolio_selected = Some(applied);
        summary.portfolio_decision = Some(decision.to_string());
    }
    summaries
}

fn session_report(
    plan: &CombatSearchSessionPlan,
    status: BranchStatus,
    action_keys: Vec<String>,
    facts: Option<&SearchCandidateFacts>,
    applied: bool,
    decision: &'static str,
) -> CombatSearchSessionReport {
    combat_search_session_report(CombatSearchSessionReportInput {
        status,
        profile_id: "canonical_combat_session",
        max_nodes: plan.total_nodes,
        wall_ms: plan.total_wall_ms,
        potion_policy: plan.potion_policy,
        max_potions_used: plan.max_potions_used,
        work_quanta: plan
            .search
            .work_quanta
            .iter()
            .map(|quantum| CombatSearchQuantumReport {
                label: quantum.label,
                additional_nodes: quantum.additional_nodes,
                soft_wall_ms: quantum.soft_wall_ms,
            })
            .collect(),
        action_keys,
        semantics_fingerprint: plan.semantics_fingerprint.clone(),
        candidate_tier: facts.map(|facts| facts.tier.as_str().to_string()),
        applied,
        decision: decision.to_string(),
        combat_final_hp: facts.map(|facts| facts.combat_final_hp),
        run_hp: facts.map(|facts| facts.run_hp),
        potions_used: facts.map(|facts| facts.potions_used),
        turns: facts.map(|facts| facts.turns),
    })
}

fn search_status(session: &RunControlSession, outcome: &RunProgressOutcome) -> BranchStatus {
    if let Some(outcome) = boundary_router::terminal_outcome(session) {
        BranchStatus::Terminal(outcome)
    } else {
        boundary_router::classify_auto_outcome(session, outcome)
    }
}

fn potion_policy_label(
    policy: sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy,
) -> &'static str {
    match policy {
        sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy::Never => "never",
        sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy::All => "all",
        sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy::SemanticBudgeted => {
            "semantic"
        }
    }
}

fn awaiting_auto_boundary(boundary: impl Into<String>, reason: String) -> BranchStatus {
    BranchStatus::AwaitingAuto {
        boundary: boundary.into(),
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::eval::run_control::RunProgressOutcome;

    #[test]
    fn stop_records_are_not_misreported_as_committed_search_progress() {
        let outcome = RunProgressOutcome::progress("gap");

        assert!(committed_progress_steps(&outcome).is_empty());
    }

    #[test]
    fn candidate_tier_uses_owner_reserve_without_rejecting_relaxed_win() {
        assert_eq!(
            session_decision(
                true,
                Some(&SearchCandidateFacts {
                    tier: SearchCandidateTier::RelaxedCompleteWin,
                    combat_final_hp: 10,
                    run_hp: 10,
                    potions_used: 0,
                    turns: 5,
                })
            ),
            "accepted_relaxed_candidate"
        );
    }
}
