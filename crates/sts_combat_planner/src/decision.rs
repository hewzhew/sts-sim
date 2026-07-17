use super::{
    CombatDecisionRootError, CombatPlannerAgendaSession, CompleteTurnOption,
    CompleteTurnOptionBoundary, ContinuationEvidence, ContinuationInterruption, OptionProspect,
    OptionProspectId, ReplayError, TurnOptionGenerationGap,
};
use sts_core::state::core::ClientInput;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Exact assumptions shared by every prospect in one comparison.
pub struct CombatEvaluationContext {
    pub oracle_exact_state: bool,
    pub continuation_turn_boundaries: u16,
    pub contract_fingerprint: &'static str,
}

impl CombatEvaluationContext {
    pub const ORACLE_EXACT_ONE_TURN: Self = Self {
        oracle_exact_state: true,
        continuation_turn_boundaries: 1,
        contract_fingerprint:
            "oracle-exact-complete-turn-option/one-turn/observed-resource-pareto-v1",
    };
}

#[derive(Clone, Debug, PartialEq)]
/// A decision is either auditable and executable or explicitly deferred.
pub enum CombatPlannerDecisionResult {
    Selected(CombatPlannerDecision),
    Deferred(CombatPlannerDecisionDeferral),
}

#[derive(Clone, Debug, PartialEq)]
/// One selected complete option plus the evidence contract that selected it.
pub struct CombatPlannerDecision {
    pub root_exact_state_hash: String,
    pub evaluation_context: CombatEvaluationContext,
    pub selected_prospect_id: OptionProspectId,
    pub selected_option: CompleteTurnOption,
    pub nondominated_alternatives: Vec<OptionProspectId>,
    pub unresolved_gaps: Vec<CombatPlannerDecisionGap>,
    pub basis: CombatPlannerDecisionBasis,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// No generic scalar score is hidden inside these selection bases.
pub enum CombatPlannerDecisionBasis {
    OnlyCompleteOption,
    VerifiedTerminalWin,
    PreferredExactWinningHorizon {
        turn_boundaries: u16,
    },
    EquivalentExactSuccessor {
        exact_successor_hash: String,
    },
    BudgetBoundedIncumbent {
        evaluator: CombatPlannerIncumbentEvaluator,
        exact_winning_horizon: Option<u16>,
        considered_prospects: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CombatPlannerIncumbentEvaluator {
    ObservedResourceParetoV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Evidence retained when this comparison contract cannot select safely.
pub struct CombatPlannerDecisionDeferral {
    pub root_exact_state_hash: String,
    pub evaluation_context: CombatEvaluationContext,
    pub nondominated_prospects: Vec<OptionProspectId>,
    pub gaps: Vec<CombatPlannerDecisionGap>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CombatPlannerDecisionGap {
    NoCompleteOptions,
    RetainedAgendaWork {
        items: usize,
    },
    RootGeneration(Vec<TurnOptionGenerationGap>),
    ProspectEvidence {
        prospect_id: OptionProspectId,
        gap: ProspectEvidenceGap,
    },
    UnresolvedBoundaryPreference {
        prospect_id: OptionProspectId,
        boundary: CompleteTurnOptionBoundary,
    },
    IncomparableExactProspects,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProspectEvidenceGap {
    PendingBoundaryVerification,
    PendingContinuationRefinement,
    Interrupted(ContinuationInterruption),
    ContinuationGeneration(Vec<TurnOptionGenerationGap>),
    ContinuationConstruction(CombatDecisionRootError),
    BoundaryVerification(ReplayError),
}

/// Applies the first finite-horizon Oracle comparison contract.
///
/// Exact evidence selects directly when it proves a unique result. At a hard
/// budget boundary, already-complete non-losing options remain executable: a
/// named Pareto incumbent evaluator may choose one while retaining every gap
/// and unresolved alternative. Partial action prefixes never enter comparison.
pub fn decide_combat_option(session: &CombatPlannerAgendaSession) -> CombatPlannerDecisionResult {
    let root_exact_state_hash = session.root().exact_state_hash().to_owned();
    let evaluation_context = CombatEvaluationContext::ORACLE_EXACT_ONE_TURN;
    let prospects = session.prospects();
    let all_ids = prospects.iter().map(OptionProspect::id).collect::<Vec<_>>();
    let mut gaps = Vec::new();

    if session.retained_agenda_items() != 0 {
        gaps.push(CombatPlannerDecisionGap::RetainedAgendaWork {
            items: session.retained_agenda_items(),
        });
    }
    if !session.root_generation_gaps().is_empty() {
        gaps.push(CombatPlannerDecisionGap::RootGeneration(
            session.root_generation_gaps().to_vec(),
        ));
    }
    for prospect in prospects {
        let gap = match prospect.continuation() {
            ContinuationEvidence::PendingBoundaryVerification => {
                Some(ProspectEvidenceGap::PendingBoundaryVerification)
            }
            ContinuationEvidence::PendingContinuationRefinement => {
                Some(ProspectEvidenceGap::PendingContinuationRefinement)
            }
            ContinuationEvidence::ExactHorizonGenerationGap(evidence) => Some(
                ProspectEvidenceGap::ContinuationGeneration(evidence.gaps.clone()),
            ),
            ContinuationEvidence::Interrupted(cause) => {
                Some(ProspectEvidenceGap::Interrupted(*cause))
            }
            ContinuationEvidence::ConstructionFailed(error) => {
                Some(ProspectEvidenceGap::ContinuationConstruction(*error))
            }
            ContinuationEvidence::VerificationFailed(error) => {
                Some(ProspectEvidenceGap::BoundaryVerification(error.clone()))
            }
            ContinuationEvidence::VerifiedBoundary(_) | ContinuationEvidence::ExactHorizon(_) => {
                None
            }
        };
        if let Some(gap) = gap {
            gaps.push(CombatPlannerDecisionGap::ProspectEvidence {
                prospect_id: prospect.id(),
                gap,
            });
        }
    }
    if prospects.is_empty() {
        gaps.push(CombatPlannerDecisionGap::NoCompleteOptions);
        return CombatPlannerDecisionResult::Deferred(CombatPlannerDecisionDeferral {
            root_exact_state_hash,
            evaluation_context,
            nondominated_prospects: all_ids,
            gaps,
        });
    }

    if !gaps.is_empty() {
        return budget_bounded_result(session, gaps, evaluation_context);
    }

    let nondominated = nondominated_indices(prospects);
    if nondominated.len() == 1 {
        let index = nondominated[0];
        let selected = &prospects[index];
        let basis = if prospects.len() == 1 {
            match selected.option().boundary() {
                CompleteTurnOptionBoundary::NextPlayerTurn => {
                    CombatPlannerDecisionBasis::OnlyCompleteOption
                }
                CompleteTurnOptionBoundary::TerminalWin => {
                    CombatPlannerDecisionBasis::VerifiedTerminalWin
                }
                boundary @ (CompleteTurnOptionBoundary::TerminalLoss
                | CompleteTurnOptionBoundary::Escape) => {
                    return CombatPlannerDecisionResult::Deferred(CombatPlannerDecisionDeferral {
                        root_exact_state_hash,
                        evaluation_context,
                        nondominated_prospects: vec![selected.id()],
                        gaps: vec![CombatPlannerDecisionGap::UnresolvedBoundaryPreference {
                            prospect_id: selected.id(),
                            boundary,
                        }],
                    });
                }
            }
        } else {
            match exact_winning_horizon(selected) {
                Some(0) => CombatPlannerDecisionBasis::VerifiedTerminalWin,
                Some(turn_boundaries) => {
                    CombatPlannerDecisionBasis::PreferredExactWinningHorizon { turn_boundaries }
                }
                None => unreachable!("only a shorter exact winning horizon can dominate"),
            }
        };
        return selected_result(
            session,
            selected,
            Vec::new(),
            Vec::new(),
            basis,
            evaluation_context,
        );
    }

    let first_successor = prospects[nondominated[0]].option().exact_successor_hash();
    if nondominated
        .iter()
        .all(|index| prospects[*index].option().exact_successor_hash() == first_successor)
    {
        let selected_index = nondominated[0];
        let selected = &prospects[selected_index];
        let alternatives = nondominated
            .iter()
            .skip(1)
            .map(|index| prospects[*index].id())
            .collect();
        return selected_result(
            session,
            selected,
            alternatives,
            Vec::new(),
            CombatPlannerDecisionBasis::EquivalentExactSuccessor {
                exact_successor_hash: first_successor.to_owned(),
            },
            evaluation_context,
        );
    }

    budget_bounded_result(
        session,
        vec![CombatPlannerDecisionGap::IncomparableExactProspects],
        evaluation_context,
    )
}

fn selected_result(
    session: &CombatPlannerAgendaSession,
    selected: &OptionProspect,
    nondominated_alternatives: Vec<OptionProspectId>,
    unresolved_gaps: Vec<CombatPlannerDecisionGap>,
    basis: CombatPlannerDecisionBasis,
    evaluation_context: CombatEvaluationContext,
) -> CombatPlannerDecisionResult {
    CombatPlannerDecisionResult::Selected(CombatPlannerDecision {
        root_exact_state_hash: session.root().exact_state_hash().to_owned(),
        evaluation_context,
        selected_prospect_id: selected.id(),
        selected_option: selected.option().clone(),
        nondominated_alternatives,
        unresolved_gaps,
        basis,
    })
}

fn budget_bounded_result(
    session: &CombatPlannerAgendaSession,
    mut gaps: Vec<CombatPlannerDecisionGap>,
    evaluation_context: CombatEvaluationContext,
) -> CombatPlannerDecisionResult {
    let prospects = session.prospects();
    let best_winning_horizon = prospects.iter().filter_map(exact_winning_horizon).min();
    let candidates = prospects
        .iter()
        .enumerate()
        .filter(|(_, prospect)| match best_winning_horizon {
            Some(horizon) => exact_winning_horizon(prospect) == Some(horizon),
            None => prospect.option().boundary() == CompleteTurnOptionBoundary::NextPlayerTurn,
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();

    if candidates.is_empty() {
        return CombatPlannerDecisionResult::Deferred(CombatPlannerDecisionDeferral {
            root_exact_state_hash: session.root().exact_state_hash().to_owned(),
            evaluation_context,
            nondominated_prospects: prospects.iter().map(OptionProspect::id).collect(),
            gaps,
        });
    }

    let selected_index = candidates
        .iter()
        .copied()
        .reduce(|incumbent, challenger| {
            if observed_resource_pareto_prefers(&prospects[challenger], &prospects[incumbent]) {
                challenger
            } else {
                incumbent
            }
        })
        .expect("non-empty budget-bounded candidates");
    let alternatives = candidates
        .iter()
        .copied()
        .filter(|index| *index != selected_index)
        .map(|index| prospects[index].id())
        .collect::<Vec<_>>();
    if !alternatives.is_empty()
        && !gaps.contains(&CombatPlannerDecisionGap::IncomparableExactProspects)
    {
        gaps.push(CombatPlannerDecisionGap::IncomparableExactProspects);
    }
    selected_result(
        session,
        &prospects[selected_index],
        alternatives,
        gaps,
        CombatPlannerDecisionBasis::BudgetBoundedIncumbent {
            evaluator: CombatPlannerIncumbentEvaluator::ObservedResourceParetoV1,
            exact_winning_horizon: best_winning_horizon,
            considered_prospects: candidates.len(),
        },
        evaluation_context,
    )
}

fn observed_resource_pareto_prefers(left: &OptionProspect, right: &OptionProspect) -> bool {
    let left_immediate = left.immediate();
    let right_immediate = right.immediate();
    let no_worse = left_immediate.player_hp.after >= right_immediate.player_hp.after
        && left_immediate.player_block.after >= right_immediate.player_block.after
        && left_immediate.gold.after >= right_immediate.gold.after
        && left_immediate.living_enemies.after <= right_immediate.living_enemies.after
        && left_immediate.total_enemy_hp.after <= right_immediate.total_enemy_hp.after
        && left_immediate.occupied_potion_slots.after
            >= right_immediate.occupied_potion_slots.after
        && left_immediate.relic_count.after >= right_immediate.relic_count.after
        && played_cards(left) >= played_cards(right);
    let strictly_better = left_immediate.player_hp.after > right_immediate.player_hp.after
        || left_immediate.player_block.after > right_immediate.player_block.after
        || left_immediate.gold.after > right_immediate.gold.after
        || left_immediate.living_enemies.after < right_immediate.living_enemies.after
        || left_immediate.total_enemy_hp.after < right_immediate.total_enemy_hp.after
        || left_immediate.occupied_potion_slots.after > right_immediate.occupied_potion_slots.after
        || left_immediate.relic_count.after > right_immediate.relic_count.after
        || played_cards(left) > played_cards(right);
    no_worse && strictly_better
}

fn played_cards(prospect: &OptionProspect) -> usize {
    prospect
        .option()
        .actions()
        .iter()
        .filter(|action| matches!(action.input, ClientInput::PlayCard { .. }))
        .count()
}

fn nondominated_indices(prospects: &[OptionProspect]) -> Vec<usize> {
    let mut dominated = vec![false; prospects.len()];
    for left in 0..prospects.len() {
        for right in 0..prospects.len() {
            if left != right
                && compare_exact_prospects(&prospects[left], &prospects[right])
                    == ExactProspectComparison::PreferLeft
            {
                dominated[right] = true;
            }
        }
    }
    dominated
        .iter()
        .enumerate()
        .filter_map(|(index, dominated)| (!dominated).then_some(index))
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExactProspectComparison {
    PreferLeft,
    PreferRight,
    Equivalent,
    Incomparable,
}

fn compare_exact_prospects(
    left: &OptionProspect,
    right: &OptionProspect,
) -> ExactProspectComparison {
    let left_horizon = exact_winning_horizon(left);
    let right_horizon = exact_winning_horizon(right);
    match (left_horizon, right_horizon) {
        (Some(left), Some(right)) if left < right => ExactProspectComparison::PreferLeft,
        (Some(left), Some(right)) if right < left => ExactProspectComparison::PreferRight,
        (Some(_), None) => ExactProspectComparison::PreferLeft,
        (None, Some(_)) => ExactProspectComparison::PreferRight,
        _ if left.option().exact_successor_hash() == right.option().exact_successor_hash() => {
            ExactProspectComparison::Equivalent
        }
        _ => ExactProspectComparison::Incomparable,
    }
}

fn exact_winning_horizon(prospect: &OptionProspect) -> Option<u16> {
    match prospect.continuation() {
        ContinuationEvidence::VerifiedBoundary(evidence)
            if evidence.boundary == CompleteTurnOptionBoundary::TerminalWin =>
        {
            Some(0)
        }
        ContinuationEvidence::ExactHorizon(evidence)
            if evidence
                .complete_options
                .iter()
                .any(|option| option.boundary() == CompleteTurnOptionBoundary::TerminalWin) =>
        {
            Some(evidence.turn_boundaries)
        }
        _ => None,
    }
}
