use std::collections::{BTreeMap, BTreeSet};

use crate::ai::card_component_marginal_value_v1::{
    evaluate_card_component_marginal_value_v1, CardComponentMarginalContextV1,
    CardComponentMarginalVerdictV1,
};
use crate::ai::card_reward_policy_v1::{CardRewardSemanticProfileV1, CardRewardSemanticRoleV1};
use crate::ai::deck_startup_profile_v1::DeckStartupProfileV1;
use crate::ai::noncombat_strategy_v1::StrategyFormationSummaryV2;
use crate::ai::strategic::{BranchSignature, RetentionBucket};
use crate::eval::branch_experiment_trajectory::{
    branch_trajectory_family_key_v1, BranchTrajectorySignatureV1,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

mod context_packet;
mod effect_coverage;

use context_packet::{
    branch_retention_context_packet_v2, context_score, BranchRetentionContextKeyV2,
    BranchRetentionContextPacketV2,
};
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
    pub act: u8,
    pub floor: i32,
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
    pub lineage_flags: Vec<String>,
    pub startup: DeckStartupProfileV1,
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
    #[serde(default)]
    pub strategic_signature: BranchSignature,
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
        strategic_signature: BranchSignature::default(),
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
    let context = branch_retention_context_packet_v2(candidate);
    let component_context = component_marginal_context_for_retention_candidate(candidate);
    let startup_rejected = added_cards_have_startup_liability(candidate);
    let component_negative =
        added_cards_have_negative_component_margin(candidate, &component_context);

    if has_package_candidate(&candidate.choice_profiles) && !startup_rejected && !component_negative
    {
        slots.push(BranchRetentionSlotV1::Package);
        reasons.push("contains a semantic package payoff candidate".to_string());
    }
    if complete_package_count(&candidate.trajectory) > 0 && !startup_rejected && !component_negative
    {
        slots.push(BranchRetentionSlotV1::Package);
        reasons.push("contains both setup and payoff for a trajectory package".to_string());
    }
    if has_engine_setup(&candidate.trajectory) && !startup_rejected && !component_negative {
        slots.push(BranchRetentionSlotV1::EngineSetup);
        reasons.push("contains a long-horizon engine or package setup seed".to_string());
    }
    if !startup_rejected
        && !component_negative
        && (context
            .keys
            .contains(&BranchRetentionContextKeyV2::ClosesPackage)
            || context
                .keys
                .contains(&BranchRetentionContextKeyV2::SupportsCommittedPackage))
    {
        slots.push(BranchRetentionSlotV1::Package);
        reasons.push("context: closes or supports an active package".to_string());
    }
    if !startup_rejected
        && !component_negative
        && context
            .keys
            .contains(&BranchRetentionContextKeyV2::OpensPackageSetup)
    {
        slots.push(BranchRetentionSlotV1::EngineSetup);
        reasons.push("context: opens a setup path worth carrying forward".to_string());
    }
    if !startup_rejected
        && !component_negative
        && candidate
            .choice_profiles
            .iter()
            .any(|profile| profile_has_any_role(profile, SCALING_ROLES))
    {
        slots.push(BranchRetentionSlotV1::Scaling);
        reasons.push("contains long-run scaling or engine setup".to_string());
    }
    if startup_rejected {
        reasons.push("startup profile rejects at least one added card".to_string());
    }
    if component_negative {
        reasons
            .push("component marginal value is negative for at least one added card".to_string());
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
    if context
        .keys
        .contains(&BranchRetentionContextKeyV2::ImmediateSafetyPatch)
    {
        slots.push(BranchRetentionSlotV1::Survival);
        reasons.push("context: patches low-hp or route pressure".to_string());
    }
    let deck_bloat_pressure = deck_bloat_pressure_high(candidate);
    let immediate_safety_patch = context
        .keys
        .contains(&BranchRetentionContextKeyV2::ImmediateSafetyPatch);

    if candidate.choice_profiles.iter().any(|profile| {
        profile
            .roles
            .contains(&CardRewardSemanticRoleV1::FrontloadDamage)
    }) && frontload_lane_allowed(
        candidate,
        &context,
        component_negative,
        immediate_safety_patch,
    ) {
        slots.push(BranchRetentionSlotV1::Frontload);
        reasons.push("contains immediate combat output".to_string());
    }
    if !startup_rejected
        && !component_negative
        && context
            .keys
            .contains(&BranchRetentionContextKeyV2::MatchesFormationFrontloadNeed)
    {
        slots.push(BranchRetentionSlotV1::Frontload);
        reasons.push("context: matches current frontload need".to_string());
    }
    if context
        .keys
        .contains(&BranchRetentionContextKeyV2::MatchesFormationBlockNeed)
    {
        slots.push(BranchRetentionSlotV1::DefenseEngine);
        reasons.push("context: matches current block or mitigation need".to_string());
    }
    if !startup_rejected
        && !component_negative
        && context
            .keys
            .contains(&BranchRetentionContextKeyV2::MatchesFormationScalingNeed)
    {
        slots.push(BranchRetentionSlotV1::Scaling);
        reasons.push("context: matches current scaling need".to_string());
    }
    if !startup_rejected
        && !component_negative
        && context
            .keys
            .contains(&BranchRetentionContextKeyV2::MatchesFormationDrawEnergyNeed)
    {
        slots.push(BranchRetentionSlotV1::EngineSetup);
        reasons.push("context: matches current draw/energy need".to_string());
    }
    let is_plain_skip = is_plain_card_reward_skip(candidate);
    if is_plain_skip && deck_bloat_pressure {
        slots.push(BranchRetentionSlotV1::CleanDeck);
        reasons.push("declines adding a card while deck size pressure is high".to_string());
    } else if !is_plain_skip
        && !deck_bloat_pressure
        && transition_attack_count(&candidate.choice_profiles) <= 1
    {
        slots.push(BranchRetentionSlotV1::CleanDeck);
        reasons.push("keeps transition-card bloat lower".to_string());
    }
    if context
        .keys
        .contains(&BranchRetentionContextKeyV2::MatchesFormationConsistencyNeed)
        && !is_plain_skip
    {
        slots.push(BranchRetentionSlotV1::CleanDeck);
        reasons.push("context: matches current consistency need".to_string());
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
        strategic_signature: strategic_signature_for_retention_candidate(
            candidate,
            &slots,
            startup_rejected,
            component_negative,
        ),
        slots,
        reasons,
    }
}

fn strategic_signature_for_retention_candidate(
    candidate: &BranchRetentionCandidateInputV1,
    slots: &[BranchRetentionSlotV1],
    startup_rejected: bool,
    component_negative: bool,
) -> BranchSignature {
    let mut buckets = Vec::new();
    if has_any_slot(
        slots,
        &[
            BranchRetentionSlotV1::Survival,
            BranchRetentionSlotV1::Frontload,
        ],
    ) {
        push_retention_bucket(&mut buckets, RetentionBucket::BestImmediateSurvival);
    }
    if has_any_slot(
        slots,
        &[
            BranchRetentionSlotV1::DefenseEngine,
            BranchRetentionSlotV1::Scaling,
        ],
    ) {
        push_retention_bucket(&mut buckets, RetentionBucket::BestBossPrepared);
    }
    if slots.contains(&BranchRetentionSlotV1::CleanDeck) {
        push_retention_bucket(&mut buckets, RetentionBucket::BestCleanDeck);
    }
    if has_any_slot(
        slots,
        &[
            BranchRetentionSlotV1::Package,
            BranchRetentionSlotV1::EngineSetup,
            BranchRetentionSlotV1::Scaling,
        ],
    ) {
        push_retention_bucket(&mut buckets, RetentionBucket::BestCoreEngine);
    }
    if has_resource_conversion_signal(candidate) {
        push_retention_bucket(&mut buckets, RetentionBucket::BestResourceConverted);
    }
    if slots.contains(&BranchRetentionSlotV1::Diversity) {
        push_retention_bucket(&mut buckets, RetentionBucket::BestHighVariance);
    }

    let defense_signals =
        count_profiles_with_any_role(&candidate.choice_profiles, DEFENSE_ENGINE_ROLES)
            + i32::from(candidate.trajectory.defense_picks > 0);
    let scaling_signals = count_profiles_with_any_role(&candidate.choice_profiles, SCALING_ROLES)
        + i32::from(candidate.trajectory.scaling_picks > 0)
        + complete_package_count(&candidate.trajectory);
    let engine_signals = candidate.trajectory.setup_keys.len() as i32
        + i32::from(candidate.trajectory.engine_generator_picks > 0)
        + i32::from(candidate.trajectory.draw_energy_picks > 0)
        + complete_package_count(&candidate.trajectory);
    let transition_attacks = transition_attack_count(&candidate.choice_profiles);
    let hp_is_safe = candidate.max_hp > 0 && candidate.hp * 100 >= candidate.max_hp * 60;
    let deck_bloat = candidate.deck_count.saturating_sub(24) as i32;
    let setup_debt_count = startup_debt_count(candidate, startup_rejected, component_negative);
    let package_coherence_count = complete_package_count(&candidate.trajectory)
        + i32::from(slots.contains(&BranchRetentionSlotV1::Package))
        + i32::from(
            candidate
                .strategy_formation
                .as_ref()
                .is_some_and(|formation| !formation.strengths.is_empty()),
        );

    BranchSignature {
        boss_readiness: bounded_signal(
            defense_signals + scaling_signals + i32::from(hp_is_safe),
            5,
        ),
        clean_score: if slots.contains(&BranchRetentionSlotV1::CleanDeck) {
            1.0
        } else if transition_attacks <= 1 && !component_negative {
            0.5
        } else {
            0.0
        },
        engine_score: bounded_signal(engine_signals, 4),
        cycle_debt: bounded_signal(deck_bloat / 4 + transition_attacks, 5),
        setup_debt: bounded_signal(setup_debt_count, 5),
        economy_conversion: if has_resource_conversion_signal(candidate) {
            1.0
        } else {
            0.0
        },
        package_coherence: bounded_signal(package_coherence_count, 3),
        buckets,
    }
}

fn has_any_slot(slots: &[BranchRetentionSlotV1], targets: &[BranchRetentionSlotV1]) -> bool {
    targets.iter().any(|target| slots.contains(target))
}

fn push_retention_bucket(buckets: &mut Vec<RetentionBucket>, bucket: RetentionBucket) {
    if !buckets.contains(&bucket) {
        buckets.push(bucket);
    }
}

fn bounded_signal(value: i32, full_scale: i32) -> f32 {
    if full_scale <= 0 {
        return 0.0;
    }
    (value.max(0) as f32 / full_scale as f32).min(1.0)
}

fn startup_debt_count(
    candidate: &BranchRetentionCandidateInputV1,
    startup_rejected: bool,
    component_negative: bool,
) -> i32 {
    i32::from(startup_rejected)
        + i32::from(component_negative)
        + i32::from(candidate.startup.has_setup_debt_high_payment_low)
        + i32::from(candidate.startup.has_fnp_duplicate_without_exhaust_engine)
        + i32::from(candidate.startup.has_dual_wield_without_target)
        + i32::from(candidate.startup.has_anger_duplicate_without_digest)
        + i32::from(candidate.startup.has_strength_payoff_without_strength)
        + i32::from(candidate.startup.has_rupture_without_self_damage)
        + i32::from(candidate.startup.has_armaments_unupgraded_duplicate)
        + i32::from(candidate.startup.has_pyramid_unupgraded_apparition)
}

fn has_resource_conversion_signal(candidate: &BranchRetentionCandidateInputV1) -> bool {
    candidate
        .choice_effect_keys
        .iter()
        .chain(candidate.lineage_flags.iter())
        .any(|key| {
            let key = key.to_ascii_lowercase();
            key.contains("shop")
                || key.contains("buy")
                || key.contains("purchase")
                || key.contains("gold")
                || key.contains("remove")
                || key.contains("potion")
                || key.contains("relic")
        })
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
    let selected = cap_payoff_only_package_saturation(candidates, positions, selected, limit);
    let selected = preserve_late_clean_branch(candidates, positions, selected, limit);
    let selected =
        effect_coverage::preserve_choice_effect_coverage(candidates, positions, selected, limit);
    let selected =
        effect_coverage::preserve_lineage_flag_coverage(candidates, positions, selected, limit);
    let selected =
        drop_excess_first_pick_prefixes_after_coverage(candidates, positions, selected, limit);
    drop_excess_payoff_only_package_after_coverage(candidates, positions, selected, limit)
}

fn preserve_late_clean_branch(
    candidates: &[BranchRetentionCandidateInputV1],
    available_positions: &[usize],
    selected_picks: Vec<BranchRetentionLanePick>,
    limit: usize,
) -> Vec<BranchRetentionLanePick> {
    if limit < 2
        || !available_positions
            .iter()
            .any(|position| late_clean_retention_applies(&candidates[*position]))
        || selected_picks
            .iter()
            .any(|pick| candidate_has_clean_deck_slot(candidates, pick.position))
    {
        return selected_picks;
    }

    let Some(clean_position) = available_positions
        .iter()
        .copied()
        .filter(|position| candidate_has_clean_deck_slot(candidates, *position))
        .max_by(|left, right| {
            compare_slot_score(candidates, *left, *right, BranchRetentionSlotV1::CleanDeck)
        })
    else {
        return selected_picks;
    };

    let mut selected = selected_picks
        .iter()
        .map(|pick| pick.position)
        .collect::<BTreeSet<_>>();
    if !selected.insert(clean_position) {
        return selected_picks;
    }

    let mut kept = selected_picks;
    if kept.len() >= limit {
        if let Some(remove_index) = kept
            .iter()
            .enumerate()
            .filter(|(_, pick)| pick.selected_by_slot != BranchRetentionSlotV1::CleanDeck)
            .min_by_key(|(_, pick)| candidates[pick.position].rank_key)
            .map(|(index, _)| index)
        {
            kept.remove(remove_index);
        } else {
            return kept;
        }
    }
    kept.push(BranchRetentionLanePick {
        position: clean_position,
        selected_by_slot: BranchRetentionSlotV1::CleanDeck,
    });
    kept
}

fn late_clean_retention_applies(candidate: &BranchRetentionCandidateInputV1) -> bool {
    (candidate.act >= 3 || candidate.act == 2 && candidate.floor >= 24)
        && candidate.max_hp > 0
        && candidate.hp * 100 >= candidate.max_hp * 35
}

fn candidate_has_clean_deck_slot(
    candidates: &[BranchRetentionCandidateInputV1],
    position: usize,
) -> bool {
    let candidate = &candidates[position];
    decide_branch_retention_v1(candidate)
        .slots
        .contains(&BranchRetentionSlotV1::CleanDeck)
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
    let left_takes_card = !is_plain_card_reward_skip(&candidates[left]);
    let right_takes_card = !is_plain_card_reward_skip(&candidates[right]);
    effective_branch_retention_rank_key_v1(&candidates[left])
        .cmp(&effective_branch_retention_rank_key_v1(&candidates[right]))
        .then_with(|| candidates[left].hp.cmp(&candidates[right].hp))
        .then_with(|| candidates[left].gold.cmp(&candidates[right].gold))
        .then_with(|| left_takes_card.cmp(&right_takes_card))
        .then_with(|| {
            candidates[right]
                .deck_count
                .cmp(&candidates[left].deck_count)
        })
        .then_with(|| candidates[right].index.cmp(&candidates[left].index))
}

pub fn effective_branch_retention_rank_key_v1(candidate: &BranchRetentionCandidateInputV1) -> i32 {
    let mut rank = candidate.rank_key;
    let component_context = component_marginal_context_for_retention_candidate(candidate);
    if added_cards_have_startup_liability(candidate) {
        rank = rank.saturating_sub(50_000);
    }
    rank = rank.saturating_add(component_margin_rank_adjustment(
        candidate,
        &component_context,
    ));
    rank
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
    let cap = if base >= 2 {
        base.saturating_add(1)
    } else {
        base
    };
    cap.min(4)
}

fn drop_excess_first_pick_prefixes_after_coverage(
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
    let protected_counts = protected_coverage_counts(candidates, &selected_picks);
    let mut prefix_counts = BTreeMap::<String, usize>::new();
    let mut kept = Vec::new();

    for pick in selected_picks {
        let candidate = &candidates[pick.position];
        let prefix = choice_prefix_key(candidate);
        let count = prefix_counts.entry(prefix).or_default();
        if *count >= max_per_prefix
            && !candidate_has_unique_protected_coverage(candidate, &protected_counts)
        {
            continue;
        }
        *count += 1;
        kept.push(pick);
    }
    kept
}

fn protected_coverage_counts(
    candidates: &[BranchRetentionCandidateInputV1],
    selected_picks: &[BranchRetentionLanePick],
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for pick in selected_picks {
        for key in protected_coverage_keys(&candidates[pick.position]) {
            *counts.entry(key).or_default() += 1;
        }
    }
    counts
}

fn candidate_has_unique_protected_coverage(
    candidate: &BranchRetentionCandidateInputV1,
    protected_counts: &BTreeMap<String, usize>,
) -> bool {
    protected_coverage_keys(candidate)
        .iter()
        .any(|key| protected_counts.get(key).copied().unwrap_or_default() == 1)
}

fn protected_coverage_keys(candidate: &BranchRetentionCandidateInputV1) -> Vec<String> {
    let mut keys = candidate
        .choice_effect_keys
        .iter()
        .map(|key| format!("effect:{key}"))
        .collect::<Vec<_>>();
    keys.extend(
        candidate
            .lineage_flags
            .iter()
            .map(|flag| format!("lineage:{flag}")),
    );
    keys.extend(
        candidate
            .trajectory
            .setup_keys
            .iter()
            .map(|key| format!("setup:{key}")),
    );
    keys.extend(
        candidate
            .trajectory
            .package_keys
            .iter()
            .map(|key| format!("package:{key}")),
    );
    keys
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

fn cap_payoff_only_package_saturation(
    candidates: &[BranchRetentionCandidateInputV1],
    available_positions: &[usize],
    selected_picks: Vec<BranchRetentionLanePick>,
    limit: usize,
) -> Vec<BranchRetentionLanePick> {
    if !available_positions
        .iter()
        .any(|position| !is_payoff_only_package_branch(&candidates[*position]))
    {
        return selected_picks;
    }

    let max_payoff_only = payoff_only_package_branch_cap(candidates, available_positions, limit);
    let mut payoff_only_count = 0usize;
    let mut capped = false;
    let mut kept = Vec::new();

    for pick in selected_picks {
        if is_payoff_only_package_branch(&candidates[pick.position]) {
            if payoff_only_count >= max_payoff_only {
                capped = true;
                continue;
            }
            payoff_only_count += 1;
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
        let allow_more_payoff_only = payoff_only_count < max_payoff_only;
        let Some(position) =
            best_fill_position_allowed(candidates, available_positions, &selected, |position| {
                allow_more_payoff_only || !is_payoff_only_package_branch(&candidates[position])
            })
        else {
            break;
        };

        if is_payoff_only_package_branch(&candidates[position]) {
            payoff_only_count += 1;
        }
        selected.insert(position);
        kept.push(BranchRetentionLanePick {
            position,
            selected_by_slot: BranchRetentionSlotV1::Diversity,
        });
    }

    kept
}

fn payoff_only_package_branch_cap(
    candidates: &[BranchRetentionCandidateInputV1],
    available_positions: &[usize],
    limit: usize,
) -> usize {
    if !available_positions
        .iter()
        .any(|position| has_committed_package_context(&candidates[*position]))
    {
        return 1;
    }
    let distinct_payoff_packages = available_positions
        .iter()
        .filter(|position| is_payoff_only_package_branch(&candidates[**position]))
        .map(|position| candidates[*position].trajectory.package_keys.join("+"))
        .collect::<BTreeSet<_>>()
        .len();
    let non_payoff_families = available_positions
        .iter()
        .filter(|position| !is_payoff_only_package_branch(&candidates[**position]))
        .map(|position| branch_family_key(&candidates[*position]))
        .collect::<BTreeSet<_>>();
    limit
        .saturating_sub(non_payoff_families.len())
        .max(distinct_payoff_packages)
        .max(1)
        .min(3)
}

fn has_committed_package_context(candidate: &BranchRetentionCandidateInputV1) -> bool {
    candidate
        .strategy_formation
        .as_ref()
        .is_some_and(|formation| !formation.strengths.is_empty())
        || complete_package_count(&candidate.trajectory) > 0
}

fn drop_excess_payoff_only_package_after_coverage(
    candidates: &[BranchRetentionCandidateInputV1],
    available_positions: &[usize],
    selected_picks: Vec<BranchRetentionLanePick>,
    limit: usize,
) -> Vec<BranchRetentionLanePick> {
    if !available_positions
        .iter()
        .any(|position| !is_payoff_only_package_branch(&candidates[*position]))
    {
        return selected_picks;
    }

    let max_payoff_only = payoff_only_package_branch_cap(candidates, available_positions, limit);
    let protected_counts = protected_coverage_counts(candidates, &selected_picks);
    let mut payoff_only_count = 0usize;
    let mut kept = Vec::new();
    for pick in selected_picks {
        let candidate = &candidates[pick.position];
        if is_payoff_only_package_branch(candidate) {
            if payoff_only_count >= max_payoff_only
                && !candidate_has_unique_protected_coverage(candidate, &protected_counts)
            {
                continue;
            }
            payoff_only_count += 1;
        }
        kept.push(pick);
    }
    kept
}

fn is_payoff_only_package_branch(candidate: &BranchRetentionCandidateInputV1) -> bool {
    !candidate.trajectory.package_keys.is_empty()
        && candidate.trajectory.setup_keys.is_empty()
        && candidate.trajectory.engine_generator_picks == 0
        && candidate.trajectory.draw_energy_picks == 0
}

fn slot_score(candidate: &BranchRetentionCandidateInputV1, slot: BranchRetentionSlotV1) -> i32 {
    let context = branch_retention_context_packet_v2(candidate);
    match slot {
        BranchRetentionSlotV1::Package => {
            package_score(&candidate.choice_profiles) * 10_000
                + complete_package_count(&candidate.trajectory) * 25_000
                + context_score(
                    &context,
                    &[
                        BranchRetentionContextKeyV2::ClosesPackage,
                        BranchRetentionContextKeyV2::SupportsCommittedPackage,
                    ],
                ) * 20_000
                + candidate.hp * 10
        }
        BranchRetentionSlotV1::EngineSetup => {
            candidate.trajectory.setup_keys.len() as i32 * 10_000
                + candidate.trajectory.engine_generator_picks as i32 * 5_000
                + candidate.trajectory.draw_energy_picks as i32 * 2_500
                + context_score(
                    &context,
                    &[
                        BranchRetentionContextKeyV2::OpensPackageSetup,
                        BranchRetentionContextKeyV2::MatchesFormationDrawEnergyNeed,
                    ],
                ) * 20_000
                + candidate.hp * 10
        }
        BranchRetentionSlotV1::Scaling => {
            count_profiles_with_any_role(&candidate.choice_profiles, SCALING_ROLES) * 10_000
                + context_score(
                    &context,
                    &[BranchRetentionContextKeyV2::MatchesFormationScalingNeed],
                ) * 20_000
                + candidate.hp * 10
        }
        BranchRetentionSlotV1::DefenseEngine => {
            count_profiles_with_any_role(&candidate.choice_profiles, DEFENSE_ENGINE_ROLES) * 10_000
                + context_score(
                    &context,
                    &[
                        BranchRetentionContextKeyV2::MatchesFormationBlockNeed,
                        BranchRetentionContextKeyV2::ImmediateSafetyPatch,
                    ],
                ) * 20_000
                + candidate.hp * 10
        }
        BranchRetentionSlotV1::Survival => {
            candidate.hp * 100
                + candidate.gold
                + context_score(
                    &context,
                    &[BranchRetentionContextKeyV2::ImmediateSafetyPatch],
                ) * 1_000
        }
        BranchRetentionSlotV1::Frontload => {
            count_profiles_with_role(
                &candidate.choice_profiles,
                CardRewardSemanticRoleV1::FrontloadDamage,
            ) * 10_000
                + context_score(
                    &context,
                    &[BranchRetentionContextKeyV2::MatchesFormationFrontloadNeed],
                ) * 20_000
                + candidate.hp * 10
        }
        BranchRetentionSlotV1::CleanDeck => {
            if is_plain_card_reward_skip(candidate) {
                candidate.deck_count as i32 * 500 + candidate.hp * 10
            } else {
                -transition_attack_count(&candidate.choice_profiles) * 10_000
                    - candidate.deck_count as i32 * 100
                    + context_score(
                        &context,
                        &[BranchRetentionContextKeyV2::MatchesFormationConsistencyNeed],
                    ) * 20_000
                    + candidate.hp * 10
            }
        }
        BranchRetentionSlotV1::Diversity => -(candidate.index as i32),
    }
}

fn component_marginal_context_for_retention_candidate(
    candidate: &BranchRetentionCandidateInputV1,
) -> CardComponentMarginalContextV1 {
    let frontload_jobs = count_profiles_with_any_role(
        &candidate.choice_profiles,
        &[
            CardRewardSemanticRoleV1::FrontloadDamage,
            CardRewardSemanticRoleV1::AoeDamage,
        ],
    ) as usize;
    let block_jobs = count_profiles_with_any_role(
        &candidate.choice_profiles,
        &[
            CardRewardSemanticRoleV1::Block,
            CardRewardSemanticRoleV1::Weak,
            CardRewardSemanticRoleV1::EnemyStrengthDown,
        ],
    ) as usize;
    CardComponentMarginalContextV1 {
        act: candidate.act,
        floor: candidate.floor,
        boss: None,
        hp: candidate.hp,
        max_hp: candidate.max_hp,
        deck_size: candidate.deck_count,
        powers: candidate.startup.setup_debt as usize,
        draw_sources: candidate.startup.strong_draw_count as usize,
        exhaust_generators: candidate.startup.exhaust_engine_count as usize,
        frontload_jobs,
        block_jobs,
        formation_needs: candidate
            .strategy_formation
            .as_ref()
            .map(|formation| formation.needs.clone())
            .unwrap_or_default(),
        startup: candidate.startup.clone(),
    }
}

fn added_cards_have_startup_liability(candidate: &BranchRetentionCandidateInputV1) -> bool {
    if candidate.choice_profiles.is_empty() {
        return false;
    }
    let risky_startup = candidate.startup.has_setup_debt_high_payment_low
        || candidate.startup.has_fnp_duplicate_without_exhaust_engine
        || candidate.startup.has_dual_wield_without_target
        || candidate.startup.has_anger_duplicate_without_digest
        || candidate.startup.has_strength_payoff_without_strength
        || candidate.startup.has_rupture_without_self_damage
        || candidate.startup.has_armaments_unupgraded_duplicate
        || candidate.startup.has_pyramid_unupgraded_apparition;
    let profile_requires_missing_startup = candidate.choice_profiles.iter().any(|profile| {
        (profile
            .roles
            .contains(&CardRewardSemanticRoleV1::StrengthPayoff)
            && candidate.startup.persistent_strength_source_count == 0)
            || (profile
                .roles
                .contains(&CardRewardSemanticRoleV1::SelfDamagePayoff)
                && candidate.startup.self_damage_source_count == 0)
            || (profile
                .roles
                .contains(&CardRewardSemanticRoleV1::ExhaustPayoff)
                && candidate.startup.exhaust_engine_count == 0)
    });
    profile_requires_missing_startup
        || (risky_startup
            && candidate
                .trajectory
                .engine_generator_picks
                .saturating_add(candidate.trajectory.engine_payoff_picks)
                > candidate.trajectory.draw_energy_picks)
}

fn frontload_lane_allowed(
    candidate: &BranchRetentionCandidateInputV1,
    context: &BranchRetentionContextPacketV2,
    component_negative: bool,
    immediate_safety_patch: bool,
) -> bool {
    if component_negative && !immediate_safety_patch {
        return false;
    }
    if deck_bloat_pressure_high(candidate)
        && !immediate_safety_patch
        && !context
            .keys
            .contains(&BranchRetentionContextKeyV2::MatchesFormationFrontloadNeed)
    {
        return false;
    }
    true
}

fn added_cards_have_negative_component_margin(
    candidate: &BranchRetentionCandidateInputV1,
    context: &CardComponentMarginalContextV1,
) -> bool {
    candidate.choice_profiles.iter().any(|profile| {
        let report = evaluate_card_component_marginal_value_v1(context, profile);
        matches!(
            report.verdict,
            CardComponentMarginalVerdictV1::Reject | CardComponentMarginalVerdictV1::SkipPreferred
        )
    })
}

fn component_margin_rank_adjustment(
    candidate: &BranchRetentionCandidateInputV1,
    context: &CardComponentMarginalContextV1,
) -> i32 {
    candidate
        .choice_profiles
        .iter()
        .map(|profile| {
            let report = evaluate_card_component_marginal_value_v1(context, profile);
            match report.verdict {
                CardComponentMarginalVerdictV1::MustTake => 12_000,
                CardComponentMarginalVerdictV1::StrongTake => 6_000,
                CardComponentMarginalVerdictV1::ContextTake => 2_000,
                CardComponentMarginalVerdictV1::Speculative => 0,
                CardComponentMarginalVerdictV1::SkipPreferred => -8_000,
                CardComponentMarginalVerdictV1::Reject => -20_000,
            }
        })
        .sum()
}

fn deck_bloat_pressure_high(candidate: &BranchRetentionCandidateInputV1) -> bool {
    candidate.deck_count >= 28
        || candidate
            .strategy_formation
            .as_ref()
            .is_some_and(|formation| {
                formation
                    .needs
                    .iter()
                    .any(|need| {
                        *need
                            == crate::ai::noncombat_strategy_v1::StrategyDeckFormationNeedV1::Consistency
                    })
            })
}

fn is_plain_card_reward_skip(candidate: &BranchRetentionCandidateInputV1) -> bool {
    candidate
        .choice_effect_keys
        .iter()
        .any(|effect| effect == "skip_reward")
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
mod tests;
