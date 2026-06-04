use super::policy::plan_card_reward_decision_v1;
use super::types::{
    CardRewardAutopilotGateReportV1, CardRewardCandidateEvidenceV1, CardRewardDecisionContextV1,
    CardRewardDecisionV1, CardRewardPolicyActionV1, CardRewardPolicyConfigV1,
    CardRewardValueEstimateV1,
};

#[derive(Clone, Debug, PartialEq)]
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
        candidates: decision.candidates.iter().map(candidate_summary).collect(),
        value_estimates: decision.value_estimates.clone(),
        autopilot_gate: decision.autopilot_gate.clone(),
        selected_candidate_id,
        stop_reason,
        diff_vs_previous,
        decision,
    }
}

fn candidate_summary(
    candidate: &CardRewardCandidateEvidenceV1,
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
    }
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
