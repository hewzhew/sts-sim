use super::policy::{
    plan_card_reward_decision_v1, plan_card_reward_decision_with_estimator_inputs_v1,
};
use super::types::{
    CardRewardAutopilotGateReportV1, CardRewardCandidateEvidenceV1, CardRewardDecisionContextV1,
    CardRewardDecisionV1, CardRewardEstimatorArbitrationV1,
    CardRewardEstimatorCandidateArbitrationV1, CardRewardEstimatorInputsV1,
    CardRewardPolicyActionV1, CardRewardPolicyConfigV1, CardRewardValueEstimateV1,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PublicRewardDecisionPacketV1 {
    pub context: CardRewardDecisionContextV1,
}

impl PublicRewardDecisionPacketV1 {
    pub fn from_context(context: &CardRewardDecisionContextV1) -> Self {
        Self {
            context: context.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardReplayCandidateSummaryV1 {
    pub index: usize,
    pub card: String,
    pub facts_summary: Vec<String>,
    pub impact_summary: Vec<String>,
    pub value_summary: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardDecisionDiffV1 {
    pub selected_candidate_before: Option<String>,
    pub selected_candidate_after: Option<String>,
    pub stop_reason_before: Option<String>,
    pub stop_reason_after: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardDecisionReplayV1 {
    pub candidates: Vec<CardRewardReplayCandidateSummaryV1>,
    pub value_estimates: Vec<CardRewardValueEstimateV1>,
    pub value_arbitration: CardRewardEstimatorArbitrationV1,
    pub autopilot_gate: CardRewardAutopilotGateReportV1,
    pub selected_candidate_id: Option<String>,
    pub stop_reason: String,
    pub diff_vs_previous: Option<CardRewardDecisionDiffV1>,
    pub decision: CardRewardDecisionV1,
}

pub fn replay_card_reward_decision_v1(
    packet: &PublicRewardDecisionPacketV1,
    config: &CardRewardPolicyConfigV1,
    previous: Option<&CardRewardDecisionV1>,
) -> CardRewardDecisionReplayV1 {
    let decision = plan_card_reward_decision_v1(&packet.context, config);
    card_reward_decision_replay(previous, decision)
}

pub fn replay_card_reward_decision_with_estimator_inputs_v1(
    packet: &PublicRewardDecisionPacketV1,
    config: &CardRewardPolicyConfigV1,
    inputs: &CardRewardEstimatorInputsV1,
    previous: Option<&CardRewardDecisionV1>,
) -> CardRewardDecisionReplayV1 {
    let decision =
        plan_card_reward_decision_with_estimator_inputs_v1(&packet.context, config, inputs);
    card_reward_decision_replay(previous, decision)
}

fn card_reward_decision_replay(
    previous: Option<&CardRewardDecisionV1>,
    decision: CardRewardDecisionV1,
) -> CardRewardDecisionReplayV1 {
    let selected_candidate_id = decision_selected_candidate_id(&decision);
    let stop_reason =
        decision_stop_reason(&decision).unwrap_or_else(|| "selected candidate".to_string());
    let diff_vs_previous = previous.map(|previous| CardRewardDecisionDiffV1 {
        selected_candidate_before: decision_selected_candidate_id(previous),
        selected_candidate_after: selected_candidate_id.clone(),
        stop_reason_before: decision_stop_reason(previous),
        stop_reason_after: decision_stop_reason(&decision),
    });

    CardRewardDecisionReplayV1 {
        candidates: decision
            .candidates
            .iter()
            .map(|candidate| candidate_summary(candidate, &decision))
            .collect(),
        value_estimates: decision.value_estimates.clone(),
        value_arbitration: decision.value_arbitration.clone(),
        autopilot_gate: decision.autopilot_gate.clone(),
        selected_candidate_id,
        stop_reason,
        diff_vs_previous,
        decision,
    }
}

fn candidate_summary(
    candidate: &CardRewardCandidateEvidenceV1,
    decision: &CardRewardDecisionV1,
) -> CardRewardReplayCandidateSummaryV1 {
    CardRewardReplayCandidateSummaryV1 {
        index: candidate.index,
        card: format!("{:?}", candidate.card),
        facts_summary: vec![
            format!("type={:?}", candidate.facts.card_type),
            format!("cost={}", candidate.facts.cost),
            format!("damage={}", candidate.facts.damage.total_damage),
            format!("block={}", candidate.facts.block),
            format!("draw={}", candidate.facts.draw_cards),
            format!("weak={}", candidate.facts.weak),
            format!("vulnerable={}", candidate.facts.vulnerable),
        ],
        impact_summary: vec![
            format!(
                "frontload_damage_delta={}",
                candidate.impact.frontload_damage_delta
            ),
            format!("block_delta={}", candidate.impact.block_delta),
            format!("draw_delta={}", candidate.impact.draw_delta),
            format!("energy_delta={}", candidate.impact.energy_delta),
            format!("plan_support={:?}", candidate.plan_delta.support),
        ],
        value_summary: candidate_value_summary(candidate, decision),
    }
}

fn candidate_value_summary(
    candidate: &CardRewardCandidateEvidenceV1,
    decision: &CardRewardDecisionV1,
) -> Vec<String> {
    let Some(report) = selected_arbitration_report(candidate, decision) else {
        let mut summary = vec!["selected_value_source=none".to_string()];
        summary.extend(candidate_strategic_delta_summary(candidate, decision));
        return summary;
    };
    let Some(estimate) = selected_arbitration_estimate(candidate, decision) else {
        let mut summary = vec!["selected_value_source=none".to_string()];
        summary.extend(candidate_strategic_delta_summary(candidate, decision));
        return summary;
    };

    let mut summary = vec![
        format!("selected_value_source={:?}", estimate.source),
        format!("selected_value_status={:?}", estimate.status),
        format!("selected_for_gate={}", report.selected_for_gate),
        format!(
            "selected_estimate_gate_eligible={}",
            report.selected_estimate_gate_eligible
        ),
    ];
    summary.extend(
        estimate
            .components
            .iter()
            .filter(|component| {
                component.name.starts_with("strategy_package_completion_")
                    || component.name.starts_with("strategy_threat_alignment_")
            })
            .map(|component| format!("component={}", component.name)),
    );
    summary.extend(candidate_strategic_delta_summary(candidate, decision));
    summary
}

fn candidate_strategic_delta_summary(
    candidate: &CardRewardCandidateEvidenceV1,
    decision: &CardRewardDecisionV1,
) -> Vec<String> {
    let candidate_id = format!("card_reward:{}:{:?}", candidate.index, candidate.card);
    let mut summary = vec![format!(
        "strategic_audit=delta_coverage:{}/{} missing={}",
        decision.strategic_trace.audit.delta_count,
        decision.strategic_trace.audit.candidate_count,
        decision.strategic_trace.audit.candidate_without_delta_count
    )];
    let Some(delta) = decision
        .strategic_trace
        .candidate_deltas
        .iter()
        .find(|delta| delta.action.candidate_id() == candidate_id)
    else {
        summary.push("strategic_delta=missing".to_string());
        return summary;
    };
    summary.push(format!("strategic_role={:?}", delta.role));
    summary.push(format!("strategic_verdict_hint={:?}", delta.verdict_hint));
    summary.push(format!("strategic_positive={:.2}", delta.positive_amount()));
    summary.push(format!("strategic_negative={:.2}", delta.negative_amount()));
    summary.extend(
        delta
            .positive
            .iter()
            .take(3)
            .map(|item| format!("strategic_plus={}", item.reason)),
    );
    summary.extend(
        delta
            .negative
            .iter()
            .take(3)
            .map(|item| format!("strategic_minus={}", item.reason)),
    );
    summary
}

fn selected_arbitration_report<'a>(
    candidate: &CardRewardCandidateEvidenceV1,
    decision: &'a CardRewardDecisionV1,
) -> Option<&'a CardRewardEstimatorCandidateArbitrationV1> {
    decision
        .value_arbitration
        .candidate_reports
        .iter()
        .find(|report| report.index == candidate.index && report.card == candidate.card)
}

fn selected_arbitration_estimate<'a>(
    candidate: &CardRewardCandidateEvidenceV1,
    decision: &'a CardRewardDecisionV1,
) -> Option<&'a CardRewardValueEstimateV1> {
    decision
        .value_arbitration
        .gate_value_estimates
        .iter()
        .find(|estimate| estimate.index == candidate.index && estimate.card == candidate.card)
}

fn decision_selected_candidate_id(decision: &CardRewardDecisionV1) -> Option<String> {
    match &decision.action {
        CardRewardPolicyActionV1::Pick { index, card, .. } => {
            Some(format!("card_reward:{index}:{card:?}"))
        }
        CardRewardPolicyActionV1::Stop { .. } => None,
    }
}

fn decision_stop_reason(decision: &CardRewardDecisionV1) -> Option<String> {
    match &decision.action {
        CardRewardPolicyActionV1::Pick { .. } => None,
        CardRewardPolicyActionV1::Stop { reason, .. } => Some(reason.clone()),
    }
}
