use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2DecisionCandidateReport, CombatSearchV2DecisionMicroscopeReport,
};
use sts_simulator::state::core::ClientInput;

use super::types::{KeyCardDecisionCandidateDigest, KeyCardDecisionTargetCandidate};

pub(super) fn target_candidate(
    microscope: &CombatSearchV2DecisionMicroscopeReport,
    card_index: usize,
) -> Option<KeyCardDecisionTargetCandidate> {
    microscope
        .candidates
        .iter()
        .find(|candidate| {
            matches!(
                candidate.input,
                ClientInput::PlayCard {
                    card_index: input_card_index,
                    target: _
                } if input_card_index == card_index
            )
        })
        .map(|candidate| KeyCardDecisionTargetCandidate {
            ordered_index: candidate.ordered_index,
            action_key: candidate.action_key.clone(),
            action_role: candidate.action_role,
            selected_by_best_complete: candidate.selected_by_best_complete,
            one_step_status: candidate.one_step.status,
            one_step_terminal: format!("{:?}", candidate.one_step.terminal),
            visible_hp_loss_if_turn_ends: candidate.one_step.visible_hp_loss_if_turn_ends,
            survival_margin: candidate.one_step.survival_margin,
            total_enemy_hp: candidate.one_step.total_enemy_hp,
        })
}

pub(super) fn candidates_before_target(
    microscope: &CombatSearchV2DecisionMicroscopeReport,
    target: &Option<KeyCardDecisionTargetCandidate>,
) -> Vec<KeyCardDecisionCandidateDigest> {
    let Some(target) = target else {
        return Vec::new();
    };
    microscope
        .candidates
        .iter()
        .filter(|candidate| candidate.ordered_index < target.ordered_index)
        .map(candidate_digest)
        .collect()
}

pub(super) fn selected_candidate(
    microscope: &CombatSearchV2DecisionMicroscopeReport,
) -> Option<KeyCardDecisionCandidateDigest> {
    microscope
        .candidates
        .iter()
        .find(|candidate| candidate.selected_by_best_complete)
        .map(candidate_digest)
}

fn candidate_digest(
    candidate: &CombatSearchV2DecisionCandidateReport,
) -> KeyCardDecisionCandidateDigest {
    KeyCardDecisionCandidateDigest {
        ordered_index: candidate.ordered_index,
        action_key: candidate.action_key.clone(),
        action_role: candidate.action_role,
        selected_by_best_complete: candidate.selected_by_best_complete,
        one_step_status: candidate.one_step.status,
        visible_hp_loss_if_turn_ends: candidate.one_step.visible_hp_loss_if_turn_ends,
        survival_margin: candidate.one_step.survival_margin,
        total_enemy_hp: candidate.one_step.total_enemy_hp,
    }
}
