use sts_simulator::ai::strategy::decision_pipeline::{
    evaluate_decision_candidate, DecisionCandidateKind, DecisionPipelineContext,
};
use sts_simulator::ai::strategy::reward_admission::RewardAdmission;

use super::owner_model::{ChoiceAnnotation, OwnerCandidateDecision};

pub(super) fn candidate_annotation(
    context: DecisionPipelineContext,
    kind: DecisionCandidateKind,
    admission: Option<RewardAdmission>,
) -> ChoiceAnnotation {
    let evaluation = evaluate_decision_candidate(context, kind, admission.as_ref());
    ChoiceAnnotation::Candidate(OwnerCandidateDecision {
        admission,
        evaluation,
        card_reward_provenance: None,
    })
}
