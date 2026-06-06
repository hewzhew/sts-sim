use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchRetentionSlotV1 {
    Package,
    Scaling,
    DefenseEngine,
    Survival,
    Frontload,
    CleanDeck,
    Diversity,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BranchRetentionCandidateInputV1 {
    pub index: usize,
    pub frontier_key: String,
    pub rank_key: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub deck_count: usize,
    pub choice_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BranchRetentionConfigV1 {
    pub max_total: usize,
    pub max_per_frontier: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchRetentionDecisionV1 {
    pub primary_slot: BranchRetentionSlotV1,
    pub slots: Vec<BranchRetentionSlotV1>,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BranchRetentionSelectionV1 {
    pub keep_indices: BTreeSet<usize>,
    pub decisions_by_index: BTreeMap<usize, BranchRetentionDecisionV1>,
    pub total_limit_hit: bool,
    pub frontier_limit_hit: bool,
}

const SLOT_ORDER: [BranchRetentionSlotV1; 7] = [
    BranchRetentionSlotV1::Package,
    BranchRetentionSlotV1::Scaling,
    BranchRetentionSlotV1::DefenseEngine,
    BranchRetentionSlotV1::Survival,
    BranchRetentionSlotV1::Frontload,
    BranchRetentionSlotV1::CleanDeck,
    BranchRetentionSlotV1::Diversity,
];

pub fn default_branch_retention_decision_v1() -> BranchRetentionDecisionV1 {
    BranchRetentionDecisionV1 {
        primary_slot: BranchRetentionSlotV1::Diversity,
        slots: vec![BranchRetentionSlotV1::Diversity],
        reasons: vec!["default branch representative".to_string()],
    }
}

pub fn select_branch_retention_portfolio_v1(
    candidates: &[BranchRetentionCandidateInputV1],
    config: BranchRetentionConfigV1,
) -> BranchRetentionSelectionV1 {
    let decisions_by_index = candidates
        .iter()
        .map(|candidate| (candidate.index, decide_branch_retention_v1(candidate)))
        .collect::<BTreeMap<_, _>>();
    if config.max_total == 0 || candidates.is_empty() {
        return BranchRetentionSelectionV1 {
            keep_indices: BTreeSet::new(),
            decisions_by_index,
            total_limit_hit: !candidates.is_empty(),
            frontier_limit_hit: config.max_per_frontier == Some(0) && !candidates.is_empty(),
        };
    }

    let mut groups = BTreeMap::<String, Vec<usize>>::new();
    for (candidate_pos, candidate) in candidates.iter().enumerate() {
        groups
            .entry(candidate.frontier_key.clone())
            .or_default()
            .push(candidate_pos);
    }

    let mut keep_indices = BTreeSet::new();
    let mut frontier_limit_hit = false;
    for group_positions in groups.into_values() {
        let configured_group_limit = config.max_per_frontier.unwrap_or(config.max_total);
        let group_limit = configured_group_limit.min(group_positions.len());
        if config.max_per_frontier.is_some() && group_limit < group_positions.len() {
            frontier_limit_hit = true;
        }
        let selected_positions = select_positions_for_slots(
            candidates,
            &decisions_by_index,
            &group_positions,
            group_limit,
        );
        for position in selected_positions {
            keep_indices.insert(candidates[position].index);
        }
    }

    let mut total_limit_hit = false;
    if keep_indices.len() > config.max_total {
        total_limit_hit = true;
        let positions = candidates
            .iter()
            .enumerate()
            .filter_map(|(position, candidate)| {
                keep_indices.contains(&candidate.index).then_some(position)
            })
            .collect::<Vec<_>>();
        keep_indices.clear();
        for position in select_positions_for_slots(
            candidates,
            &decisions_by_index,
            &positions,
            config.max_total,
        ) {
            keep_indices.insert(candidates[position].index);
        }
    }

    BranchRetentionSelectionV1 {
        keep_indices,
        decisions_by_index,
        total_limit_hit: total_limit_hit || candidates.len() > config.max_total,
        frontier_limit_hit,
    }
}

pub fn decide_branch_retention_v1(
    candidate: &BranchRetentionCandidateInputV1,
) -> BranchRetentionDecisionV1 {
    let labels = candidate
        .choice_labels
        .iter()
        .map(|label| base_card_label(label))
        .collect::<Vec<_>>();
    let mut slots = Vec::new();
    let mut reasons = Vec::new();

    if has_package_candidate(&labels) {
        slots.push(BranchRetentionSlotV1::Package);
        reasons.push("contains an explicit package candidate".to_string());
    }
    if labels
        .iter()
        .any(|label| SCALING_CARDS.contains(&label.as_str()))
    {
        slots.push(BranchRetentionSlotV1::Scaling);
        reasons.push("contains long-run scaling or engine setup".to_string());
    }
    if labels
        .iter()
        .any(|label| DEFENSE_ENGINE_CARDS.contains(&label.as_str()))
    {
        slots.push(BranchRetentionSlotV1::DefenseEngine);
        reasons.push("contains block, weak, or defensive engine support".to_string());
    }
    if candidate.max_hp > 0 && candidate.hp * 100 >= candidate.max_hp * 80 {
        slots.push(BranchRetentionSlotV1::Survival);
        reasons.push("preserves high current HP".to_string());
    }
    if labels
        .iter()
        .any(|label| FRONTLOAD_CARDS.contains(&label.as_str()))
    {
        slots.push(BranchRetentionSlotV1::Frontload);
        reasons.push("contains immediate combat output".to_string());
    }
    if transition_attack_count(&labels) <= 1 {
        slots.push(BranchRetentionSlotV1::CleanDeck);
        reasons.push("keeps transition-card bloat lower".to_string());
    }
    slots.push(BranchRetentionSlotV1::Diversity);
    reasons.push("kept as a diversity representative".to_string());

    slots.sort();
    slots.dedup();
    slots.sort_by_key(|slot| slot_priority(*slot));
    let primary_slot = slots
        .first()
        .copied()
        .unwrap_or(BranchRetentionSlotV1::Diversity);

    BranchRetentionDecisionV1 {
        primary_slot,
        slots,
        reasons,
    }
}

fn select_positions_for_slots(
    candidates: &[BranchRetentionCandidateInputV1],
    decisions_by_index: &BTreeMap<usize, BranchRetentionDecisionV1>,
    positions: &[usize],
    limit: usize,
) -> Vec<usize> {
    if limit == 0 {
        return Vec::new();
    }
    let mut selected = BTreeSet::<usize>::new();
    for slot in SLOT_ORDER {
        if selected.len() >= limit {
            break;
        }
        if let Some(position) =
            best_position_for_slot(candidates, decisions_by_index, positions, slot, &selected)
        {
            selected.insert(position);
        }
    }
    while selected.len() < limit {
        let Some(position) = positions
            .iter()
            .copied()
            .filter(|position| !selected.contains(position))
            .max_by(|left, right| compare_rank(candidates, *left, *right))
        else {
            break;
        };
        selected.insert(position);
    }
    selected.into_iter().collect()
}

fn best_position_for_slot(
    candidates: &[BranchRetentionCandidateInputV1],
    decisions_by_index: &BTreeMap<usize, BranchRetentionDecisionV1>,
    positions: &[usize],
    slot: BranchRetentionSlotV1,
    selected: &BTreeSet<usize>,
) -> Option<usize> {
    let covered_prefixes = selected
        .iter()
        .map(|position| choice_prefix_key(&candidates[*position]))
        .collect::<BTreeSet<_>>();
    positions
        .iter()
        .copied()
        .filter(|position| !selected.contains(position))
        .filter(|position| {
            decisions_by_index
                .get(&candidates[*position].index)
                .is_some_and(|decision| decision.slots.contains(&slot))
        })
        .max_by(|left, right| {
            compare_prefix_then_slot_score(candidates, *left, *right, slot, &covered_prefixes)
        })
}

fn compare_rank(
    candidates: &[BranchRetentionCandidateInputV1],
    left: usize,
    right: usize,
) -> std::cmp::Ordering {
    candidates[left]
        .rank_key
        .cmp(&candidates[right].rank_key)
        .then_with(|| candidates[left].hp.cmp(&candidates[right].hp))
        .then_with(|| candidates[left].gold.cmp(&candidates[right].gold))
        .then_with(|| {
            candidates[right]
                .deck_count
                .cmp(&candidates[left].deck_count)
        })
        .then_with(|| candidates[right].index.cmp(&candidates[left].index))
}

fn compare_slot_score(
    candidates: &[BranchRetentionCandidateInputV1],
    left: usize,
    right: usize,
    slot: BranchRetentionSlotV1,
) -> std::cmp::Ordering {
    let left_score = slot_score(&candidates[left], slot);
    let right_score = slot_score(&candidates[right], slot);
    left_score
        .cmp(&right_score)
        .then_with(|| compare_rank(candidates, left, right))
}

fn compare_prefix_then_slot_score(
    candidates: &[BranchRetentionCandidateInputV1],
    left: usize,
    right: usize,
    slot: BranchRetentionSlotV1,
    covered_prefixes: &BTreeSet<String>,
) -> std::cmp::Ordering {
    let left_new_prefix = !covered_prefixes.contains(&choice_prefix_key(&candidates[left]));
    let right_new_prefix = !covered_prefixes.contains(&choice_prefix_key(&candidates[right]));
    left_new_prefix
        .cmp(&right_new_prefix)
        .then_with(|| compare_slot_score(candidates, left, right, slot))
}

fn choice_prefix_key(candidate: &BranchRetentionCandidateInputV1) -> String {
    candidate
        .choice_labels
        .first()
        .map(|label| base_card_label(label))
        .unwrap_or_else(|| "no_card_reward_choice".to_string())
}

fn slot_score(candidate: &BranchRetentionCandidateInputV1, slot: BranchRetentionSlotV1) -> i32 {
    let labels = candidate
        .choice_labels
        .iter()
        .map(|label| base_card_label(label))
        .collect::<Vec<_>>();
    match slot {
        BranchRetentionSlotV1::Package => package_score(&labels) * 10_000 + candidate.hp * 10,
        BranchRetentionSlotV1::Scaling => {
            count_matching(&labels, SCALING_CARDS) * 10_000 + candidate.hp * 10
        }
        BranchRetentionSlotV1::DefenseEngine => {
            count_matching(&labels, DEFENSE_ENGINE_CARDS) * 10_000 + candidate.hp * 10
        }
        BranchRetentionSlotV1::Survival => candidate.hp * 100 + candidate.gold,
        BranchRetentionSlotV1::Frontload => {
            count_matching(&labels, FRONTLOAD_CARDS) * 10_000 + candidate.hp * 10
        }
        BranchRetentionSlotV1::CleanDeck => {
            -transition_attack_count(&labels) * 10_000 - candidate.deck_count as i32 * 100
                + candidate.hp * 10
        }
        BranchRetentionSlotV1::Diversity => -(candidate.index as i32),
    }
}

fn has_package_candidate(labels: &[String]) -> bool {
    package_score(labels) > 0
}

fn package_score(labels: &[String]) -> i32 {
    let has = |name: &str| labels.iter().any(|label| label == name);
    let mut score = 0;
    if has("Barricade") && (has("Entrench") || has("Body Slam")) {
        score += 3;
    }
    if has("Limit Break")
        && labels
            .iter()
            .any(|label| STRENGTH_CARDS.contains(&label.as_str()))
    {
        score += 2;
    }
    if labels
        .iter()
        .filter(|label| EXHAUST_PACKAGE_CARDS.contains(&label.as_str()))
        .count()
        >= 2
    {
        score += 1;
    }
    score
}

fn transition_attack_count(labels: &[String]) -> i32 {
    count_matching(labels, TRANSITION_ATTACKS)
}

fn count_matching(labels: &[String], names: &[&str]) -> i32 {
    labels
        .iter()
        .filter(|label| names.contains(&label.as_str()))
        .count() as i32
}

fn base_card_label(label: &str) -> String {
    let trimmed = label.trim();
    if let Some(base) = trimmed.strip_suffix('+') {
        return base.to_string();
    }
    if let Some((base, suffix)) = trimmed.rsplit_once('+') {
        if suffix.chars().all(|ch| ch.is_ascii_digit()) {
            return base.to_string();
        }
    }
    trimmed.to_string()
}

fn slot_priority(slot: BranchRetentionSlotV1) -> usize {
    SLOT_ORDER
        .iter()
        .position(|candidate| *candidate == slot)
        .unwrap_or(SLOT_ORDER.len())
}

const FRONTLOAD_CARDS: &[&str] = &[
    "Anger",
    "Carnage",
    "Clash",
    "Cleave",
    "Clothesline",
    "Headbutt",
    "Heavy Blade",
    "Hemokinesis",
    "Immolate",
    "Iron Wave",
    "Perfected Strike",
    "Pommel Strike",
    "Pummel",
    "Reckless Charge",
    "Searing Blow",
    "Sever Soul",
    "Sword Boomerang",
    "Thunderclap",
    "Twin Strike",
    "Uppercut",
    "Whirlwind",
    "Wild Strike",
];

const TRANSITION_ATTACKS: &[&str] = &[
    "Carnage",
    "Clash",
    "Cleave",
    "Clothesline",
    "Iron Wave",
    "Perfected Strike",
    "Pommel Strike",
    "Reckless Charge",
    "Searing Blow",
    "Sever Soul",
    "Sword Boomerang",
    "Thunderclap",
    "Twin Strike",
    "Uppercut",
    "Wild Strike",
];

const DEFENSE_ENGINE_CARDS: &[&str] = &[
    "Armaments",
    "Barricade",
    "Body Slam",
    "Disarm",
    "Entrench",
    "Feel No Pain",
    "Flame Barrier",
    "Impervious",
    "Metallicize",
    "Power Through",
    "Second Wind",
    "Shockwave",
    "Shrug It Off",
];

const SCALING_CARDS: &[&str] = &[
    "Barricade",
    "Berserk",
    "Corruption",
    "Dark Embrace",
    "Demon Form",
    "Feel No Pain",
    "Inflame",
    "Limit Break",
    "Metallicize",
    "Rupture",
    "Spot Weakness",
];

const STRENGTH_CARDS: &[&str] = &[
    "Demon Form",
    "Flex",
    "Inflame",
    "Limit Break",
    "Rupture",
    "Spot Weakness",
];

const EXHAUST_PACKAGE_CARDS: &[&str] = &[
    "Burning Pact",
    "Corruption",
    "Dark Embrace",
    "Feel No Pain",
    "Fiend Fire",
    "Power Through",
    "Second Wind",
    "Sever Soul",
    "True Grit",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portfolio_retention_keeps_package_branch_over_second_frontload_branch() {
        let candidates = vec![
            BranchRetentionCandidateInputV1 {
                index: 0,
                frontier_key: "same-frontier".to_string(),
                rank_key: 10_900,
                hp: 78,
                max_hp: 80,
                gold: 120,
                deck_count: 14,
                choice_labels: vec![
                    "Twin Strike".to_string(),
                    "Perfected Strike".to_string(),
                    "Iron Wave".to_string(),
                ],
            },
            BranchRetentionCandidateInputV1 {
                index: 1,
                frontier_key: "same-frontier".to_string(),
                rank_key: 10_850,
                hp: 73,
                max_hp: 80,
                gold: 120,
                deck_count: 14,
                choice_labels: vec![
                    "Barricade".to_string(),
                    "Entrench".to_string(),
                    "Body Slam".to_string(),
                ],
            },
            BranchRetentionCandidateInputV1 {
                index: 2,
                frontier_key: "same-frontier".to_string(),
                rank_key: 10_840,
                hp: 75,
                max_hp: 80,
                gold: 120,
                deck_count: 14,
                choice_labels: vec![
                    "Wild Strike".to_string(),
                    "Cleave".to_string(),
                    "Pommel Strike".to_string(),
                ],
            },
        ];

        let selection = select_branch_retention_portfolio_v1(
            &candidates,
            BranchRetentionConfigV1 {
                max_total: 2,
                max_per_frontier: Some(2),
            },
        );

        assert_eq!(selection.keep_indices.len(), 2);
        assert!(selection.keep_indices.contains(&0));
        assert!(
            selection.keep_indices.contains(&1),
            "a package candidate should survive instead of keeping a second short-term frontload branch"
        );
        assert!(!selection.keep_indices.contains(&2));
        assert_eq!(
            selection.decisions_by_index[&1].primary_slot,
            BranchRetentionSlotV1::Package
        );
    }

    #[test]
    fn portfolio_retention_prefers_distinct_choice_prefixes_when_slots_are_redundant() {
        let candidates = vec![
            retention_candidate(0, 10_900, &["Twin Strike", "Iron Wave"]),
            retention_candidate(1, 10_850, &["Twin Strike", "Uppercut"]),
            retention_candidate(2, 10_800, &["Clash", "Pommel Strike"]),
            retention_candidate(3, 10_750, &["Sever Soul", "Clothesline"]),
        ];

        let selection = select_branch_retention_portfolio_v1(
            &candidates,
            BranchRetentionConfigV1 {
                max_total: 3,
                max_per_frontier: Some(3),
            },
        );

        assert_eq!(selection.keep_indices.len(), 3);
        assert!(selection.keep_indices.contains(&0));
        assert!(
            selection.keep_indices.contains(&2),
            "a different first-pick family should be kept before a second Twin Strike prefix"
        );
        assert!(
            selection.keep_indices.contains(&3),
            "portfolio fill should cover another distinct first-pick family"
        );
        assert!(!selection.keep_indices.contains(&1));
    }

    fn retention_candidate(
        index: usize,
        rank_key: i32,
        choice_labels: &[&str],
    ) -> BranchRetentionCandidateInputV1 {
        BranchRetentionCandidateInputV1 {
            index,
            frontier_key: "same-frontier".to_string(),
            rank_key,
            hp: 78,
            max_hp: 80,
            gold: 120,
            deck_count: 14,
            choice_labels: choice_labels
                .iter()
                .map(|label| (*label).to_string())
                .collect(),
        }
    }
}
