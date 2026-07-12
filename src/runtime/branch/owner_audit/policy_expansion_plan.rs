use std::collections::BTreeSet;

use sts_simulator::ai::strategy::candidate_pressure_response::{
    assess_candidate_pressure_response, CandidatePressureResponse, StrategyCommitmentKind,
};
use sts_simulator::ai::strategy::challenger_choice_policy::{
    seed_challenger_policy, select_challenger_choice, PolicyCandidateView,
};
use sts_simulator::ai::strategy::challenger_policy_state::{
    ChallengerPolicyState, CommitmentStatus, PolicyProgress,
};
use sts_simulator::ai::strategy::decision_pipeline::{CandidateLane, DecisionCandidateKind};
use sts_simulator::ai::strategy::deck_strategic_deficit::DeckStrategicDeficitSummary;
use sts_simulator::ai::strategy::pressure_assessment::PressureAxis;

use super::branch_policy_lane::BranchPolicyLane;
use super::owner_model::OwnerChoice;

#[derive(Clone, Debug)]
pub(super) struct PolicyExpansion {
    pub(super) choice_index: usize,
    pub(super) child_lane: BranchPolicyLane,
}

pub(super) fn plan_policy_expansions(
    lane: &BranchPolicyLane,
    facts: DeckStrategicDeficitSummary,
    choices: &[OwnerChoice],
    lane_budget: usize,
    checkpoint_ref: &str,
) -> Vec<PolicyExpansion> {
    if lane_budget == 0 {
        return Vec::new();
    }
    let production_index = choices.iter().position(OwnerChoice::auto_expand_allowed);
    let candidate_views = policy_candidate_views(choices);
    match lane {
        BranchPolicyLane::Baseline { .. } => plan_baseline_expansions(
            lane,
            facts,
            production_index,
            &candidate_views,
            lane_budget,
            checkpoint_ref,
        ),
        BranchPolicyLane::Challenger { policy } => plan_existing_challenger(
            lane,
            policy,
            production_index,
            &candidate_views,
            checkpoint_ref,
        ),
    }
}

fn plan_baseline_expansions(
    lane: &BranchPolicyLane,
    facts: DeckStrategicDeficitSummary,
    production_index: Option<usize>,
    candidates: &[PolicyCandidateView],
    lane_budget: usize,
    checkpoint_ref: &str,
) -> Vec<PolicyExpansion> {
    let Some(production_index) = production_index else {
        return Vec::new();
    };
    let mut baseline_lane = lane.clone();
    let mut expansions = vec![PolicyExpansion {
        choice_index: production_index,
        child_lane: baseline_lane.clone(),
    }];
    let mut signatures = BTreeSet::new();
    for candidate in candidates {
        if expansions.len() >= lane_budget || candidate.choice_index == production_index {
            continue;
        }
        if !candidate_can_seed(candidate) {
            continue;
        }
        let mut issued_lane = baseline_lane.clone();
        let Some(lane_id) = issued_lane.issue_challenger_id() else {
            break;
        };
        let Some(policy) =
            seed_challenger_policy(lane_id, checkpoint_ref, facts, &candidate.response)
        else {
            continue;
        };
        if !signatures.insert(policy_signature(&policy)) {
            continue;
        }
        baseline_lane = issued_lane;
        expansions.push(PolicyExpansion {
            choice_index: candidate.choice_index,
            child_lane: BranchPolicyLane::challenger(policy),
        });
    }
    expansions[0].child_lane = baseline_lane;
    expansions
}

fn plan_existing_challenger(
    lane: &BranchPolicyLane,
    policy: &ChallengerPolicyState,
    production_index: Option<usize>,
    candidates: &[PolicyCandidateView],
    checkpoint_ref: &str,
) -> Vec<PolicyExpansion> {
    let selected_index = select_challenger_choice(policy, candidates).or(production_index);
    let Some(selected_index) = selected_index else {
        return Vec::new();
    };
    let mut child_lane = lane.clone();
    let BranchPolicyLane::Challenger {
        policy: child_policy,
    } = &mut child_lane
    else {
        unreachable!("challenger planner requires challenger lane")
    };
    if production_index != Some(selected_index) {
        if let Some(response) = candidates
            .iter()
            .find(|candidate| candidate.choice_index == selected_index)
            .map(|candidate| &candidate.response)
        {
            child_policy.record_divergence(checkpoint_ref, response);
        }
    }
    child_policy.advance(PolicyProgress::DecisionBoundary);
    vec![PolicyExpansion {
        choice_index: selected_index,
        child_lane,
    }]
}

fn policy_candidate_views(choices: &[OwnerChoice]) -> Vec<PolicyCandidateView> {
    choices
        .iter()
        .enumerate()
        .filter_map(|(choice_index, choice)| {
            let decision = choice.annotation.candidate()?;
            let kind = decision.evaluation.candidate.kind;
            let response = candidate_card_identity(kind)
                .zip(decision.admission.as_ref())
                .map(|(card, admission)| assess_candidate_pressure_response(Some(card), admission))
                .unwrap_or_else(CandidatePressureResponse::default);
            Some(PolicyCandidateView {
                choice_index,
                lane: decision.evaluation.lane,
                auto_allowed: choice.auto_expand_allowed(),
                response,
            })
        })
        .collect()
}

fn candidate_card_identity(
    kind: DecisionCandidateKind,
) -> Option<(sts_simulator::content::cards::CardId, u8)> {
    match kind {
        DecisionCandidateKind::CardRewardPick { card, upgrades }
        | DecisionCandidateKind::ShopBuyCard { card, upgrades, .. } => Some((card, upgrades)),
        _ => None,
    }
}

fn candidate_can_seed(candidate: &PolicyCandidateView) -> bool {
    candidate.lane != CandidateLane::Reject
        && (candidate.auto_allowed
            || (candidate.lane == CandidateLane::Probe
                && (!candidate.response.axes.is_empty()
                    || !candidate.response.opens_commitments.is_empty()
                    || !candidate.response.supports_commitments.is_empty())))
}

fn policy_signature(
    policy: &ChallengerPolicyState,
) -> (Vec<PressureAxis>, Vec<StrategyCommitmentKind>) {
    let mut axes = policy
        .active_pressure
        .iter()
        .map(|hypothesis| hypothesis.axis)
        .collect::<Vec<_>>();
    axes.sort();
    axes.dedup();
    let mut commitments = policy
        .commitments
        .iter()
        .filter(|commitment| commitment.status == CommitmentStatus::Active)
        .map(|commitment| commitment.kind)
        .collect::<Vec<_>>();
    commitments.sort();
    commitments.dedup();
    (axes, commitments)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::ai::analysis::card_semantics::PayoffRequirement;
    use sts_simulator::ai::strategy::candidate_pressure_response::StrategyCommitmentKind;
    use sts_simulator::ai::strategy::challenger_policy_state::{
        ChallengerPolicyState, CommitmentHorizon, CommitmentRequirement, CommitmentStatus,
        StrategyCommitment,
    };
    use sts_simulator::ai::strategy::decision_pipeline::{
        CandidateEvaluation, CandidateLane, CandidateLaneAdjudication, DecisionCandidateIr,
        DecisionCandidateKind, ExpansionPlan,
    };
    use sts_simulator::ai::strategy::deck_strategic_deficit::{
        DeckStrategicDeficitSummary, StrategicBurdenLevel, StrategicDeficitLevel,
    };
    use sts_simulator::ai::strategy::package_transition::PackageKind;
    use sts_simulator::ai::strategy::reward_admission::{
        skip_reward_admission, RewardAdmission, RewardAdmissionClass, RewardAdmissionReason,
    };
    use sts_simulator::content::cards::CardId;
    use sts_simulator::eval::run_control::RunControlCommand;

    use super::super::branch_policy_lane::BranchPolicyLane;
    use super::super::owner_model::{
        ChoiceAnnotation, OwnerCandidateDecision, OwnerChoice, OwnerChoiceExpansion,
    };

    fn candidate_choice(
        kind: DecisionCandidateKind,
        lane: CandidateLane,
        admission: RewardAdmission,
    ) -> OwnerChoice {
        let auto = lane != CandidateLane::Probe;
        OwnerChoice {
            key: None,
            action: RunControlCommand::Noop,
            label: format!("{kind:?}"),
            annotation: ChoiceAnnotation::Candidate(OwnerCandidateDecision {
                evaluation: CandidateEvaluation {
                    candidate: DecisionCandidateIr { kind },
                    lane,
                    adjudication: CandidateLaneAdjudication::uncapped(lane),
                    expansion: if auto {
                        ExpansionPlan::Auto
                    } else {
                        ExpansionPlan::InspectOnly("probe fixture")
                    },
                    scores: Vec::new(),
                },
                admission: Some(admission),
            }),
            expansion: if auto {
                OwnerChoiceExpansion::AutoAllowed
            } else {
                OwnerChoiceExpansion::InspectOnly("probe fixture")
            },
        }
    }

    fn skip_choice() -> OwnerChoice {
        candidate_choice(
            DecisionCandidateKind::CardRewardSkip,
            CandidateLane::Skip,
            skip_reward_admission(),
        )
    }

    fn probe_card_choice(card: CardId, reasons: Vec<RewardAdmissionReason>) -> OwnerChoice {
        candidate_choice(
            DecisionCandidateKind::CardRewardPick { card, upgrades: 0 },
            CandidateLane::Probe,
            RewardAdmission {
                card: Some(card),
                class: RewardAdmissionClass::EngineSeed,
                reasons,
            },
        )
    }

    fn adequate_facts() -> DeckStrategicDeficitSummary {
        DeckStrategicDeficitSummary {
            frontload_damage: StrategicDeficitLevel::Adequate,
            aoe_or_minion_control: StrategicDeficitLevel::Adequate,
            block_or_mitigation: StrategicDeficitLevel::Adequate,
            boss_scaling_plan: StrategicDeficitLevel::Adequate,
            deck_access: StrategicDeficitLevel::Adequate,
            energy_or_playability: StrategicDeficitLevel::Adequate,
            deck_burden: StrategicBurdenLevel::Clean,
            too_many_low_impact_attacks: false,
            opening_hand_pollution: false,
            severe_curse_burden: false,
        }
    }

    #[test]
    fn baseline_keeps_production_choice_and_seeds_distinct_probe_challenger() {
        let mut facts = adequate_facts();
        facts.frontload_damage = StrategicDeficitLevel::Missing;
        let choices = vec![
            skip_choice(),
            probe_card_choice(
                CardId::PommelStrike,
                vec![RewardAdmissionReason::FrontloadDamage],
            ),
        ];

        let plan = plan_policy_expansions(
            &BranchPolicyLane::default(),
            facts,
            &choices,
            3,
            "branch-0/step-0",
        );

        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].choice_index, 0);
        assert_eq!(plan[0].child_lane.label(), "baseline");
        assert_eq!(plan[1].choice_index, 1);
        assert_eq!(plan[1].child_lane.label(), "challenger-1");
    }

    #[test]
    fn existing_challenger_emits_only_one_later_choice() {
        let mut policy = ChallengerPolicyState::new(1);
        policy.open_commitment(StrategyCommitment {
            kind: StrategyCommitmentKind::ExhaustEngine,
            status: CommitmentStatus::Active,
            requirements: vec![CommitmentRequirement::Payoff],
            horizon: CommitmentHorizon::CurrentActBoss,
            burden_units: 1,
        });
        let choices = vec![
            skip_choice(),
            probe_card_choice(
                CardId::DarkEmbrace,
                vec![
                    RewardAdmissionReason::Supports(PackageKind::Exhaust),
                    RewardAdmissionReason::Closes(PayoffRequirement::WantsEventStream(
                        sts_simulator::ai::analysis::card_semantics::CombatEvent::CardExhausted,
                    )),
                ],
            ),
        ];

        let plan = plan_policy_expansions(
            &BranchPolicyLane::challenger(policy),
            adequate_facts(),
            &choices,
            3,
            "branch-2/step-4",
        );

        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].choice_index, 1);
        assert_eq!(plan[0].child_lane.label(), "challenger-1");
    }

    #[test]
    fn semantically_equivalent_seed_candidates_do_not_consume_both_lanes() {
        let mut facts = adequate_facts();
        facts.frontload_damage = StrategicDeficitLevel::Missing;
        let choices = vec![
            skip_choice(),
            probe_card_choice(
                CardId::PommelStrike,
                vec![RewardAdmissionReason::FrontloadDamage],
            ),
            probe_card_choice(
                CardId::Headbutt,
                vec![RewardAdmissionReason::FrontloadDamage],
            ),
        ];

        let plan = plan_policy_expansions(
            &BranchPolicyLane::default(),
            facts,
            &choices,
            3,
            "branch-0/step-0",
        );

        assert_eq!(
            plan.iter()
                .filter(|item| item.child_lane.challenger_policy().is_some())
                .count(),
            1
        );
    }
}
