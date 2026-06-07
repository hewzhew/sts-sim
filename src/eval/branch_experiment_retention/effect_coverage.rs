use std::collections::{BTreeMap, BTreeSet};

use super::{
    best_fill_position_allowed, compare_rank, BranchRetentionCandidateInputV1,
    BranchRetentionLanePick, BranchRetentionSlotV1,
};

pub(super) fn preserve_choice_effect_coverage(
    candidates: &[BranchRetentionCandidateInputV1],
    available_positions: &[usize],
    selected_picks: Vec<BranchRetentionLanePick>,
    limit: usize,
) -> Vec<BranchRetentionLanePick> {
    if limit == 0 || selected_picks.is_empty() {
        return selected_picks;
    }

    let available_effects = available_choice_effect_keys(candidates, available_positions);
    if available_effects.len() <= 1 {
        return selected_picks;
    }

    let mut kept = selected_picks;
    let mut selected = kept
        .iter()
        .map(|pick| pick.position)
        .collect::<BTreeSet<_>>();
    for effect in ordered_choice_effect_keys(&available_effects) {
        if selected
            .iter()
            .any(|position| candidate_has_choice_effect(&candidates[*position], &effect))
        {
            continue;
        }

        let Some(position) =
            best_position_for_choice_effect(candidates, available_positions, &selected, &effect)
        else {
            continue;
        };

        if kept.len() < limit {
            kept.push(BranchRetentionLanePick {
                position,
                selected_by_slot: BranchRetentionSlotV1::Diversity,
            });
            selected.insert(position);
            continue;
        }

        let Some(replace_index) = replaceable_pick_index_for_choice_effect(candidates, &kept)
        else {
            continue;
        };
        selected.remove(&kept[replace_index].position);
        kept[replace_index] = BranchRetentionLanePick {
            position,
            selected_by_slot: BranchRetentionSlotV1::Diversity,
        };
        selected.insert(position);
    }

    kept
}

fn available_choice_effect_keys(
    candidates: &[BranchRetentionCandidateInputV1],
    positions: &[usize],
) -> BTreeSet<String> {
    positions
        .iter()
        .flat_map(|position| candidates[*position].choice_effect_keys.iter().cloned())
        .filter(|effect| !effect.is_empty())
        .collect()
}

fn ordered_choice_effect_keys(available: &BTreeSet<String>) -> Vec<String> {
    CHOICE_EFFECT_COVERAGE_ORDER
        .iter()
        .filter(|effect| available.contains(**effect))
        .map(|effect| (*effect).to_string())
        .chain(
            available
                .iter()
                .filter(|effect| !CHOICE_EFFECT_COVERAGE_ORDER.contains(&effect.as_str()))
                .cloned(),
        )
        .collect()
}

fn best_position_for_choice_effect(
    candidates: &[BranchRetentionCandidateInputV1],
    available_positions: &[usize],
    selected: &BTreeSet<usize>,
    effect: &str,
) -> Option<usize> {
    best_fill_position_allowed(candidates, available_positions, selected, |position| {
        candidate_has_choice_effect(&candidates[position], effect)
    })
}

fn candidate_has_choice_effect(candidate: &BranchRetentionCandidateInputV1, effect: &str) -> bool {
    candidate
        .choice_effect_keys
        .iter()
        .any(|candidate_effect| candidate_effect == effect)
}

fn replaceable_pick_index_for_choice_effect(
    candidates: &[BranchRetentionCandidateInputV1],
    picks: &[BranchRetentionLanePick],
) -> Option<usize> {
    let mut effect_counts = BTreeMap::<String, usize>::new();
    for pick in picks {
        for effect in &candidates[pick.position].choice_effect_keys {
            if !effect.is_empty() {
                *effect_counts.entry(effect.clone()).or_default() += 1;
            }
        }
    }

    picks
        .iter()
        .enumerate()
        .filter(|(_, pick)| {
            let effects = &candidates[pick.position].choice_effect_keys;
            !effects.is_empty()
                && effects
                    .iter()
                    .all(|effect| effect_counts.get(effect).copied().unwrap_or_default() > 1)
        })
        .min_by(|(_, left), (_, right)| compare_rank(candidates, left.position, right.position))
        .map(|(index, _)| index)
}

const CHOICE_EFFECT_COVERAGE_ORDER: &[&str] = &[
    "skip_reward",
    "singing_bowl",
    "remove_card",
    "transform_card",
    "upgrade_card",
    "duplicate_card",
    "bottle_card",
    "rest",
    "dig",
    "lift",
    "recall",
    "boss_relic",
    "event_choice",
    "take_card",
    "other",
];
