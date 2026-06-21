use crate::ai::route_planner_v1::RouteSafetyFlagV1;
use crate::eval::branch_experiment::{
    BranchExperimentRouteCandidatePoolV1, BranchExperimentRouteDecisionV1,
};

use super::model::{BranchCampaignRouteEvidenceExampleV1, BranchCampaignRouteEvidenceSummaryV1};

pub(super) fn merge_campaign_route_decisions_v1(
    summary: &mut BranchCampaignRouteEvidenceSummaryV1,
    decisions: &[BranchExperimentRouteDecisionV1],
) {
    for decision in decisions {
        add_campaign_route_decision_v1(summary, decision);
    }
}

pub(super) fn merge_campaign_route_candidate_pools_v1(
    summary: &mut BranchCampaignRouteEvidenceSummaryV1,
    pools: &[BranchExperimentRouteCandidatePoolV1],
) {
    for pool in pools {
        add_campaign_route_candidate_pool_v1(summary, pool);
    }
}

pub(super) fn merge_campaign_route_evidence_summary_v1(
    target: &mut BranchCampaignRouteEvidenceSummaryV1,
    incoming: BranchCampaignRouteEvidenceSummaryV1,
) {
    if incoming.decisions == 0 {
        return;
    }
    target.avg_elite_prep_bp = weighted_average_bp(
        target.avg_elite_prep_bp,
        target.decisions,
        incoming.avg_elite_prep_bp,
        incoming.decisions,
    );
    target.decisions = target.decisions.saturating_add(incoming.decisions);
    target.candidate_pools = target
        .candidate_pools
        .saturating_add(incoming.candidate_pools);
    target.candidate_pool_candidates = target
        .candidate_pool_candidates
        .saturating_add(incoming.candidate_pool_candidates);
    target.candidate_pool_ok = target
        .candidate_pool_ok
        .saturating_add(incoming.candidate_pool_ok);
    target.candidate_pool_risky = target
        .candidate_pool_risky
        .saturating_add(incoming.candidate_pool_risky);
    target.candidate_pool_rejected = target
        .candidate_pool_rejected
        .saturating_add(incoming.candidate_pool_rejected);
    target.complete_candidate_pools = target
        .complete_candidate_pools
        .saturating_add(incoming.complete_candidate_pools);
    target.first_elite_forced = target
        .first_elite_forced
        .saturating_add(incoming.first_elite_forced);
    target.first_elite_optional = target
        .first_elite_optional
        .saturating_add(incoming.first_elite_optional);
    target.first_elite_none = target
        .first_elite_none
        .saturating_add(incoming.first_elite_none);
    target.rest_bailout = target.rest_bailout.saturating_add(incoming.rest_bailout);
    target.shop_bailout = target.shop_bailout.saturating_add(incoming.shop_bailout);
    target.underprepared_first_elite = target
        .underprepared_first_elite
        .saturating_add(incoming.underprepared_first_elite);
    for example in incoming.examples {
        if target.examples.len() >= 4 {
            break;
        }
        target.examples.push(example);
    }
    for example in incoming.underprepared_examples {
        if target.underprepared_examples.len() >= 4 {
            break;
        }
        target.underprepared_examples.push(example);
    }
}

fn add_campaign_route_candidate_pool_v1(
    summary: &mut BranchCampaignRouteEvidenceSummaryV1,
    pool: &BranchExperimentRouteCandidatePoolV1,
) {
    summary.candidate_pools = summary.candidate_pools.saturating_add(1);
    summary.candidate_pool_candidates = summary
        .candidate_pool_candidates
        .saturating_add(pool.candidates.len());
    if pool
        .candidate_pool_provenance
        .as_ref()
        .is_some_and(|provenance| provenance.complete_legal_pool)
    {
        summary.complete_candidate_pools = summary.complete_candidate_pools.saturating_add(1);
    }
    for candidate in &pool.candidates {
        match candidate.resolved_safety_flag() {
            RouteSafetyFlagV1::Ok => {
                summary.candidate_pool_ok = summary.candidate_pool_ok.saturating_add(1)
            }
            RouteSafetyFlagV1::RiskyButAllowed => {
                summary.candidate_pool_risky = summary.candidate_pool_risky.saturating_add(1)
            }
            RouteSafetyFlagV1::RejectUnlessNoAlternative => {
                summary.candidate_pool_rejected = summary.candidate_pool_rejected.saturating_add(1)
            }
        }
    }
}

fn add_campaign_route_decision_v1(
    summary: &mut BranchCampaignRouteEvidenceSummaryV1,
    decision: &BranchExperimentRouteDecisionV1,
) {
    summary.avg_elite_prep_bp = weighted_average_bp(
        summary.avg_elite_prep_bp,
        summary.decisions,
        decision.elite_prep_bp,
        1,
    );
    summary.decisions = summary.decisions.saturating_add(1);
    if decision.first_elite.paths_with_first_elite == 0 {
        summary.first_elite_none = summary.first_elite_none.saturating_add(1);
    } else if decision.first_elite.forced {
        summary.first_elite_forced = summary.first_elite_forced.saturating_add(1);
    } else if decision.first_elite.optional {
        summary.first_elite_optional = summary.first_elite_optional.saturating_add(1);
    }
    if decision.first_elite.can_bail_to_rest_before {
        summary.rest_bailout = summary.rest_bailout.saturating_add(1);
    }
    if decision.first_elite.can_bail_to_shop_before {
        summary.shop_bailout = summary.shop_bailout.saturating_add(1);
    }
    if route_decision_has_underprepared_first_elite_v1(decision) {
        summary.underprepared_first_elite = summary.underprepared_first_elite.saturating_add(1);
    }
    if summary.examples.len() < 4 {
        summary.examples.push(BranchCampaignRouteEvidenceExampleV1 {
            target: decision.target.clone(),
            first_elite: render_branch_campaign_first_elite_evidence_v1(decision),
            elite_prep_bp: decision.elite_prep_bp,
        });
    }
    if route_decision_has_underprepared_first_elite_v1(decision)
        && summary.underprepared_examples.len() < 4
    {
        summary
            .underprepared_examples
            .push(BranchCampaignRouteEvidenceExampleV1 {
                target: decision.target.clone(),
                first_elite: render_branch_campaign_first_elite_evidence_v1(decision),
                elite_prep_bp: decision.elite_prep_bp,
            });
    }
}

fn weighted_average_bp(
    left_avg: i32,
    left_count: usize,
    right_avg: i32,
    right_count: usize,
) -> i32 {
    let total_count = left_count.saturating_add(right_count);
    if total_count == 0 {
        return 0;
    }
    let total = i64::from(left_avg) * left_count as i64 + i64::from(right_avg) * right_count as i64;
    (total / total_count as i64) as i32
}

fn route_decision_has_underprepared_first_elite_v1(
    decision: &BranchExperimentRouteDecisionV1,
) -> bool {
    decision.first_elite.paths_with_first_elite > 0
        && decision.first_elite.forced
        && !decision.first_elite.can_bail_to_rest_before
        && !decision.first_elite.can_bail_to_shop_before
        && decision.first_elite.max_hallway_fights_before < 2
}

fn render_branch_campaign_first_elite_evidence_v1(
    decision: &BranchExperimentRouteDecisionV1,
) -> String {
    let first_elite = &decision.first_elite;
    if first_elite.paths_with_first_elite == 0 {
        return "none".to_string();
    }
    let kind = if first_elite.forced {
        "forced"
    } else if first_elite.optional {
        "optional"
    } else {
        "present"
    };
    format!(
        "{kind} hallways={}-{} fires={}-{} shops={}-{} rest_bailout={} shop_bailout={}",
        first_elite.min_hallway_fights_before,
        first_elite.max_hallway_fights_before,
        first_elite.min_fires_before,
        first_elite.max_fires_before,
        first_elite.min_shops_before,
        first_elite.max_shops_before,
        first_elite.can_bail_to_rest_before,
        first_elite.can_bail_to_shop_before
    )
}
