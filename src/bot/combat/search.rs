use crate::runtime::combat::CombatState;
use crate::state::core::ClientInput;
use crate::state::EngineState;
use std::collections::BTreeMap;
use std::time::Instant;

use super::decision::{
    build_exact_turn_verdict, classify_proposal_class, classify_regime, compare_decision_outcomes,
    frontier_outcome_from_candidate, CombatRegime, DecisionOutcome, ExactnessLevel, ProposalClass,
    ProposalDisposition, ProposalTrace, ScreenRejection, ScreenRejectionKind,
};
use super::equivalence::{reduce_equivalent_inputs, SearchEquivalenceMode};
use super::exact_turn_solver::{solve_exact_turn_with_config, ExactTurnConfig};
use super::legal_moves::get_legal_moves;
use super::ordering::{compare_candidates, end_turn_tiebreak};
use super::planner::plan_candidate;
use super::profile::SearchProfileBreakdown;
use super::terminal::terminal_outcome;
use super::types::CombatCandidate;
use super::value::{compare_values, projected_frontier, CombatValue};

const ROOT_SCREENING_MULTIPLIER: usize = 2;
const ROOT_EXACT_ADJUDICATION_LIMIT: usize = 3;
const ROOT_EXACT_ADJUDICATION_MAX_NODES: usize = 1_200;

#[derive(Clone)]
pub(super) struct ExploredCandidate {
    pub(super) candidate: CombatCandidate,
    pub(super) proposal_class: ProposalClass,
    pub(super) frontier_outcome: DecisionOutcome,
    pub(super) search_value: CombatValue,
    pub(super) exact_outcome: Option<DecisionOutcome>,
    pub(super) exact_confidence: ExactnessLevel,
    pub(super) explored_nodes: u32,
}

pub(super) struct RootExploreResult {
    pub(super) explored: Vec<ExploredCandidate>,
    pub(super) timed_out: bool,
    pub(super) proposal_count: usize,
    pub(super) screened_count: usize,
    pub(super) exact_adjudicated_count: usize,
    pub(super) proposal_class_counts: BTreeMap<String, usize>,
    pub(super) screened_out: Vec<ScreenRejection>,
    pub(super) proposal_trace: Vec<ProposalTrace>,
    pub(super) regime: CombatRegime,
}

#[derive(Clone, Copy)]
struct SearchOutcome {
    value: CombatValue,
    explored_nodes: u32,
    timed_out: bool,
}

#[derive(Clone)]
struct RootProposal {
    candidate: CombatCandidate,
    proposal_class: ProposalClass,
    frontier_outcome: DecisionOutcome,
    exact_outcome: Option<DecisionOutcome>,
    exact_confidence: ExactnessLevel,
}

struct ScreenedRootProposals {
    kept: Vec<RootProposal>,
    rejected: Vec<ScreenRejection>,
    trimmed: Vec<RootProposal>,
}

pub(super) fn explore_root_with_inputs(
    engine: &EngineState,
    combat: &CombatState,
    legal_moves: Vec<ClientInput>,
    max_decision_depth: usize,
    root_width: usize,
    branch_width: usize,
    max_engine_steps: usize,
    root_node_budget: usize,
    deadline: Option<Instant>,
    equivalence_mode: SearchEquivalenceMode,
    profile: &mut SearchProfileBreakdown,
) -> RootExploreResult {
    if legal_moves.is_empty() {
        return RootExploreResult {
            explored: Vec::new(),
            timed_out: false,
            proposal_count: 0,
            screened_count: 0,
            exact_adjudicated_count: 0,
            proposal_class_counts: BTreeMap::new(),
            screened_out: Vec::new(),
            proposal_trace: Vec::new(),
            regime: CombatRegime::Advantage,
        };
    }

    let regime = classify_regime(combat);
    let mut proposals = propose_root_candidates(
        engine,
        combat,
        legal_moves,
        branch_width,
        max_engine_steps,
        deadline,
        equivalence_mode,
        profile,
    );
    let proposal_class_counts = summarize_proposal_classes(&proposals);
    let proposal_count = proposals.len();
    let screened = screen_root_proposals(regime, proposals, combat, root_width.max(1));
    let screened_count = screened.kept.len();
    let screened_out = screened.rejected;
    let trimmed_after_screening = screened.trimmed;
    proposals = screened.kept;
    proposals.sort_by(|left, right| compare_root_proposals(regime, left, right));
    let exact_adjudicated_count = exact_adjudicate_root_proposals(
        engine,
        combat,
        regime,
        &mut proposals,
        max_engine_steps,
        root_node_budget,
        deadline,
    );
    proposals.sort_by(|left, right| compare_root_proposals(regime, left, right));
    let deferred_proposals = proposals
        .iter()
        .skip(root_width.max(1))
        .cloned()
        .collect::<Vec<_>>();

    let mut timed_out = false;
    let mut explored = Vec::new();
    let mut consumed_nodes = 0usize;
    for proposal in proposals.into_iter().take(root_width.max(1)) {
        if consumed_nodes >= root_node_budget && !explored.is_empty() {
            timed_out = true;
            profile.note_timeout_source("root_node_budget");
            break;
        }
        if deadline.is_some_and(|limit| Instant::now() >= limit) && !explored.is_empty() {
            timed_out = true;
            profile.note_timeout_source("wall_clock_deadline");
            break;
        }
        let outcome = evaluate_state(
            &proposal.candidate.frontier_engine,
            &proposal.candidate.frontier_combat,
            max_decision_depth.saturating_sub(1),
            branch_width,
            max_engine_steps,
            root_node_budget.saturating_sub(consumed_nodes),
            deadline,
            equivalence_mode,
            profile,
        );
        timed_out |= outcome.timed_out;
        consumed_nodes = consumed_nodes
            .saturating_add(proposal.candidate.planner_nodes as usize)
            .saturating_add(outcome.explored_nodes as usize);
        explored.push(ExploredCandidate {
            explored_nodes: outcome.explored_nodes + proposal.candidate.planner_nodes,
            candidate: proposal.candidate,
            proposal_class: proposal.proposal_class,
            frontier_outcome: proposal.frontier_outcome,
            search_value: outcome.value,
            exact_outcome: proposal.exact_outcome,
            exact_confidence: proposal.exact_confidence,
        });
    }

    explored.sort_by(|left, right| compare_explored_candidates(regime, left, right));
    let proposal_trace = build_proposal_trace(
        &explored,
        &deferred_proposals,
        &trimmed_after_screening,
        &screened_out,
    );
    RootExploreResult {
        explored,
        timed_out,
        proposal_count,
        screened_count,
        exact_adjudicated_count,
        proposal_class_counts,
        screened_out,
        proposal_trace,
        regime,
    }
}

fn propose_root_candidates(
    engine: &EngineState,
    combat: &CombatState,
    legal_moves: Vec<ClientInput>,
    branch_width: usize,
    max_engine_steps: usize,
    deadline: Option<Instant>,
    equivalence_mode: SearchEquivalenceMode,
    profile: &mut SearchProfileBreakdown,
) -> Vec<RootProposal> {
    let clusters = reduce_equivalent_inputs(combat, legal_moves, equivalence_mode);
    let mut proposals = clusters
        .iter()
        .map(|cluster| {
            let mut candidate = plan_candidate(
                engine,
                combat,
                &cluster.representative,
                branch_width,
                max_engine_steps,
                deadline,
                profile,
            );
            candidate.cluster_size = cluster.collapsed_inputs.len() + 1;
            candidate.collapsed_inputs = cluster.collapsed_inputs.clone();
            let frontier_outcome = frontier_outcome_from_candidate(combat, &candidate);
            RootProposal {
                candidate,
                proposal_class: classify_proposal_class(combat, &cluster.representative),
                frontier_outcome,
                exact_outcome: None,
                exact_confidence: ExactnessLevel::Unavailable,
            }
        })
        .collect::<Vec<_>>();
    proposals.sort_by(|left, right| compare_candidates(&left.candidate, &right.candidate));
    proposals
}

fn screen_root_proposals(
    regime: CombatRegime,
    proposals: Vec<RootProposal>,
    combat: &CombatState,
    root_width: usize,
) -> ScreenedRootProposals {
    if proposals.is_empty() {
        return ScreenedRootProposals {
            kept: proposals,
            rejected: Vec::new(),
            trimmed: Vec::new(),
        };
    }

    let screening_width = match regime {
        CombatRegime::Crisis | CombatRegime::Fragile => {
            root_width.max(1) * (ROOT_SCREENING_MULTIPLIER + 1)
        }
        CombatRegime::Contested | CombatRegime::Advantage => {
            root_width.max(1) * ROOT_SCREENING_MULTIPLIER
        }
    }
    .max(3);
    let any_survivor = proposals.iter().any(|proposal| proposal.candidate.survives);
    let best_unblocked = proposals
        .iter()
        .map(|proposal| proposal.candidate.projected_unblocked)
        .min()
        .unwrap_or(i32::MAX);
    let best_enemy_total = proposals
        .iter()
        .map(|proposal| proposal.candidate.projected_enemy_total)
        .min()
        .unwrap_or(i32::MAX);
    let best_non_endturn_unblocked = proposals
        .iter()
        .filter(|proposal| !matches!(proposal.candidate.input, ClientInput::EndTurn))
        .map(|proposal| proposal.candidate.projected_unblocked)
        .min()
        .unwrap_or(best_unblocked);
    let best_survival = proposals
        .iter()
        .map(|proposal| proposal.frontier_outcome.survival)
        .max()
        .unwrap_or(proposals[0].frontier_outcome.survival);
    let player_hp = combat.entities.player.current_hp.max(1);

    let mut kept = Vec::new();
    let mut overflow: Vec<(RootProposal, ScreenRejectionKind)> = Vec::new();
    for (idx, proposal) in proposals.iter().enumerate() {
        let root_progress_dominated = proposals
            .iter()
            .enumerate()
            .any(|(other_idx, other)| other_idx != idx && root_progress_dominates(other, proposal));
        let root_resource_dominated = proposals
            .iter()
            .enumerate()
            .any(|(other_idx, other)| other_idx != idx && root_resource_dominates(other, proposal));
        let rejection_reason = screen_rejection_reason(
            regime,
            proposal,
            any_survivor,
            root_progress_dominated,
            root_resource_dominated,
            best_unblocked,
            best_non_endturn_unblocked,
            best_enemy_total,
            best_survival,
            player_hp,
        );
        if let Some(reason) = rejection_reason {
            overflow.push((proposal.clone(), reason));
        } else {
            kept.push(proposal.clone());
        }
    }

    let min_keep = root_width.max(1);
    if kept.len() < min_keep {
        let refill_count = min_keep - kept.len();
        let refill = overflow
            .drain(0..overflow.len().min(refill_count))
            .collect::<Vec<_>>();
        kept.extend(refill.into_iter().map(|(proposal, _)| proposal));
    }
    let rejected = overflow
        .into_iter()
        .map(|(proposal, reason)| ScreenRejection {
            input: format!("{:?}", proposal.candidate.input),
            proposal_class: proposal.proposal_class,
            frontier_outcome: proposal.frontier_outcome,
            reason,
        })
        .collect::<Vec<_>>();
    let trimmed = if kept.len() > screening_width {
        kept.split_off(screening_width)
    } else {
        Vec::new()
    };
    ScreenedRootProposals {
        kept,
        rejected,
        trimmed,
    }
}

fn screen_rejection_reason(
    regime: CombatRegime,
    proposal: &RootProposal,
    any_survivor: bool,
    root_progress_dominated: bool,
    root_resource_dominated: bool,
    best_unblocked: i32,
    best_non_endturn_unblocked: i32,
    best_enemy_total: i32,
    best_survival: super::decision::SurvivalJudgement,
    player_hp: i32,
) -> Option<ScreenRejectionKind> {
    if any_survivor && !proposal.candidate.survives {
        return Some(ScreenRejectionKind::UnsurvivableWhileSurvivorExists);
    }
    if root_progress_dominated {
        return Some(ScreenRejectionKind::DominatedRootProgress);
    }
    if root_resource_dominated {
        return Some(ScreenRejectionKind::DominatedRootResources);
    }

    match regime {
        CombatRegime::Crisis => {
            if proposal.candidate.projected_unblocked >= player_hp && best_unblocked < player_hp {
                Some(ScreenRejectionKind::ImmediateLethalWhenSaferExists)
            } else if matches!(proposal.candidate.input, ClientInput::EndTurn)
                && best_non_endturn_unblocked < proposal.candidate.projected_unblocked
            {
                Some(ScreenRejectionKind::EndTurnWorseThanPlayableAlternative)
            } else if proposal.frontier_outcome.survival < best_survival
                && proposal.candidate.projected_unblocked > best_unblocked
            {
                Some(ScreenRejectionKind::DominatedFrontierSurvival)
            } else {
                None
            }
        }
        CombatRegime::Fragile => {
            if proposal.candidate.projected_unblocked > best_unblocked + 6
                && proposal.candidate.projected_enemy_total >= best_enemy_total + 8
            {
                Some(ScreenRejectionKind::FragileRiskOutlier)
            } else if matches!(proposal.candidate.input, ClientInput::EndTurn)
                && best_non_endturn_unblocked + 3 < proposal.candidate.projected_unblocked
            {
                Some(ScreenRejectionKind::EndTurnWorseThanPlayableAlternative)
            } else {
                None
            }
        }
        CombatRegime::Contested | CombatRegime::Advantage => None,
    }
}

fn summarize_proposal_classes(proposals: &[RootProposal]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for proposal in proposals {
        *counts
            .entry(proposal.proposal_class.as_str().to_string())
            .or_insert(0) += 1;
    }
    counts
}

fn build_proposal_trace(
    explored: &[ExploredCandidate],
    deferred: &[RootProposal],
    trimmed: &[RootProposal],
    screened_out: &[ScreenRejection],
) -> Vec<ProposalTrace> {
    let mut trace = Vec::new();
    for (idx, candidate) in explored.iter().enumerate() {
        trace.push(ProposalTrace {
            input: format!("{:?}", candidate.candidate.input),
            proposal_class: candidate.proposal_class,
            disposition: if idx == 0 {
                ProposalDisposition::FrontierChosen
            } else {
                ProposalDisposition::Considered
            },
            frontier_outcome: candidate.frontier_outcome.clone(),
            exact_outcome: candidate.exact_outcome.clone(),
            exact_confidence: candidate.exact_confidence,
            reasons: if idx == 0 {
                Vec::new()
            } else {
                vec!["ranked_below_frontier_after_deeper_search".to_string()]
            },
        });
    }
    for proposal in deferred {
        trace.push(ProposalTrace {
            input: format!("{:?}", proposal.candidate.input),
            proposal_class: proposal.proposal_class,
            disposition: ProposalDisposition::Considered,
            frontier_outcome: proposal.frontier_outcome.clone(),
            exact_outcome: proposal.exact_outcome.clone(),
            exact_confidence: proposal.exact_confidence,
            reasons: vec!["trimmed_before_deeper_search".to_string()],
        });
    }
    for proposal in trimmed {
        trace.push(ProposalTrace {
            input: format!("{:?}", proposal.candidate.input),
            proposal_class: proposal.proposal_class,
            disposition: ProposalDisposition::Considered,
            frontier_outcome: proposal.frontier_outcome.clone(),
            exact_outcome: proposal.exact_outcome.clone(),
            exact_confidence: proposal.exact_confidence,
            reasons: vec![ScreenRejectionKind::TrimmedByScreeningWidth
                .as_str()
                .to_string()],
        });
    }
    for rejection in screened_out {
        trace.push(ProposalTrace {
            input: rejection.input.clone(),
            proposal_class: rejection.proposal_class,
            disposition: ProposalDisposition::ScreenedOut,
            frontier_outcome: rejection.frontier_outcome.clone(),
            exact_outcome: None,
            exact_confidence: ExactnessLevel::Unavailable,
            reasons: vec![rejection.reason.as_str().to_string()],
        });
    }
    trace
}

fn exact_adjudicate_root_proposals(
    engine: &EngineState,
    combat: &CombatState,
    regime: CombatRegime,
    proposals: &mut [RootProposal],
    max_engine_steps: usize,
    root_node_budget: usize,
    deadline: Option<Instant>,
) -> usize {
    if !matches!(regime, CombatRegime::Crisis | CombatRegime::Fragile) {
        return 0;
    }

    let limit = proposals.len().min(ROOT_EXACT_ADJUDICATION_LIMIT.max(1));
    let max_nodes = ROOT_EXACT_ADJUDICATION_MAX_NODES.min(root_node_budget.max(1) * 25);
    let mut adjudicated = 0usize;
    for proposal in proposals.iter_mut().take(limit) {
        if deadline.is_some_and(|limit| Instant::now() >= limit) {
            break;
        }
        let solution = solve_exact_turn_with_config(
            engine,
            combat,
            ExactTurnConfig {
                max_nodes,
                max_engine_steps,
                deadline,
                root_inputs: Some(vec![proposal.candidate.input.clone()]),
            },
        );
        let verdict = build_exact_turn_verdict(
            &proposal.candidate.input,
            &proposal.frontier_outcome,
            &solution,
        );
        proposal.exact_outcome = verdict.best_outcome;
        proposal.exact_confidence = verdict.confidence;
        adjudicated += 1;
    }
    adjudicated
}

fn compare_root_proposals(
    regime: CombatRegime,
    left: &RootProposal,
    right: &RootProposal,
) -> std::cmp::Ordering {
    compare_root_survival(
        left.candidate.survives,
        &left.frontier_outcome,
        right.candidate.survives,
        &right.frontier_outcome,
    )
    .then_with(|| compare_root_progress(left, right))
    .then_with(|| compare_root_resource_dominance(left, right))
    .then_with(|| {
        compare_exact_adjudicated_outcomes(
            regime,
            left.exact_outcome.as_ref(),
            left.exact_confidence,
            right.exact_outcome.as_ref(),
            right.exact_confidence,
        )
    })
    .then_with(|| compare_candidates(&left.candidate, &right.candidate))
}

fn compare_explored_candidates(
    regime: CombatRegime,
    left: &ExploredCandidate,
    right: &ExploredCandidate,
) -> std::cmp::Ordering {
    compare_root_survival(
        left.candidate.survives,
        &left.frontier_outcome,
        right.candidate.survives,
        &right.frontier_outcome,
    )
    .then_with(|| {
        compare_root_progress_facts(
            &left.candidate,
            &left.frontier_outcome,
            &right.candidate,
            &right.frontier_outcome,
        )
    })
    .then_with(|| {
        compare_root_resource_dominance_facts(
            &left.candidate,
            &left.frontier_outcome,
            &right.candidate,
            &right.frontier_outcome,
        )
    })
    .then_with(|| {
        compare_exact_adjudicated_outcomes(
            regime,
            left.exact_outcome.as_ref(),
            left.exact_confidence,
            right.exact_outcome.as_ref(),
            right.exact_confidence,
        )
    })
    .then_with(|| {
        compare_values(&left.search_value, &right.search_value).then_with(|| {
            end_turn_tiebreak(
                &left.candidate.input,
                &right.candidate.input,
                &left.search_value,
            )
        })
    })
}

fn compare_root_survival(
    left_survives: bool,
    left_outcome: &DecisionOutcome,
    right_survives: bool,
    right_outcome: &DecisionOutcome,
) -> std::cmp::Ordering {
    right_survives
        .cmp(&left_survives)
        .then_with(|| right_outcome.survival.cmp(&left_outcome.survival))
}

fn compare_root_progress(left: &RootProposal, right: &RootProposal) -> std::cmp::Ordering {
    compare_root_progress_facts(
        &left.candidate,
        &left.frontier_outcome,
        &right.candidate,
        &right.frontier_outcome,
    )
}

fn compare_root_progress_facts(
    left_candidate: &CombatCandidate,
    left_outcome: &DecisionOutcome,
    right_candidate: &CombatCandidate,
    right_outcome: &DecisionOutcome,
) -> std::cmp::Ordering {
    if root_progress_dominates_facts(left_candidate, left_outcome, right_candidate, right_outcome) {
        std::cmp::Ordering::Less
    } else if root_progress_dominates_facts(
        right_candidate,
        right_outcome,
        left_candidate,
        left_outcome,
    ) {
        std::cmp::Ordering::Greater
    } else {
        std::cmp::Ordering::Equal
    }
}

fn root_progress_dominates(left: &RootProposal, right: &RootProposal) -> bool {
    root_progress_dominates_facts(
        &left.candidate,
        &left.frontier_outcome,
        &right.candidate,
        &right.frontier_outcome,
    )
}

fn root_progress_dominates_facts(
    left_candidate: &CombatCandidate,
    left_outcome: &DecisionOutcome,
    right_candidate: &CombatCandidate,
    right_outcome: &DecisionOutcome,
) -> bool {
    if !root_survival_not_worse(left_candidate, left_outcome, right_candidate, right_outcome) {
        return false;
    }

    let left_clears = root_clears_combat(left_candidate, left_outcome);
    let right_clears = root_clears_combat(right_candidate, right_outcome);
    if left_clears && !right_clears {
        return root_resources_not_worse(left_outcome, right_outcome, true);
    }

    if matches!(right_candidate.input, ClientInput::EndTurn)
        && !matches!(left_candidate.input, ClientInput::EndTurn)
        && !right_clears
    {
        return root_end_turn_progress_dominated(
            left_candidate,
            left_outcome,
            right_candidate,
            right_outcome,
        );
    }

    false
}

fn compare_root_resource_dominance(
    left: &RootProposal,
    right: &RootProposal,
) -> std::cmp::Ordering {
    compare_root_resource_dominance_facts(
        &left.candidate,
        &left.frontier_outcome,
        &right.candidate,
        &right.frontier_outcome,
    )
}

fn compare_root_resource_dominance_facts(
    left_candidate: &CombatCandidate,
    left_outcome: &DecisionOutcome,
    right_candidate: &CombatCandidate,
    right_outcome: &DecisionOutcome,
) -> std::cmp::Ordering {
    if root_resource_dominates_facts(left_candidate, left_outcome, right_candidate, right_outcome) {
        std::cmp::Ordering::Less
    } else if root_resource_dominates_facts(
        right_candidate,
        right_outcome,
        left_candidate,
        left_outcome,
    ) {
        std::cmp::Ordering::Greater
    } else {
        std::cmp::Ordering::Equal
    }
}

fn root_resource_dominates(left: &RootProposal, right: &RootProposal) -> bool {
    root_resource_dominates_facts(
        &left.candidate,
        &left.frontier_outcome,
        &right.candidate,
        &right.frontier_outcome,
    )
}

fn root_resource_dominates_facts(
    left_candidate: &CombatCandidate,
    left_outcome: &DecisionOutcome,
    right_candidate: &CombatCandidate,
    right_outcome: &DecisionOutcome,
) -> bool {
    if left_candidate.survives != right_candidate.survives
        || left_outcome.survival != right_outcome.survival
    {
        return false;
    }
    if left_outcome.position < right_outcome.position
        || left_outcome.terminality < right_outcome.terminality
        || left_candidate.projected_unblocked > right_candidate.projected_unblocked
        || left_candidate.projected_enemy_total > right_candidate.projected_enemy_total
        || left_candidate.projected_hp < right_candidate.projected_hp
        || left_candidate.projected_block < right_candidate.projected_block
    {
        return false;
    }

    let ignore_block_after_clear = root_clears_combat(left_candidate, left_outcome)
        && root_clears_combat(right_candidate, right_outcome);
    root_resources_not_worse(left_outcome, right_outcome, ignore_block_after_clear)
        && root_resources_strictly_better(left_outcome, right_outcome, ignore_block_after_clear)
}

fn root_survival_not_worse(
    left_candidate: &CombatCandidate,
    left_outcome: &DecisionOutcome,
    right_candidate: &CombatCandidate,
    right_outcome: &DecisionOutcome,
) -> bool {
    (!right_candidate.survives || left_candidate.survives)
        && left_outcome.survival >= right_outcome.survival
}

fn root_clears_combat(candidate: &CombatCandidate, outcome: &DecisionOutcome) -> bool {
    matches!(
        outcome.terminality,
        super::decision::TerminalForecast::LethalWin
    ) || candidate.projected_enemy_total <= 0
}

fn root_end_turn_progress_dominated(
    left_candidate: &CombatCandidate,
    left_outcome: &DecisionOutcome,
    right_candidate: &CombatCandidate,
    right_outcome: &DecisionOutcome,
) -> bool {
    root_resources_not_worse(left_outcome, right_outcome, false)
        && left_candidate.projected_unblocked <= right_candidate.projected_unblocked
        && left_candidate.projected_enemy_total <= right_candidate.projected_enemy_total
        && left_candidate.projected_hp >= right_candidate.projected_hp
        && left_candidate.projected_block >= right_candidate.projected_block
        && (left_candidate.projected_enemy_total < right_candidate.projected_enemy_total
            || left_candidate.projected_unblocked < right_candidate.projected_unblocked
            || left_candidate.projected_hp > right_candidate.projected_hp
            || left_candidate.projected_block > right_candidate.projected_block
            || left_outcome.terminality > right_outcome.terminality
            || root_resources_strictly_better(left_outcome, right_outcome, false))
}

fn root_resources_not_worse(
    left: &DecisionOutcome,
    right: &DecisionOutcome,
    ignore_block_after_clear: bool,
) -> bool {
    left.resource_delta.spent_potions <= right.resource_delta.spent_potions
        && left.resource_delta.hp_lost <= right.resource_delta.hp_lost
        && left.resource_delta.exhausted_cards <= right.resource_delta.exhausted_cards
        && left.resource_delta.final_hp >= right.resource_delta.final_hp
        && (ignore_block_after_clear
            || left.resource_delta.final_block >= right.resource_delta.final_block)
}

fn root_resources_strictly_better(
    left: &DecisionOutcome,
    right: &DecisionOutcome,
    ignore_block_after_clear: bool,
) -> bool {
    left.resource_delta.spent_potions < right.resource_delta.spent_potions
        || left.resource_delta.hp_lost < right.resource_delta.hp_lost
        || left.resource_delta.exhausted_cards < right.resource_delta.exhausted_cards
        || left.resource_delta.final_hp > right.resource_delta.final_hp
        || (!ignore_block_after_clear
            && left.resource_delta.final_block > right.resource_delta.final_block)
}

fn compare_exact_adjudicated_outcomes(
    regime: CombatRegime,
    left_outcome: Option<&DecisionOutcome>,
    left_confidence: ExactnessLevel,
    right_outcome: Option<&DecisionOutcome>,
    right_confidence: ExactnessLevel,
) -> std::cmp::Ordering {
    if !matches!(regime, CombatRegime::Crisis | CombatRegime::Fragile) {
        return std::cmp::Ordering::Equal;
    }

    match (left_outcome, right_outcome) {
        (Some(left), Some(right)) => compare_decision_outcomes(right, left)
            .then_with(|| left_confidence.cmp(&right_confidence)),
        (Some(_), None) if left_confidence != ExactnessLevel::Unavailable => {
            std::cmp::Ordering::Less
        }
        (None, Some(_)) if right_confidence != ExactnessLevel::Unavailable => {
            std::cmp::Ordering::Greater
        }
        _ => std::cmp::Ordering::Equal,
    }
}

fn evaluate_state(
    engine: &EngineState,
    combat: &CombatState,
    depth_left: usize,
    branch_width: usize,
    max_engine_steps: usize,
    node_budget: usize,
    deadline: Option<Instant>,
    equivalence_mode: SearchEquivalenceMode,
    profile: &mut SearchProfileBreakdown,
) -> SearchOutcome {
    if deadline.is_some_and(|limit| Instant::now() >= limit) {
        profile.note_timeout_source("wall_clock_deadline");
        return SearchOutcome {
            value: evaluate_projected_value(engine, combat, max_engine_steps, deadline, profile),
            explored_nodes: 1,
            timed_out: true,
        };
    }

    if let Some(outcome) = terminal_outcome(engine, combat) {
        return SearchOutcome {
            value: CombatValue::Terminal(outcome),
            explored_nodes: 1,
            timed_out: false,
        };
    }

    if depth_left == 0 {
        return SearchOutcome {
            value: evaluate_projected_value(engine, combat, max_engine_steps, deadline, profile),
            explored_nodes: 1,
            timed_out: false,
        };
    }

    let legal_moves = get_legal_moves(engine, combat);
    if legal_moves.is_empty() {
        return SearchOutcome {
            value: evaluate_projected_value(engine, combat, max_engine_steps, deadline, profile),
            explored_nodes: 1,
            timed_out: false,
        };
    }

    let clusters = reduce_equivalent_inputs(combat, legal_moves, equivalence_mode);
    let mut candidates = clusters
        .iter()
        .map(|cluster| {
            let mut candidate = {
                plan_candidate(
                    engine,
                    combat,
                    &cluster.representative,
                    branch_width,
                    max_engine_steps,
                    deadline,
                    profile,
                )
            };
            candidate.cluster_size = cluster.collapsed_inputs.len() + 1;
            candidate.collapsed_inputs = cluster.collapsed_inputs.clone();
            candidate
        })
        .collect::<Vec<_>>();
    candidates.sort_by(compare_candidates);

    let mut best: Option<SearchOutcome> = None;
    let mut explored_nodes = 0;
    let mut timed_out = false;
    for candidate in candidates.into_iter().take(branch_width.max(1)) {
        if explored_nodes as usize >= node_budget && best.is_some() {
            timed_out = true;
            profile.note_timeout_source("recursive_node_budget");
            break;
        }
        if deadline.is_some_and(|limit| Instant::now() >= limit) && best.is_some() {
            timed_out = true;
            profile.note_timeout_source("wall_clock_deadline");
            break;
        }
        let child = evaluate_state(
            &candidate.frontier_engine,
            &candidate.frontier_combat,
            depth_left.saturating_sub(1),
            branch_width,
            max_engine_steps,
            node_budget.saturating_sub(explored_nodes as usize),
            deadline,
            equivalence_mode,
            profile,
        );
        explored_nodes += child.explored_nodes + candidate.planner_nodes;
        timed_out |= child.timed_out;
        match best {
            Some(current) if compare_values(&current.value, &child.value).is_le() => {}
            _ => {
                best = Some(child);
            }
        }
    }

    best.unwrap_or(SearchOutcome {
        value: evaluate_projected_value(engine, combat, max_engine_steps, deadline, profile),
        explored_nodes: explored_nodes.max(1),
        timed_out,
    })
}

fn evaluate_projected_value(
    engine: &EngineState,
    combat: &CombatState,
    max_engine_steps: usize,
    deadline: Option<Instant>,
    profile: &mut SearchProfileBreakdown,
) -> CombatValue {
    projected_frontier(engine, combat, max_engine_steps, deadline, profile).2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bot::combat::decision::{
        PositionClass, ResourceDeltaSummary, SurvivalJudgement, TerminalForecast,
    };
    use crate::bot::combat::terminal::{TerminalKind, TerminalOutcome};
    use crate::test_support::blank_test_combat;

    fn decision_outcome(
        survival: SurvivalJudgement,
        position: PositionClass,
        efficiency_score: f32,
    ) -> DecisionOutcome {
        decision_outcome_with_terminality(
            survival,
            position,
            TerminalForecast::SurvivesWindow,
            efficiency_score,
        )
    }

    fn decision_outcome_with_terminality(
        survival: SurvivalJudgement,
        position: PositionClass,
        terminality: TerminalForecast,
        efficiency_score: f32,
    ) -> DecisionOutcome {
        DecisionOutcome {
            survival,
            position,
            terminality,
            resource_delta: ResourceDeltaSummary {
                spent_potions: 0,
                hp_lost: 0,
                exhausted_cards: 0,
                final_hp: 20,
                final_block: 0,
            },
            efficiency_score,
        }
    }

    fn with_resources(
        mut outcome: DecisionOutcome,
        spent_potions: u8,
        hp_lost: i32,
        exhausted_cards: u16,
        final_hp: i32,
        final_block: i32,
    ) -> DecisionOutcome {
        outcome.resource_delta = ResourceDeltaSummary {
            spent_potions,
            hp_lost,
            exhausted_cards,
            final_hp,
            final_block,
        };
        outcome
    }

    fn proposal(
        input: ClientInput,
        survives: bool,
        projected_unblocked: i32,
        projected_enemy_total: i32,
        frontier_outcome: DecisionOutcome,
        exact_outcome: Option<DecisionOutcome>,
    ) -> RootProposal {
        let combat = blank_test_combat();
        let proposal_class = classify_proposal_class(&combat, &input);
        let exact_confidence = if exact_outcome.is_some() {
            ExactnessLevel::Exact
        } else {
            ExactnessLevel::Unavailable
        };
        let projected_hp = if survives {
            frontier_outcome.resource_delta.final_hp
        } else {
            0
        };
        let projected_block = frontier_outcome.resource_delta.final_block;
        RootProposal {
            candidate: CombatCandidate {
                input,
                next_combat: combat.clone(),
                frontier_engine: EngineState::CombatPlayerTurn,
                frontier_combat: combat,
                local_plan: Vec::new(),
                planner_nodes: 1,
                value: CombatValue::Terminal(TerminalOutcome {
                    kind: TerminalKind::Ongoing,
                    final_hp: 20,
                    final_block: 0,
                }),
                projection_truncated: false,
                cluster_size: 1,
                collapsed_inputs: Vec::new(),
                projected_hp,
                projected_block,
                projected_enemy_total,
                projected_unblocked,
                survives,
                diagnostic_score: 0.0,
            },
            proposal_class,
            frontier_outcome,
            exact_outcome,
            exact_confidence,
        }
    }

    fn terminal_defeat_value() -> CombatValue {
        CombatValue::Terminal(TerminalOutcome {
            kind: TerminalKind::Defeat,
            final_hp: 0,
            final_block: 0,
        })
    }

    fn explored_from(proposal: RootProposal, search_value: CombatValue) -> ExploredCandidate {
        ExploredCandidate {
            explored_nodes: 1,
            candidate: proposal.candidate,
            proposal_class: proposal.proposal_class,
            frontier_outcome: proposal.frontier_outcome,
            search_value,
            exact_outcome: proposal.exact_outcome,
            exact_confidence: proposal.exact_confidence,
        }
    }

    #[test]
    fn root_survival_dominance_beats_recursive_defeat_tiebreak_in_all_regimes() {
        let survivor = proposal(
            ClientInput::PlayCard {
                card_index: 2,
                target: None,
            },
            true,
            10,
            47,
            decision_outcome(
                SurvivalJudgement::SevereRisk,
                PositionClass::Collapsing,
                -2.0,
            ),
            None,
        );
        let doomed_end_turn = proposal(
            ClientInput::EndTurn,
            false,
            0,
            51,
            decision_outcome(
                SurvivalJudgement::ForcedLoss,
                PositionClass::Collapsing,
                -20.0,
            ),
            None,
        );

        for regime in [
            CombatRegime::Crisis,
            CombatRegime::Fragile,
            CombatRegime::Contested,
            CombatRegime::Advantage,
        ] {
            assert_eq!(
                compare_root_proposals(regime, &survivor, &doomed_end_turn),
                std::cmp::Ordering::Less,
                "root survival should dominate proposal ordering in {regime:?}"
            );

            let survivor_explored = explored_from(survivor.clone(), terminal_defeat_value());
            let doomed_explored = explored_from(doomed_end_turn.clone(), terminal_defeat_value());
            assert_eq!(
                compare_explored_candidates(regime, &survivor_explored, &doomed_explored),
                std::cmp::Ordering::Less,
                "root survival should dominate recursive defeat tie-breaks in {regime:?}"
            );
        }
    }

    #[test]
    fn root_survival_class_dominates_recursive_search_value() {
        let stable = proposal(
            ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
            true,
            0,
            60,
            decision_outcome(SurvivalJudgement::Stable, PositionClass::DefensiveBind, 0.0),
            None,
        );
        let severe = proposal(
            ClientInput::PlayCard {
                card_index: 1,
                target: None,
            },
            true,
            8,
            40,
            decision_outcome(
                SurvivalJudgement::SevereRisk,
                PositionClass::Collapsing,
                50.0,
            ),
            None,
        );

        let stable_explored = explored_from(stable, terminal_defeat_value());
        let severe_explored = explored_from(
            severe,
            CombatValue::Terminal(TerminalOutcome {
                kind: TerminalKind::CombatCleared,
                final_hp: 1,
                final_block: 0,
            }),
        );

        assert_eq!(
            compare_explored_candidates(
                CombatRegime::Advantage,
                &stable_explored,
                &severe_explored
            ),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn root_lethal_progress_dominates_end_turn_recursive_value_in_all_regimes() {
        let lethal = proposal(
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(0),
            },
            true,
            0,
            0,
            decision_outcome_with_terminality(
                SurvivalJudgement::Safe,
                PositionClass::WinningLine,
                TerminalForecast::LethalWin,
                10.0,
            ),
            None,
        );
        let delaying_end_turn = proposal(
            ClientInput::EndTurn,
            true,
            6,
            1,
            decision_outcome(SurvivalJudgement::Safe, PositionClass::TempoNeutral, 100.0),
            None,
        );

        for regime in [
            CombatRegime::Crisis,
            CombatRegime::Fragile,
            CombatRegime::Contested,
            CombatRegime::Advantage,
        ] {
            assert_eq!(
                compare_root_proposals(regime, &lethal, &delaying_end_turn),
                std::cmp::Ordering::Less,
                "root lethal progress should dominate EndTurn proposal ordering in {regime:?}"
            );

            let lethal_explored = explored_from(lethal.clone(), terminal_defeat_value());
            let end_turn_explored = explored_from(
                delaying_end_turn.clone(),
                CombatValue::Terminal(TerminalOutcome {
                    kind: TerminalKind::CombatCleared,
                    final_hp: 20,
                    final_block: 0,
                }),
            );
            assert_eq!(
                compare_explored_candidates(regime, &lethal_explored, &end_turn_explored),
                std::cmp::Ordering::Less,
                "root lethal progress should dominate recursive search values in {regime:?}"
            );
        }
    }

    #[test]
    fn root_progress_blocks_terminal_defeat_end_turn_tiebreak() {
        let progress = proposal(
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(0),
            },
            true,
            0,
            0,
            decision_outcome_with_terminality(
                SurvivalJudgement::Safe,
                PositionClass::WinningLine,
                TerminalForecast::LethalWin,
                10.0,
            ),
            None,
        );
        let end_turn = proposal(
            ClientInput::EndTurn,
            true,
            0,
            1,
            decision_outcome(SurvivalJudgement::Safe, PositionClass::TempoNeutral, 10.0),
            None,
        );

        let progress_explored = explored_from(progress, terminal_defeat_value());
        let end_turn_explored = explored_from(end_turn, terminal_defeat_value());

        assert_eq!(
            compare_explored_candidates(
                CombatRegime::Advantage,
                &progress_explored,
                &end_turn_explored,
            ),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn same_survival_resource_dominance_beats_recursive_value_in_all_regimes() {
        let resource_better = proposal(
            ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
            true,
            0,
            24,
            with_resources(
                decision_outcome(
                    SurvivalJudgement::Stable,
                    PositionClass::TempoNeutral,
                    -20.0,
                ),
                0,
                0,
                0,
                30,
                8,
            ),
            None,
        );
        let resource_worse = proposal(
            ClientInput::PlayCard {
                card_index: 1,
                target: None,
            },
            true,
            0,
            24,
            with_resources(
                decision_outcome(
                    SurvivalJudgement::Stable,
                    PositionClass::TempoNeutral,
                    100.0,
                ),
                1,
                2,
                0,
                28,
                8,
            ),
            None,
        );

        for regime in [
            CombatRegime::Crisis,
            CombatRegime::Fragile,
            CombatRegime::Contested,
            CombatRegime::Advantage,
        ] {
            assert_eq!(
                compare_root_proposals(regime, &resource_better, &resource_worse),
                std::cmp::Ordering::Less,
                "same-survival root resource dominance should affect proposal ordering in {regime:?}"
            );

            let better_explored = explored_from(resource_better.clone(), terminal_defeat_value());
            let worse_explored = explored_from(
                resource_worse.clone(),
                CombatValue::Terminal(TerminalOutcome {
                    kind: TerminalKind::CombatCleared,
                    final_hp: 99,
                    final_block: 99,
                }),
            );
            assert_eq!(
                compare_explored_candidates(regime, &better_explored, &worse_explored),
                std::cmp::Ordering::Less,
                "same-survival root resource dominance should beat recursive value in {regime:?}"
            );
        }
    }

    #[test]
    fn contested_screening_drops_same_survival_resource_dominated_root() {
        let combat = blank_test_combat();
        let resource_worse = proposal(
            ClientInput::PlayCard {
                card_index: 1,
                target: None,
            },
            true,
            0,
            20,
            with_resources(
                decision_outcome(
                    SurvivalJudgement::Stable,
                    PositionClass::TempoNeutral,
                    100.0,
                ),
                1,
                1,
                0,
                19,
                4,
            ),
            None,
        );
        let resource_better = proposal(
            ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
            true,
            0,
            20,
            with_resources(
                decision_outcome(
                    SurvivalJudgement::Stable,
                    PositionClass::TempoNeutral,
                    -10.0,
                ),
                0,
                0,
                0,
                20,
                4,
            ),
            None,
        );

        let screened = screen_root_proposals(
            CombatRegime::Contested,
            vec![resource_worse, resource_better],
            &combat,
            1,
        );

        assert_eq!(screened.kept.len(), 1);
        assert!(screened.kept.iter().any(|proposal| matches!(
            proposal.candidate.input,
            ClientInput::PlayCard { card_index: 0, .. }
        )));
        assert!(screened.rejected.iter().any(|rejection| matches!(
            rejection.reason,
            ScreenRejectionKind::DominatedRootResources
        )));
    }

    #[test]
    fn advantage_screening_drops_end_turn_when_root_lethal_exists() {
        let combat = blank_test_combat();
        let proposals = vec![
            proposal(
                ClientInput::EndTurn,
                true,
                6,
                1,
                decision_outcome(SurvivalJudgement::Safe, PositionClass::TempoNeutral, 100.0),
                None,
            ),
            proposal(
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(0),
                },
                true,
                0,
                0,
                decision_outcome_with_terminality(
                    SurvivalJudgement::Safe,
                    PositionClass::WinningLine,
                    TerminalForecast::LethalWin,
                    10.0,
                ),
                None,
            ),
        ];

        let screened = screen_root_proposals(CombatRegime::Advantage, proposals, &combat, 1);

        assert_eq!(screened.kept.len(), 1);
        assert!(!screened
            .kept
            .iter()
            .any(|proposal| matches!(proposal.candidate.input, ClientInput::EndTurn)));
        assert!(screened.rejected.iter().any(|rejection| matches!(
            rejection.reason,
            ScreenRejectionKind::DominatedRootProgress
        )));
    }

    #[test]
    fn crisis_root_order_prefers_exact_adjudicated_survival() {
        let better = proposal(
            ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
            true,
            0,
            20,
            decision_outcome(
                SurvivalJudgement::SevereRisk,
                PositionClass::Collapsing,
                10.0,
            ),
            Some(decision_outcome(
                SurvivalJudgement::Stable,
                PositionClass::Stabilizing,
                1.0,
            )),
        );
        let worse = proposal(
            ClientInput::EndTurn,
            true,
            6,
            20,
            decision_outcome(
                SurvivalJudgement::SevereRisk,
                PositionClass::Collapsing,
                20.0,
            ),
            Some(decision_outcome(
                SurvivalJudgement::SevereRisk,
                PositionClass::Collapsing,
                20.0,
            )),
        );

        assert_eq!(
            compare_root_proposals(CombatRegime::Crisis, &better, &worse),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn contested_screening_drops_unsurvivable_root_when_survivor_exists() {
        let combat = blank_test_combat();
        let proposals = vec![
            proposal(
                ClientInput::EndTurn,
                false,
                20,
                12,
                decision_outcome(
                    SurvivalJudgement::ForcedLoss,
                    PositionClass::Collapsing,
                    50.0,
                ),
                None,
            ),
            proposal(
                ClientInput::PlayCard {
                    card_index: 0,
                    target: None,
                },
                true,
                0,
                10,
                decision_outcome(SurvivalJudgement::Stable, PositionClass::Stabilizing, 5.0),
                None,
            ),
        ];

        let screened = screen_root_proposals(CombatRegime::Contested, proposals, &combat, 1);

        assert_eq!(screened.kept.len(), 1);
        assert!(!screened
            .kept
            .iter()
            .any(|proposal| matches!(proposal.candidate.input, ClientInput::EndTurn)));
        assert!(screened.rejected.iter().any(|rejection| matches!(
            rejection.reason,
            ScreenRejectionKind::UnsurvivableWhileSurvivorExists
        )));
    }

    #[test]
    fn crisis_screening_drops_unsurvivable_late_proposals_when_survivor_exists() {
        let combat = blank_test_combat();
        let proposals = vec![
            proposal(
                ClientInput::EndTurn,
                false,
                20,
                12,
                decision_outcome(
                    SurvivalJudgement::ForcedLoss,
                    PositionClass::Collapsing,
                    50.0,
                ),
                None,
            ),
            proposal(
                ClientInput::PlayCard {
                    card_index: 0,
                    target: None,
                },
                true,
                0,
                10,
                decision_outcome(SurvivalJudgement::Stable, PositionClass::Stabilizing, 5.0),
                None,
            ),
            proposal(
                ClientInput::PlayCard {
                    card_index: 1,
                    target: None,
                },
                true,
                2,
                8,
                decision_outcome(
                    SurvivalJudgement::RiskyButPlayable,
                    PositionClass::TempoNeutral,
                    3.0,
                ),
                None,
            ),
        ];

        let screened = screen_root_proposals(CombatRegime::Crisis, proposals, &combat, 1);

        assert_eq!(screened.kept.len(), 1);
        assert!(!screened
            .kept
            .iter()
            .any(|proposal| matches!(proposal.candidate.input, ClientInput::EndTurn)));
        assert!(screened.rejected.iter().any(|rejection| matches!(
            rejection.reason,
            ScreenRejectionKind::UnsurvivableWhileSurvivorExists
                | ScreenRejectionKind::EndTurnWorseThanPlayableAlternative
        )));
    }

    #[test]
    fn exact_confidence_breaks_ties_in_favor_of_exact_over_bounded() {
        let base_outcome =
            decision_outcome(SurvivalJudgement::Stable, PositionClass::TempoNeutral, 4.0);
        let exact = proposal(
            ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
            true,
            0,
            10,
            base_outcome.clone(),
            Some(base_outcome.clone()),
        );
        let mut bounded = proposal(
            ClientInput::PlayCard {
                card_index: 1,
                target: None,
            },
            true,
            0,
            10,
            base_outcome.clone(),
            Some(base_outcome),
        );
        bounded.exact_confidence = ExactnessLevel::Bounded;

        assert_eq!(
            compare_root_proposals(CombatRegime::Crisis, &exact, &bounded),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn proposal_trace_keeps_trimmed_and_screened_candidates() {
        let frontier = ExploredCandidate {
            explored_nodes: 1,
            candidate: CombatCandidate {
                input: ClientInput::PlayCard {
                    card_index: 0,
                    target: None,
                },
                next_combat: blank_test_combat(),
                frontier_engine: EngineState::CombatPlayerTurn,
                frontier_combat: blank_test_combat(),
                local_plan: Vec::new(),
                planner_nodes: 1,
                value: CombatValue::Terminal(TerminalOutcome {
                    kind: TerminalKind::Ongoing,
                    final_hp: 20,
                    final_block: 0,
                }),
                projection_truncated: false,
                cluster_size: 1,
                collapsed_inputs: Vec::new(),
                projected_hp: 20,
                projected_block: 0,
                projected_enemy_total: 10,
                projected_unblocked: 0,
                survives: true,
                diagnostic_score: 0.0,
            },
            proposal_class: ProposalClass::Attack,
            frontier_outcome: decision_outcome(
                SurvivalJudgement::Stable,
                PositionClass::TempoNeutral,
                4.0,
            ),
            search_value: CombatValue::Terminal(TerminalOutcome {
                kind: TerminalKind::Ongoing,
                final_hp: 20,
                final_block: 0,
            }),
            exact_outcome: None,
            exact_confidence: ExactnessLevel::Unavailable,
        };
        let deferred = vec![proposal(
            ClientInput::EndTurn,
            true,
            2,
            12,
            decision_outcome(
                SurvivalJudgement::RiskyButPlayable,
                PositionClass::TempoNeutral,
                1.0,
            ),
            None,
        )];
        let trimmed = vec![proposal(
            ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            },
            true,
            0,
            12,
            decision_outcome(SurvivalJudgement::Stable, PositionClass::Stabilizing, 2.0),
            None,
        )];
        let screened = vec![ScreenRejection {
            input: "EndTurn".to_string(),
            proposal_class: ProposalClass::EndTurn,
            frontier_outcome: decision_outcome(
                SurvivalJudgement::ForcedLoss,
                PositionClass::Collapsing,
                -5.0,
            ),
            reason: ScreenRejectionKind::EndTurnWorseThanPlayableAlternative,
        }];

        let trace = build_proposal_trace(&[frontier], &deferred, &trimmed, &screened);

        assert!(trace.iter().any(|entry| entry
            .reasons
            .iter()
            .any(|reason| reason == "trimmed_before_deeper_search")));
        assert!(trace.iter().any(|entry| entry
            .reasons
            .iter()
            .any(|reason| reason == "trimmed_by_screening_width")));
        assert!(trace.iter().any(|entry| entry
            .reasons
            .iter()
            .any(|reason| reason == "end_turn_worse_than_playable_alternative")));
    }
}
