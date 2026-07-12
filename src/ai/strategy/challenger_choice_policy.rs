use crate::ai::strategy::candidate_pressure_response::CandidatePressureResponse;
use crate::ai::strategy::challenger_policy_state::ChallengerPolicyState;
use crate::ai::strategy::decision_pipeline::CandidateLane;
use crate::ai::strategy::deck_strategic_deficit::{
    DeckStrategicDeficitSummary, StrategicDeficitLevel,
};
use crate::ai::strategy::pressure_assessment::{
    EvidenceConfidence, PressureAxis, PressureCoverage, PressureEvidence, PressureEvidenceSource,
    PressureHypothesis,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyCandidateView {
    pub choice_index: usize,
    pub lane: CandidateLane,
    pub auto_allowed: bool,
    pub response: CandidatePressureResponse,
}

pub fn open_inventory_pressure(facts: DeckStrategicDeficitSummary) -> Vec<PressureHypothesis> {
    let mut hypotheses = Vec::new();
    push_if_open(
        &mut hypotheses,
        facts.frontload_damage,
        PressureAxis::ResolutionTempo,
        "frontload inventory is missing or thin",
    );
    push_if_open(
        &mut hypotheses,
        facts.aoe_or_minion_control,
        PressureAxis::MultiTargetControl,
        "multi-target inventory is missing or thin",
    );
    push_if_open(
        &mut hypotheses,
        facts.block_or_mitigation,
        PressureAxis::DelayCapacity,
        "delay inventory is missing or thin",
    );
    push_if_open(
        &mut hypotheses,
        facts.boss_scaling_plan,
        PressureAxis::GrowthHorizon,
        "scaling inventory is missing or thin",
    );
    if is_open(facts.deck_access) || is_open(facts.energy_or_playability) {
        hypotheses.push(open_hypothesis(
            PressureAxis::Deployability,
            "access or playability inventory is missing or thin",
        ));
    }
    hypotheses.sort_by_key(|item| item.axis);
    hypotheses.dedup_by_key(|item| item.axis);
    hypotheses
}

pub fn seed_challenger_policy(
    lane_id: u8,
    checkpoint_ref: impl Into<String>,
    facts: DeckStrategicDeficitSummary,
    response: &CandidatePressureResponse,
) -> Option<ChallengerPolicyState> {
    let active_pressure = open_inventory_pressure(facts)
        .into_iter()
        .filter(|hypothesis| response.axes.contains(&hypothesis.axis))
        .collect::<Vec<_>>();
    if active_pressure.is_empty() {
        return None;
    }
    let mut policy = ChallengerPolicyState::new(lane_id);
    policy.active_pressure = active_pressure;
    policy.record_divergence(checkpoint_ref, response);
    Some(policy)
}

pub fn select_challenger_choice(
    policy: &ChallengerPolicyState,
    candidates: &[PolicyCandidateView],
) -> Option<usize> {
    candidates
        .iter()
        .filter(|candidate| candidate_is_eligible(candidate))
        .min_by_key(|candidate| {
            let supports_commitment =
                policy.candidate_supports_active_commitment(&candidate.response);
            let answers_pressure = policy
                .active_pressure
                .iter()
                .any(|hypothesis| candidate.response.axes.contains(&hypothesis.axis));
            (
                u8::from(!supports_commitment),
                u8::from(!answers_pressure),
                candidate.choice_index,
            )
        })
        .map(|candidate| candidate.choice_index)
}

fn is_open(level: StrategicDeficitLevel) -> bool {
    matches!(
        level,
        StrategicDeficitLevel::Missing | StrategicDeficitLevel::Thin
    )
}

fn push_if_open(
    hypotheses: &mut Vec<PressureHypothesis>,
    level: StrategicDeficitLevel,
    axis: PressureAxis,
    label: &'static str,
) {
    if is_open(level) {
        hypotheses.push(open_hypothesis(axis, label));
    }
}

fn open_hypothesis(axis: PressureAxis, label: &'static str) -> PressureHypothesis {
    PressureHypothesis {
        axis,
        coverage: PressureCoverage::Open,
        confidence: EvidenceConfidence::Low,
        supporting_evidence: vec![PressureEvidence {
            source: PressureEvidenceSource::DeckCapability,
            label: label.to_string(),
        }],
        contradicting_evidence: Vec::new(),
    }
}

fn candidate_is_eligible(candidate: &PolicyCandidateView) -> bool {
    if candidate.lane == CandidateLane::Reject {
        return false;
    }
    candidate.auto_allowed
        || (candidate.lane == CandidateLane::Probe
            && (!candidate.response.axes.is_empty()
                || !candidate.response.supports_commitments.is_empty()
                || !candidate.response.opens_commitments.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::strategy::candidate_pressure_response::{
        CandidatePressureResponse, StrategyCommitmentKind,
    };
    use crate::ai::strategy::challenger_policy_state::{
        ChallengerPolicyState, CommitmentHorizon, CommitmentRequirement, CommitmentStatus,
        StrategyCommitment,
    };
    use crate::ai::strategy::decision_pipeline::CandidateLane;
    use crate::ai::strategy::deck_strategic_deficit::{
        DeckStrategicDeficitSummary, StrategicBurdenLevel, StrategicDeficitLevel,
    };
    use crate::ai::strategy::pressure_assessment::{PressureAxis, PressureCoverage};

    fn summary() -> DeckStrategicDeficitSummary {
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
    fn static_adequacy_never_emits_covered_pressure() {
        let hypotheses = open_inventory_pressure(summary());

        assert!(hypotheses.is_empty());
    }

    #[test]
    fn missing_tempo_and_delay_remain_distinct_open_hypotheses() {
        let mut facts = summary();
        facts.frontload_damage = StrategicDeficitLevel::Missing;
        facts.block_or_mitigation = StrategicDeficitLevel::Thin;

        let hypotheses = open_inventory_pressure(facts);

        assert!(hypotheses
            .iter()
            .any(|item| item.axis == PressureAxis::ResolutionTempo));
        assert!(hypotheses
            .iter()
            .any(|item| item.axis == PressureAxis::DelayCapacity));
        assert!(hypotheses
            .iter()
            .all(|item| item.coverage == PressureCoverage::Open));
    }

    #[test]
    fn seed_keeps_only_pressure_axes_the_candidate_can_answer() {
        let mut facts = summary();
        facts.frontload_damage = StrategicDeficitLevel::Missing;
        facts.block_or_mitigation = StrategicDeficitLevel::Thin;
        let response = CandidatePressureResponse {
            axes: vec![PressureAxis::DelayCapacity],
            ..CandidatePressureResponse::default()
        };

        let state = seed_challenger_policy(1, "branch-0/step-0", facts, &response)
            .expect("delay response should seed a challenger");

        assert_eq!(state.active_pressure.len(), 1);
        assert_eq!(state.active_pressure[0].axis, PressureAxis::DelayCapacity);
        assert_eq!(state.divergence_count, 1);
    }

    #[test]
    fn active_commitment_support_beats_baseline_order_on_later_boundary() {
        let mut policy = ChallengerPolicyState::new(1);
        policy.open_commitment(StrategyCommitment {
            kind: StrategyCommitmentKind::ExhaustEngine,
            status: CommitmentStatus::Active,
            requirements: vec![CommitmentRequirement::Payoff],
            horizon: CommitmentHorizon::CurrentActBoss,
            burden_units: 1,
        });
        let candidates = vec![
            PolicyCandidateView {
                choice_index: 0,
                lane: CandidateLane::Skip,
                auto_allowed: true,
                response: CandidatePressureResponse::default(),
            },
            PolicyCandidateView {
                choice_index: 1,
                lane: CandidateLane::Probe,
                auto_allowed: false,
                response: CandidatePressureResponse {
                    supports_commitments: vec![StrategyCommitmentKind::ExhaustEngine],
                    ..CandidatePressureResponse::default()
                },
            },
        ];

        assert_eq!(select_challenger_choice(&policy, &candidates), Some(1));
    }

    #[test]
    fn reject_candidate_cannot_become_a_challenger_action() {
        let policy = ChallengerPolicyState::new(1);
        let candidates = vec![PolicyCandidateView {
            choice_index: 3,
            lane: CandidateLane::Reject,
            auto_allowed: false,
            response: CandidatePressureResponse {
                axes: vec![PressureAxis::ResolutionTempo],
                ..CandidatePressureResponse::default()
            },
        }];

        assert_eq!(select_challenger_choice(&policy, &candidates), None);
    }
}
