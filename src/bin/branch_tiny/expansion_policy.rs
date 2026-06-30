use sts_simulator::ai::strategy::decision_pipeline::{
    CandidateEvaluation, CleanupTarget, DecisionCandidateKind,
};

use super::owners::{ChoiceAnnotation, OwnerChoiceExpansion};

pub(super) fn expansion_from_evaluation(
    evaluation: Option<&CandidateEvaluation>,
) -> OwnerChoiceExpansion {
    let Some(evaluation) = evaluation else {
        return inspect_only();
    };
    match evaluation.inspect_only_reason() {
        None => OwnerChoiceExpansion::AutoAllowed,
        Some(reason) => OwnerChoiceExpansion::InspectOnly(reason),
    }
}

pub(super) fn shop_tiny_choice_expansion(
    annotation: &ChoiceAnnotation,
    auto_purge_targets: &mut Vec<CleanupTarget>,
) -> OwnerChoiceExpansion {
    let Some(decision) = annotation.candidate() else {
        return inspect_only();
    };
    match decision.evaluation.candidate.kind {
        DecisionCandidateKind::ShopPurge { target } if decision.evaluation.auto_expands() => {
            if auto_purge_targets.contains(&target) {
                inspect_only()
            } else {
                auto_purge_targets.push(target);
                expansion_from_evaluation(Some(&decision.evaluation))
            }
        }
        _ => expansion_from_evaluation(Some(&decision.evaluation)),
    }
}

pub(super) fn inspect_only() -> OwnerChoiceExpansion {
    OwnerChoiceExpansion::InspectOnly("shop tiny keeps this atomic shop action inspect-only")
}
