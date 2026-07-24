use sts_simulator::ai::boss_mechanics_v1::boss_mechanic_pressure_profile_v1;
use sts_simulator::ai::card_component_signal_v1::{
    evaluate_card_component_signals_v1, CardComponentSignalContextV1, CardComponentSignalKindV1,
};
use sts_simulator::ai::card_reward_policy_v1::{
    card_reward_semantic_profile_v1, CardRewardSemanticRoleV1,
};
use sts_simulator::ai::deck_startup_profile_v1::{
    deck_startup_profile_v1, startup_liability_for_candidate_v1, startup_support_for_candidate_v1,
};
use sts_simulator::ai::noncombat_strategy_v1::{
    build_run_strategy_snapshot_from_run_state_v2, threat_coverage_after_card_v1,
    threat_relevant_capability_improvements_v1, StrategyCapabilityCoverageV1,
    StrategyCapabilityKindV1, StrategyThreatCoverageLedgerV1, StrategyThreatSourceV1,
};
use sts_simulator::ai::strategy::decision_pipeline::{
    candidate_lane_rank, CandidateLane, DecisionCandidateKind, DecisionPipelineContext,
};
use sts_simulator::ai::strategy::deck_plan::DeckPlanSnapshot;
use sts_simulator::ai::strategy::reward_admission::{
    assess_reward_admission_from_master_deck, reward_admission_order_key_v1, skip_reward_admission,
    RewardAdmission, RewardAdmissionClass, RewardAdmissionOrderKeyV1, RewardAdmissionReason,
};
use sts_simulator::eval::run_control::{
    CardRewardFunctionV1, CardRewardObligationDeltaV1, CardRewardObligationSourceV1,
    CardRewardOwnerProvenanceV1, DecisionCandidateKey, DecisionSurface, RunControlSession,
};
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::state::rewards::RewardCard;
use sts_simulator::state::run::RunState;

use super::candidate_ir_adapter::{card_reward_kind, is_card_reward_key};
use super::expansion_policy::expansion_from_evaluation;
use super::owner_candidate_eval::candidate_annotation;
use super::owner_commands::executable_choices;
use super::owner_model::{ChoiceAnnotation, OwnerChoice, OwnerChoiceExpansion};
use super::shop_route_evidence::forced_future_elite_distance;

const MAX_OBLIGATION_DEADLINE_NODES: usize = 16;

/// Residual strategic obligations ordered by their real route deadline.
/// Index zero is the current boundary; smaller indices are compared first.
/// This keeps an unavoidable elite before a farther boss while keeping both
/// ahead of optional encounter-pool coverage.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
struct StrategicObligationOrderKeyV1 {
    unavoidable_gaps_by_deadline: [usize; MAX_OBLIGATION_DEADLINE_NODES + 1],
    possible_pool_gaps: usize,
}

#[derive(Clone, Debug)]
struct CardRewardFunctionalEvidenceV1 {
    functions: Vec<CardRewardFunctionV1>,
    hard_liability: Option<&'static str>,
    component_debt_count: usize,
    access_saturated: bool,
}

impl CardRewardFunctionalEvidenceV1 {
    fn has(&self, function: CardRewardFunctionV1) -> bool {
        self.functions.contains(&function)
    }

    fn has_unburdened_independent_value(&self) -> bool {
        self.hard_liability.is_none()
            && !self.access_saturated
            && (self.has(CardRewardFunctionV1::Access)
                || self.has(CardRewardFunctionV1::Amplifier)
                || self.has(CardRewardFunctionV1::Recovery))
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
struct CapabilityResidualOrderKeyV1 {
    missing_capabilities: usize,
    thin_capabilities: usize,
    supported_capabilities: usize,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CardRewardProductionOrderKeyV1 {
    surface_rank: u8,
    lane_rank: u8,
    unavoidable_gaps_by_deadline: [usize; MAX_OBLIGATION_DEADLINE_NODES + 1],
    possible_pool_gaps: usize,
    residual: CapabilityResidualOrderKeyV1,
    hard_startup_liability: bool,
    component_debt_count: usize,
    access_saturated: bool,
    lacks_independent_function: bool,
    legacy_score_tiebreak_rank: i32,
    candidate_tiebreak_rank: u8,
}

pub(super) fn card_reward_owner_choices(
    session: &RunControlSession,
    surface: &DecisionSurface,
) -> Vec<OwnerChoice> {
    let post_boss_reward = is_post_boss_card_reward(&session.run_state);
    let deck_plan =
        DeckPlanSnapshot::from_run_state(&session.run_state).with_boss_key(if post_boss_reward {
            None
        } else {
            session.run_state.boss_key
        });
    let context = DecisionPipelineContext::reward(deck_plan);
    let strategy_run_state = card_reward_strategy_run_state(&session.run_state);
    let strategy = build_run_strategy_snapshot_from_run_state_v2(&strategy_run_state);
    let mut choices = executable_choices(surface)
        .into_iter()
        .filter(|choice| is_card_reward_key(&choice.key))
        .map(|mut choice| {
            choice.annotation = reward_annotation_for_choice(session, &choice, context);
            choice
        })
        .enumerate()
        .collect::<Vec<_>>();
    let has_mainline_take = choices
        .iter()
        .any(|(_, choice)| is_mainline_card_reward_take(session, &strategy, choice));
    for (_, choice) in &mut choices {
        choice.expansion = card_reward_choice_expansion(session, &strategy, choice);
    }
    choices.sort_by_key(|(index, choice)| {
        (
            card_reward_choice_rank(session, &strategy, choice, has_mainline_take),
            *index,
        )
    });
    let rank_keys = choices
        .iter()
        .map(|(_, choice)| card_reward_choice_rank(session, &strategy, choice, has_mainline_take))
        .collect::<Vec<_>>();
    for (sorted_index, ((surface_index, choice), rank_key)) in
        choices.iter_mut().zip(rank_keys.iter()).enumerate()
    {
        let tie_break_applied = rank_keys.iter().filter(|other| *other == rank_key).count() > 1;
        attach_card_reward_provenance(
            session,
            &strategy,
            choice,
            *surface_index,
            sorted_index + 1,
            tie_break_applied,
        );
    }
    choices.into_iter().map(|(_, choice)| choice).collect()
}

fn reward_annotation_for_choice(
    session: &RunControlSession,
    choice: &OwnerChoice,
    context: DecisionPipelineContext,
) -> ChoiceAnnotation {
    match card_reward_kind(&choice.key) {
        Some(DecisionCandidateKind::CardRewardPick { card, upgrades }) => {
            let deck = &session.run_state.master_deck;
            candidate_annotation(
                context,
                DecisionCandidateKind::CardRewardPick { card, upgrades },
                Some(assess_reward_admission_from_master_deck(
                    deck, card, upgrades,
                )),
            )
        }
        Some(DecisionCandidateKind::CardRewardSkip) => candidate_annotation(
            context,
            DecisionCandidateKind::CardRewardSkip,
            Some(skip_reward_admission()),
        ),
        _ => ChoiceAnnotation::None,
    }
}

fn card_reward_choice_expansion(
    session: &RunControlSession,
    strategy: &sts_simulator::ai::noncombat_strategy_v1::RunStrategySnapshotV2,
    choice: &OwnerChoice,
) -> OwnerChoiceExpansion {
    match &choice.key {
        Some(DecisionCandidateKey::CardRewardOpen { .. })
        | Some(DecisionCandidateKey::CardRewardSingingBowl { .. })
        | Some(DecisionCandidateKey::CardRewardSkip { .. }) => OwnerChoiceExpansion::AutoAllowed,
        Some(DecisionCandidateKey::CardRewardPick { .. })
            if card_reward_effective_lane(session, strategy, choice) == CandidateLane::Mainline =>
        {
            OwnerChoiceExpansion::AutoAllowed
        }
        Some(DecisionCandidateKey::CardRewardPick { .. }) => {
            expansion_from_evaluation(choice.annotation.evaluation())
        }
        _ => OwnerChoiceExpansion::InspectOnly("unsupported card reward candidate"),
    }
}

fn card_reward_choice_rank(
    session: &RunControlSession,
    strategy: &sts_simulator::ai::noncombat_strategy_v1::RunStrategySnapshotV2,
    choice: &OwnerChoice,
    has_mainline_take: bool,
) -> CardRewardProductionOrderKeyV1 {
    let current_obligation_key =
        strategic_obligation_order_key(session, &session.run_state, &strategy.threat_coverage);
    let current_residual_key = capability_residual_order_key(&strategy.threat_coverage);
    match &choice.key {
        Some(DecisionCandidateKey::CardRewardOpen { .. }) => CardRewardProductionOrderKeyV1 {
            surface_rank: 0,
            lane_rank: 0,
            unavoidable_gaps_by_deadline: current_obligation_key.unavoidable_gaps_by_deadline,
            possible_pool_gaps: current_obligation_key.possible_pool_gaps,
            residual: current_residual_key,
            hard_startup_liability: false,
            component_debt_count: 0,
            access_saturated: false,
            lacks_independent_function: false,
            legacy_score_tiebreak_rank: 0,
            candidate_tiebreak_rank: 0,
        },
        Some(DecisionCandidateKey::CardRewardPick { card, upgrades, .. }) => {
            let trial = run_state_after_card(&session.run_state, *card, *upgrades);
            let after = threat_coverage_after_card_v1(
                &session.run_state,
                &strategy.threats,
                *card,
                *upgrades,
            );
            let after_obligation_key = strategic_obligation_order_key(session, &trial, &after);
            let after_residual_key = capability_residual_order_key(&after);
            let evaluation_key = choice
                .annotation
                .evaluation()
                .map(|evaluation| evaluation.order_key(has_mainline_take));
            let coverage_improves = after_obligation_key < current_obligation_key
                || (after_obligation_key == current_obligation_key
                    && after_residual_key < current_residual_key);
            let functional = card_reward_functional_evidence(
                &session.run_state,
                strategy,
                *card,
                *upgrades,
                coverage_improves,
            );
            CardRewardProductionOrderKeyV1 {
                surface_rank: 1,
                lane_rank: candidate_lane_rank(
                    card_reward_effective_lane(session, strategy, choice),
                    has_mainline_take,
                ),
                unavoidable_gaps_by_deadline: after_obligation_key.unavoidable_gaps_by_deadline,
                possible_pool_gaps: after_obligation_key.possible_pool_gaps,
                residual: after_residual_key,
                hard_startup_liability: functional.hard_liability.is_some(),
                component_debt_count: functional.component_debt_count,
                access_saturated: functional.access_saturated,
                lacks_independent_function: !functional.has_unburdened_independent_value()
                    && !functional.has(CardRewardFunctionV1::Answer),
                legacy_score_tiebreak_rank: evaluation_key.map_or(0, |key| key.score_rank),
                candidate_tiebreak_rank: evaluation_key.map_or(9, |key| key.tiebreak_rank),
            }
        }
        Some(DecisionCandidateKey::CardRewardSingingBowl { .. })
        | Some(DecisionCandidateKey::CardRewardSkip { .. }) => {
            let evaluation_key = choice
                .annotation
                .evaluation()
                .map(|evaluation| evaluation.order_key(has_mainline_take));
            CardRewardProductionOrderKeyV1 {
                surface_rank: 1,
                lane_rank: candidate_lane_rank(CandidateLane::Skip, has_mainline_take),
                unavoidable_gaps_by_deadline: current_obligation_key.unavoidable_gaps_by_deadline,
                possible_pool_gaps: current_obligation_key.possible_pool_gaps,
                residual: current_residual_key,
                hard_startup_liability: false,
                component_debt_count: 0,
                access_saturated: false,
                lacks_independent_function: true,
                legacy_score_tiebreak_rank: evaluation_key.map_or(0, |key| key.score_rank),
                candidate_tiebreak_rank: evaluation_key.map_or(7, |key| key.tiebreak_rank),
            }
        }
        _ => CardRewardProductionOrderKeyV1 {
            surface_rank: 2,
            lane_rank: candidate_lane_rank(CandidateLane::Reject, has_mainline_take),
            unavoidable_gaps_by_deadline: current_obligation_key.unavoidable_gaps_by_deadline,
            possible_pool_gaps: current_obligation_key.possible_pool_gaps,
            residual: current_residual_key,
            hard_startup_liability: true,
            component_debt_count: usize::MAX,
            access_saturated: true,
            lacks_independent_function: true,
            legacy_score_tiebreak_rank: 0,
            candidate_tiebreak_rank: 9,
        },
    }
}

fn is_mainline_card_reward_take(
    session: &RunControlSession,
    strategy: &sts_simulator::ai::noncombat_strategy_v1::RunStrategySnapshotV2,
    choice: &OwnerChoice,
) -> bool {
    matches!(
        choice.key,
        Some(DecisionCandidateKey::CardRewardPick { .. })
    ) && card_reward_effective_lane(session, strategy, choice) == CandidateLane::Mainline
}

fn card_reward_effective_lane(
    session: &RunControlSession,
    strategy: &sts_simulator::ai::noncombat_strategy_v1::RunStrategySnapshotV2,
    choice: &OwnerChoice,
) -> CandidateLane {
    if card_reward_repairs_or_strengthens_active_obligation(session, strategy, choice) {
        return CandidateLane::Mainline;
    }
    if card_reward_has_supported_recovery(session, strategy, choice) {
        return CandidateLane::Mainline;
    }
    choice
        .annotation
        .evaluation()
        .map_or(CandidateLane::Reject, |evaluation| evaluation.lane)
}

fn card_reward_has_supported_recovery(
    session: &RunControlSession,
    strategy: &sts_simulator::ai::noncombat_strategy_v1::RunStrategySnapshotV2,
    choice: &OwnerChoice,
) -> bool {
    let Some(DecisionCandidateKey::CardRewardPick { card, upgrades, .. }) = &choice.key else {
        return false;
    };
    if card_reward_has_hard_duplicate_liability(choice) {
        return false;
    }
    let functional =
        card_reward_functional_evidence(&session.run_state, strategy, *card, *upgrades, false);
    functional.has(CardRewardFunctionV1::Recovery)
        && functional.has(CardRewardFunctionV1::Amplifier)
        && functional.hard_liability.is_none()
        && functional.component_debt_count == 0
}

fn card_reward_repairs_or_strengthens_active_obligation(
    session: &RunControlSession,
    strategy: &sts_simulator::ai::noncombat_strategy_v1::RunStrategySnapshotV2,
    choice: &OwnerChoice,
) -> bool {
    let Some(DecisionCandidateKey::CardRewardPick { card, upgrades, .. }) = &choice.key else {
        return false;
    };
    if card_reward_has_hard_duplicate_liability(choice) {
        return false;
    }
    let before =
        strategic_obligation_order_key(session, &session.run_state, &strategy.threat_coverage);
    let trial = run_state_after_card(&session.run_state, *card, *upgrades);
    let after_coverage =
        threat_coverage_after_card_v1(&session.run_state, &strategy.threats, *card, *upgrades);
    let after = strategic_obligation_order_key(session, &trial, &after_coverage);
    if after.unavoidable_gaps_by_deadline < before.unavoidable_gaps_by_deadline {
        return true;
    }
    let strengthened = threat_relevant_capability_improvements_v1(
        &strategy.threats,
        &strategy.threat_coverage,
        &after_coverage,
    );
    if !strengthened.contains(&StrategyCapabilityKindV1::SustainedDefense) {
        return false;
    }
    let functional =
        card_reward_functional_evidence(&session.run_state, strategy, *card, *upgrades, false);
    functional.has(CardRewardFunctionV1::Answer)
        && functional.hard_liability.is_none()
        && functional.component_debt_count == 0
}

fn card_reward_has_hard_duplicate_liability(choice: &OwnerChoice) -> bool {
    choice.annotation.admission().is_some_and(|admission| {
        admission.reasons.iter().any(|reason| match reason {
            RewardAdmissionReason::DuplicateBurden(_) => true,
            RewardAdmissionReason::DuplicateConcern(concern) => concern.is_hard_penalty(),
            _ => false,
        })
    })
}

fn attach_card_reward_provenance(
    session: &RunControlSession,
    strategy: &sts_simulator::ai::noncombat_strategy_v1::RunStrategySnapshotV2,
    choice: &mut OwnerChoice,
    stable_surface_index: usize,
    owner_rank: usize,
    tie_break_applied: bool,
) {
    let (trial, after, functional) = match &choice.key {
        Some(DecisionCandidateKey::CardRewardPick { card, upgrades, .. }) => {
            let trial = run_state_after_card(&session.run_state, *card, *upgrades);
            let after = threat_coverage_after_card_v1(
                &session.run_state,
                &strategy.threats,
                *card,
                *upgrades,
            );
            let before_obligation = strategic_obligation_order_key(
                session,
                &session.run_state,
                &strategy.threat_coverage,
            );
            let after_obligation = strategic_obligation_order_key(session, &trial, &after);
            let before_residual = capability_residual_order_key(&strategy.threat_coverage);
            let after_residual = capability_residual_order_key(&after);
            let coverage_improves = after_obligation < before_obligation
                || (after_obligation == before_obligation && after_residual < before_residual);
            let functional = card_reward_functional_evidence(
                &session.run_state,
                strategy,
                *card,
                *upgrades,
                coverage_improves,
            );
            (trial, after, functional)
        }
        Some(DecisionCandidateKey::CardRewardSkip { .. }) => (
            session.run_state.clone(),
            strategy.threat_coverage.clone(),
            CardRewardFunctionalEvidenceV1 {
                functions: Vec::new(),
                hard_liability: None,
                component_debt_count: 0,
                access_saturated: false,
            },
        ),
        _ => return,
    };
    let provenance = CardRewardOwnerProvenanceV1 {
        functions: functional.functions,
        obligations: card_reward_obligation_deltas(
            session,
            &session.run_state,
            &strategy.threat_coverage,
            &trial,
            &after,
        ),
        strengthened_capabilities: threat_relevant_capability_improvements_v1(
            &strategy.threats,
            &strategy.threat_coverage,
            &after,
        ),
        hard_startup_liability: functional.hard_liability.is_some(),
        hard_duplicate_liability: card_reward_has_hard_duplicate_liability(choice),
        component_debt_count: functional.component_debt_count,
        access_saturated: functional.access_saturated,
        stable_surface_index,
        owner_rank,
        tie_break_applied,
    };
    if let ChoiceAnnotation::Candidate(decision) = &mut choice.annotation {
        decision.card_reward_provenance = Some(provenance);
    }
}

fn card_reward_obligation_deltas(
    session: &RunControlSession,
    before_run: &RunState,
    before: &StrategyThreatCoverageLedgerV1,
    after_run: &RunState,
    after: &StrategyThreatCoverageLedgerV1,
) -> Vec<CardRewardObligationDeltaV1> {
    let mut obligations = Vec::new();
    if let Some(boss) = live_act_boss_obligation(before_run) {
        obligations.push(CardRewardObligationDeltaV1 {
            source: CardRewardObligationSourceV1::KnownBoss,
            subject: format!("{boss:?}"),
            deadline_nodes: Some(floors_to_act_boss(before_run).max(0) as usize),
            gaps_before: boss_obligation_gap_count(before_run, before),
            gaps_after: boss_obligation_gap_count(after_run, after),
        });
    }

    if let Some(distance) = forced_future_elite_distance(session) {
        let exact_subject = before_run
            .peek_next_elite()
            .map(|encounter| format!("{encounter:?}"));
        obligations.push(CardRewardObligationDeltaV1 {
            source: CardRewardObligationSourceV1::CommittedRoute,
            subject: exact_subject
                .clone()
                .unwrap_or_else(|| "act_elite_pool".to_string()),
            deadline_nodes: Some(usize::from(distance)),
            gaps_before: forced_elite_gap_count(before, exact_subject.as_deref()),
            gaps_after: forced_elite_gap_count(after, exact_subject.as_deref()),
        });
    }

    obligations.push(CardRewardObligationDeltaV1 {
        source: CardRewardObligationSourceV1::PossiblePool,
        subject: "uncommitted_encounter_pools".to_string(),
        deadline_nodes: None,
        gaps_before: strategic_obligation_order_key(session, before_run, before).possible_pool_gaps,
        gaps_after: strategic_obligation_order_key(session, after_run, after).possible_pool_gaps,
    });
    obligations
}

fn boss_obligation_gap_count(
    run_state: &RunState,
    ledger: &StrategyThreatCoverageLedgerV1,
) -> usize {
    let mechanic_gaps = run_state.boss_key.map_or(0, |boss| {
        boss_mechanic_pressure_profile_v1(run_state, boss)
            .missing_answers
            .len()
    });
    mechanic_gaps.saturating_add(
        ledger
            .gaps
            .iter()
            .filter(|gap| gap.source == StrategyThreatSourceV1::ActBoss)
            .count(),
    )
}

fn forced_elite_gap_count(
    ledger: &StrategyThreatCoverageLedgerV1,
    exact_subject: Option<&str>,
) -> usize {
    ledger
        .gaps
        .iter()
        .filter(|gap| match exact_subject {
            Some(subject) => {
                gap.source == StrategyThreatSourceV1::ActEliteEncounter && gap.subject == subject
            }
            None => gap.source == StrategyThreatSourceV1::ActElitePool,
        })
        .count()
}

fn coverage_aware_admission_order_key(
    admission: &RewardAdmission,
    coverage_improves: bool,
    functional: &CardRewardFunctionalEvidenceV1,
) -> RewardAdmissionOrderKeyV1 {
    if coverage_improves || admission_has_independent_strategic_value(admission, functional) {
        reward_admission_order_key_v1(admission)
    } else {
        RewardAdmissionOrderKeyV1::empty_or_deferred()
    }
}

fn admission_has_independent_strategic_value(
    admission: &RewardAdmission,
    functional: &CardRewardFunctionalEvidenceV1,
) -> bool {
    matches!(
        admission.class,
        RewardAdmissionClass::ClosesRequirement
            | RewardAdmissionClass::BuildsSupportedPackage
            | RewardAdmissionClass::EngineSeed
    ) || functional.has_unburdened_independent_value()
        || admission.reasons.iter().any(|reason| {
            matches!(
                reason,
                RewardAdmissionReason::RunReward(_)
                    | RewardAdmissionReason::CombatUpgrade
                    | RewardAdmissionReason::RecoverCurrentHp
            )
        })
}

fn strategic_obligation_order_key(
    session: &RunControlSession,
    run_state: &RunState,
    ledger: &StrategyThreatCoverageLedgerV1,
) -> StrategicObligationOrderKeyV1 {
    let mut key = StrategicObligationOrderKeyV1::default();

    if let Some(boss) = live_act_boss_obligation(run_state) {
        let mechanic_gaps = boss_mechanic_pressure_profile_v1(run_state, boss)
            .missing_answers
            .len();
        let generic_gaps = ledger
            .gaps
            .iter()
            .filter(|gap| gap.source == StrategyThreatSourceV1::ActBoss)
            .count();
        // Boss-mechanic answers (for example Champ transition control) and
        // broad capability coverage (for example sustained scaling) are
        // distinct obligations.  Do not let the richer mechanic profile hide
        // a generic capability gap, or vice versa.
        let gap_count = mechanic_gaps.saturating_add(generic_gaps);
        add_unavoidable_gaps(&mut key, floors_to_act_boss(run_state), gap_count);
    }

    let forced_elite_distance = forced_future_elite_distance(session);
    let forced_elite_subject = forced_elite_distance
        .and_then(|_| run_state.peek_next_elite())
        .map(|encounter| format!("{encounter:?}"));
    let forced_elite_gaps = forced_elite_distance.map_or(0, |_| {
        if let Some(subject) = forced_elite_subject.as_deref() {
            ledger
                .gaps
                .iter()
                .filter(|gap| {
                    gap.source == StrategyThreatSourceV1::ActEliteEncounter
                        && gap.subject == subject
                })
                .count()
        } else {
            ledger
                .gaps
                .iter()
                .filter(|gap| gap.source == StrategyThreatSourceV1::ActElitePool)
                .count()
        }
    });
    if let Some(distance) = forced_elite_distance {
        add_unavoidable_gaps(&mut key, i32::from(distance), forced_elite_gaps);
    }

    key.possible_pool_gaps = ledger
        .gaps
        .iter()
        .filter(|gap| {
            if gap.source == StrategyThreatSourceV1::ActBoss {
                return false;
            }
            if forced_elite_distance.is_none() {
                return true;
            }
            match gap.source {
                StrategyThreatSourceV1::ActElitePool => forced_elite_gaps == 0,
                StrategyThreatSourceV1::ActEliteEncounter => forced_elite_subject
                    .as_deref()
                    .is_none_or(|subject| gap.subject != subject),
                StrategyThreatSourceV1::ActHallwayPool => true,
                StrategyThreatSourceV1::ActBoss => false,
            }
        })
        .count();
    key
}

fn add_unavoidable_gaps(key: &mut StrategicObligationOrderKeyV1, deadline_nodes: i32, gaps: usize) {
    let deadline = deadline_nodes.clamp(0, MAX_OBLIGATION_DEADLINE_NODES as i32) as usize;
    key.unavoidable_gaps_by_deadline[deadline] =
        key.unavoidable_gaps_by_deadline[deadline].saturating_add(gaps);
}

fn run_state_after_card(
    run_state: &RunState,
    card: sts_simulator::content::cards::CardId,
    upgrades: u8,
) -> RunState {
    let mut trial = run_state.clone();
    let uuid = trial
        .master_deck
        .iter()
        .map(|card| card.uuid)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let mut candidate = CombatCard::new(card, uuid);
    candidate.upgrades = upgrades;
    trial.master_deck.push(candidate);
    trial
}

fn card_reward_functional_evidence(
    run_state: &RunState,
    strategy: &sts_simulator::ai::noncombat_strategy_v1::RunStrategySnapshotV2,
    card: sts_simulator::content::cards::CardId,
    upgrades: u8,
    answers_obligation: bool,
) -> CardRewardFunctionalEvidenceV1 {
    let startup = deck_startup_profile_v1(run_state);
    let profile = card_reward_semantic_profile_v1(&RewardCard::new(card, upgrades));
    let same_card_count = run_state
        .master_deck
        .iter()
        .filter(|deck_card| deck_card.id == card)
        .count();
    let components = evaluate_card_component_signals_v1(
        &CardComponentSignalContextV1 {
            same_card_count,
            formation_needs: strategy.formation_summary().needs,
            startup: startup.clone(),
            block_plan: sts_simulator::ai::block_plan_profile_v1::block_plan_profile_v1(run_state),
        },
        &profile,
    );
    let hard_liability = startup_liability_for_candidate_v1(&startup, card, run_state.act_num);
    let support = startup_support_for_candidate_v1(&startup, card);
    let mut functions = Vec::new();
    let mut push = |value| {
        if !functions.contains(&value) {
            functions.push(value);
        }
    };
    if answers_obligation
        || components.positive_signals.iter().any(|signal| {
            matches!(
                signal,
                CardComponentSignalKindV1::FormationNeedCoverage
                    | CardComponentSignalKindV1::DamageMitigation
            )
        })
    {
        push(CardRewardFunctionV1::Answer);
    }
    if components.positive_signals.iter().any(|signal| {
        matches!(
            signal,
            CardComponentSignalKindV1::DrawEnergyAccess | CardComponentSignalKindV1::ExhaustAccess
        )
    }) {
        push(CardRewardFunctionV1::Access);
    }
    if support.is_some()
        || components.positive_signals.iter().any(|signal| {
            matches!(
                signal,
                CardComponentSignalKindV1::ExhaustPayoffSupported
                    | CardComponentSignalKindV1::BlockPayoffSupported
                    | CardComponentSignalKindV1::ExhaustEngineEnabler
                    | CardComponentSignalKindV1::FnpEngineUnlock
                    | CardComponentSignalKindV1::SelfDamagePayoffSupported
                    | CardComponentSignalKindV1::StrengthPayoffConvertibleBurstSupported
                    | CardComponentSignalKindV1::StrengthPayoffSupported
            )
        })
    {
        push(CardRewardFunctionV1::Amplifier);
    }
    if profile
        .roles
        .contains(&CardRewardSemanticRoleV1::CombatSustain)
    {
        push(CardRewardFunctionV1::Recovery);
    }
    if hard_liability.is_some() || !components.debt_signals.is_empty() {
        push(CardRewardFunctionV1::Liability);
    }
    let access_saturated = startup.strong_draw_count >= 2
        && components
            .debt_signals
            .contains(&CardComponentSignalKindV1::DuplicateNoDrawAccessDebt);
    CardRewardFunctionalEvidenceV1 {
        functions,
        hard_liability,
        component_debt_count: components.debt_signals.len(),
        access_saturated,
    }
}

fn floors_to_act_boss(run_state: &RunState) -> i32 {
    let boss_floor = match run_state.act_num {
        1 => 16,
        2 => 32,
        3 => 48,
        _ => run_state.floor_num,
    };
    boss_floor.saturating_sub(run_state.floor_num)
}

fn live_act_boss_obligation(
    run_state: &RunState,
) -> Option<sts_simulator::content::monsters::factory::EncounterId> {
    run_state
        .boss_key
        .filter(|_| floors_to_act_boss(run_state) > 0)
}

fn is_post_boss_card_reward(run_state: &RunState) -> bool {
    run_state.act_num < 3 && run_state.boss_key.is_some() && floors_to_act_boss(run_state) <= 0
}

fn card_reward_strategy_run_state(run_state: &RunState) -> RunState {
    if !is_post_boss_card_reward(run_state) {
        return run_state.clone();
    }
    let mut projected = run_state.clone();
    projected.act_num = projected.act_num.saturating_add(1);
    projected.boss_key = None;
    projected
}

fn capability_residual_order_key(
    ledger: &StrategyThreatCoverageLedgerV1,
) -> CapabilityResidualOrderKeyV1 {
    CapabilityResidualOrderKeyV1 {
        missing_capabilities: ledger
            .capabilities
            .iter()
            .filter(|capability| capability.coverage == StrategyCapabilityCoverageV1::Missing)
            .count(),
        thin_capabilities: ledger
            .capabilities
            .iter()
            .filter(|capability| capability.coverage == StrategyCapabilityCoverageV1::Thin)
            .count(),
        supported_capabilities: ledger
            .capabilities
            .iter()
            .filter(|capability| capability.coverage == StrategyCapabilityCoverageV1::Supported)
            .count(),
    }
}

#[cfg(test)]
mod tests {
    use sts_simulator::content::cards::CardId;
    use sts_simulator::content::monsters::factory::EncounterId;
    use sts_simulator::content::relics::{RelicId, RelicState};
    use sts_simulator::eval::run_control::{
        build_decision_surface, RunControlConfig, RunControlSession,
    };
    use sts_simulator::runtime::combat::CombatCard;
    use sts_simulator::state::core::EngineState;
    use sts_simulator::state::map::node::{MapEdge, MapRoomNode, RoomType};
    use sts_simulator::state::map::state::MapState;
    use sts_simulator::state::rewards::{RewardCard, RewardItem, RewardState};

    use super::super::candidate_ir_adapter::card_reward_kind;
    use super::*;

    fn ordered_cards(cards: &[CardId]) -> Vec<Option<CardId>> {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 2;
        session.run_state.floor_num = 19;
        session.run_state.boss_key = Some(EncounterId::TheChamp);
        session.run_state.add_card_to_deck(CardId::Clothesline);
        session.run_state.add_card_to_deck(CardId::ShrugItOff);
        let reward_cards = cards
            .iter()
            .copied()
            .map(|card| RewardCard::new(card, 1))
            .collect::<Vec<_>>();
        let mut reward = RewardState::new();
        reward.items = vec![RewardItem::Card {
            cards: reward_cards.clone(),
        }];
        reward.pending_card_choice = Some(reward_cards);
        reward.pending_card_reward_index = Some(0);
        session.engine_state = EngineState::RewardScreen(reward);
        let surface = build_decision_surface(&session);
        let choices = card_reward_owner_choices(&session, &surface);
        let strategy = build_run_strategy_snapshot_from_run_state_v2(&session.run_state);
        let has_mainline_take = choices
            .iter()
            .any(|choice| is_mainline_card_reward_take(&session, &strategy, choice));
        for choice in &choices {
            eprintln!(
                "exact reward candidate {:?}: rank={:?} admission={:?}",
                card_reward_kind(&choice.key),
                card_reward_choice_rank(&session, &strategy, choice, has_mainline_take),
                choice.annotation.admission(),
            );
        }
        choices
            .iter()
            .map(|choice| match card_reward_kind(&choice.key) {
                Some(DecisionCandidateKind::CardRewardPick { card, .. }) => Some(card),
                Some(DecisionCandidateKind::CardRewardSkip) => None,
                _ => None,
            })
            .collect()
    }

    fn ordered_reward_for_state(
        mut session: RunControlSession,
        cards: &[(CardId, u8)],
    ) -> (Vec<Option<CardId>>, Vec<String>) {
        let reward_cards = cards
            .iter()
            .map(|(card, upgrades)| RewardCard::new(*card, *upgrades))
            .collect::<Vec<_>>();
        let mut reward = RewardState::new();
        reward.items = vec![RewardItem::Card {
            cards: reward_cards.clone(),
        }];
        reward.pending_card_choice = Some(reward_cards);
        reward.pending_card_reward_index = Some(0);
        session.engine_state = EngineState::RewardScreen(reward);
        let surface = build_decision_surface(&session);
        let choices = card_reward_owner_choices(&session, &surface);
        let strategy = build_run_strategy_snapshot_from_run_state_v2(&session.run_state);
        let has_mainline_take = choices
            .iter()
            .any(|choice| is_mainline_card_reward_take(&session, &strategy, choice));
        let diagnostics = choices
            .iter()
            .map(|choice| {
                format!(
                    "candidate={:?} rank={:?} evaluation={:?} admission={:?}",
                    card_reward_kind(&choice.key),
                    card_reward_choice_rank(&session, &strategy, choice, has_mainline_take),
                    choice.annotation.evaluation(),
                    choice.annotation.admission(),
                )
            })
            .collect();
        let ordered = choices
            .iter()
            .filter_map(|choice| match card_reward_kind(&choice.key) {
                Some(DecisionCandidateKind::CardRewardPick { card, .. }) => Some(Some(card)),
                Some(DecisionCandidateKind::CardRewardSkip) => Some(None),
                _ => None,
            })
            .collect();
        (ordered, diagnostics)
    }

    fn exact_deck(cards: &[(CardId, u8)]) -> Vec<CombatCard> {
        cards
            .iter()
            .enumerate()
            .map(|(index, (card, upgrades))| {
                let mut card = CombatCard::new(*card, 10_000 + index as u32);
                card.upgrades = *upgrades;
                card
            })
            .collect()
    }

    fn force_next_elite(session: &mut RunControlSession, encounter: EncounterId) {
        let mut current = MapRoomNode::new(0, 0);
        current.class = Some(RoomType::MonsterRoom);
        current.edges.insert(MapEdge::new(0, 0, 0, 1));
        let mut elite = MapRoomNode::new(0, 1);
        elite.class = Some(RoomType::MonsterRoomElite);
        session.run_state.map = MapState::new(vec![vec![current], vec![elite]]);
        session.run_state.map.current_x = 0;
        session.run_state.map.current_y = 0;
        session.run_state.elite_monster_list = vec![encounter];
    }

    #[test]
    fn card_reward_owner_prefers_shockwave_to_skipping_an_open_coverage_gap() {
        let ordered = ordered_cards(&[CardId::Shockwave, CardId::WildStrike, CardId::TwinStrike]);
        assert!(
            ordered
                .iter()
                .position(|card| *card == Some(CardId::Shockwave))
                < ordered.iter().position(Option::is_none)
        );
    }

    #[test]
    fn card_reward_owner_prefers_disarm_to_skipping_an_open_coverage_gap() {
        let ordered = ordered_cards(&[CardId::Anger, CardId::DualWield, CardId::Disarm]);
        assert_eq!(ordered.first().copied().flatten(), Some(CardId::Disarm));
        assert!(
            ordered
                .iter()
                .position(|card| *card == Some(CardId::Disarm))
                < ordered.iter().position(Option::is_none)
        );
    }

    #[test]
    fn seed20260713009_first_act2_reward_strengthens_sustained_defense_with_disarm() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 2;
        session.run_state.floor_num = 17;
        session.run_state.boss_key = Some(EncounterId::Collector);
        session.run_state.master_deck = exact_deck(&[
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::Armaments, 0),
            (CardId::Cleave, 0),
            (CardId::ShrugItOff, 1),
            (CardId::Bloodletting, 0),
            (CardId::Rupture, 0),
            (CardId::Headbutt, 0),
            (CardId::Whirlwind, 0),
            (CardId::Impervious, 0),
        ]);

        let (ordered, diagnostics) = ordered_reward_for_state(
            session,
            &[
                (CardId::Disarm, 0),
                (CardId::HeavyBlade, 1),
                (CardId::Warcry, 0),
            ],
        );

        assert!(
            ordered
                .iter()
                .position(|card| *card == Some(CardId::Disarm))
                < ordered.iter().position(Option::is_none),
            "a clean improvement from supported to strong sustained defense against active Act 2 threats must remain visible above skip: {ordered:?}; {diagnostics:#?}"
        );
    }

    #[test]
    fn duplicate_disarm_does_not_claim_a_second_sustained_defense_improvement() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 2;
        session.run_state.floor_num = 17;
        session.run_state.boss_key = Some(EncounterId::Collector);
        session.run_state.master_deck = exact_deck(&[
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::ShrugItOff, 1),
            (CardId::Impervious, 0),
            (CardId::Disarm, 0),
        ]);
        let reward_cards = vec![RewardCard::new(CardId::Disarm, 0)];
        let mut reward = RewardState::new();
        reward.items = vec![RewardItem::Card {
            cards: reward_cards.clone(),
        }];
        reward.pending_card_choice = Some(reward_cards);
        reward.pending_card_reward_index = Some(0);
        session.engine_state = EngineState::RewardScreen(reward);
        let surface = build_decision_surface(&session);
        let choices = card_reward_owner_choices(&session, &surface);
        let strategy = build_run_strategy_snapshot_from_run_state_v2(&session.run_state);
        let disarm = choices
            .iter()
            .find(|choice| {
                matches!(
                    card_reward_kind(&choice.key),
                    Some(DecisionCandidateKind::CardRewardPick {
                        card: CardId::Disarm,
                        ..
                    })
                )
            })
            .expect("Disarm candidate");

        assert!(
            !card_reward_repairs_or_strengthens_active_obligation(
                &session, &strategy, disarm
            ),
            "a capability already rated strong must not manufacture another categorical improvement"
        );
    }

    #[test]
    fn card_reward_owner_keeps_feed_ahead_of_static_skip_boundary() {
        let ordered = ordered_cards(&[CardId::Clash, CardId::Feed, CardId::PommelStrike]);
        assert!(
            ordered.iter().position(|card| *card == Some(CardId::Feed))
                < ordered.iter().position(Option::is_none)
        );
    }

    #[test]
    fn generic_immediate_work_without_coverage_delta_falls_behind_skip() {
        let admission = assess_reward_admission_from_master_deck(
            &RunControlSession::new(RunControlConfig::default())
                .run_state
                .master_deck,
            CardId::Headbutt,
            0,
        );
        let no_independent_function = CardRewardFunctionalEvidenceV1 {
            functions: Vec::new(),
            hard_liability: None,
            component_debt_count: 0,
            access_saturated: false,
        };
        assert_eq!(
            coverage_aware_admission_order_key(&admission, false, &no_independent_function,),
            RewardAdmissionOrderKeyV1::empty_or_deferred()
        );
        assert!(
            RewardAdmissionOrderKeyV1::static_skip_boundary()
                < coverage_aware_admission_order_key(&admission, false, &no_independent_function,)
        );
    }

    #[test]
    fn owner_possible_pool_sort_cannot_promote_a_pipeline_probe_over_skip() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 1;
        session.run_state.floor_num = 5;
        session.run_state.boss_key = None;
        session.run_state.master_deck = exact_deck(&[
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 0),
            (CardId::Immolate, 0),
            (CardId::Cleave, 0),
            (CardId::PommelStrike, 0),
        ]);

        let (ordered, diagnostics) = ordered_reward_for_state(session, &[(CardId::IronWave, 0)]);

        assert!(
            ordered.iter().position(Option::is_none)
                < ordered
                    .iter()
                    .position(|card| *card == Some(CardId::IronWave)),
            "the owner must preserve the pipeline admission boundary; possible-pool coverage alone cannot turn a probe filler into a production take: {ordered:?}; {diagnostics:#?}"
        );
    }

    #[test]
    fn seed20260713008_base_warcry_does_not_close_hexaghost_access_obligation() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 1;
        session.run_state.floor_num = 2;
        session.run_state.boss_key = Some(EncounterId::Hexaghost);
        session.run_state.master_deck = exact_deck(&[
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 0),
            (CardId::Clothesline, 0),
        ]);

        let strategy = build_run_strategy_snapshot_from_run_state_v2(&session.run_state);
        let before =
            strategic_obligation_order_key(&session, &session.run_state, &strategy.threat_coverage);
        let after =
            threat_coverage_after_card_v1(&session.run_state, &strategy.threats, CardId::Warcry, 0);
        let trial = run_state_after_card(&session.run_state, CardId::Warcry, 0);
        let after = strategic_obligation_order_key(&session, &trial, &after);
        assert_eq!(
            after.unavoidable_gaps_by_deadline,
            before.unavoidable_gaps_by_deadline,
            "base Warcry is hand-neutral topdeck control, not a complete Hexaghost status-flood answer"
        );

        let (ordered, diagnostics) = ordered_reward_for_state(
            session,
            &[
                (CardId::IronWave, 0),
                (CardId::InfernalBlade, 0),
                (CardId::Warcry, 0),
            ],
        );
        assert!(
            ordered.iter().position(Option::is_none)
                < ordered
                    .iter()
                    .position(|card| *card == Some(CardId::Warcry)),
            "the exact seed008 F2 owner should skip rather than auto-take base Warcry: {ordered:?}; {diagnostics:#?}"
        );
    }

    #[test]
    fn seed20260713008_first_burst_block_plan_beats_skip_before_hexaghost() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 1;
        session.run_state.floor_num = 13;
        session.run_state.current_hp = 55;
        session.run_state.max_hp = 93;
        session.run_state.boss_key = Some(EncounterId::Hexaghost);
        session.run_state.master_deck = exact_deck(&[
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::Clothesline, 1),
            (CardId::BurningPact, 0),
        ]);

        let (ordered, diagnostics) = ordered_reward_for_state(
            session,
            &[
                (CardId::PowerThrough, 0),
                (CardId::Sentinel, 0),
                (CardId::SwordBoomerang, 0),
            ],
        );

        assert!(
            ordered
                .iter()
                .position(|card| *card == Some(CardId::PowerThrough))
                < ordered.iter().position(Option::is_none),
            "the first high-quality block chunk is a concrete Hexaghost survival repair: {ordered:?}; {diagnostics:#?}"
        );
    }

    #[test]
    fn seed20260713008_fuel_backed_second_wind_beats_fnp_and_skip_after_three_byrds() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 2;
        session.run_state.floor_num = 17;
        session.run_state.current_hp = 85;
        session.run_state.max_hp = 93;
        session.run_state.boss_key = Some(EncounterId::TheChamp);
        session.run_state.master_deck = exact_deck(&[
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::Clothesline, 1),
            (CardId::BurningPact, 1),
            (CardId::PowerThrough, 0),
            (CardId::Inflame, 1),
            (CardId::Offering, 0),
        ]);

        let (ordered, diagnostics) = ordered_reward_for_state(
            session,
            &[
                (CardId::IronWave, 0),
                (CardId::FeelNoPain, 1),
                (CardId::SecondWind, 1),
            ],
        );
        let second_wind = ordered
            .iter()
            .position(|card| *card == Some(CardId::SecondWind));

        assert!(
            second_wind < ordered.iter().position(|card| *card == Some(CardId::FeelNoPain)),
            "an immediately executable mass-exhaust block plan should outrank its slower payoff: {ordered:?}; {diagnostics:#?}"
        );
        assert!(
            second_wind < ordered.iter().position(Option::is_none),
            "the exact fuel-backed Second Wind+ must remain above skip: {ordered:?}; {diagnostics:#?}"
        );
    }

    #[test]
    fn fuel_backed_fnp_engine_outranks_an_additional_damage_payoff() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 2;
        session.run_state.floor_num = 18;
        session.run_state.current_hp = 93;
        session.run_state.max_hp = 93;
        session.run_state.boss_key = Some(EncounterId::TheChamp);
        session.run_state.master_deck = exact_deck(&[
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::Clothesline, 0),
            (CardId::BurningPact, 1),
            (CardId::PowerThrough, 0),
            (CardId::Inflame, 1),
            (CardId::Offering, 0),
            (CardId::SecondWind, 1),
        ]);

        let (ordered, diagnostics) = ordered_reward_for_state(
            session,
            &[
                (CardId::FeelNoPain, 1),
                (CardId::Pummel, 1),
                (CardId::Clothesline, 0),
            ],
        );

        assert_eq!(
            ordered.first(),
            Some(&Some(CardId::FeelNoPain)),
            "an upgraded FNP backed by broad exact exhaust and status fuel closes the missing repeatable defensive engine before another supported damage payoff or a redundant debuff copy: {ordered:?}; {diagnostics:#?}"
        );
    }

    #[test]
    fn unsupported_fnp_does_not_outrank_a_supported_damage_payoff() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 2;
        session.run_state.floor_num = 18;
        session.run_state.boss_key = Some(EncounterId::TheChamp);
        session.run_state.master_deck = exact_deck(&[
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::Clothesline, 0),
            (CardId::Inflame, 1),
            (CardId::Offering, 0),
        ]);

        let (ordered, diagnostics) =
            ordered_reward_for_state(session, &[(CardId::FeelNoPain, 1), (CardId::Pummel, 1)]);

        assert!(
            ordered
                .iter()
                .position(|card| *card == Some(CardId::Pummel))
                < ordered
                    .iter()
                    .position(|card| *card == Some(CardId::FeelNoPain)),
            "without an exhaust source, the prospective FNP payoff must not receive the fuel-backed engine preference: {ordered:?}; {diagnostics:#?}"
        );
    }

    #[test]
    fn seed20260713006_act1_owner_prefers_first_battle_trance_to_skip() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 1;
        session.run_state.floor_num = 13;
        session.run_state.boss_key = Some(EncounterId::TheGuardian);
        session.run_state.master_deck = exact_deck(&[
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::WildStrike, 0),
            (CardId::ShrugItOff, 0),
            (CardId::Clothesline, 0),
            (CardId::Flex, 0),
            (CardId::Feed, 0),
            (CardId::Havoc, 1),
        ]);

        let (ordered, diagnostics) = ordered_reward_for_state(
            session,
            &[
                (CardId::HeavyBlade, 0),
                (CardId::BattleTrance, 1),
                (CardId::Havoc, 1),
            ],
        );

        assert!(
            ordered.iter().position(|card| *card == Some(CardId::BattleTrance))
                < ordered.iter().position(Option::is_none),
            "first Battle Trance+ should outrank skipping in the exact Act 1 deck: {ordered:?}; {diagnostics:#?}"
        );
    }

    #[test]
    fn card_reward_snapshot_records_typed_owner_provenance() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 1;
        session.run_state.floor_num = 13;
        session.run_state.boss_key = Some(EncounterId::TheGuardian);
        session.run_state.master_deck = exact_deck(&[
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::WildStrike, 0),
            (CardId::ShrugItOff, 0),
        ]);
        let reward_cards = vec![RewardCard::new(CardId::BattleTrance, 1)];
        let mut reward = RewardState::new();
        reward.items = vec![RewardItem::Card {
            cards: reward_cards.clone(),
        }];
        reward.pending_card_choice = Some(reward_cards);
        reward.pending_card_reward_index = Some(0);
        session.engine_state = EngineState::RewardScreen(reward);

        let surface = build_decision_surface(&session);
        let choices = card_reward_owner_choices(&session, &surface);
        let battle_trance = choices
            .iter()
            .find(|choice| {
                matches!(
                    card_reward_kind(&choice.key),
                    Some(DecisionCandidateKind::CardRewardPick {
                        card: CardId::BattleTrance,
                        ..
                    })
                )
            })
            .expect("Battle Trance candidate");
        let snapshot = super::super::branch_path::ChoiceAnnotationSnapshot::from_annotation(
            &battle_trance.annotation,
        );
        let value = serde_json::to_value(snapshot).expect("serializable owner provenance");
        let provenance = &value["card_reward_provenance"];

        assert!(provenance["functions"]
            .as_array()
            .is_some_and(|functions| functions.iter().any(|value| value == "Access")));
        assert!(provenance["obligations"]
            .as_array()
            .is_some_and(|obligations| obligations
                .iter()
                .any(|value| value["source"] == "KnownBoss")));
        assert_eq!(provenance["ordering"]["owner_rank"], 1);
        assert_eq!(provenance["ordering"]["is_discrepancy_if_selected"], false);
    }

    #[test]
    fn saturated_no_draw_access_does_not_make_every_battle_trance_beat_skip() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 2;
        session.run_state.floor_num = 20;
        session.run_state.boss_key = Some(EncounterId::TheChamp);
        session.run_state.master_deck = exact_deck(&[
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::BattleTrance, 1),
            (CardId::BattleTrance, 1),
            (CardId::Offering, 0),
            (CardId::BurningPact, 0),
            (CardId::ShrugItOff, 1),
            (CardId::Inflame, 1),
            (CardId::HeavyBlade, 0),
        ]);

        let (ordered, diagnostics) =
            ordered_reward_for_state(session, &[(CardId::BattleTrance, 1)]);

        assert!(
            ordered.iter().position(Option::is_none)
                < ordered
                    .iter()
                    .position(|card| *card == Some(CardId::BattleTrance)),
            "a third no-draw access copy in an already saturated access package must not inherit the first-copy guarantee: {ordered:?}; {diagnostics:#?}"
        );
    }

    #[test]
    fn supported_per_hit_scaler_is_not_discarded_at_the_donu_deca_deadline() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 3;
        session.run_state.floor_num = 46;
        session.run_state.boss_key = Some(EncounterId::DonuAndDeca);
        session.run_state.master_deck = exact_deck(&[
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::Inflame, 1),
            (CardId::HeavyBlade, 0),
            (CardId::SwordBoomerang, 1),
            (CardId::Offering, 1),
            (CardId::SecondWind, 1),
            (CardId::FeelNoPain, 1),
        ]);

        let (ordered, diagnostics) = ordered_reward_for_state(
            session,
            &[
                (CardId::SeverSoul, 0),
                (CardId::TwinStrike, 0),
                (CardId::Whirlwind, 1),
            ],
        );

        assert_eq!(
            ordered.first(),
            Some(&Some(CardId::Whirlwind)),
            "an upgraded per-hit scaler backed by persistent Strength must remain visible at the two-target boss deadline: {ordered:?}; {diagnostics:#?}"
        );
        assert!(
            ordered
                .iter()
                .position(|card| *card == Some(CardId::Whirlwind))
                < ordered.iter().position(Option::is_none),
            "supported Whirlwind must not fall behind skip: {ordered:?}; {diagnostics:#?}"
        );
    }

    #[test]
    fn nearer_forced_sentries_obligation_can_outrank_farther_boss_scaling() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 1;
        session.run_state.floor_num = 6;
        session.run_state.boss_key = Some(EncounterId::TheGuardian);
        session.run_state.master_deck = exact_deck(&[
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::ShrugItOff, 0),
        ]);
        force_next_elite(&mut session, EncounterId::ThreeSentries);

        let (ordered, diagnostics) =
            ordered_reward_for_state(session, &[(CardId::Inflame, 1), (CardId::ThunderClap, 0)]);

        assert!(
            ordered
                .iter()
                .position(|card| *card == Some(CardId::ThunderClap))
                < ordered
                    .iter()
                    .position(|card| *card == Some(CardId::Inflame)),
            "a forced next Sentries fight must be allowed to put immediate multi-target coverage ahead of a farther boss amplifier: {ordered:?}; {diagnostics:#?}"
        );
    }

    #[test]
    fn repeated_status_generator_without_digest_can_fall_behind_skip() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 2;
        session.run_state.floor_num = 20;
        session.run_state.boss_key = Some(EncounterId::TheChamp);
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::RunicPyramid));
        session.run_state.master_deck = exact_deck(&[
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::WildStrike, 0),
            (CardId::ShrugItOff, 1),
            (CardId::Inflame, 1),
            (CardId::Reaper, 0),
        ]);

        let (ordered, diagnostics) = ordered_reward_for_state(session, &[(CardId::WildStrike, 0)]);

        assert!(
            ordered.iter().position(Option::is_none)
                < ordered
                    .iter()
                    .position(|card| *card == Some(CardId::WildStrike)),
            "Pyramid plus an existing status generator and no digest must expose liability instead of globally privileging frontload: {ordered:?}; {diagnostics:#?}"
        );
    }

    #[test]
    fn status_digest_keeps_second_wild_strike_contextual_but_below_skip() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 1;
        session.run_state.floor_num = 3;
        session.run_state.boss_key = Some(EncounterId::TheGuardian);
        session.run_state.master_deck = exact_deck(&[
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 0),
            (CardId::WildStrike, 0),
            (CardId::Evolve, 0),
        ]);

        let (ordered, diagnostics) = ordered_reward_for_state(session, &[(CardId::WildStrike, 0)]);

        assert!(
            ordered.iter().position(Option::is_none)
                < ordered
                    .iter()
                    .position(|card| *card == Some(CardId::WildStrike)),
            "status digest removes the unconditional clutter rejection, but must not make a second Wild Strike an automatic early take: {ordered:?}; {diagnostics:#?}"
        );
    }

    #[test]
    fn seed20260713006_pre_champ_owner_prefers_inflame_to_thunderclap() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 2;
        session.run_state.floor_num = 29;
        session.run_state.boss_key = Some(EncounterId::TheChamp);
        session.run_state.current_hp = 35;
        session.run_state.max_hp = 89;
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::RunicPyramid));
        session.run_state.master_deck = exact_deck(&[
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::WildStrike, 0),
            (CardId::WildStrike, 0),
            (CardId::ShrugItOff, 1),
            (CardId::Clothesline, 0),
            (CardId::Flex, 0),
            (CardId::Feed, 0),
            (CardId::Havoc, 1),
            (CardId::Shockwave, 1),
            (CardId::Disarm, 1),
            (CardId::DeepBreath, 1),
            (CardId::TrueGrit, 1),
        ]);

        let (ordered, diagnostics) = ordered_reward_for_state(
            session,
            &[
                (CardId::Inflame, 1),
                (CardId::ThunderClap, 0),
                (CardId::Carnage, 0),
            ],
        );

        assert!(
            ordered.iter().position(|card| *card == Some(CardId::Inflame))
                < ordered.iter().position(|card| *card == Some(CardId::ThunderClap)),
            "known Champ plus no persistent scaling should put Inflame+ ahead of Thunderclap: {ordered:?}; {diagnostics:#?}"
        );
        assert!(
            ordered
                .iter()
                .position(|card| *card == Some(CardId::Inflame))
                < ordered.iter().position(Option::is_none),
            "Inflame+ should remain a live known-Champ candidate: {ordered:?}; {diagnostics:#?}"
        );
    }

    #[test]
    fn post_boss_card_reward_does_not_keep_the_defeated_boss_as_an_obligation() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 2;
        session.run_state.floor_num = 32;
        session.run_state.current_hp = 14;
        session.run_state.max_hp = 72;
        session.run_state.boss_key = Some(EncounterId::Collector);
        session.run_state.master_deck = exact_deck(&[
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::Bloodletting, 1),
            (CardId::Rupture, 0),
            (CardId::Whirlwind, 1),
            (CardId::Disarm, 0),
            (CardId::SecondWind, 0),
            (CardId::PowerThrough, 1),
            (CardId::SwordBoomerang, 0),
        ]);

        let reward_cards = vec![
            RewardCard::new(CardId::Reaper, 0),
            RewardCard::new(CardId::LimitBreak, 0),
            RewardCard::new(CardId::DemonForm, 0),
        ];
        let mut reward = RewardState::new();
        reward.items = vec![RewardItem::Card {
            cards: reward_cards.clone(),
        }];
        reward.pending_card_choice = Some(reward_cards);
        reward.pending_card_reward_index = Some(0);
        session.engine_state = EngineState::RewardScreen(reward);

        let surface = build_decision_surface(&session);
        let choices = card_reward_owner_choices(&session, &surface);
        assert_eq!(
            card_reward_kind(&choices[0].key),
            Some(DecisionCandidateKind::CardRewardPick {
                card: CardId::Reaper,
                upgrades: 0,
            }),
            "supported combat sustain should outrank the stale scaling answer and skip"
        );
        let strategy_state = card_reward_strategy_run_state(&session.run_state);
        let strategy = build_run_strategy_snapshot_from_run_state_v2(&strategy_state);
        let has_mainline_take = choices
            .iter()
            .any(|choice| is_mainline_card_reward_take(&session, &strategy, choice));
        for choice in &choices {
            eprintln!(
                "{}: rank={:?}, evaluation={:?}, admission={:?}",
                choice.label,
                card_reward_choice_rank(&session, &strategy, choice, has_mainline_take),
                choice.annotation.evaluation(),
                choice.annotation.admission(),
            );
            let ChoiceAnnotation::Candidate(decision) = &choice.annotation else {
                continue;
            };
            let provenance = decision
                .card_reward_provenance
                .as_ref()
                .expect("card reward provenance");
            if matches!(
                card_reward_kind(&choice.key),
                Some(DecisionCandidateKind::CardRewardPick {
                    card: CardId::Reaper,
                    ..
                })
            ) {
                assert!(
                    provenance
                        .functions
                        .contains(&CardRewardFunctionV1::Recovery),
                    "Reaper must retain its independent combat recovery function"
                );
            }
            assert!(
                provenance
                    .obligations
                    .iter()
                    .all(|obligation| obligation.source != CardRewardObligationSourceV1::KnownBoss),
                "the defeated Collector must not remain a live deadline-zero obligation for {}",
                choice.label
            );
        }
    }

    #[test]
    fn unsupported_recovery_conversion_does_not_become_an_owner_guarantee() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 2;
        session.run_state.floor_num = 20;
        session.run_state.boss_key = Some(EncounterId::Collector);
        session.run_state.master_deck = exact_deck(&[
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
        ]);
        let reward_cards = vec![RewardCard::new(CardId::Reaper, 0)];
        let mut reward = RewardState::new();
        reward.items = vec![RewardItem::Card {
            cards: reward_cards.clone(),
        }];
        reward.pending_card_choice = Some(reward_cards);
        reward.pending_card_reward_index = Some(0);
        session.engine_state = EngineState::RewardScreen(reward);

        let surface = build_decision_surface(&session);
        let choices = card_reward_owner_choices(&session, &surface);
        let strategy = build_run_strategy_snapshot_from_run_state_v2(&session.run_state);
        let reaper = choices
            .iter()
            .find(|choice| {
                matches!(
                    card_reward_kind(&choice.key),
                    Some(DecisionCandidateKind::CardRewardPick {
                        card: CardId::Reaper,
                        ..
                    })
                )
            })
            .expect("Reaper candidate");

        assert!(
            !card_reward_has_supported_recovery(&session, &strategy, reaper),
            "combat recovery without a supported conversion axis may remain a candidate, but must not receive the supported-recovery guarantee"
        );
    }
}
