use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2DecisionCandidateReport, CombatSearchV2DecisionMicroscopeReport,
};

use super::types::DuelSelection;

pub(super) fn select_duel_candidate_indices(
    microscope: &CombatSearchV2DecisionMicroscopeReport,
) -> Vec<DuelSelection> {
    let mut selections = Vec::new();
    push_first_candidate(
        &mut selections,
        &microscope.candidates,
        "ordering_top",
        |_| true,
    );
    push_first_candidate(
        &mut selections,
        &microscope.candidates,
        "selected_by_best_complete",
        |candidate| candidate.selected_by_best_complete,
    );
    for (reason, role) in [
        ("first_key_setup_card", "key_setup_card"),
        ("first_damage_progress", "damage_progress"),
        ("first_prevent_hp_loss", "prevent_hp_loss"),
        ("first_tactical_potion", "tactical_potion"),
    ] {
        push_first_candidate(
            &mut selections,
            &microscope.candidates,
            reason,
            |candidate| candidate.action_role == role,
        );
    }
    selections
}

fn push_first_candidate(
    selections: &mut Vec<DuelSelection>,
    candidates: &[CombatSearchV2DecisionCandidateReport],
    reason: &'static str,
    predicate: impl Fn(&CombatSearchV2DecisionCandidateReport) -> bool,
) {
    let Some((candidate_index, candidate)) = candidates
        .iter()
        .enumerate()
        .find(|(_, candidate)| predicate(candidate))
    else {
        return;
    };
    if let Some(existing) = selections.iter_mut().find(|selection| {
        candidates
            .get(selection.candidate_index)
            .is_some_and(|selected| selected.action_key == candidate.action_key)
    }) {
        if !existing.reasons.contains(&reason) {
            existing.reasons.push(reason);
        }
        return;
    }
    selections.push(DuelSelection {
        candidate_index,
        reasons: vec![reason],
    });
}
