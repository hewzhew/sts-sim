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
    cap_pure_transition_saturation(candidates, positions, selected, limit)
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
            (BranchRetentionSlotV1::Package, 3),
            (BranchRetentionSlotV1::EngineSetup, 2),
            (BranchRetentionSlotV1::Scaling, 2),
            (BranchRetentionSlotV1::DefenseEngine, 3),
            (BranchRetentionSlotV1::Survival, 2),
            (BranchRetentionSlotV1::Frontload, 2),
            (BranchRetentionSlotV1::CleanDeck, 2),
            (BranchRetentionSlotV1::Diversity, 4),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::noncombat_strategy_v1::{
        StrategyDeckFormationNeedV1, StrategyDeckFormationStageV1, StrategyPackageIdV2,
    };

    #[test]
    fn portfolio_retention_keeps_package_branch_over_second_frontload_branch() {
        let candidates = vec![
            retention_candidate(0, 10_900, &["Twin Strike", "Perfected Strike", "Iron Wave"]),
            BranchRetentionCandidateInputV1 {
                index: 1,
                frontier_key: "same-frontier".to_string(),
                rank_key: 10_850,
                hp: 73,
                max_hp: 80,
                gold: 120,
                deck_count: 14,
                strategy_formation: None,
                trajectory:
                    super::super::branch_experiment_trajectory::summarize_branch_trajectory_v1(&[
                        semantic_profile("Barricade", &[CardRewardSemanticRoleV1::BlockPayoff]),
                        semantic_profile("Entrench", &[CardRewardSemanticRoleV1::BlockPayoff]),
                        semantic_profile("Body Slam", &[CardRewardSemanticRoleV1::BlockPayoff]),
                    ]),
                choice_profiles: vec![
                    semantic_profile("Barricade", &[CardRewardSemanticRoleV1::BlockPayoff]),
                    semantic_profile("Entrench", &[CardRewardSemanticRoleV1::BlockPayoff]),
                    semantic_profile("Body Slam", &[CardRewardSemanticRoleV1::BlockPayoff]),
                ],
            },
            retention_candidate(2, 10_840, &["Wild Strike", "Cleave", "Pommel Strike"]),
        ];

        let selection =
            select_branch_retention_portfolio_v1(&candidates, retention_config(2, Some(2)));

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

        let selection =
            select_branch_retention_portfolio_v1(&candidates, retention_config(3, Some(3)));

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

    #[test]
    fn portfolio_fill_continues_preferring_new_prefixes_after_slot_pass() {
        let candidates = vec![
            retention_candidate(0, 10_900, &["Twin Strike", "Iron Wave"]),
            retention_candidate(1, 10_850, &["Twin Strike", "Uppercut"]),
            retention_candidate(2, 10_800, &["Clash", "Pommel Strike"]),
            retention_candidate(3, 10_750, &["Sever Soul", "Clothesline"]),
            retention_candidate(4, 10_700, &["Shockwave", "Body Slam"]),
        ];

        let selection =
            select_branch_retention_portfolio_v1(&candidates, retention_config(4, Some(4)));

        assert_eq!(selection.keep_indices.len(), 4);
        assert!(selection.keep_indices.contains(&0));
        assert!(selection.keep_indices.contains(&2));
        assert!(selection.keep_indices.contains(&3));
        assert!(
            selection.keep_indices.contains(&4),
            "fill stage should keep a lower-ranked new first-pick family before a duplicate prefix"
        );
        assert!(!selection.keep_indices.contains(&1));
    }

    #[test]
    fn portfolio_retention_does_not_fill_budget_with_redundant_first_pick_variants() {
        let candidates = vec![
            retention_candidate(0, 10_900, &["Twin Strike", "Iron Wave"]),
            retention_candidate(1, 10_890, &["Twin Strike", "Uppercut"]),
            retention_candidate(2, 10_880, &["Twin Strike", "Clothesline"]),
            retention_candidate(3, 10_870, &["Twin Strike", "Pommel Strike"]),
            retention_candidate(4, 10_860, &["Twin Strike", "Cleave"]),
            retention_candidate(5, 10_700, &["Shockwave", "Body Slam"]),
            retention_candidate(6, 10_650, &["Armaments", "Searing Blow"]),
        ];

        let selection =
            select_branch_retention_portfolio_v1(&candidates, retention_config(6, Some(6)));

        let twin_strike_kept = selection
            .keep_indices
            .iter()
            .filter(|index| candidates[**index].choice_profiles[0].name == "Twin Strike")
            .count();

        assert!(
            twin_strike_kept <= 2,
            "same first-pick variants should not fill most of an exploration budget"
        );
        assert!(selection.keep_indices.contains(&5));
        assert!(selection.keep_indices.contains(&6));
        assert!(
            selection.keep_indices.len() < 6,
            "max_total is an upper bound; redundant filler branches can be left unkept"
        );
    }

    #[test]
    fn portfolio_retention_caps_dominant_first_pick_across_distinct_families() {
        fn sever_soul_candidate(
            index: usize,
            rank_key: i32,
            setup_keys: &[&str],
            package_keys: &[&str],
            engine_generator_picks: u8,
            engine_payoff_picks: u8,
            defense_picks: u8,
        ) -> BranchRetentionCandidateInputV1 {
            named_semantic_retention_candidate(
                index,
                rank_key,
                "Sever Soul",
                trajectory_with(
                    setup_keys,
                    package_keys,
                    0,
                    engine_generator_picks,
                    engine_payoff_picks,
                    defense_picks,
                ),
                &[CardRewardSemanticRoleV1::ExhaustGenerator],
            )
        }

        let candidates = vec![
            sever_soul_candidate(0, 10_900, &["exhaust_engine"], &[], 1, 0, 0),
            sever_soul_candidate(1, 10_890, &["status_package"], &[], 1, 0, 0),
            sever_soul_candidate(2, 10_880, &["exhaust_engine"], &["exhaust_engine"], 1, 1, 0),
            sever_soul_candidate(3, 10_870, &[], &["block_engine"], 0, 1, 1),
            sever_soul_candidate(4, 10_860, &[], &["upgrade_sink"], 0, 1, 0),
            sever_soul_candidate(5, 10_850, &["exhaust_engine"], &["upgrade_sink"], 1, 1, 0),
            named_semantic_retention_candidate(
                6,
                10_300,
                "Shockwave",
                trajectory_with(&[], &[], 0, 0, 0, 1),
                &[
                    CardRewardSemanticRoleV1::Weak,
                    CardRewardSemanticRoleV1::EnemyStrengthDown,
                ],
            ),
            named_semantic_retention_candidate(
                7,
                10_200,
                "Armaments",
                trajectory_with(&[], &["upgrade_sink"], 0, 0, 1, 1),
                &[
                    CardRewardSemanticRoleV1::Block,
                    CardRewardSemanticRoleV1::UpgradePayoff,
                ],
            ),
        ];

        let selection =
            select_branch_retention_portfolio_v1(&candidates, retention_config(6, Some(6)));

        let sever_soul_kept = selection
            .keep_indices
            .iter()
            .filter(|index| candidates[**index].choice_profiles[0].name == "Sever Soul")
            .count();

        assert!(
            sever_soul_kept <= 3,
            "one first-pick prefix should not dominate the exploration budget just because its later trajectory families differ"
        );
        assert!(selection.keep_indices.contains(&6));
        assert!(selection.keep_indices.contains(&7));
    }

    #[test]
    fn portfolio_retention_preserves_distinct_formations_under_same_first_pick() {
        let mut starter = retention_candidate(0, 10_900, &["Twin Strike", "Iron Wave"]);
        starter.strategy_formation = Some(formation(
            StrategyDeckFormationStageV1::StarterShell,
            &[StrategyDeckFormationNeedV1::Frontload],
            &[],
        ));
        let mut duplicate_starter =
            retention_candidate(1, 10_890, &["Twin Strike", "Pommel Strike"]);
        duplicate_starter.strategy_formation = starter.strategy_formation.clone();
        let mut block_plan = retention_candidate(2, 10_760, &["Twin Strike", "Body Slam"]);
        block_plan.strategy_formation = Some(formation(
            StrategyDeckFormationStageV1::PlanSeeded,
            &[StrategyDeckFormationNeedV1::DrawEnergy],
            &[StrategyPackageIdV2::BlockEngine],
        ));
        let mut strength_plan = retention_candidate(3, 10_740, &["Twin Strike", "Heavy Blade"]);
        strength_plan.strategy_formation = Some(formation(
            StrategyDeckFormationStageV1::PlanSeeded,
            &[StrategyDeckFormationNeedV1::Block],
            &[StrategyPackageIdV2::StrengthScaling],
        ));
        let mut other_first_pick = retention_candidate(4, 10_700, &["Shockwave", "Cleave"]);
        other_first_pick.strategy_formation = Some(formation(
            StrategyDeckFormationStageV1::StarterShell,
            &[StrategyDeckFormationNeedV1::Frontload],
            &[],
        ));

        let selection = select_branch_retention_portfolio_v1(
            &[
                starter,
                duplicate_starter,
                block_plan,
                strength_plan,
                other_first_pick,
            ],
            retention_config(4, Some(4)),
        );

        assert!(selection.keep_indices.contains(&0));
        assert!(!selection.keep_indices.contains(&1));
        assert!(selection.keep_indices.contains(&2));
        assert!(selection.keep_indices.contains(&3));
        assert!(selection.keep_indices.contains(&4));
    }

    #[test]
    fn portfolio_retention_preserves_distinct_trajectories_under_same_formation() {
        let formation = formation(
            StrategyDeckFormationStageV1::PlanSeeded,
            &[StrategyDeckFormationNeedV1::Scaling],
            &[],
        );
        let mut transition = retention_candidate(0, 10_900, &["Twin Strike", "Cleave"]);
        transition.strategy_formation = Some(formation.clone());
        transition.trajectory =
            super::super::branch_experiment_trajectory::summarize_branch_trajectory_v1(
                &transition.choice_profiles,
            );
        let mut duplicate_transition =
            retention_candidate(1, 10_890, &["Twin Strike", "Pommel Strike"]);
        duplicate_transition.strategy_formation = Some(formation.clone());
        duplicate_transition.trajectory = transition.trajectory.clone();
        let block_engine = BranchRetentionCandidateInputV1 {
            index: 2,
            frontier_key: "same-frontier".to_string(),
            rank_key: 10_760,
            hp: 70,
            max_hp: 80,
            gold: 120,
            deck_count: 14,
            strategy_formation: Some(formation.clone()),
            trajectory: super::super::branch_experiment_trajectory::summarize_branch_trajectory_v1(
                &[
                    semantic_profile("Barricade", &[CardRewardSemanticRoleV1::BlockRetention]),
                    semantic_profile("Body Slam", &[CardRewardSemanticRoleV1::BlockPayoff]),
                ],
            ),
            choice_profiles: vec![
                semantic_profile("Barricade", &[CardRewardSemanticRoleV1::BlockRetention]),
                semantic_profile("Body Slam", &[CardRewardSemanticRoleV1::BlockPayoff]),
            ],
        };
        let mut other_first_pick = retention_candidate(3, 10_700, &["Shockwave", "Clash"]);
        other_first_pick.strategy_formation = Some(formation);

        let selection = select_branch_retention_portfolio_v1(
            &[
                transition,
                duplicate_transition,
                block_engine,
                other_first_pick,
            ],
            retention_config(3, Some(3)),
        );

        assert!(selection.keep_indices.contains(&0));
        assert!(!selection.keep_indices.contains(&1));
        assert!(selection.keep_indices.contains(&2));
        assert!(selection.keep_indices.contains(&3));
    }

    #[test]
    fn portfolio_budget_keeps_setup_payoff_clean_and_survival_representatives() {
        let candidates = vec![
            semantic_retention_candidate(
                0,
                10_900,
                78,
                80,
                trajectory_with(&[], &[], 3, 0, 0, 0),
                &[CardRewardSemanticRoleV1::FrontloadDamage],
            ),
            semantic_retention_candidate(
                1,
                10_890,
                77,
                80,
                trajectory_with(&["exhaust_engine"], &[], 2, 1, 0, 0),
                &[CardRewardSemanticRoleV1::ExhaustGenerator],
            ),
            semantic_retention_candidate(
                2,
                10_880,
                76,
                80,
                trajectory_with(&["exhaust_engine"], &[], 2, 1, 0, 0),
                &[CardRewardSemanticRoleV1::ExhaustGenerator],
            ),
            semantic_retention_candidate(
                3,
                10_700,
                70,
                80,
                trajectory_with(&[], &["block_engine"], 0, 0, 1, 2),
                &[CardRewardSemanticRoleV1::BlockPayoff],
            ),
            semantic_retention_candidate(
                4,
                10_650,
                72,
                80,
                trajectory_with(&[], &[], 0, 0, 0, 1),
                &[CardRewardSemanticRoleV1::Block],
            ),
            semantic_retention_candidate(
                5,
                10_600,
                50,
                80,
                trajectory_with(&["status_package"], &[], 0, 1, 0, 0),
                &[CardRewardSemanticRoleV1::StatusGenerator],
            ),
            semantic_retention_candidate(
                6,
                10_550,
                74,
                80,
                trajectory_with(&[], &["strength_scaling"], 1, 0, 1, 0),
                &[CardRewardSemanticRoleV1::StrengthPayoff],
            ),
            semantic_retention_candidate(
                7,
                10_500,
                73,
                80,
                trajectory_with(&["block_engine"], &["block_engine"], 0, 1, 1, 2),
                &[
                    CardRewardSemanticRoleV1::BlockRetention,
                    CardRewardSemanticRoleV1::BlockPayoff,
                ],
            ),
        ];

        let selection =
            select_branch_retention_portfolio_v1(&candidates, retention_config(6, Some(6)));

        assert!(
            selection.keep_indices.contains(&0),
            "survival/frontload representative"
        );
        assert!(
            selection.keep_indices.contains(&1),
            "setup-only representative"
        );
        assert!(
            !selection.keep_indices.contains(&2),
            "duplicate setup family should not displace other buckets"
        );
        assert!(selection.keep_indices.contains(&3), "payoff representative");
        assert!(
            selection.keep_indices.contains(&4),
            "clean/defense representative"
        );
        assert!(
            selection.keep_indices.contains(&6),
            "second payoff family representative"
        );
        assert!(
            selection.keep_indices.contains(&7),
            "setup+payoff engine representative"
        );
    }

    #[test]
    fn portfolio_budget_does_not_saturate_with_pure_transition_branches() {
        let candidates = vec![
            retention_candidate(0, 10_900, &["Twin Strike", "Clash"]),
            retention_candidate(1, 10_890, &["Wild Strike", "Cleave"]),
            retention_candidate(2, 10_880, &["Pommel Strike", "Sword Boomerang"]),
            semantic_retention_candidate(
                3,
                10_500,
                50,
                80,
                trajectory_with(&["exhaust_engine"], &[], 0, 1, 0, 0),
                &[CardRewardSemanticRoleV1::ExhaustGenerator],
            ),
            semantic_retention_candidate(
                4,
                10_450,
                50,
                80,
                trajectory_with(&["status_package"], &[], 0, 1, 0, 0),
                &[CardRewardSemanticRoleV1::StatusGenerator],
            ),
        ];

        let selection =
            select_branch_retention_portfolio_v1(&candidates, retention_config(3, Some(3)));

        let pure_transition_kept = selection
            .keep_indices
            .iter()
            .filter(|index| **index <= 2)
            .count();

        assert_eq!(
            pure_transition_kept, 1,
            "pure transition output should have a representative, but not saturate the budget"
        );
        assert!(
            selection.keep_indices.contains(&3),
            "an exhaust setup branch should survive short-horizon transition pressure"
        );
        assert!(
            selection.keep_indices.contains(&4),
            "a status setup branch should survive short-horizon transition pressure"
        );
    }

    #[test]
    fn package_slot_prefers_setup_and_payoff_closure_over_payoff_only() {
        let payoff_only = semantic_retention_candidate(
            0,
            10_900,
            78,
            80,
            trajectory_with(&[], &["block_engine"], 0, 0, 1, 1),
            &[CardRewardSemanticRoleV1::BlockPayoff],
        );
        let setup_and_payoff = semantic_retention_candidate(
            1,
            10_500,
            72,
            80,
            trajectory_with(&["block_engine"], &["block_engine"], 0, 1, 1, 2),
            &[
                CardRewardSemanticRoleV1::BlockRetention,
                CardRewardSemanticRoleV1::BlockPayoff,
            ],
        );

        let selection = select_branch_retention_portfolio_v1(
            &[payoff_only, setup_and_payoff],
            retention_config(1, Some(1)),
        );

        assert!(
            selection.keep_indices.contains(&1),
            "a branch with both setup and payoff should be the package representative"
        );
    }

    #[test]
    fn portfolio_records_the_lane_that_selected_each_kept_branch() {
        let package_closure = semantic_retention_candidate(
            0,
            10_300,
            58,
            80,
            trajectory_with(&["block_engine"], &["block_engine"], 0, 1, 1, 2),
            &[
                CardRewardSemanticRoleV1::BlockRetention,
                CardRewardSemanticRoleV1::BlockPayoff,
            ],
        );
        let high_hp_payoff = semantic_retention_candidate(
            1,
            10_900,
            79,
            80,
            trajectory_with(&[], &["generic_package"], 1, 0, 1, 0),
            &[CardRewardSemanticRoleV1::PackagePayoff],
        );

        let selection = select_branch_retention_portfolio_v1(
            &[package_closure, high_hp_payoff],
            BranchRetentionConfigV1 {
                max_total: 2,
                max_per_frontier: Some(2),
                budget_profile: BranchRetentionBudgetProfileV1::Survival,
            },
        );

        assert_eq!(
            selection.decisions_by_index[&0].selected_by_slot,
            Some(BranchRetentionSlotV1::DefenseEngine)
        );
        assert_eq!(
            selection.decisions_by_index[&0].primary_slot,
            BranchRetentionSlotV1::Package,
            "a branch can be semantically package-shaped while being retained by the survival profile's defense lane"
        );
        assert_eq!(
            selection.decisions_by_index[&1].primary_slot,
            BranchRetentionSlotV1::Package,
            "candidate identity still records the highest semantic slot it qualifies for"
        );
        assert_eq!(
            selection.decisions_by_index[&1].selected_by_slot,
            Some(BranchRetentionSlotV1::Survival),
            "portfolio reporting should say this branch consumed the survival lane, not another package lane"
        );
    }

    #[test]
    fn survival_profile_prioritizes_survival_defense_and_frontload_lanes() {
        let candidates = vec![
            semantic_retention_candidate(
                0,
                10_900,
                50,
                80,
                trajectory_with(&["exhaust_engine"], &["exhaust_engine"], 0, 1, 1, 0),
                &[
                    CardRewardSemanticRoleV1::ExhaustGenerator,
                    CardRewardSemanticRoleV1::ExhaustPayoff,
                ],
            ),
            semantic_retention_candidate(
                1,
                10_850,
                50,
                80,
                trajectory_with(&["status_package"], &[], 0, 1, 0, 0),
                &[CardRewardSemanticRoleV1::StatusGenerator],
            ),
            semantic_retention_candidate(
                2,
                10_800,
                50,
                80,
                trajectory_with(&[], &[], 0, 0, 0, 2),
                &[CardRewardSemanticRoleV1::Weak],
            ),
            semantic_retention_candidate(
                3,
                10_700,
                79,
                80,
                trajectory_with(&[], &[], 0, 0, 0, 0),
                &[],
            ),
            semantic_retention_candidate(
                4,
                10_600,
                50,
                80,
                trajectory_with(&[], &[], 2, 0, 0, 0),
                &[CardRewardSemanticRoleV1::FrontloadDamage],
            ),
        ];

        let selection = select_branch_retention_portfolio_v1(
            &candidates,
            BranchRetentionConfigV1 {
                max_total: 3,
                max_per_frontier: Some(3),
                budget_profile: BranchRetentionBudgetProfileV1::Survival,
            },
        );
        let lanes = selected_lanes(&selection);

        assert_eq!(
            lanes,
            vec![
                BranchRetentionSlotV1::DefenseEngine,
                BranchRetentionSlotV1::Survival,
                BranchRetentionSlotV1::Frontload,
            ],
            "survival profile should spend its small budget on immediate safety lanes before long-horizon setup"
        );
    }

    #[test]
    fn package_profile_prioritizes_package_engine_and_scaling_lanes() {
        let candidates = vec![
            semantic_retention_candidate(
                0,
                10_900,
                79,
                80,
                trajectory_with(&[], &[], 0, 0, 0, 0),
                &[],
            ),
            semantic_retention_candidate(
                1,
                10_800,
                60,
                80,
                trajectory_with(&[], &[], 2, 0, 0, 0),
                &[CardRewardSemanticRoleV1::FrontloadDamage],
            ),
            semantic_retention_candidate(
                2,
                10_700,
                60,
                80,
                trajectory_with(&["block_engine"], &["block_engine"], 0, 1, 1, 2),
                &[
                    CardRewardSemanticRoleV1::BlockRetention,
                    CardRewardSemanticRoleV1::BlockPayoff,
                ],
            ),
            semantic_retention_candidate(
                3,
                10_600,
                60,
                80,
                trajectory_with(&["exhaust_engine"], &[], 0, 1, 0, 0),
                &[CardRewardSemanticRoleV1::ExhaustGenerator],
            ),
            semantic_retention_candidate(
                4,
                10_500,
                60,
                80,
                trajectory_with(&[], &["strength_scaling"], 0, 0, 1, 0),
                &[CardRewardSemanticRoleV1::StrengthPayoff],
            ),
        ];

        let selection = select_branch_retention_portfolio_v1(
            &candidates,
            BranchRetentionConfigV1 {
                max_total: 3,
                max_per_frontier: Some(3),
                budget_profile: BranchRetentionBudgetProfileV1::Package,
            },
        );
        let lanes = selected_lanes(&selection);

        assert_eq!(
            lanes,
            vec![
                BranchRetentionSlotV1::Package,
                BranchRetentionSlotV1::EngineSetup,
                BranchRetentionSlotV1::Scaling,
            ],
            "package profile should preserve long-horizon package structure before short-term safety fillers"
        );
    }

    #[test]
    fn setup_only_branch_gets_engine_setup_retention_slot() {
        let setup_only = semantic_retention_candidate(
            0,
            10_500,
            70,
            80,
            trajectory_with(&["exhaust_engine"], &[], 0, 1, 0, 0),
            &[CardRewardSemanticRoleV1::ExhaustGenerator],
        );

        let decision = decide_branch_retention_v1(&setup_only);

        assert_eq!(decision.primary_slot, BranchRetentionSlotV1::EngineSetup);
        assert!(
            decision.slots.contains(&BranchRetentionSlotV1::EngineSetup),
            "setup-only engine branches should be preserved by an explicit long-horizon slot"
        );
    }

    #[test]
    fn retention_slots_come_from_semantic_profiles_not_card_names() {
        let candidate = BranchRetentionCandidateInputV1 {
            index: 0,
            frontier_key: "same-frontier".to_string(),
            rank_key: 10_000,
            hp: 70,
            max_hp: 80,
            gold: 120,
            deck_count: 12,
            strategy_formation: None,
            trajectory: super::super::branch_experiment_trajectory::summarize_branch_trajectory_v1(
                &[semantic_profile(
                    "Unfamiliar Card Name",
                    &[CardRewardSemanticRoleV1::BlockPayoff],
                )],
            ),
            choice_profiles: vec![semantic_profile(
                "Unfamiliar Card Name",
                &[CardRewardSemanticRoleV1::BlockPayoff],
            )],
        };

        let decision = decide_branch_retention_v1(&candidate);

        assert!(decision.slots.contains(&BranchRetentionSlotV1::Package));
        assert!(decision
            .slots
            .contains(&BranchRetentionSlotV1::DefenseEngine));
        assert!(!decision.slots.contains(&BranchRetentionSlotV1::Frontload));
    }

    fn retention_candidate(
        index: usize,
        rank_key: i32,
        choice_labels: &[&str],
    ) -> BranchRetentionCandidateInputV1 {
        let choice_profiles = choice_labels
            .iter()
            .map(|label| semantic_profile(label, &[CardRewardSemanticRoleV1::FrontloadDamage]))
            .collect::<Vec<_>>();
        let trajectory = super::super::branch_experiment_trajectory::summarize_branch_trajectory_v1(
            &choice_profiles,
        );
        BranchRetentionCandidateInputV1 {
            index,
            frontier_key: "same-frontier".to_string(),
            rank_key,
            hp: 78,
            max_hp: 80,
            gold: 120,
            deck_count: 14,
            strategy_formation: None,
            trajectory,
            choice_profiles,
        }
    }

    fn semantic_profile(
        name: &str,
        roles: &[CardRewardSemanticRoleV1],
    ) -> CardRewardSemanticProfileV1 {
        CardRewardSemanticProfileV1 {
            card: crate::content::cards::CardId::Strike,
            name: name.to_string(),
            roles: roles.to_vec(),
            dependencies: Vec::new(),
            unsupported_mechanics: Vec::new(),
        }
    }

    fn semantic_retention_candidate(
        index: usize,
        rank_key: i32,
        hp: i32,
        max_hp: i32,
        trajectory: BranchTrajectorySignatureV1,
        roles: &[CardRewardSemanticRoleV1],
    ) -> BranchRetentionCandidateInputV1 {
        BranchRetentionCandidateInputV1 {
            index,
            frontier_key: "same-frontier".to_string(),
            rank_key,
            hp,
            max_hp,
            gold: 120,
            deck_count: 14,
            strategy_formation: Some(formation(
                StrategyDeckFormationStageV1::PlanSeeded,
                &[StrategyDeckFormationNeedV1::Scaling],
                &[],
            )),
            trajectory,
            choice_profiles: vec![semantic_profile("Semantic Candidate", roles)],
        }
    }

    fn named_semantic_retention_candidate(
        index: usize,
        rank_key: i32,
        first_pick: &str,
        trajectory: BranchTrajectorySignatureV1,
        roles: &[CardRewardSemanticRoleV1],
    ) -> BranchRetentionCandidateInputV1 {
        let mut candidate =
            semantic_retention_candidate(index, rank_key, 78, 80, trajectory, roles);
        candidate.choice_profiles[0].name = first_pick.to_string();
        candidate
    }

    fn trajectory_with(
        setup_keys: &[&str],
        package_keys: &[&str],
        transition_frontload_picks: u8,
        engine_generator_picks: u8,
        engine_payoff_picks: u8,
        defense_picks: u8,
    ) -> BranchTrajectorySignatureV1 {
        BranchTrajectorySignatureV1 {
            frontload_picks: transition_frontload_picks,
            transition_frontload_picks,
            scaling_picks: engine_payoff_picks,
            defense_picks,
            engine_generator_picks,
            engine_payoff_picks,
            draw_energy_picks: 0,
            setup_keys: setup_keys.iter().map(|key| key.to_string()).collect(),
            package_keys: package_keys.iter().map(|key| key.to_string()).collect(),
        }
    }

    fn formation(
        stage: StrategyDeckFormationStageV1,
        needs: &[StrategyDeckFormationNeedV1],
        strengths: &[StrategyPackageIdV2],
    ) -> StrategyFormationSummaryV2 {
        StrategyFormationSummaryV2 {
            stage,
            needs: needs.to_vec(),
            strengths: strengths.to_vec(),
        }
    }

    fn selected_lanes(selection: &BranchRetentionSelectionV1) -> Vec<BranchRetentionSlotV1> {
        selection
            .decisions_by_index
            .values()
            .filter_map(|decision| decision.selected_by_slot)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn retention_config(
        max_total: usize,
        max_per_frontier: Option<usize>,
    ) -> BranchRetentionConfigV1 {
        BranchRetentionConfigV1 {
            max_total,
            max_per_frontier,
            budget_profile: BranchRetentionBudgetProfileV1::Balanced,
        }
    }
}
