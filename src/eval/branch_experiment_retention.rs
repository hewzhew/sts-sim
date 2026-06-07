use std::collections::{BTreeMap, BTreeSet};

use crate::ai::card_reward_policy_v1::{CardRewardSemanticProfileV1, CardRewardSemanticRoleV1};
use crate::ai::noncombat_strategy_v1::StrategyFormationSummaryV2;
use crate::eval::branch_experiment_trajectory::{
    branch_trajectory_family_key_v1, BranchTrajectorySignatureV1,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchRetentionSlotV1 {
    Package,
    EngineSetup,
    Scaling,
    DefenseEngine,
    Survival,
    Frontload,
    CleanDeck,
    Diversity,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchRetentionBudgetProfileV1 {
    #[default]
    Balanced,
    Exploration,
    Survival,
    Package,
}

impl fmt::Display for BranchRetentionBudgetProfileV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            BranchRetentionBudgetProfileV1::Balanced => "balanced",
            BranchRetentionBudgetProfileV1::Exploration => "exploration",
            BranchRetentionBudgetProfileV1::Survival => "survival",
            BranchRetentionBudgetProfileV1::Package => "package",
        })
    }
}

impl FromStr for BranchRetentionBudgetProfileV1 {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "balanced" => Ok(Self::Balanced),
            "exploration" | "explore" => Ok(Self::Exploration),
            "survival" | "safe" => Ok(Self::Survival),
            "package" | "packages" => Ok(Self::Package),
            other => Err(format!(
                "invalid retention profile `{other}`; expected balanced, exploration, survival, or package"
            )),
        }
    }
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
    pub strategy_formation: Option<StrategyFormationSummaryV2>,
    pub trajectory: BranchTrajectorySignatureV1,
    pub choice_profiles: Vec<CardRewardSemanticProfileV1>,
    pub choice_effect_keys: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BranchRetentionConfigV1 {
    pub max_total: usize,
    pub max_per_frontier: Option<usize>,
    pub budget_profile: BranchRetentionBudgetProfileV1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchRetentionDecisionV1 {
    pub primary_slot: BranchRetentionSlotV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_by_slot: Option<BranchRetentionSlotV1>,
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

const SLOT_ORDER: [BranchRetentionSlotV1; 8] = [
    BranchRetentionSlotV1::Package,
    BranchRetentionSlotV1::EngineSetup,
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
        selected_by_slot: Some(BranchRetentionSlotV1::Diversity),
        slots: vec![BranchRetentionSlotV1::Diversity],
        reasons: vec!["default branch representative".to_string()],
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BranchRetentionLanePick {
    position: usize,
    selected_by_slot: BranchRetentionSlotV1,
}

pub fn select_branch_retention_portfolio_v1(
    candidates: &[BranchRetentionCandidateInputV1],
    config: BranchRetentionConfigV1,
) -> BranchRetentionSelectionV1 {
    let mut decisions_by_index = candidates
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

    let mut selected_picks = Vec::<BranchRetentionLanePick>::new();
    let mut frontier_limit_hit = false;
    for group_positions in groups.into_values() {
        let configured_group_limit = config.max_per_frontier.unwrap_or(config.max_total);
        let group_limit = configured_group_limit.min(group_positions.len());
        if config.max_per_frontier.is_some() && group_limit < group_positions.len() {
            frontier_limit_hit = true;
        }
        let group_picks = select_positions_for_slots(
            candidates,
            &decisions_by_index,
            &group_positions,
            group_limit,
            config.budget_profile,
        );
        selected_picks.extend(group_picks);
    }

    let mut total_limit_hit = false;
    if selected_picks.len() > config.max_total {
        total_limit_hit = true;
        let positions = candidates
            .iter()
            .enumerate()
            .filter_map(|(position, candidate)| {
                selected_picks
                    .iter()
                    .any(|pick| candidates[pick.position].index == candidate.index)
                    .then_some(position)
            })
            .collect::<Vec<_>>();
        selected_picks = select_positions_for_slots(
            candidates,
            &decisions_by_index,
            &positions,
            config.max_total,
            config.budget_profile,
        );
    }

    for decision in decisions_by_index.values_mut() {
        decision.selected_by_slot = None;
    }
    let mut keep_indices = BTreeSet::new();
    for pick in selected_picks {
        let candidate_index = candidates[pick.position].index;
        keep_indices.insert(candidate_index);
        if let Some(decision) = decisions_by_index.get_mut(&candidate_index) {
            decision.selected_by_slot = Some(pick.selected_by_slot);
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
    let mut slots = Vec::new();
    let mut reasons = Vec::new();

    if has_package_candidate(&candidate.choice_profiles) {
        slots.push(BranchRetentionSlotV1::Package);
        reasons.push("contains a semantic package payoff candidate".to_string());
    }
    if complete_package_count(&candidate.trajectory) > 0 {
        slots.push(BranchRetentionSlotV1::Package);
        reasons.push("contains both setup and payoff for a trajectory package".to_string());
    }
    if has_engine_setup(&candidate.trajectory) {
        slots.push(BranchRetentionSlotV1::EngineSetup);
        reasons.push("contains a long-horizon engine or package setup seed".to_string());
    }
    if candidate
        .choice_profiles
        .iter()
        .any(|profile| profile_has_any_role(profile, SCALING_ROLES))
    {
        slots.push(BranchRetentionSlotV1::Scaling);
        reasons.push("contains long-run scaling or engine setup".to_string());
    }
    if candidate
        .choice_profiles
        .iter()
        .any(|profile| profile_has_any_role(profile, DEFENSE_ENGINE_ROLES))
    {
        slots.push(BranchRetentionSlotV1::DefenseEngine);
        reasons.push("contains block, weak, or defensive engine support".to_string());
    }
    if candidate.max_hp > 0 && candidate.hp * 100 >= candidate.max_hp * 80 {
        slots.push(BranchRetentionSlotV1::Survival);
        reasons.push("preserves high current HP".to_string());
    }
    if candidate.choice_profiles.iter().any(|profile| {
        profile
            .roles
            .contains(&CardRewardSemanticRoleV1::FrontloadDamage)
    }) {
        slots.push(BranchRetentionSlotV1::Frontload);
        reasons.push("contains immediate combat output".to_string());
    }
    if transition_attack_count(&candidate.choice_profiles) <= 1 {
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
        selected_by_slot: None,
        slots,
        reasons,
    }
}

fn select_positions_for_slots(
    candidates: &[BranchRetentionCandidateInputV1],
    decisions_by_index: &BTreeMap<usize, BranchRetentionDecisionV1>,
    positions: &[usize],
    limit: usize,
    budget_profile: BranchRetentionBudgetProfileV1,
) -> Vec<BranchRetentionLanePick> {
    if limit == 0 {
        return Vec::new();
    }
    let mut selected = Vec::<BranchRetentionLanePick>::new();
    let mut selected_set = BTreeSet::<usize>::new();
    for slot in retention_lane_sequence(budget_profile, limit) {
        if selected.len() >= limit {
            break;
        }
        if let Some(position) = best_position_for_slot(
            candidates,
            decisions_by_index,
            positions,
            slot,
            &selected_set,
        ) {
            selected.push(BranchRetentionLanePick {
                position,
                selected_by_slot: slot,
            });
            selected_set.insert(position);
        }
    }
    while selected.len() < limit {
        let Some(position) = best_fill_position(candidates, positions, &selected_set) else {
            break;
        };
        selected.push(BranchRetentionLanePick {
            position,
            selected_by_slot: BranchRetentionSlotV1::Diversity,
        });
        selected_set.insert(position);
    }
    let selected = cap_redundant_choice_prefixes(candidates, positions, selected, limit);
    let selected = cap_redundant_first_pick_prefixes(candidates, positions, selected, limit);
    let selected = cap_pure_transition_saturation(candidates, positions, selected, limit);
    preserve_choice_effect_coverage(candidates, positions, selected, limit)
}

fn retention_lane_sequence(
    profile: BranchRetentionBudgetProfileV1,
    limit: usize,
) -> Vec<BranchRetentionSlotV1> {
    let weights = retention_lane_weights(profile);
    let mut sequence = Vec::new();
    let mut counts = BTreeMap::<BranchRetentionSlotV1, usize>::new();
    while sequence.len() < limit {
        let mut added = false;
        for &(slot, cap) in weights {
            if counts.get(&slot).copied().unwrap_or_default() < cap {
                sequence.push(slot);
                *counts.entry(slot).or_default() += 1;
                added = true;
                if sequence.len() >= limit {
                    break;
                }
            }
        }
        if !added {
            for &(slot, _) in weights {
                sequence.push(slot);
                if sequence.len() >= limit {
                    break;
                }
            }
        }
    }
    sequence
}

fn retention_lane_weights(
    profile: BranchRetentionBudgetProfileV1,
) -> &'static [(BranchRetentionSlotV1, usize)] {
    match profile {
        BranchRetentionBudgetProfileV1::Balanced => &[
            (BranchRetentionSlotV1::Package, 4),
            (BranchRetentionSlotV1::EngineSetup, 3),
            (BranchRetentionSlotV1::Scaling, 2),
            (BranchRetentionSlotV1::DefenseEngine, 3),
            (BranchRetentionSlotV1::Survival, 2),
            (BranchRetentionSlotV1::Frontload, 2),
            (BranchRetentionSlotV1::CleanDeck, 2),
            (BranchRetentionSlotV1::Diversity, 2),
        ],
        BranchRetentionBudgetProfileV1::Exploration => &[
            (BranchRetentionSlotV1::Package, 5),
            (BranchRetentionSlotV1::EngineSetup, 4),
            (BranchRetentionSlotV1::Scaling, 3),
            (BranchRetentionSlotV1::DefenseEngine, 2),
            (BranchRetentionSlotV1::Survival, 1),
            (BranchRetentionSlotV1::Frontload, 1),
            (BranchRetentionSlotV1::CleanDeck, 2),
            (BranchRetentionSlotV1::Diversity, 2),
        ],
        BranchRetentionBudgetProfileV1::Survival => &[
            (BranchRetentionSlotV1::DefenseEngine, 4),
            (BranchRetentionSlotV1::Survival, 4),
            (BranchRetentionSlotV1::Frontload, 3),
            (BranchRetentionSlotV1::CleanDeck, 3),
            (BranchRetentionSlotV1::Package, 2),
            (BranchRetentionSlotV1::EngineSetup, 1),
            (BranchRetentionSlotV1::Scaling, 1),
            (BranchRetentionSlotV1::Diversity, 2),
        ],
        BranchRetentionBudgetProfileV1::Package => &[
            (BranchRetentionSlotV1::Package, 6),
            (BranchRetentionSlotV1::EngineSetup, 4),
            (BranchRetentionSlotV1::Scaling, 3),
            (BranchRetentionSlotV1::CleanDeck, 2),
            (BranchRetentionSlotV1::DefenseEngine, 2),
            (BranchRetentionSlotV1::Survival, 1),
            (BranchRetentionSlotV1::Frontload, 1),
            (BranchRetentionSlotV1::Diversity, 1),
        ],
    }
}

fn best_position_for_slot(
    candidates: &[BranchRetentionCandidateInputV1],
    decisions_by_index: &BTreeMap<usize, BranchRetentionDecisionV1>,
    positions: &[usize],
    slot: BranchRetentionSlotV1,
    selected: &BTreeSet<usize>,
) -> Option<usize> {
    let covered_families = selected
        .iter()
        .map(|position| branch_family_key(&candidates[*position]))
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
            compare_family_then_slot_score(candidates, *left, *right, slot, &covered_families)
        })
}

fn best_fill_position(
    candidates: &[BranchRetentionCandidateInputV1],
    positions: &[usize],
    selected: &BTreeSet<usize>,
) -> Option<usize> {
    best_fill_position_allowed(candidates, positions, selected, |_| true)
}

fn best_fill_position_allowed<F>(
    candidates: &[BranchRetentionCandidateInputV1],
    positions: &[usize],
    selected: &BTreeSet<usize>,
    is_allowed: F,
) -> Option<usize>
where
    F: Fn(usize) -> bool,
{
    let covered_families = selected
        .iter()
        .map(|position| branch_family_key(&candidates[*position]))
        .collect::<BTreeSet<_>>();
    positions
        .iter()
        .copied()
        .filter(|position| !selected.contains(position))
        .filter(|position| is_allowed(*position))
        .max_by(|left, right| {
            let left_new_family =
                !covered_families.contains(&branch_family_key(&candidates[*left]));
            let right_new_family =
                !covered_families.contains(&branch_family_key(&candidates[*right]));
            left_new_family
                .cmp(&right_new_family)
                .then_with(|| compare_rank(candidates, *left, *right))
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

fn compare_family_then_slot_score(
    candidates: &[BranchRetentionCandidateInputV1],
    left: usize,
    right: usize,
    slot: BranchRetentionSlotV1,
    covered_families: &BTreeSet<String>,
) -> std::cmp::Ordering {
    let left_new_family = !covered_families.contains(&branch_family_key(&candidates[left]));
    let right_new_family = !covered_families.contains(&branch_family_key(&candidates[right]));
    left_new_family
        .cmp(&right_new_family)
        .then_with(|| compare_slot_score(candidates, left, right, slot))
}

fn choice_prefix_key(candidate: &BranchRetentionCandidateInputV1) -> String {
    candidate
        .choice_profiles
        .first()
        .map(|profile| profile.name.clone())
        .unwrap_or_else(|| "no_card_reward_choice".to_string())
}

fn branch_family_key(candidate: &BranchRetentionCandidateInputV1) -> String {
    let formation = candidate
        .strategy_formation
        .as_ref()
        .map(strategy_formation_key)
        .unwrap_or_else(|| "formation_unknown".to_string());
    let trajectory = branch_trajectory_family_key_v1(&candidate.trajectory);
    format!("{}|{formation}|{trajectory}", choice_prefix_key(candidate))
}

fn strategy_formation_key(formation: &StrategyFormationSummaryV2) -> String {
    let needs = formation
        .needs
        .iter()
        .map(|need| format!("{need:?}"))
        .collect::<Vec<_>>()
        .join("+");
    let strengths = formation
        .strengths
        .iter()
        .map(|strength| format!("{strength:?}"))
        .collect::<Vec<_>>()
        .join("+");
    format!("{:?}|needs={needs}|strengths={strengths}", formation.stage)
}

fn cap_redundant_choice_prefixes(
    candidates: &[BranchRetentionCandidateInputV1],
    available_positions: &[usize],
    selected_picks: Vec<BranchRetentionLanePick>,
    limit: usize,
) -> Vec<BranchRetentionLanePick> {
    let distinct_families = available_positions
        .iter()
        .map(|position| branch_family_key(&candidates[*position]))
        .collect::<BTreeSet<_>>();
    if distinct_families.len() <= 1 {
        return selected_picks;
    }

    let max_per_family = limit.div_ceil(distinct_families.len()).max(1);
    let mut counts = BTreeMap::<String, usize>::new();
    let mut kept = Vec::new();
    for pick in selected_picks {
        let family = branch_family_key(&candidates[pick.position]);
        let count = counts.entry(family).or_default();
        if *count >= max_per_family {
            continue;
        }
        *count += 1;
        kept.push(pick);
    }
    kept
}

fn cap_redundant_first_pick_prefixes(
    candidates: &[BranchRetentionCandidateInputV1],
    available_positions: &[usize],
    selected_picks: Vec<BranchRetentionLanePick>,
    limit: usize,
) -> Vec<BranchRetentionLanePick> {
    let distinct_prefixes = available_positions
        .iter()
        .map(|position| choice_prefix_key(&candidates[*position]))
        .collect::<BTreeSet<_>>();
    if distinct_prefixes.len() <= 1 {
        return selected_picks;
    }

    let max_per_prefix = first_pick_prefix_cap(limit, distinct_prefixes.len());
    let mut counts = BTreeMap::<String, usize>::new();
    let mut kept = Vec::new();
    let mut capped = false;
    for pick in selected_picks {
        let prefix = choice_prefix_key(&candidates[pick.position]);
        let count = counts.entry(prefix).or_default();
        if *count >= max_per_prefix {
            capped = true;
            continue;
        }
        *count += 1;
        kept.push(pick);
    }

    if !capped {
        return kept;
    }

    let mut selected = kept
        .iter()
        .map(|pick| pick.position)
        .collect::<BTreeSet<_>>();
    while kept.len() < limit {
        let Some(position) =
            best_fill_position_allowed(candidates, available_positions, &selected, |position| {
                counts
                    .get(&choice_prefix_key(&candidates[position]))
                    .copied()
                    .unwrap_or_default()
                    < max_per_prefix
            })
        else {
            break;
        };

        *counts
            .entry(choice_prefix_key(&candidates[position]))
            .or_default() += 1;
        selected.insert(position);
        kept.push(BranchRetentionLanePick {
            position,
            selected_by_slot: BranchRetentionSlotV1::Diversity,
        });
    }

    kept
}

fn first_pick_prefix_cap(limit: usize, distinct_prefixes: usize) -> usize {
    if limit == 0 || distinct_prefixes == 0 {
        return 0;
    }
    let base = limit.div_ceil(distinct_prefixes).max(1);
    if base >= 2 {
        base + 1
    } else {
        base
    }
}

fn cap_pure_transition_saturation(
    candidates: &[BranchRetentionCandidateInputV1],
    available_positions: &[usize],
    selected_picks: Vec<BranchRetentionLanePick>,
    limit: usize,
) -> Vec<BranchRetentionLanePick> {
    if !available_positions
        .iter()
        .any(|position| !is_pure_transition_branch(&candidates[*position]))
    {
        return selected_picks;
    }

    let max_pure_transition = pure_transition_branch_cap(candidates, available_positions, limit);
    let mut pure_transition_count = 0usize;
    let mut capped = false;
    let mut kept = Vec::new();

    for pick in selected_picks {
        if is_pure_transition_branch(&candidates[pick.position]) {
            if pure_transition_count >= max_pure_transition {
                capped = true;
                continue;
            }
            pure_transition_count += 1;
        }
        kept.push(pick);
    }

    if !capped {
        return kept;
    }

    let mut selected = kept
        .iter()
        .map(|pick| pick.position)
        .collect::<BTreeSet<_>>();
    while kept.len() < limit {
        let allow_more_pure_transition = pure_transition_count < max_pure_transition;
        let Some(position) =
            best_fill_position_allowed(candidates, available_positions, &selected, |position| {
                allow_more_pure_transition || !is_pure_transition_branch(&candidates[position])
            })
        else {
            break;
        };

        if is_pure_transition_branch(&candidates[position]) {
            pure_transition_count += 1;
        }
        selected.insert(position);
        kept.push(BranchRetentionLanePick {
            position,
            selected_by_slot: BranchRetentionSlotV1::Diversity,
        });
    }

    kept
}

fn preserve_choice_effect_coverage(
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

fn pure_transition_branch_cap(
    candidates: &[BranchRetentionCandidateInputV1],
    available_positions: &[usize],
    limit: usize,
) -> usize {
    let non_transition_families = available_positions
        .iter()
        .filter(|position| !is_pure_transition_branch(&candidates[**position]))
        .map(|position| branch_family_key(&candidates[*position]))
        .collect::<BTreeSet<_>>();
    limit
        .saturating_sub(non_transition_families.len())
        .max(1)
        .min(3)
}

fn is_pure_transition_branch(candidate: &BranchRetentionCandidateInputV1) -> bool {
    let has_committed_formation_strength = candidate
        .strategy_formation
        .as_ref()
        .is_some_and(|formation| !formation.strengths.is_empty());

    candidate.trajectory.transition_frontload_picks > 0
        && !has_committed_formation_strength
        && candidate.trajectory.setup_keys.is_empty()
        && candidate.trajectory.package_keys.is_empty()
        && candidate.trajectory.scaling_picks == 0
        && candidate.trajectory.defense_picks == 0
        && candidate.trajectory.engine_generator_picks == 0
        && candidate.trajectory.engine_payoff_picks == 0
        && candidate.trajectory.draw_energy_picks == 0
}

fn slot_score(candidate: &BranchRetentionCandidateInputV1, slot: BranchRetentionSlotV1) -> i32 {
    match slot {
        BranchRetentionSlotV1::Package => {
            package_score(&candidate.choice_profiles) * 10_000
                + complete_package_count(&candidate.trajectory) * 25_000
                + candidate.hp * 10
        }
        BranchRetentionSlotV1::EngineSetup => {
            candidate.trajectory.setup_keys.len() as i32 * 10_000
                + candidate.trajectory.engine_generator_picks as i32 * 5_000
                + candidate.trajectory.draw_energy_picks as i32 * 2_500
                + candidate.hp * 10
        }
        BranchRetentionSlotV1::Scaling => {
            count_profiles_with_any_role(&candidate.choice_profiles, SCALING_ROLES) * 10_000
                + candidate.hp * 10
        }
        BranchRetentionSlotV1::DefenseEngine => {
            count_profiles_with_any_role(&candidate.choice_profiles, DEFENSE_ENGINE_ROLES) * 10_000
                + candidate.hp * 10
        }
        BranchRetentionSlotV1::Survival => candidate.hp * 100 + candidate.gold,
        BranchRetentionSlotV1::Frontload => {
            count_profiles_with_role(
                &candidate.choice_profiles,
                CardRewardSemanticRoleV1::FrontloadDamage,
            ) * 10_000
                + candidate.hp * 10
        }
        BranchRetentionSlotV1::CleanDeck => {
            -transition_attack_count(&candidate.choice_profiles) * 10_000
                - candidate.deck_count as i32 * 100
                + candidate.hp * 10
        }
        BranchRetentionSlotV1::Diversity => -(candidate.index as i32),
    }
}

fn has_package_candidate(profiles: &[CardRewardSemanticProfileV1]) -> bool {
    package_score(profiles) > 0
}

fn package_score(profiles: &[CardRewardSemanticProfileV1]) -> i32 {
    count_profiles_with_any_role(profiles, PACKAGE_PAYOFF_ROLES)
}

fn complete_package_count(trajectory: &BranchTrajectorySignatureV1) -> i32 {
    let setup_keys = trajectory.setup_keys.iter().collect::<BTreeSet<_>>();
    trajectory
        .package_keys
        .iter()
        .filter(|package| setup_keys.contains(package))
        .count() as i32
}

fn has_engine_setup(trajectory: &BranchTrajectorySignatureV1) -> bool {
    !trajectory.setup_keys.is_empty()
        || trajectory.engine_generator_picks > 0
        || trajectory.draw_energy_picks > 0
}

fn transition_attack_count(profiles: &[CardRewardSemanticProfileV1]) -> i32 {
    profiles
        .iter()
        .filter(|profile| {
            profile
                .roles
                .contains(&CardRewardSemanticRoleV1::FrontloadDamage)
                && !profile_has_any_role(profile, NON_TRANSITION_ROLES)
        })
        .count() as i32
}

fn count_profiles_with_role(
    profiles: &[CardRewardSemanticProfileV1],
    role: CardRewardSemanticRoleV1,
) -> i32 {
    profiles
        .iter()
        .filter(|profile| profile.roles.contains(&role))
        .count() as i32
}

fn count_profiles_with_any_role(
    profiles: &[CardRewardSemanticProfileV1],
    roles: &[CardRewardSemanticRoleV1],
) -> i32 {
    profiles
        .iter()
        .filter(|profile| profile_has_any_role(profile, roles))
        .count() as i32
}

fn profile_has_any_role(
    profile: &CardRewardSemanticProfileV1,
    roles: &[CardRewardSemanticRoleV1],
) -> bool {
    roles.iter().any(|role| profile.roles.contains(role))
}

fn slot_priority(slot: BranchRetentionSlotV1) -> usize {
    SLOT_ORDER
        .iter()
        .position(|candidate| *candidate == slot)
        .unwrap_or(SLOT_ORDER.len())
}

const PACKAGE_PAYOFF_ROLES: &[CardRewardSemanticRoleV1] = &[
    CardRewardSemanticRoleV1::BlockPayoff,
    CardRewardSemanticRoleV1::StrengthPayoff,
    CardRewardSemanticRoleV1::StrikePayoff,
    CardRewardSemanticRoleV1::UpgradePayoff,
    CardRewardSemanticRoleV1::ExhaustPayoff,
    CardRewardSemanticRoleV1::StatusPayoff,
    CardRewardSemanticRoleV1::SelfDamagePayoff,
    CardRewardSemanticRoleV1::PackagePayoff,
];

const DEFENSE_ENGINE_ROLES: &[CardRewardSemanticRoleV1] = &[
    CardRewardSemanticRoleV1::Block,
    CardRewardSemanticRoleV1::BlockPayoff,
    CardRewardSemanticRoleV1::Weak,
    CardRewardSemanticRoleV1::EnemyStrengthDown,
];

const SCALING_ROLES: &[CardRewardSemanticRoleV1] = &[
    CardRewardSemanticRoleV1::ScalingSource,
    CardRewardSemanticRoleV1::StrengthPayoff,
    CardRewardSemanticRoleV1::BlockPayoff,
    CardRewardSemanticRoleV1::ExhaustPayoff,
    CardRewardSemanticRoleV1::StatusPayoff,
    CardRewardSemanticRoleV1::SelfDamagePayoff,
];

const NON_TRANSITION_ROLES: &[CardRewardSemanticRoleV1] = &[
    CardRewardSemanticRoleV1::Block,
    CardRewardSemanticRoleV1::CardDraw,
    CardRewardSemanticRoleV1::EnergySource,
    CardRewardSemanticRoleV1::Vulnerable,
    CardRewardSemanticRoleV1::Weak,
    CardRewardSemanticRoleV1::EnemyStrengthDown,
    CardRewardSemanticRoleV1::ScalingSource,
    CardRewardSemanticRoleV1::StrengthPayoff,
    CardRewardSemanticRoleV1::BlockPayoff,
    CardRewardSemanticRoleV1::StrikePayoff,
    CardRewardSemanticRoleV1::UpgradePayoff,
    CardRewardSemanticRoleV1::ExhaustGenerator,
    CardRewardSemanticRoleV1::ExhaustPayoff,
    CardRewardSemanticRoleV1::StatusGenerator,
    CardRewardSemanticRoleV1::StatusPayoff,
    CardRewardSemanticRoleV1::SelfDamagePayoff,
    CardRewardSemanticRoleV1::PackagePayoff,
];

const CHOICE_EFFECT_COVERAGE_ORDER: &[&str] = &[
    "skip_reward",
    "singing_bowl",
    "remove_card",
    "transform_card",
    "upgrade_card",
    "duplicate_card",
    "bottle_card",
    "rest",
    "boss_relic",
    "event_choice",
    "take_card",
    "other",
];

#[cfg(test)]
mod tests;
