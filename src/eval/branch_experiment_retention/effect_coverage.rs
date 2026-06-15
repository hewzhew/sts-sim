use std::collections::{BTreeMap, BTreeSet};

use super::{
    best_fill_position_allowed, candidate_has_hard_slot_blocking_admission_liability, compare_rank,
    BranchRetentionCandidateInputV1, BranchRetentionLanePick, BranchRetentionSlotV1,
};

pub(super) fn preserve_choice_effect_coverage(
    candidates: &[BranchRetentionCandidateInputV1],
    available_positions: &[usize],
    selected_picks: Vec<BranchRetentionLanePick>,
    limit: usize,
) -> Vec<BranchRetentionLanePick> {
    preserve_string_key_coverage(
        candidates,
        available_positions,
        selected_picks,
        limit,
        CHOICE_EFFECT_COVERAGE_ORDER,
        |candidate| &candidate.choice_effect_keys,
    )
}

pub(super) fn preserve_lineage_flag_coverage(
    candidates: &[BranchRetentionCandidateInputV1],
    available_positions: &[usize],
    selected_picks: Vec<BranchRetentionLanePick>,
    limit: usize,
) -> Vec<BranchRetentionLanePick> {
    preserve_string_key_coverage(
        candidates,
        available_positions,
        selected_picks,
        limit,
        LINEAGE_FLAG_COVERAGE_ORDER,
        |candidate| &candidate.lineage_flags,
    )
}

fn preserve_string_key_coverage<F>(
    candidates: &[BranchRetentionCandidateInputV1],
    available_positions: &[usize],
    selected_picks: Vec<BranchRetentionLanePick>,
    limit: usize,
    priority_order: &[&str],
    keys_for_candidate: F,
) -> Vec<BranchRetentionLanePick>
where
    F: Fn(&BranchRetentionCandidateInputV1) -> &[String],
{
    if limit == 0 || selected_picks.is_empty() {
        return selected_picks;
    }

    let available_keys =
        available_string_keys(candidates, available_positions, &keys_for_candidate);
    if available_keys.is_empty() {
        return selected_picks;
    }

    let mut kept = selected_picks;
    let mut selected = kept
        .iter()
        .map(|pick| pick.position)
        .collect::<BTreeSet<_>>();
    for key in ordered_string_keys(&available_keys, priority_order) {
        if selected.iter().any(|position| {
            candidate_has_string_key(&candidates[*position], &key, &keys_for_candidate)
        }) {
            continue;
        }

        let Some(position) = best_position_for_string_key(
            candidates,
            available_positions,
            &selected,
            &key,
            &keys_for_candidate,
        ) else {
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

        let Some(replace_index) =
            replaceable_pick_index_for_string_key(candidates, &kept, &keys_for_candidate)
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

fn available_string_keys<F>(
    candidates: &[BranchRetentionCandidateInputV1],
    positions: &[usize],
    keys_for_candidate: F,
) -> BTreeSet<String>
where
    F: Fn(&BranchRetentionCandidateInputV1) -> &[String],
{
    positions
        .iter()
        .flat_map(|position| keys_for_candidate(&candidates[*position]).iter().cloned())
        .filter(|key| !key.is_empty())
        .collect()
}

fn ordered_string_keys(available: &BTreeSet<String>, priority_order: &[&str]) -> Vec<String> {
    priority_order
        .iter()
        .filter(|key| available.contains(**key))
        .map(|key| (*key).to_string())
        .chain(
            available
                .iter()
                .filter(|key| !priority_order.contains(&key.as_str()))
                .cloned(),
        )
        .collect()
}

fn best_position_for_string_key<F>(
    candidates: &[BranchRetentionCandidateInputV1],
    available_positions: &[usize],
    selected: &BTreeSet<usize>,
    key: &str,
    keys_for_candidate: F,
) -> Option<usize>
where
    F: Fn(&BranchRetentionCandidateInputV1) -> &[String],
{
    best_fill_position_allowed(candidates, available_positions, selected, |position| {
        candidate_has_string_key(&candidates[position], key, &keys_for_candidate)
            && !candidate_has_hard_slot_blocking_admission_liability(&candidates[position])
    })
}

fn candidate_has_string_key<F>(
    candidate: &BranchRetentionCandidateInputV1,
    key: &str,
    keys_for_candidate: F,
) -> bool
where
    F: Fn(&BranchRetentionCandidateInputV1) -> &[String],
{
    keys_for_candidate(candidate)
        .iter()
        .any(|candidate_key| candidate_key == key)
}

fn replaceable_pick_index_for_string_key<F>(
    candidates: &[BranchRetentionCandidateInputV1],
    picks: &[BranchRetentionLanePick],
    keys_for_candidate: F,
) -> Option<usize>
where
    F: Fn(&BranchRetentionCandidateInputV1) -> &[String],
{
    let mut key_counts = BTreeMap::<String, usize>::new();
    for pick in picks {
        for key in keys_for_candidate(&candidates[pick.position]) {
            if !key.is_empty() {
                *key_counts.entry(key.clone()).or_default() += 1;
            }
        }
    }

    picks
        .iter()
        .enumerate()
        .filter(|(_, pick)| {
            let keys = keys_for_candidate(&candidates[pick.position]);
            keys.is_empty()
                || keys
                    .iter()
                    .all(|key| key_counts.get(key).copied().unwrap_or_default() > 1)
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

const LINEAGE_FLAG_COVERAGE_ORDER: &[&str] = &[
    "question_card_reward_count_plus_1",
    "prayer_wheel_extra_normal_combat_card_reward",
    "busted_crown_reward_count_minus_2",
    "prismatic_shard_any_color_pool",
    "nloths_gift_triple_rare_chance",
    "molten_egg_upgrade_attack_previews",
    "toxic_egg_upgrade_skill_previews",
    "frozen_egg_upgrade_power_previews",
];
