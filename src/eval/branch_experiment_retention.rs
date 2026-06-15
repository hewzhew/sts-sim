use std::collections::{BTreeMap, BTreeSet};

use crate::ai::card_reward_policy_v1::{CardRewardSemanticProfileV1, CardRewardSemanticRoleV1};
use crate::ai::deck_startup_profile_v1::DeckStartupProfileV1;
use crate::ai::noncombat_strategy_v1::StrategyFormationSummaryV2;
use crate::ai::strategic::{BranchSignature, RetentionBucket};
use crate::eval::branch_experiment::BranchExperimentChoiceDecisionSignalV1;
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
    pub curse_count: usize,
    pub strategy_formation: Option<StrategyFormationSummaryV2>,
    pub trajectory: BranchTrajectorySignatureV1,
    pub choice_profiles: Vec<CardRewardSemanticProfileV1>,
    pub choice_effect_keys: Vec<String>,
    pub lineage_flags: Vec<String>,
    pub decision_signals: Vec<BranchExperimentChoiceDecisionSignalV1>,
    pub strategic_debt_tags: Vec<String>,
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
    #[serde(default)]
    pub coverage_selection: BranchRetentionCoverageSelectionV1,
    #[serde(default, alias = "legacy_strategy_adjustment")]
    pub rank_adjustment: BranchRetentionRankAdjustmentV1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchRetentionCoverageSelectionV1 {
    pub primary_slot: BranchRetentionSlotV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_by_slot: Option<BranchRetentionSlotV1>,
    pub slots: Vec<BranchRetentionSlotV1>,
    pub reasons: Vec<String>,
}

impl Default for BranchRetentionCoverageSelectionV1 {
    fn default() -> Self {
        Self {
            primary_slot: BranchRetentionSlotV1::Diversity,
            selected_by_slot: None,
            slots: vec![BranchRetentionSlotV1::Diversity],
            reasons: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchRetentionRankAdjustmentV1 {
    pub base_rank_key: i32,
    pub startup_adjustment: i32,
    #[serde(default)]
    pub strategic_debt_adjustment: i32,
    #[serde(default)]
    pub decision_signal_adjustment: i32,
    /// Deprecated report-only pressure field kept for trace compatibility.
    /// Branch retention no longer consumes card-admission verdicts as gates.
    #[serde(default, alias = "component_adjustment")]
    pub admission_pressure: i32,
    pub effective_rank_key: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_keys: Vec<String>,
    /// Report-only lane evidence. Portfolio selection uses rank, branch-family
    /// coverage, and effect coverage; it does not consume these lane-local scores.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slot_scores: Vec<BranchRetentionSlotEvidenceScoreV1>,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchRetentionSlotEvidenceScoreV1 {
    pub slot: BranchRetentionSlotV1,
    pub score: i32,
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
    let slots = vec![BranchRetentionSlotV1::Diversity];
    let reasons = vec!["default branch representative".to_string()];
    BranchRetentionDecisionV1 {
        primary_slot: BranchRetentionSlotV1::Diversity,
        selected_by_slot: Some(BranchRetentionSlotV1::Diversity),
        slots: slots.clone(),
        reasons: reasons.clone(),
        strategic_signature: BranchSignature::default(),
        coverage_selection: BranchRetentionCoverageSelectionV1 {
            primary_slot: BranchRetentionSlotV1::Diversity,
            selected_by_slot: Some(BranchRetentionSlotV1::Diversity),
            slots,
            reasons,
        },
        rank_adjustment: BranchRetentionRankAdjustmentV1::default(),
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
        let group_picks = select_positions_for_retention_portfolio(
            candidates,
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
        selected_picks = select_positions_for_retention_portfolio(
            candidates,
            &positions,
            config.max_total,
            config.budget_profile,
        );
    }

    for decision in decisions_by_index.values_mut() {
        decision.selected_by_slot = None;
        decision.coverage_selection.selected_by_slot = None;
    }
    let mut keep_indices = BTreeSet::new();
    for pick in selected_picks {
        let candidate_index = candidates[pick.position].index;
        keep_indices.insert(candidate_index);
        if let Some(decision) = decisions_by_index.get_mut(&candidate_index) {
            decision.selected_by_slot = Some(pick.selected_by_slot);
            decision.coverage_selection.selected_by_slot = Some(pick.selected_by_slot);
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
    let current_startup_liability = branch_retention_current_startup_liability(candidate);

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
    if current_startup_liability {
        reasons.push("current deck has unresolved startup liability".to_string());
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
    let deck_bloat_pressure = deck_bloat_pressure_high(candidate);

    if candidate.choice_profiles.iter().any(|profile| {
        profile
            .roles
            .contains(&CardRewardSemanticRoleV1::FrontloadDamage)
    }) {
        slots.push(BranchRetentionSlotV1::Frontload);
        reasons.push("contains immediate combat output".to_string());
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
    slots.push(BranchRetentionSlotV1::Diversity);
    reasons.push("kept as a diversity representative".to_string());

    slots.sort();
    slots.dedup();
    slots.sort_by_key(|slot| slot_priority(*slot));
    let primary_slot = slots
        .first()
        .copied()
        .unwrap_or(BranchRetentionSlotV1::Diversity);

    let coverage_selection = BranchRetentionCoverageSelectionV1 {
        primary_slot,
        selected_by_slot: None,
        slots: slots.clone(),
        reasons: reasons.clone(),
    };
    let rank_adjustment = branch_retention_rank_adjustment_v1(candidate);

    BranchRetentionDecisionV1 {
        primary_slot,
        selected_by_slot: None,
        strategic_signature: strategic_signature_for_retention_candidate(
            candidate,
            &slots,
            current_startup_liability,
            false,
        ),
        slots,
        reasons,
        coverage_selection,
        rank_adjustment,
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
    let curse_debt = candidate.curse_count as i32;
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
        clean_score: if curse_debt > 0 {
            0.0
        } else if slots.contains(&BranchRetentionSlotV1::CleanDeck) {
            1.0
        } else if transition_attacks <= 1 && !component_negative {
            0.5
        } else {
            0.0
        },
        engine_score: bounded_signal(engine_signals, 4),
        cycle_debt: bounded_signal(deck_bloat / 4 + transition_attacks + curse_debt * 2, 5),
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
        + i32::from(candidate.startup.has_corruption_duplicate_without_payoff)
        + i32::from(candidate.startup.has_havoc_duplicate_without_payoff)
        + i32::from(
            candidate
                .startup
                .has_status_generator_saturation_without_digest,
        )
        + i32::from(candidate.startup.has_clash_playability_debt)
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
        .any(|key| resource_conversion_effect_key_v1(key))
}

fn resource_conversion_effect_key_v1(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "shop_buy_card"
            | "shop_buy_relic"
            | "shop_buy_potion"
            | "shop_buy_combo"
            | "shop_purge"
            | "event_pay_resource"
            | "event_trade"
    )
}

fn select_positions_for_retention_portfolio(
    candidates: &[BranchRetentionCandidateInputV1],
    positions: &[usize],
    limit: usize,
    _budget_profile: BranchRetentionBudgetProfileV1,
) -> Vec<BranchRetentionLanePick> {
    if limit == 0 {
        return Vec::new();
    }
    let mut selected = Vec::<BranchRetentionLanePick>::new();
    let mut selected_set = BTreeSet::<usize>::new();
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
    let selected =
        effect_coverage::preserve_choice_effect_coverage(candidates, positions, selected, limit);
    let selected =
        effect_coverage::preserve_lineage_flag_coverage(candidates, positions, selected, limit);
    drop_excess_first_pick_prefixes_after_coverage(candidates, positions, selected, limit)
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
    branch_retention_order_rank_key_v1(&candidates[left])
        .cmp(&branch_retention_order_rank_key_v1(&candidates[right]))
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

pub fn branch_retention_order_rank_key_v1(candidate: &BranchRetentionCandidateInputV1) -> i32 {
    branch_retention_adjusted_rank_key_v1(candidate)
}

pub fn branch_retention_adjusted_rank_key_v1(candidate: &BranchRetentionCandidateInputV1) -> i32 {
    branch_retention_rank_adjustment_v1(candidate).effective_rank_key
}

pub fn branch_retention_rank_adjustment_v1(
    candidate: &BranchRetentionCandidateInputV1,
) -> BranchRetentionRankAdjustmentV1 {
    let context = branch_retention_context_packet_v2(candidate);
    let current_startup_debt_adjustment = current_startup_debt_rank_adjustment_v1(candidate);
    let startup_adjustment = current_startup_debt_adjustment;
    let admission_pressure = 0;
    let mut reasons = Vec::new();

    if current_startup_debt_adjustment != 0 {
        reasons.push(format!(
            "current_startup_debt_rank_adjustment:{current_startup_debt_adjustment}"
        ));
    }
    if startup_adjustment != 0 {
        reasons.push(format!(
            "startup_rank_adjustment_total:{startup_adjustment}"
        ));
    }

    let strategic_debt_adjustment = branch_strategic_debt_rank_adjustment_v1(candidate);
    if strategic_debt_adjustment != 0 {
        if candidate.curse_count > 0 {
            reasons.push(format!("curse_debt_count:{}", candidate.curse_count));
        }
        for tag in &candidate.strategic_debt_tags {
            reasons.push(format!("strategic_debt_tag:{tag}"));
        }
        reasons.push(format!(
            "strategic_debt_rank_adjustment:{strategic_debt_adjustment}"
        ));
    }
    let decision_signal_adjustment = branch_decision_signal_rank_adjustment_v1(candidate);
    if decision_signal_adjustment != 0 {
        reasons.push(format!(
            "decision_signal_component_rank_adjustment:{decision_signal_adjustment}"
        ));
    }

    let effective_rank_key = candidate
        .rank_key
        .saturating_add(startup_adjustment)
        .saturating_add(strategic_debt_adjustment)
        .saturating_add(decision_signal_adjustment);

    BranchRetentionRankAdjustmentV1 {
        base_rank_key: candidate.rank_key,
        startup_adjustment,
        strategic_debt_adjustment,
        decision_signal_adjustment,
        admission_pressure,
        effective_rank_key,
        context_keys: context
            .keys
            .iter()
            .map(|key| branch_retention_context_key_label(*key).to_string())
            .collect(),
        slot_scores: branch_retention_slot_evidence_scores_v1(candidate),
        reasons,
    }
}

fn branch_decision_signal_rank_adjustment_v1(candidate: &BranchRetentionCandidateInputV1) -> i32 {
    candidate
        .decision_signals
        .iter()
        .map(|signal| signal.component_net_rank)
        .sum()
}

fn branch_strategic_debt_rank_adjustment_v1(candidate: &BranchRetentionCandidateInputV1) -> i32 {
    let curse_adjustment = -(candidate.curse_count as i32).saturating_mul(1_200);
    curse_adjustment
        + candidate
            .strategic_debt_tags
            .iter()
            .map(|tag| match tag.as_str() {
                "relic_constraint:sozu_potion_lock" => -700,
                "bottle_debt:high_opening_hand" => -1_200,
                "bottle_debt:situational_opening_hand" => -800,
                "bottle_debt:power_vs_awakened_one" => -1_000,
                "bottle_debt:temporary_strength_burst" => -600,
                _ => 0,
            })
            .sum::<i32>()
}

fn branch_retention_slot_evidence_scores_v1(
    candidate: &BranchRetentionCandidateInputV1,
) -> Vec<BranchRetentionSlotEvidenceScoreV1> {
    SLOT_ORDER
        .iter()
        .copied()
        .filter_map(|slot| {
            let score = branch_retention_slot_evidence_score_v1(candidate, slot);
            (score > 0).then_some(BranchRetentionSlotEvidenceScoreV1 { slot, score })
        })
        .collect()
}

fn branch_retention_context_key_label(key: BranchRetentionContextKeyV2) -> &'static str {
    match key {
        BranchRetentionContextKeyV2::MatchesFormationFrontloadNeed => {
            "matches_formation_frontload_need"
        }
        BranchRetentionContextKeyV2::MatchesFormationBlockNeed => "matches_formation_block_need",
        BranchRetentionContextKeyV2::MatchesFormationScalingNeed => {
            "matches_formation_scaling_need"
        }
        BranchRetentionContextKeyV2::MatchesFormationDrawEnergyNeed => {
            "matches_formation_draw_energy_need"
        }
        BranchRetentionContextKeyV2::MatchesFormationConsistencyNeed => {
            "matches_formation_consistency_need"
        }
        BranchRetentionContextKeyV2::OpensPackageSetup => "opens_package_setup",
        BranchRetentionContextKeyV2::ClosesPackage => "closes_package",
        BranchRetentionContextKeyV2::SupportsCommittedPackage => "supports_committed_package",
        BranchRetentionContextKeyV2::ImmediateSafetyPatch => "immediate_safety_patch",
    }
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

fn branch_retention_slot_evidence_score_v1(
    candidate: &BranchRetentionCandidateInputV1,
    slot: BranchRetentionSlotV1,
) -> i32 {
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

fn branch_retention_current_startup_liability(candidate: &BranchRetentionCandidateInputV1) -> bool {
    candidate.startup.has_setup_debt_high_payment_low
        || candidate.startup.has_fnp_duplicate_without_exhaust_engine
        || candidate.startup.has_corruption_duplicate_without_payoff
        || candidate.startup.has_havoc_duplicate_without_payoff
        || candidate
            .startup
            .has_status_generator_saturation_without_digest
        || candidate.startup.has_clash_playability_debt
        || candidate.startup.has_dual_wield_without_target
        || candidate.startup.has_anger_duplicate_without_digest
        || candidate.startup.has_strength_payoff_without_strength
        || candidate.startup.has_rupture_without_self_damage
        || candidate.startup.has_armaments_unupgraded_duplicate
        || candidate.startup.has_pyramid_unupgraded_apparition
}

fn current_startup_debt_rank_adjustment_v1(candidate: &BranchRetentionCandidateInputV1) -> i32 {
    let debt_count = startup_debt_count(candidate, false, false);
    if debt_count <= 0 {
        return 0;
    }
    -1_000 * debt_count.min(6)
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
