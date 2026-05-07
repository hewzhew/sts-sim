use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::bot::{card_facts, card_structure};
use crate::content::cards::{self, CardId, CardType};
use crate::content::monsters::EnemyId;
use crate::content::powers::PowerId;
use crate::content::relics::{get_relic_subscriptions, RelicId};
use crate::projection::combat::monster_preview_total_damage_in_combat;
use crate::runtime::combat::{CombatCard, CombatState};
use crate::state::core::ClientInput;
use crate::state::EngineState;

use super::legal_moves::get_legal_moves;
use super::profile::SearchProfileBreakdown;
use super::stepping::simulate_input_bounded;

pub const COMBAT_TURN_PLAN_PROBE_SCHEMA_VERSION: &str = "combat_turn_plan_probe_v2_3_1";
pub const COMBAT_DRAW_MARGINAL_PROBE_SCHEMA_VERSION: &str = "combat_draw_marginal_probe_v1";

#[derive(Clone, Copy, Debug)]
pub struct CombatTurnPlanProbeConfig {
    pub max_depth: usize,
    pub max_nodes: usize,
    pub beam_width: usize,
    pub max_engine_steps_per_action: usize,
}

impl Default for CombatTurnPlanProbeConfig {
    fn default() -> Self {
        Self {
            max_depth: 6,
            max_nodes: 2_000,
            beam_width: 32,
            max_engine_steps_per_action: 200,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatTurnPlanProbeReport {
    pub schema_version: String,
    pub source_trace: serde_json::Value,
    pub state_summary: CombatPlanStateSummary,
    pub hand_cards: Vec<CombatPlanHandCard>,
    pub plan_queries: Vec<CombatPlanQueryReport>,
    pub first_action_affordances: Vec<CombatFirstActionAffordance>,
    pub plans: Vec<CombatPlanReport>,
    pub sequence_classes: Vec<CombatPlanSequenceClass>,
    pub risk_notes: Vec<CombatPlanRiskNote>,
    pub probe_limits: CombatPlanProbeLimits,
    pub truth_warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatDrawMarginalProbeReport {
    pub schema_version: String,
    pub source_trace: serde_json::Value,
    pub target_action_card: String,
    pub target_card_id: String,
    pub target_granularity: String,
    pub target_card_uuid: Option<u32>,
    pub target_hand_index: Option<usize>,
    pub target_action_key: Option<String>,
    pub status: String,
    pub branches: Vec<CombatDrawMarginalBranchReport>,
    pub marginal: Option<CombatDrawMarginalSummary>,
    pub truth_warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatDrawMarginalBranchReport {
    pub branch_name: String,
    pub status: String,
    pub target_action_keys: Vec<String>,
    pub plan_queries: Vec<CombatPlanQueryReport>,
    pub probe_limits: CombatPlanProbeLimits,
    pub sequence_count: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CombatDrawMarginalSummary {
    pub comparison_query: String,
    pub damage_delta: i32,
    pub block_delta: i32,
    pub unblocked_reduction: i32,
    pub hp_loss_reduction: i32,
    pub remaining_energy_delta: i32,
    pub remaining_hand_delta: i32,
    pub setup_gain: bool,
    pub lethal_gain: bool,
    pub full_block_gain: bool,
    pub marginal_score: i32,
    pub label_strength: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatPlanStateSummary {
    pub engine_state: String,
    pub player_hp: i32,
    pub player_block: i32,
    pub energy: i32,
    pub turn_count: u32,
    pub visible_incoming_damage: i32,
    pub unblocked_incoming_damage: i32,
    pub alive_monster_count: usize,
    pub total_monster_hp: i32,
    pub hand_count: usize,
    pub draw_count: usize,
    pub discard_count: usize,
    pub exhaust_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatPlanHandCard {
    pub hand_index: usize,
    pub card_instance_id: u32,
    pub card_id: String,
    pub upgraded: bool,
    pub cost_for_turn: i8,
    pub playable: bool,
    pub base_semantics: Vec<String>,
    pub transient_tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatPlanReport {
    pub plan_name: String,
    pub best_sequence_key: Option<String>,
    pub best_actions: Vec<String>,
    pub best_action_keys: Vec<String>,
    pub best_score: Option<PlanScoreBreakdown>,
    pub candidate_sequence_count: usize,
    pub explanation: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatFirstActionAffordance {
    pub action_key: String,
    pub action_label: String,
    pub supported_plans: Vec<CombatPlanActionSupport>,
    pub best_plan_rank: Option<usize>,
    pub sequence_count: usize,
    pub best_sequence_key: Option<String>,
    pub best_sequence_actions: Vec<String>,
    pub best_sequence_score: Option<PlanScoreBreakdown>,
    pub component_max: PlanScoreBreakdown,
    pub major_tradeoffs: Vec<String>,
    pub risk_note_kinds: Vec<String>,
    pub order_sensitive_reasons: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatPlanActionSupport {
    pub plan_name: String,
    pub rank: usize,
    pub plan_score: i32,
    pub best_plan_score: i32,
    pub score_gap_to_best: i32,
    pub support_level: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatPlanQueryReport {
    pub query_name: String,
    pub status: String,
    pub best_sequence_key: Option<String>,
    pub best_action_keys: Vec<String>,
    pub best_actions: Vec<String>,
    pub outcome: Option<CombatPlanSequenceOutcome>,
    pub failed_constraints: Vec<String>,
    pub needs_deeper_search: bool,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatPlanSequenceClass {
    pub sequence_equivalence_key: String,
    pub actions: Vec<String>,
    pub action_keys: Vec<String>,
    pub order_sensitive_reasons: Vec<String>,
    pub compression_notes: Vec<String>,
    pub diagnostics: PlanScoreBreakdown,
    pub outcome: CombatPlanSequenceOutcome,
    pub pruned_as_equivalent: bool,
    pub pruned_by_budget: bool,
    pub pruned_by_dominated_state: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CombatPlanSequenceOutcome {
    pub damage_done: i32,
    pub block_after: i32,
    pub projected_unblocked_damage: i32,
    pub hp_loss_actual: i32,
    pub remaining_energy: i32,
    pub remaining_hand_count: usize,
    pub enemy_deaths: usize,
    pub living_monster_count: usize,
    pub total_monster_hp: i32,
    pub played_setup_or_scaling: bool,
    pub played_kill_window_card: bool,
    pub random_risk_present: bool,
    pub ended_turn: bool,
    pub kill_window_target_count: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct PlanScoreBreakdown {
    pub total_score: i32,
    pub lethal_score: i32,
    pub block_score: i32,
    pub hp_loss_score: i32,
    pub enemy_death_score: i32,
    pub damage_score: i32,
    pub setup_score: i32,
    pub exhaust_value: i32,
    pub key_card_risk: i32,
    pub random_risk: i32,
    pub future_hand_penalty: i32,
    pub strength_projection: i32,
    pub dex_projection: i32,
    pub vulnerable_projection: i32,
    pub optimistic_bound_score: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatPlanRiskNote {
    pub sequence_action_index: usize,
    pub action_key: String,
    pub kind: String,
    pub message: String,
    pub chance_model: Option<String>,
    pub exact_rng_branches: bool,
    pub risk_is_overlay_only: bool,
    pub bad_branch_probability_milli: Option<i32>,
    pub good_branch_probability_milli: Option<i32>,
    pub affected_cards: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatPlanProbeLimits {
    pub max_depth: usize,
    pub max_nodes: usize,
    pub beam_width: usize,
    pub max_engine_steps_per_action: usize,
    pub nodes_expanded: usize,
    pub actions_considered: usize,
    pub actions_simulated: usize,
    pub sequence_classes_kept: usize,
    pub pruned_as_equivalent: usize,
    pub pruned_by_abstract_equivalence: usize,
    pub abstract_equivalence_candidates: usize,
    pub abstract_equivalence_blocked_by_context: usize,
    pub abstract_equivalence_blocked_by_action_semantics: usize,
    pub abstract_equivalence_rejected_by_engine: usize,
    pub pruned_by_verified_abstract_equivalence: usize,
    pub generation_canonical_candidates: usize,
    pub generation_canonical_blocked_by_context: usize,
    pub generation_canonical_blocked_by_action_semantics: usize,
    pub pruned_by_generation_canonical_order: usize,
    pub pruned_by_generation_duplicate_card: usize,
    pub pruned_by_generation_same_lane_order: usize,
    pub pruned_by_generation_target_order: usize,
    pub pruned_by_generation_lane_order: usize,
    pub generation_duplicate_prune_effects: BTreeMap<String, usize>,
    pub pruned_by_plan_expansion_gate: usize,
    pub plan_expansion_gate_reasons: BTreeMap<String, usize>,
    pub plan_expansion_gate_examples: Vec<PlanExpansionGateExample>,
    pub pruned_by_optimistic_bound: usize,
    pub pruned_by_budget: usize,
    pub pruned_by_dominated_state: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct PlanExpansionGateExample {
    pub reason: String,
    pub depth: usize,
    pub partial_action_keys: Vec<String>,
    pub partial_actions: Vec<String>,
    pub pruned_action_key: String,
    pub pruned_action: String,
    pub current_energy: i32,
    pub current_block: i32,
    pub visible_incoming_damage: i32,
    pub hand_size: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CombatPlanKind {
    Lethal,
    KillThreateningEnemy,
    FullBlock,
    BlockEnoughThenDamage,
    MaxDamage,
    SetupPowerOrScaling,
    ExhaustBadCards,
    PreserveKeyCards,
}

#[derive(Clone, Debug)]
struct ProbeNode {
    engine: EngineState,
    combat: CombatState,
    actions: Vec<String>,
    action_keys: Vec<String>,
    order_sensitive_reasons: BTreeSet<String>,
    sequence_residues: BTreeSet<String>,
    canonical_last_keys: BTreeMap<String, String>,
    compression_notes: BTreeSet<String>,
    risk_notes: Vec<CombatPlanRiskNote>,
    accumulated: AccumulatedSequenceEffects,
    depth: usize,
    ended_turn: bool,
}

#[derive(Clone, Debug, Default)]
struct CombatTurnPlanExploreOptions {
    forbidden_target: Option<CombatDrawMarginalTarget>,
    require_first_target: Option<CombatDrawMarginalTarget>,
}

#[derive(Clone, Debug)]
pub struct CombatDrawMarginalTarget {
    pub card_id: CardId,
    pub card_uuid: Option<u32>,
    pub hand_index: Option<usize>,
    pub root_action_key: Option<String>,
}

impl CombatDrawMarginalTarget {
    pub fn card(card_id: CardId) -> Self {
        Self {
            card_id,
            card_uuid: None,
            hand_index: None,
            root_action_key: None,
        }
    }

    pub fn hand_instance(card_id: CardId, hand_index: usize, card_uuid: u32) -> Self {
        Self {
            card_id,
            card_uuid: Some(card_uuid),
            hand_index: Some(hand_index),
            root_action_key: None,
        }
    }

    pub fn with_root_action_key(mut self, action_key: String) -> Self {
        self.root_action_key = Some(action_key);
        self
    }

    fn granularity(&self) -> &'static str {
        if self.root_action_key.is_some() {
            "root_action_key"
        } else if self.card_uuid.is_some() {
            "hand_instance"
        } else {
            "card_id"
        }
    }
}

#[derive(Clone, Debug, Default)]
struct ActionEffectSummary {
    pure_damage: bool,
    pure_block: bool,
    draws_cards: bool,
    energy_gain: bool,
    debuff: bool,
    setup_or_scaling: bool,
    exhaust_or_discard: bool,
    creates_cards: bool,
    immediate_action_space_change: bool,
    random_effect: bool,
    possible_kill: bool,
    target_sensitive: bool,
    damage_estimate: i32,
    block_estimate: i32,
    attack_hit_count: i32,
    strength_gain: i32,
    dex_gain: i32,
    vulnerable_projection: i32,
}

#[derive(Clone, Debug)]
struct AbstractSeen {
    verification_snapshot: AbstractVerificationSnapshot,
    score: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AbstractVerificationSnapshot {
    engine: String,
    player: String,
    monsters: String,
    hand_state: String,
    hand_eval_cache: String,
    draw_state: String,
    draw_eval_cache: String,
    discard_state: String,
    discard_eval_cache: String,
    exhaust_state: String,
    exhaust_eval_cache: String,
    turn_counters: String,
    played_card_ids: String,
    relics: String,
    powers: String,
}

impl AbstractVerificationSnapshot {
    fn behaviorally_matches(&self, other: &Self) -> bool {
        self.engine == other.engine
            && self.player == other.player
            && self.monsters == other.monsters
            && self.hand_state == other.hand_state
            && self.draw_state == other.draw_state
            && self.discard_state == other.discard_state
            && self.exhaust_state == other.exhaust_state
            && self.turn_counters == other.turn_counters
            && self.played_card_ids == other.played_card_ids
            && self.relics == other.relics
            && self.powers == other.powers
    }

    fn diff_reasons(&self, other: &Self) -> Vec<&'static str> {
        let mut reasons = Vec::new();
        if self.engine != other.engine {
            reasons.push("engine_state_diff");
        }
        if self.player != other.player {
            reasons.push("player_resource_diff");
        }
        if self.monsters != other.monsters {
            reasons.push("monster_state_diff");
        }
        if self.hand_state != other.hand_state {
            reasons.push("hand_card_state_diff");
        }
        if self.draw_state != other.draw_state
            || self.discard_state != other.discard_state
            || self.exhaust_state != other.exhaust_state
        {
            reasons.push("deck_zone_card_state_diff");
        }
        if self.turn_counters != other.turn_counters
            || self.played_card_ids != other.played_card_ids
        {
            reasons.push("turn_counter_diff");
        }
        if self.relics != other.relics {
            reasons.push("relic_state_diff");
        }
        if self.powers != other.powers {
            reasons.push("power_state_diff");
        }
        if reasons.is_empty() {
            reasons.push("unknown_signature_diff");
        }
        reasons
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum AbstractCompressionDecision {
    Candidate,
    BlockedByActionSemantics(&'static str),
    BlockedByContext(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum GenerationCanonicalDecision {
    Candidate(GenerationCanonicalKeys),
    BlockedByActionSemantics(&'static str),
    BlockedByContext(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GenerationCanonicalKeys {
    group_key: String,
    sort_key: String,
    effect_key: String,
    class_label: String,
    lane_label: String,
    target_label: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GenerationPruneKind {
    DuplicateCard,
    SameLaneOrder,
    TargetOrder,
    LaneOrder,
}

#[derive(Clone, Debug, Default)]
struct AccumulatedSequenceEffects {
    setup_score: i32,
    exhaust_value: i32,
    key_card_risk: i32,
    random_risk: i32,
    future_hand_penalty: i32,
    action_space_change_count: usize,
    strength_projection: i32,
    dex_projection: i32,
    vulnerable_projection: i32,
    played_setup_or_scaling: bool,
    played_kill_window_card: bool,
    random_risk_present: bool,
}

pub fn probe_turn_plans(
    engine: &EngineState,
    combat: &CombatState,
    config: CombatTurnPlanProbeConfig,
) -> CombatTurnPlanProbeReport {
    probe_turn_plans_with_options(
        engine,
        combat,
        config,
        &CombatTurnPlanExploreOptions::default(),
    )
}

fn probe_turn_plans_with_options(
    engine: &EngineState,
    combat: &CombatState,
    config: CombatTurnPlanProbeConfig,
    options: &CombatTurnPlanExploreOptions,
) -> CombatTurnPlanProbeReport {
    let start_summary = summarize_state(engine, combat);
    let hand_cards = build_probe_hand_cards(combat);
    let (sequence_classes, limits, risk_notes) =
        explore_sequence_classes(engine, combat, config, options);
    let plan_kinds = [
        CombatPlanKind::Lethal,
        CombatPlanKind::KillThreateningEnemy,
        CombatPlanKind::FullBlock,
        CombatPlanKind::BlockEnoughThenDamage,
        CombatPlanKind::MaxDamage,
        CombatPlanKind::SetupPowerOrScaling,
        CombatPlanKind::ExhaustBadCards,
        CombatPlanKind::PreserveKeyCards,
    ];
    let plans = plan_kinds
        .iter()
        .map(|plan| build_plan_report(*plan, &sequence_classes))
        .collect();
    let plan_queries = build_plan_queries(combat, &sequence_classes, &limits);
    let first_action_affordances =
        build_first_action_affordances(&plan_kinds, &sequence_classes, &risk_notes);

    CombatTurnPlanProbeReport {
        schema_version: COMBAT_TURN_PLAN_PROBE_SCHEMA_VERSION.to_string(),
        source_trace: serde_json::Value::Null,
        state_summary: start_summary,
        hand_cards,
        plan_queries,
        first_action_affordances,
        plans,
        sequence_classes,
        risk_notes,
        probe_limits: limits,
        truth_warnings: vec![
            "current_turn_only_horizon".to_string(),
            "no_future_seed_oracle".to_string(),
            "role_scores_are_heuristic_not_truth".to_string(),
            "plan_expansion_gate_is_query_oriented_and_diagnostic".to_string(),
            "affine_buff_projection_is_heuristic_not_engine_truth".to_string(),
            "generation_canonical_order_pruning_is_conservative_pure_damage_block_only".to_string(),
            "abstract_sequence_compression_is_engine_verified_and_conservative".to_string(),
            "abstract_rejection_diff_notes_are_diagnostic_not_policy_labels".to_string(),
            "abstract_verification_ignores_recomputed_card_eval_cache".to_string(),
            "static_random_risk_overlay_is_not_engine_rng_branch_enumeration".to_string(),
            "budget_pruning_can_hide_lower_ranked_sequences".to_string(),
        ],
    }
}

pub fn probe_draw_marginal_value(
    engine: &EngineState,
    combat: &CombatState,
    target_card: CardId,
    config: CombatTurnPlanProbeConfig,
) -> CombatDrawMarginalProbeReport {
    probe_draw_marginal_value_for_target(
        engine,
        combat,
        CombatDrawMarginalTarget::card(target_card),
        config,
    )
}

pub fn probe_draw_marginal_value_for_target(
    engine: &EngineState,
    combat: &CombatState,
    target: CombatDrawMarginalTarget,
    config: CombatTurnPlanProbeConfig,
) -> CombatDrawMarginalProbeReport {
    let target = normalize_draw_marginal_target(engine, combat, target);
    let target_action_card = cards::java_id(target.card_id).to_string();
    let target_card_id = format!("{:?}", target.card_id);
    let target_granularity = target.granularity().to_string();
    let target_action_keys = matching_legal_root_action_keys(engine, combat, &target);

    let free = probe_turn_plans_with_options(
        engine,
        combat,
        config,
        &CombatTurnPlanExploreOptions::default(),
    );
    let no_draw = probe_turn_plans_with_options(
        engine,
        combat,
        config,
        &CombatTurnPlanExploreOptions {
            forbidden_target: Some(target.clone()),
            require_first_target: None,
        },
    );
    let forced = if target_action_keys.is_empty() {
        None
    } else {
        Some(probe_turn_plans_with_options(
            engine,
            combat,
            config,
            &CombatTurnPlanExploreOptions {
                forbidden_target: None,
                require_first_target: Some(target.clone()),
            },
        ))
    };

    let mut branches = Vec::new();
    branches.push(draw_marginal_branch("free_best", "ok", Vec::new(), &free));
    branches.push(draw_marginal_branch(
        "no_draw_best",
        "ok",
        Vec::new(),
        &no_draw,
    ));
    if let Some(forced_report) = &forced {
        branches.push(draw_marginal_branch(
            "forced_draw_best",
            "ok",
            target_action_keys,
            forced_report,
        ));
    } else {
        branches.push(CombatDrawMarginalBranchReport {
            branch_name: "forced_draw_best".to_string(),
            status: "not_applicable".to_string(),
            target_action_keys: Vec::new(),
            plan_queries: Vec::new(),
            probe_limits: empty_probe_limits(config),
            sequence_count: 0,
        });
    }

    let marginal = forced
        .as_ref()
        .and_then(|forced_report| summarize_draw_marginal(&no_draw, forced_report));
    let status = if forced.is_some() {
        "ok"
    } else {
        "not_applicable"
    }
    .to_string();

    CombatDrawMarginalProbeReport {
        schema_version: COMBAT_DRAW_MARGINAL_PROBE_SCHEMA_VERSION.to_string(),
        source_trace: serde_json::Value::Null,
        target_action_card,
        target_card_id,
        target_granularity,
        target_card_uuid: target.card_uuid,
        target_hand_index: target.hand_index,
        target_action_key: target.root_action_key,
        status,
        branches,
        marginal,
        truth_warnings: vec![
            "current_turn_only_horizon".to_string(),
            "forced_draw_branch_forces_target_as_first_action".to_string(),
            "no_draw_branch_excludes_target_from_current_turn_sequences".to_string(),
            "marginal_delta_includes_target_card_body_not_only_draw_text".to_string(),
            "marginal_summary_is_plan_query_delta_not_card_choice_truth".to_string(),
            "card_id_granularity_forbids_or_forces_all_same_card_copies".to_string(),
            "hand_instance_granularity_tracks_card_uuid_across_hand_index_shifts".to_string(),
            "sample_distribution_must_be_built_by_batching_multiple_author_specs".to_string(),
        ],
    }
}

fn draw_marginal_branch(
    branch_name: &str,
    status: &str,
    target_action_keys: Vec<String>,
    report: &CombatTurnPlanProbeReport,
) -> CombatDrawMarginalBranchReport {
    CombatDrawMarginalBranchReport {
        branch_name: branch_name.to_string(),
        status: status.to_string(),
        target_action_keys,
        plan_queries: report.plan_queries.clone(),
        probe_limits: report.probe_limits.clone(),
        sequence_count: report.sequence_classes.len(),
    }
}

fn summarize_draw_marginal(
    no_draw: &CombatTurnPlanProbeReport,
    forced: &CombatTurnPlanProbeReport,
) -> Option<CombatDrawMarginalSummary> {
    let comparison_query = "CanFullBlockThenMaxDamage";
    let no_query = query_by_name(no_draw, comparison_query)
        .or_else(|| query_by_name(no_draw, "CanLethal"))
        .or_else(|| no_draw.plan_queries.first())?;
    let forced_query = query_by_name(forced, comparison_query)
        .or_else(|| query_by_name(forced, "CanLethal"))
        .or_else(|| forced.plan_queries.first())?;
    let no_outcome = no_query.outcome.as_ref().cloned().unwrap_or_default();
    let forced_outcome = forced_query.outcome.as_ref().cloned().unwrap_or_default();
    let no_lethal = query_status_is(no_draw, "CanLethal", "feasible");
    let forced_lethal = query_status_is(forced, "CanLethal", "feasible");
    let no_full_block = query_status_is(no_draw, "CanFullBlock", "feasible");
    let forced_full_block = query_status_is(forced, "CanFullBlock", "feasible");

    let damage_delta = forced_outcome.damage_done - no_outcome.damage_done;
    let block_delta = forced_outcome.block_after - no_outcome.block_after;
    let unblocked_reduction =
        no_outcome.projected_unblocked_damage - forced_outcome.projected_unblocked_damage;
    let hp_loss_reduction = no_outcome.hp_loss_actual - forced_outcome.hp_loss_actual;
    let remaining_energy_delta = forced_outcome.remaining_energy - no_outcome.remaining_energy;
    let remaining_hand_delta =
        forced_outcome.remaining_hand_count as i32 - no_outcome.remaining_hand_count as i32;
    let setup_gain = forced_outcome.played_setup_or_scaling && !no_outcome.played_setup_or_scaling;
    let lethal_gain = forced_lethal && !no_lethal;
    let full_block_gain = forced_full_block && !no_full_block;
    let marginal_score = damage_delta
        + block_delta
        + unblocked_reduction * 6
        + hp_loss_reduction * 10
        + remaining_energy_delta * 2
        + remaining_hand_delta
        + if setup_gain { 15 } else { 0 }
        + if lethal_gain { 80 } else { 0 }
        + if full_block_gain { 40 } else { 0 };
    let label_strength = if marginal_score >= 60 && hp_loss_reduction >= -6 {
        "robust_positive"
    } else if marginal_score >= 15 {
        "conditional_positive"
    } else if marginal_score <= -25 {
        "harmful"
    } else if marginal_score.abs() <= 8 {
        "equivalent"
    } else {
        "inconclusive"
    }
    .to_string();

    Some(CombatDrawMarginalSummary {
        comparison_query: forced_query.query_name.clone(),
        damage_delta,
        block_delta,
        unblocked_reduction,
        hp_loss_reduction,
        remaining_energy_delta,
        remaining_hand_delta,
        setup_gain,
        lethal_gain,
        full_block_gain,
        marginal_score,
        label_strength,
    })
}

fn query_by_name<'a>(
    report: &'a CombatTurnPlanProbeReport,
    name: &str,
) -> Option<&'a CombatPlanQueryReport> {
    report
        .plan_queries
        .iter()
        .find(|query| query.query_name == name)
}

fn query_status_is(report: &CombatTurnPlanProbeReport, name: &str, status: &str) -> bool {
    query_by_name(report, name).is_some_and(|query| query.status == status)
}

fn matching_legal_root_action_keys(
    engine: &EngineState,
    combat: &CombatState,
    target: &CombatDrawMarginalTarget,
) -> Vec<String> {
    get_legal_moves(engine, combat)
        .into_iter()
        .filter(|action| allowed_probe_action(engine, action))
        .filter(|action| draw_marginal_target_matches_action(combat, action, target, true))
        .map(|action| probe_action_key(combat, &action))
        .collect()
}

fn allowed_by_draw_marginal_options(
    combat: &CombatState,
    action: &ClientInput,
    node: &ProbeNode,
    options: &CombatTurnPlanExploreOptions,
) -> bool {
    if options.forbidden_target.as_ref().is_some_and(|forbidden| {
        draw_marginal_target_matches_action(combat, action, forbidden, false)
    }) {
        return false;
    }
    if node.depth == 0
        && node.actions.is_empty()
        && options
            .require_first_target
            .as_ref()
            .is_some_and(|required| {
                !draw_marginal_target_matches_action(combat, action, required, true)
            })
    {
        return false;
    }
    true
}

fn normalize_draw_marginal_target(
    engine: &EngineState,
    combat: &CombatState,
    mut target: CombatDrawMarginalTarget,
) -> CombatDrawMarginalTarget {
    if let Some(hand_index) = target.hand_index {
        if let Some(card) = combat.zones.hand.get(hand_index) {
            if card.id == target.card_id {
                target.card_uuid = Some(card.uuid);
            }
        }
    }
    if target.card_uuid.is_none() {
        if let Some(root_action_key) = target.root_action_key.as_deref() {
            for action in get_legal_moves(engine, combat)
                .into_iter()
                .filter(|action| allowed_probe_action(engine, action))
            {
                if probe_action_key(combat, &action) == root_action_key {
                    if let ClientInput::PlayCard { card_index, .. } = action {
                        if let Some(card) = combat.zones.hand.get(card_index) {
                            if card.id == target.card_id {
                                target.card_uuid = Some(card.uuid);
                                target.hand_index = Some(card_index);
                            }
                        }
                    }
                    break;
                }
            }
        }
    }
    target
}

fn draw_marginal_target_matches_action(
    combat: &CombatState,
    action: &ClientInput,
    target: &CombatDrawMarginalTarget,
    is_root_action: bool,
) -> bool {
    if is_root_action {
        if let Some(root_action_key) = target.root_action_key.as_deref() {
            return probe_action_key(combat, action) == root_action_key;
        }
    }
    let Some(card_id) = action_card_id(combat, action) else {
        return false;
    };
    if card_id != target.card_id {
        return false;
    }
    if let Some(card_uuid) = target.card_uuid {
        return action_card_uuid(combat, action) == Some(card_uuid);
    }
    true
}

fn action_card_id(combat: &CombatState, action: &ClientInput) -> Option<CardId> {
    let ClientInput::PlayCard { card_index, .. } = action else {
        return None;
    };
    combat.zones.hand.get(*card_index).map(|card| card.id)
}

fn action_card_uuid(combat: &CombatState, action: &ClientInput) -> Option<u32> {
    let ClientInput::PlayCard { card_index, .. } = action else {
        return None;
    };
    combat.zones.hand.get(*card_index).map(|card| card.uuid)
}

fn empty_probe_limits(config: CombatTurnPlanProbeConfig) -> CombatPlanProbeLimits {
    CombatPlanProbeLimits {
        max_depth: config.max_depth,
        max_nodes: config.max_nodes,
        beam_width: config.beam_width,
        max_engine_steps_per_action: config.max_engine_steps_per_action,
        nodes_expanded: 0,
        actions_considered: 0,
        actions_simulated: 0,
        sequence_classes_kept: 0,
        pruned_as_equivalent: 0,
        pruned_by_abstract_equivalence: 0,
        abstract_equivalence_candidates: 0,
        abstract_equivalence_blocked_by_context: 0,
        abstract_equivalence_blocked_by_action_semantics: 0,
        abstract_equivalence_rejected_by_engine: 0,
        pruned_by_verified_abstract_equivalence: 0,
        generation_canonical_candidates: 0,
        generation_canonical_blocked_by_context: 0,
        generation_canonical_blocked_by_action_semantics: 0,
        pruned_by_generation_canonical_order: 0,
        pruned_by_generation_duplicate_card: 0,
        pruned_by_generation_same_lane_order: 0,
        pruned_by_generation_target_order: 0,
        pruned_by_generation_lane_order: 0,
        generation_duplicate_prune_effects: BTreeMap::new(),
        pruned_by_plan_expansion_gate: 0,
        plan_expansion_gate_reasons: BTreeMap::new(),
        plan_expansion_gate_examples: Vec::new(),
        pruned_by_optimistic_bound: 0,
        pruned_by_budget: 0,
        pruned_by_dominated_state: 0,
    }
}

fn build_first_action_affordances(
    plan_kinds: &[CombatPlanKind],
    sequences: &[CombatPlanSequenceClass],
    risk_notes: &[CombatPlanRiskNote],
) -> Vec<CombatFirstActionAffordance> {
    let mut by_first_action = BTreeMap::<String, Vec<&CombatPlanSequenceClass>>::new();
    for sequence in sequences {
        if let Some(first_action) = sequence.action_keys.first() {
            by_first_action
                .entry(first_action.clone())
                .or_default()
                .push(sequence);
        }
    }

    let mut plan_rankings = Vec::<(CombatPlanKind, Vec<(String, i32)>)>::new();
    for plan in plan_kinds {
        let mut best_by_first_action = BTreeMap::<String, i32>::new();
        for sequence in sequences {
            let Some(first_action) = sequence.action_keys.first() else {
                continue;
            };
            let score = score_for_plan(*plan, &sequence.diagnostics);
            best_by_first_action
                .entry(first_action.clone())
                .and_modify(|existing| *existing = (*existing).max(score))
                .or_insert(score);
        }
        let mut ranked = best_by_first_action.into_iter().collect::<Vec<_>>();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        plan_rankings.push((*plan, ranked));
    }

    let mut affordances = by_first_action
        .into_iter()
        .map(|(action_key, action_sequences)| {
            let best_sequence = action_sequences
                .iter()
                .copied()
                .max_by_key(|sequence| sequence.diagnostics.total_score);
            let mut supported_plans = Vec::new();
            for (plan, ranked) in &plan_rankings {
                let Some(best_plan_score) = ranked.first().map(|(_, score)| *score) else {
                    continue;
                };
                let best_for_action = ranked
                    .iter()
                    .enumerate()
                    .find(|(_, (first_action, _))| first_action == &action_key);
                let Some((rank_idx, (_, plan_score))) = best_for_action else {
                    continue;
                };
                let rank = rank_idx + 1;
                let gap = best_plan_score - *plan_score;
                let support_level = if rank == 1 {
                    "top"
                } else if rank <= 3 || gap <= 80 {
                    "near_top"
                } else {
                    "weak"
                };
                if support_level != "weak" {
                    supported_plans.push(CombatPlanActionSupport {
                        plan_name: plan_label(*plan).to_string(),
                        rank,
                        plan_score: *plan_score,
                        best_plan_score,
                        score_gap_to_best: gap,
                        support_level: support_level.to_string(),
                    });
                }
            }
            supported_plans.sort_by(|a, b| {
                a.rank
                    .cmp(&b.rank)
                    .then_with(|| a.score_gap_to_best.cmp(&b.score_gap_to_best))
                    .then_with(|| a.plan_name.cmp(&b.plan_name))
            });
            let best_plan_rank = supported_plans.iter().map(|support| support.rank).min();
            let component_max = aggregate_component_max(&action_sequences);
            let action_risk_kinds = risk_notes
                .iter()
                .filter(|note| note.action_key == action_key)
                .map(|note| note.kind.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            let order_sensitive_reasons = action_sequences
                .iter()
                .flat_map(|sequence| sequence.order_sensitive_reasons.iter().cloned())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            CombatFirstActionAffordance {
                action_label: probe_action_label_from_key(&action_key),
                action_key,
                supported_plans,
                best_plan_rank,
                sequence_count: action_sequences.len(),
                best_sequence_key: best_sequence
                    .map(|sequence| sequence.sequence_equivalence_key.clone()),
                best_sequence_actions: best_sequence
                    .map(|sequence| sequence.actions.clone())
                    .unwrap_or_default(),
                best_sequence_score: best_sequence.map(|sequence| sequence.diagnostics.clone()),
                component_max: component_max.clone(),
                major_tradeoffs: major_tradeoffs_for_first_action(
                    &component_max,
                    &action_risk_kinds,
                    &order_sensitive_reasons,
                ),
                risk_note_kinds: action_risk_kinds,
                order_sensitive_reasons,
            }
        })
        .collect::<Vec<_>>();

    affordances.sort_by(|a, b| {
        a.best_plan_rank
            .unwrap_or(usize::MAX)
            .cmp(&b.best_plan_rank.unwrap_or(usize::MAX))
            .then_with(|| {
                score_value_for_sort(b.best_sequence_score.as_ref())
                    .cmp(&score_value_for_sort(a.best_sequence_score.as_ref()))
            })
            .then_with(|| a.action_key.cmp(&b.action_key))
    });
    affordances
}

fn build_plan_queries(
    start: &CombatState,
    sequences: &[CombatPlanSequenceClass],
    limits: &CombatPlanProbeLimits,
) -> Vec<CombatPlanQueryReport> {
    vec![
        query_can_lethal(start, sequences, limits),
        query_can_full_block(start, sequences, limits),
        query_can_full_block_then_max_damage(start, sequences, limits),
        query_can_play_setup_and_still_block(start, sequences, limits),
        query_can_preserve_kill_window(start, sequences, limits),
    ]
}

fn query_can_lethal(
    _start: &CombatState,
    sequences: &[CombatPlanSequenceClass],
    limits: &CombatPlanProbeLimits,
) -> CombatPlanQueryReport {
    let candidates = current_turn_sequences(sequences);
    let best_lethal = candidates
        .iter()
        .copied()
        .filter(|sequence| sequence.outcome.living_monster_count == 0)
        .max_by_key(|sequence| {
            (
                sequence.outcome.damage_done,
                -sequence.outcome.hp_loss_actual,
                sequence.outcome.remaining_energy,
            )
        });
    if let Some(sequence) = best_lethal {
        return query_report(
            "CanLethal",
            "feasible",
            Some(sequence),
            Vec::new(),
            vec!["current-turn lethal sequence found".to_string()],
            query_needs_deeper_search(Some(sequence), limits, "feasible", false),
        );
    }

    let best_damage = candidates.iter().copied().max_by_key(|sequence| {
        (
            sequence.outcome.damage_done,
            -sequence.outcome.total_monster_hp,
        )
    });
    let Some(sequence) = best_damage else {
        return query_report(
            "CanLethal",
            "not_feasible",
            None,
            vec!["no_sequences_explored".to_string()],
            vec!["no current-turn action sequence was available".to_string()],
            limits.pruned_by_budget > 0,
        );
    };
    let status = if sequence.outcome.damage_done > 0 {
        "partial"
    } else {
        "not_feasible"
    };
    query_report(
        "CanLethal",
        status,
        Some(sequence),
        vec![
            "combat_not_ended".to_string(),
            format!("missing_damage:{}", sequence.outcome.total_monster_hp),
        ],
        vec![
            format!("max_damage_done:{}", sequence.outcome.damage_done),
            format!("remaining_monster_hp:{}", sequence.outcome.total_monster_hp),
        ],
        query_needs_deeper_search(Some(sequence), limits, status, false),
    )
}

fn query_can_full_block(
    start: &CombatState,
    sequences: &[CombatPlanSequenceClass],
    limits: &CombatPlanProbeLimits,
) -> CombatPlanQueryReport {
    if visible_incoming_damage(start) <= 0 {
        return query_report(
            "CanFullBlock",
            "not_applicable",
            None,
            Vec::new(),
            vec!["no visible incoming damage".to_string()],
            false,
        );
    }
    let candidates = current_turn_sequences(sequences);
    let best_full_block = candidates
        .iter()
        .copied()
        .filter(|sequence| sequence.outcome.projected_unblocked_damage == 0)
        .max_by_key(|sequence| {
            (
                sequence.outcome.remaining_energy,
                sequence.outcome.damage_done,
            )
        });
    if let Some(sequence) = best_full_block {
        return query_report(
            "CanFullBlock",
            "feasible",
            Some(sequence),
            Vec::new(),
            vec![format!(
                "remaining_energy:{}",
                sequence.outcome.remaining_energy
            )],
            query_needs_deeper_search(Some(sequence), limits, "feasible", false),
        );
    }
    let best_partial = candidates.iter().copied().max_by_key(|sequence| {
        (
            -sequence.outcome.projected_unblocked_damage,
            sequence.outcome.block_after,
            sequence.outcome.remaining_energy,
        )
    });
    let Some(sequence) = best_partial else {
        return query_report(
            "CanFullBlock",
            "not_feasible",
            None,
            vec!["no_current_turn_sequences".to_string()],
            Vec::new(),
            limits.pruned_by_budget > 0,
        );
    };
    query_report(
        "CanFullBlock",
        "partial",
        Some(sequence),
        vec![format!(
            "unblocked_damage:{}",
            sequence.outcome.projected_unblocked_damage
        )],
        vec![format!("max_block_after:{}", sequence.outcome.block_after)],
        query_needs_deeper_search(Some(sequence), limits, "partial", false),
    )
}

fn query_can_full_block_then_max_damage(
    start: &CombatState,
    sequences: &[CombatPlanSequenceClass],
    limits: &CombatPlanProbeLimits,
) -> CombatPlanQueryReport {
    let candidates = current_turn_sequences(sequences);
    let full_block_candidates = candidates
        .iter()
        .copied()
        .filter(|sequence| {
            visible_incoming_damage(start) <= 0 || sequence.outcome.projected_unblocked_damage == 0
        })
        .collect::<Vec<_>>();
    if let Some(sequence) = full_block_candidates
        .iter()
        .copied()
        .max_by_key(|sequence| {
            (
                sequence.outcome.damage_done,
                sequence.outcome.remaining_energy,
            )
        })
    {
        return query_report(
            "CanFullBlockThenMaxDamage",
            "feasible",
            Some(sequence),
            Vec::new(),
            vec![
                "max damage under full-block constraint".to_string(),
                format!("damage_done:{}", sequence.outcome.damage_done),
            ],
            query_needs_deeper_search(Some(sequence), limits, "feasible", false),
        );
    }

    let best_partial = candidates.iter().copied().max_by_key(|sequence| {
        (
            -sequence.outcome.projected_unblocked_damage,
            sequence.outcome.damage_done,
            sequence.outcome.block_after,
        )
    });
    let Some(sequence) = best_partial else {
        return query_report(
            "CanFullBlockThenMaxDamage",
            "not_feasible",
            None,
            vec!["no_current_turn_sequences".to_string()],
            Vec::new(),
            limits.pruned_by_budget > 0,
        );
    };
    query_report(
        "CanFullBlockThenMaxDamage",
        "partial",
        Some(sequence),
        vec![format!(
            "unblocked_damage:{}",
            sequence.outcome.projected_unblocked_damage
        )],
        vec![format!("damage_done:{}", sequence.outcome.damage_done)],
        query_needs_deeper_search(Some(sequence), limits, "partial", false),
    )
}

fn query_can_play_setup_and_still_block(
    start: &CombatState,
    sequences: &[CombatPlanSequenceClass],
    limits: &CombatPlanProbeLimits,
) -> CombatPlanQueryReport {
    if !has_setup_or_scaling_card_in_hand(start) {
        return query_report(
            "CanPlaySetupAndStillBlock",
            "not_applicable",
            None,
            Vec::new(),
            vec!["no setup/scaling card in hand".to_string()],
            false,
        );
    }
    let setup_sequences = current_turn_sequences(sequences)
        .into_iter()
        .filter(|sequence| sequence.outcome.played_setup_or_scaling)
        .collect::<Vec<_>>();
    if setup_sequences.is_empty() {
        return query_report(
            "CanPlaySetupAndStillBlock",
            "not_feasible",
            None,
            vec!["setup_card_not_played".to_string()],
            Vec::new(),
            limits.pruned_by_budget > 0,
        );
    }
    let best_full_block = setup_sequences
        .iter()
        .copied()
        .filter(|sequence| {
            visible_incoming_damage(start) <= 0 || sequence.outcome.projected_unblocked_damage == 0
        })
        .max_by_key(|sequence| {
            (
                sequence.outcome.damage_done,
                sequence.outcome.remaining_energy,
            )
        });
    if let Some(sequence) = best_full_block {
        return query_report(
            "CanPlaySetupAndStillBlock",
            "feasible",
            Some(sequence),
            Vec::new(),
            vec!["setup/scaling played while meeting block constraint".to_string()],
            query_needs_deeper_search(Some(sequence), limits, "feasible", false),
        );
    }
    let sequence = setup_sequences
        .iter()
        .copied()
        .max_by_key(|sequence| {
            (
                -sequence.outcome.projected_unblocked_damage,
                sequence.outcome.damage_done,
                sequence.outcome.remaining_energy,
            )
        })
        .expect("setup_sequences is non-empty");
    query_report(
        "CanPlaySetupAndStillBlock",
        "partial",
        Some(sequence),
        vec![format!(
            "unblocked_damage_after_setup:{}",
            sequence.outcome.projected_unblocked_damage
        )],
        vec!["setup/scaling can be played but full block is not met".to_string()],
        query_needs_deeper_search(Some(sequence), limits, "partial", false),
    )
}

fn query_can_preserve_kill_window(
    start: &CombatState,
    sequences: &[CombatPlanSequenceClass],
    limits: &CombatPlanProbeLimits,
) -> CombatPlanQueryReport {
    let kill_window_cards = kill_window_card_labels_in_hand(start);
    if kill_window_cards.is_empty() {
        return query_report(
            "CanPreserveKillWindow",
            "not_applicable",
            None,
            Vec::new(),
            vec!["no Feed/HandOfGreed/RitualDagger in hand".to_string()],
            false,
        );
    }
    let best_preserve = current_turn_sequences(sequences)
        .into_iter()
        .filter(|sequence| {
            !sequence.outcome.played_kill_window_card
                && sequence.outcome.kill_window_target_count > 0
        })
        .max_by_key(|sequence| {
            (
                sequence.outcome.damage_done,
                -sequence.outcome.projected_unblocked_damage,
                sequence.outcome.kill_window_target_count as i32,
            )
        });
    if let Some(sequence) = best_preserve {
        return query_report(
            "CanPreserveKillWindow",
            "feasible",
            Some(sequence),
            Vec::new(),
            vec![
                format!("kill_window_cards:{}", kill_window_cards.join(",")),
                format!(
                    "kill_window_target_count:{}",
                    sequence.outcome.kill_window_target_count
                ),
            ],
            query_needs_deeper_search(Some(sequence), limits, "feasible", true),
        );
    }
    let best = current_turn_sequences(sequences)
        .into_iter()
        .max_by_key(|sequence| {
            (
                sequence.outcome.damage_done,
                -sequence.outcome.projected_unblocked_damage,
            )
        });
    query_report(
        "CanPreserveKillWindow",
        "not_feasible",
        best,
        vec!["no_preserved_kill_window_target".to_string()],
        vec![format!("kill_window_cards:{}", kill_window_cards.join(","))],
        query_needs_deeper_search(best, limits, "not_feasible", true),
    )
}

fn query_report(
    query_name: &str,
    status: &str,
    sequence: Option<&CombatPlanSequenceClass>,
    failed_constraints: Vec<String>,
    notes: Vec<String>,
    needs_deeper_search: bool,
) -> CombatPlanQueryReport {
    CombatPlanQueryReport {
        query_name: query_name.to_string(),
        status: status.to_string(),
        best_sequence_key: sequence.map(|sequence| sequence.sequence_equivalence_key.clone()),
        best_action_keys: sequence
            .map(|sequence| sequence.action_keys.clone())
            .unwrap_or_default(),
        best_actions: sequence
            .map(|sequence| sequence.actions.clone())
            .unwrap_or_default(),
        outcome: sequence.map(|sequence| sequence.outcome.clone()),
        failed_constraints,
        needs_deeper_search,
        notes,
    }
}

fn query_needs_deeper_search(
    sequence: Option<&CombatPlanSequenceClass>,
    limits: &CombatPlanProbeLimits,
    status: &str,
    kill_window_query: bool,
) -> bool {
    let budget_limited = limits.pruned_by_budget > 0 && status != "feasible";
    let Some(sequence) = sequence else {
        return budget_limited;
    };
    let random_risk = sequence.outcome.random_risk_present;
    let kill_window_precision = kill_window_query
        && (sequence.outcome.living_monster_count > 1
            || !sequence.order_sensitive_reasons.is_empty()
            || random_risk);
    budget_limited || random_risk || kill_window_precision
}

fn current_turn_sequences(sequences: &[CombatPlanSequenceClass]) -> Vec<&CombatPlanSequenceClass> {
    sequences
        .iter()
        .filter(|sequence| !sequence.outcome.ended_turn)
        .collect()
}

fn score_value_for_sort(score: Option<&PlanScoreBreakdown>) -> i32 {
    score.map(|score| score.total_score).unwrap_or(i32::MIN)
}

fn aggregate_component_max(sequences: &[&CombatPlanSequenceClass]) -> PlanScoreBreakdown {
    let mut aggregate = PlanScoreBreakdown {
        total_score: i32::MIN,
        lethal_score: i32::MIN,
        block_score: i32::MIN,
        hp_loss_score: i32::MIN,
        enemy_death_score: i32::MIN,
        damage_score: i32::MIN,
        setup_score: i32::MIN,
        exhaust_value: i32::MIN,
        key_card_risk: i32::MIN,
        random_risk: i32::MIN,
        future_hand_penalty: i32::MIN,
        strength_projection: i32::MIN,
        dex_projection: i32::MIN,
        vulnerable_projection: i32::MIN,
        optimistic_bound_score: i32::MIN,
    };
    for sequence in sequences {
        let score = &sequence.diagnostics;
        aggregate.total_score = aggregate.total_score.max(score.total_score);
        aggregate.lethal_score = aggregate.lethal_score.max(score.lethal_score);
        aggregate.block_score = aggregate.block_score.max(score.block_score);
        aggregate.hp_loss_score = aggregate.hp_loss_score.max(score.hp_loss_score);
        aggregate.enemy_death_score = aggregate.enemy_death_score.max(score.enemy_death_score);
        aggregate.damage_score = aggregate.damage_score.max(score.damage_score);
        aggregate.setup_score = aggregate.setup_score.max(score.setup_score);
        aggregate.exhaust_value = aggregate.exhaust_value.max(score.exhaust_value);
        aggregate.key_card_risk = aggregate.key_card_risk.max(score.key_card_risk);
        aggregate.random_risk = aggregate.random_risk.max(score.random_risk);
        aggregate.future_hand_penalty =
            aggregate.future_hand_penalty.max(score.future_hand_penalty);
        aggregate.strength_projection =
            aggregate.strength_projection.max(score.strength_projection);
        aggregate.dex_projection = aggregate.dex_projection.max(score.dex_projection);
        aggregate.vulnerable_projection = aggregate
            .vulnerable_projection
            .max(score.vulnerable_projection);
        aggregate.optimistic_bound_score = aggregate
            .optimistic_bound_score
            .max(score.optimistic_bound_score);
    }
    if aggregate.total_score == i32::MIN {
        PlanScoreBreakdown::default()
    } else {
        aggregate
    }
}

fn major_tradeoffs_for_first_action(
    score: &PlanScoreBreakdown,
    risk_note_kinds: &[String],
    order_sensitive_reasons: &[String],
) -> Vec<String> {
    let mut tradeoffs = Vec::new();
    if score.lethal_score > 0 {
        tradeoffs.push("can_end_combat".to_string());
    }
    if score.enemy_death_score > 0 {
        tradeoffs.push("can_kill_enemy".to_string());
    }
    if score.block_score >= 80 {
        tradeoffs.push("strong_defense_line".to_string());
    } else if score.block_score > 0 {
        tradeoffs.push("partial_defense_line".to_string());
    }
    if score.damage_score >= 72 {
        tradeoffs.push("strong_damage_progress".to_string());
    } else if score.damage_score > 0 {
        tradeoffs.push("damage_progress".to_string());
    }
    if score.setup_score > 0 {
        tradeoffs.push("setup_or_scaling".to_string());
    }
    if score.strength_projection > 0 {
        tradeoffs.push("strength_payoff_projection".to_string());
    }
    if score.vulnerable_projection > 0 {
        tradeoffs.push("vulnerable_payoff_projection".to_string());
    }
    if score.exhaust_value > 0 {
        tradeoffs.push("exhaust_cleanup_or_synergy".to_string());
    }
    if score.hp_loss_score < 0 {
        tradeoffs.push("accepts_hp_loss".to_string());
    }
    if score.future_hand_penalty < 0 {
        tradeoffs.push("spends_or_destroys_hand".to_string());
    }
    if score.key_card_risk < 0 || score.random_risk < 0 || !risk_note_kinds.is_empty() {
        tradeoffs.push("explicit_risk_note".to_string());
    }
    if !order_sensitive_reasons.is_empty() {
        tradeoffs.push("order_sensitive".to_string());
    }
    tradeoffs
}

fn explore_sequence_classes(
    engine: &EngineState,
    combat: &CombatState,
    config: CombatTurnPlanProbeConfig,
    options: &CombatTurnPlanExploreOptions,
) -> (
    Vec<CombatPlanSequenceClass>,
    CombatPlanProbeLimits,
    Vec<CombatPlanRiskNote>,
) {
    let mut queue = VecDeque::new();
    queue.push_back(ProbeNode {
        engine: engine.clone(),
        combat: combat.clone(),
        actions: Vec::new(),
        action_keys: Vec::new(),
        order_sensitive_reasons: BTreeSet::new(),
        sequence_residues: BTreeSet::new(),
        canonical_last_keys: BTreeMap::new(),
        compression_notes: BTreeSet::new(),
        risk_notes: Vec::new(),
        accumulated: AccumulatedSequenceEffects::default(),
        depth: 0,
        ended_turn: false,
    });

    let mut seen = BTreeMap::<String, i32>::new();
    let mut seen_abstract = BTreeMap::<String, Vec<AbstractSeen>>::new();
    let mut kept = Vec::new();
    let mut all_risk_notes = Vec::new();
    let mut nodes_expanded = 0usize;
    let mut actions_considered = 0usize;
    let mut actions_simulated = 0usize;
    let mut pruned_as_equivalent = 0usize;
    let mut pruned_by_abstract_equivalence = 0usize;
    let mut abstract_equivalence_candidates = 0usize;
    let mut abstract_equivalence_blocked_by_context = 0usize;
    let mut abstract_equivalence_blocked_by_action_semantics = 0usize;
    let mut abstract_equivalence_rejected_by_engine = 0usize;
    let mut pruned_by_verified_abstract_equivalence = 0usize;
    let mut generation_canonical_candidates = 0usize;
    let mut generation_canonical_blocked_by_context = 0usize;
    let mut generation_canonical_blocked_by_action_semantics = 0usize;
    let mut pruned_by_generation_canonical_order = 0usize;
    let mut pruned_by_generation_duplicate_card = 0usize;
    let mut pruned_by_generation_same_lane_order = 0usize;
    let mut pruned_by_generation_target_order = 0usize;
    let mut pruned_by_generation_lane_order = 0usize;
    let mut generation_duplicate_prune_effects = BTreeMap::<String, usize>::new();
    let mut pruned_by_plan_expansion_gate = 0usize;
    let mut plan_expansion_gate_reasons = BTreeMap::<String, usize>::new();
    let mut plan_expansion_gate_examples = Vec::new();
    let mut pruned_by_optimistic_bound = 0usize;
    let mut pruned_by_budget = 0usize;
    let pruned_by_dominated_state = 0usize;
    let mut profile = SearchProfileBreakdown::default();
    let mut best_total_score = i32::MIN;

    while let Some(node) = queue.pop_front() {
        if nodes_expanded >= config.max_nodes {
            pruned_by_budget += queue.len() + 1;
            break;
        }
        nodes_expanded += 1;

        if !node.actions.is_empty() {
            let mut diagnostics = diagnose_sequence(combat, &node.combat, &node.accumulated);
            diagnostics.optimistic_bound_score = optimistic_bound_score(
                &node.combat,
                &node.order_sensitive_reasons,
                &node.sequence_residues,
            );
            let outcome =
                build_sequence_outcome(combat, &node.combat, &node.accumulated, node.ended_turn);
            let key = sequence_equivalence_key(&node.engine, &node.combat);
            let mut compression_notes = node.compression_notes.iter().cloned().collect::<Vec<_>>();
            if !node.order_sensitive_reasons.is_empty()
                && !compression_notes
                    .iter()
                    .any(|note| note == "order_sensitive_sequence")
            {
                compression_notes.push("order_sensitive_sequence".to_string());
            }
            kept.push(CombatPlanSequenceClass {
                sequence_equivalence_key: key,
                actions: node.actions.clone(),
                action_keys: node.action_keys.clone(),
                order_sensitive_reasons: node
                    .order_sensitive_reasons
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>(),
                compression_notes,
                diagnostics,
                outcome,
                pruned_as_equivalent: false,
                pruned_by_budget: false,
                pruned_by_dominated_state: false,
            });
            all_risk_notes.extend(node.risk_notes.clone());
            best_total_score = best_total_score.max(
                kept.last()
                    .map(|sequence| sequence.diagnostics.total_score)
                    .unwrap_or(i32::MIN),
            );
        }

        if node.depth >= config.max_depth || node.ended_turn || !is_probe_frontier(&node.engine) {
            continue;
        }
        let node_diag = diagnose_sequence(combat, &node.combat, &node.accumulated);
        let node_bound = node_diag.total_score
            + optimistic_bound_score(
                &node.combat,
                &node.order_sensitive_reasons,
                &node.sequence_residues,
            );
        if best_total_score != i32::MIN
            && can_bound_prune(
                &node.engine,
                &node.combat,
                &node.order_sensitive_reasons,
                &node.sequence_residues,
            )
            && node_bound + 300 < best_total_score
        {
            pruned_by_optimistic_bound += 1;
            continue;
        }

        let mut legal = get_legal_moves(&node.engine, &node.combat)
            .into_iter()
            .filter(|action| allowed_probe_action(&node.engine, action))
            .filter(|action| allowed_by_draw_marginal_options(&node.combat, action, &node, options))
            .collect::<Vec<_>>();
        legal.sort_by_key(|action| -action_order_score(&node.combat, action));
        legal.truncate(config.beam_width);

        let mut seen_generation_sibling_keys = BTreeSet::new();
        for action in legal {
            if nodes_expanded + queue.len() >= config.max_nodes {
                pruned_by_budget += 1;
                continue;
            }
            actions_considered += 1;
            let action_key = probe_action_key(&node.combat, &action);
            let action_summary = summarize_action_effect(&node.combat, &action);
            if let Some(reason) = plan_expansion_gate_reason(
                &node.engine,
                &node.combat,
                &action,
                &action_summary,
                &node.accumulated,
                &node.sequence_residues,
            ) {
                pruned_by_plan_expansion_gate += 1;
                increment_counter(&mut plan_expansion_gate_reasons, reason.to_string());
                if plan_expansion_gate_examples.len() < 32 {
                    plan_expansion_gate_examples.push(plan_expansion_gate_example(
                        reason,
                        &node,
                        &action_key,
                        &action,
                    ));
                }
                continue;
            }
            let mut generation_canonical_notes = BTreeSet::new();
            let mut next_canonical_last_keys = BTreeMap::new();
            match generation_canonical_decision(
                &node.combat,
                &node.engine,
                &action,
                &action_summary,
                &node.order_sensitive_reasons,
                &node.sequence_residues,
            ) {
                GenerationCanonicalDecision::Candidate(keys) => {
                    generation_canonical_candidates += 1;
                    generation_canonical_notes.insert("generation_canonical_candidate".to_string());
                    generation_canonical_notes
                        .insert(format!("generation_canonical_class:{}", keys.class_label));
                    generation_canonical_notes
                        .insert(format!("generation_canonical_lane:{}", keys.lane_label));
                    if !seen_generation_sibling_keys.insert(keys.sort_key.clone()) {
                        pruned_by_generation_canonical_order += 1;
                        pruned_by_generation_duplicate_card += 1;
                        increment_counter(
                            &mut generation_duplicate_prune_effects,
                            keys.effect_key.clone(),
                        );
                        continue;
                    }
                    if let Some(last_key) = node.canonical_last_keys.get(&keys.group_key) {
                        if keys.sort_key < *last_key {
                            pruned_by_generation_canonical_order += 1;
                            match classify_generation_prune_kind(last_key, &keys.sort_key) {
                                GenerationPruneKind::DuplicateCard => {
                                    pruned_by_generation_duplicate_card += 1;
                                    increment_counter(
                                        &mut generation_duplicate_prune_effects,
                                        keys.effect_key.clone(),
                                    );
                                }
                                GenerationPruneKind::SameLaneOrder => {
                                    pruned_by_generation_same_lane_order += 1;
                                }
                                GenerationPruneKind::TargetOrder => {
                                    pruned_by_generation_target_order += 1;
                                }
                                GenerationPruneKind::LaneOrder => {
                                    pruned_by_generation_lane_order += 1;
                                }
                            }
                            continue;
                        }
                    }
                    next_canonical_last_keys = node.canonical_last_keys.clone();
                    next_canonical_last_keys.insert(keys.group_key, keys.sort_key);
                }
                GenerationCanonicalDecision::BlockedByActionSemantics(reason) => {
                    generation_canonical_blocked_by_action_semantics += 1;
                    generation_canonical_notes
                        .insert("generation_canonical_blocked_by_action_semantics".to_string());
                    generation_canonical_notes
                        .insert(format!("generation_canonical_action_blocker:{reason}"));
                }
                GenerationCanonicalDecision::BlockedByContext(reason) => {
                    generation_canonical_blocked_by_context += 1;
                    generation_canonical_notes
                        .insert("generation_canonical_blocked_by_context".to_string());
                    generation_canonical_notes
                        .insert(format!("generation_canonical_context_blocker:{reason}"));
                }
            }
            let mut next_accumulated = node.accumulated.clone();
            let mut next_reasons = node.order_sensitive_reasons.clone();
            let mut next_residues = node.sequence_residues.clone();
            let mut next_compression_notes = node.compression_notes.clone();
            next_compression_notes.extend(generation_canonical_notes);
            let mut next_notes = node.risk_notes.clone();
            for residue in action_sequence_residues(&node.combat, &action) {
                next_residues.insert(residue.to_string());
                next_compression_notes.insert(format!("sequence_residue:{residue}"));
            }
            accumulate_action_effects(
                &node.combat,
                &action,
                &action_summary,
                node.actions.len(),
                &action_key,
                &mut next_accumulated,
                &mut next_reasons,
                &mut next_notes,
            );

            actions_simulated += 1;
            let (next_engine, next_combat, outcome) = simulate_input_bounded(
                &node.engine,
                &node.combat,
                &action,
                config.max_engine_steps_per_action,
                None,
                &mut profile,
            );
            let next_key = sequence_equivalence_key(&next_engine, &next_combat);
            let next_diag = diagnose_sequence(combat, &next_combat, &next_accumulated);
            let next_score = next_diag.total_score;
            match abstract_compression_decision(
                &node.combat,
                &next_engine,
                &action_summary,
                &next_reasons,
                &next_residues,
            ) {
                AbstractCompressionDecision::Candidate => {
                    abstract_equivalence_candidates += 1;
                    next_compression_notes.insert("abstract_candidate".to_string());
                    let abstract_key = abstract_sequence_key(&next_engine, &next_combat);
                    let verification_snapshot =
                        abstract_verification_snapshot(&next_engine, &next_combat);
                    let mut rejected_by_engine = false;
                    let mut replace_seen_score = None;
                    let mut rejection_reasons = Vec::new();
                    if let Some(previous_entries) = seen_abstract.get_mut(&abstract_key) {
                        if let Some((idx, previous)) =
                            previous_entries.iter_mut().enumerate().find(|(_, entry)| {
                                entry
                                    .verification_snapshot
                                    .behaviorally_matches(&verification_snapshot)
                            })
                        {
                            if previous.score >= next_score {
                                pruned_by_abstract_equivalence += 1;
                                pruned_by_verified_abstract_equivalence += 1;
                                continue;
                            }
                            replace_seen_score = Some((idx, next_score));
                        } else if !previous_entries.is_empty() {
                            abstract_equivalence_rejected_by_engine += 1;
                            rejected_by_engine = true;
                            rejection_reasons = abstract_rejection_reasons(
                                previous_entries.as_slice(),
                                &verification_snapshot,
                            );
                        }
                    }
                    if rejected_by_engine {
                        next_compression_notes.insert("abstract_rejected_by_engine".to_string());
                        for reason in rejection_reasons {
                            next_compression_notes.insert(format!("abstract_reject_diff:{reason}"));
                        }
                    } else {
                        next_compression_notes.insert("verified_abstract_equivalence".to_string());
                    }
                    let previous_entries = seen_abstract.entry(abstract_key).or_default();
                    if let Some((idx, score)) = replace_seen_score {
                        previous_entries[idx].score = score;
                    } else {
                        previous_entries.push(AbstractSeen {
                            verification_snapshot,
                            score: next_score,
                        });
                    }
                }
                AbstractCompressionDecision::BlockedByActionSemantics(reason) => {
                    abstract_equivalence_blocked_by_action_semantics += 1;
                    next_compression_notes
                        .insert("abstract_blocked_by_action_semantics".to_string());
                    next_compression_notes.insert(format!("abstract_action_blocker:{reason}"));
                }
                AbstractCompressionDecision::BlockedByContext(reason) => {
                    abstract_equivalence_blocked_by_context += 1;
                    next_compression_notes.insert("abstract_blocked_by_context".to_string());
                    next_compression_notes.insert(format!("abstract_context_blocker:{reason}"));
                }
            }
            if let Some(previous_score) = seen.get(&next_key) {
                if *previous_score >= next_score {
                    pruned_as_equivalent += 1;
                    continue;
                }
            }
            seen.insert(next_key, next_score);

            let mut actions = node.actions.clone();
            actions.push(format!("{action:?}"));
            let mut action_keys = node.action_keys.clone();
            action_keys.push(action_key);
            queue.push_back(ProbeNode {
                engine: next_engine,
                combat: next_combat,
                actions,
                action_keys,
                order_sensitive_reasons: next_reasons,
                sequence_residues: next_residues,
                canonical_last_keys: next_canonical_last_keys,
                compression_notes: next_compression_notes,
                risk_notes: next_notes,
                accumulated: next_accumulated,
                depth: node.depth + 1,
                ended_turn: matches!(action, ClientInput::EndTurn) || !outcome.alive,
            });
        }
    }

    kept.sort_by(|a, b| b.diagnostics.total_score.cmp(&a.diagnostics.total_score));
    kept.truncate(64);
    all_risk_notes.sort_by(|a, b| {
        a.sequence_action_index
            .cmp(&b.sequence_action_index)
            .then_with(|| a.action_key.cmp(&b.action_key))
            .then_with(|| a.kind.cmp(&b.kind))
    });
    all_risk_notes.dedup_by(|a, b| {
        a.sequence_action_index == b.sequence_action_index
            && a.action_key == b.action_key
            && a.kind == b.kind
            && a.message == b.message
    });

    let limits = CombatPlanProbeLimits {
        max_depth: config.max_depth,
        max_nodes: config.max_nodes,
        beam_width: config.beam_width,
        max_engine_steps_per_action: config.max_engine_steps_per_action,
        nodes_expanded,
        actions_considered,
        actions_simulated,
        sequence_classes_kept: kept.len(),
        pruned_as_equivalent,
        pruned_by_abstract_equivalence,
        abstract_equivalence_candidates,
        abstract_equivalence_blocked_by_context,
        abstract_equivalence_blocked_by_action_semantics,
        abstract_equivalence_rejected_by_engine,
        pruned_by_verified_abstract_equivalence,
        generation_canonical_candidates,
        generation_canonical_blocked_by_context,
        generation_canonical_blocked_by_action_semantics,
        pruned_by_generation_canonical_order,
        pruned_by_generation_duplicate_card,
        pruned_by_generation_same_lane_order,
        pruned_by_generation_target_order,
        pruned_by_generation_lane_order,
        generation_duplicate_prune_effects,
        pruned_by_plan_expansion_gate,
        plan_expansion_gate_reasons,
        plan_expansion_gate_examples,
        pruned_by_optimistic_bound,
        pruned_by_budget,
        pruned_by_dominated_state,
    };
    (kept, limits, all_risk_notes)
}

fn build_plan_report(
    plan: CombatPlanKind,
    sequences: &[CombatPlanSequenceClass],
) -> CombatPlanReport {
    let mut best: Option<(i32, PlanScoreBreakdown, &CombatPlanSequenceClass)> = None;
    for sequence in sequences {
        let score = score_for_plan(plan, &sequence.diagnostics);
        let mut breakdown = sequence.diagnostics.clone();
        breakdown.total_score = score;
        if best
            .as_ref()
            .is_none_or(|(best_score, _, _)| score > *best_score)
        {
            best = Some((score, breakdown, sequence));
        }
    }

    CombatPlanReport {
        plan_name: plan_label(plan).to_string(),
        best_sequence_key: best
            .as_ref()
            .map(|(_, _, sequence)| sequence.sequence_equivalence_key.clone()),
        best_actions: best
            .as_ref()
            .map(|(_, _, sequence)| sequence.actions.clone())
            .unwrap_or_default(),
        best_action_keys: best
            .as_ref()
            .map(|(_, _, sequence)| sequence.action_keys.clone())
            .unwrap_or_default(),
        best_score: best.map(|(_, breakdown, _)| breakdown),
        candidate_sequence_count: sequences.len(),
        explanation: plan_explanation(plan).to_string(),
    }
}

fn score_for_plan(plan: CombatPlanKind, score: &PlanScoreBreakdown) -> i32 {
    let risk = score.key_card_risk + score.random_risk + score.future_hand_penalty;
    match plan {
        CombatPlanKind::Lethal => {
            score.lethal_score * 5 + score.damage_score * 2 + score.enemy_death_score + risk
        }
        CombatPlanKind::KillThreateningEnemy => {
            score.enemy_death_score * 3 + score.block_score + score.damage_score + risk
        }
        CombatPlanKind::FullBlock => score.block_score * 5 + score.hp_loss_score * 2 + risk / 2,
        CombatPlanKind::BlockEnoughThenDamage => {
            score.block_score * 3 + score.damage_score * 2 + score.enemy_death_score + risk / 2
        }
        CombatPlanKind::MaxDamage => score.damage_score * 4 + score.enemy_death_score + risk / 2,
        CombatPlanKind::SetupPowerOrScaling => {
            score.setup_score * 5 + score.block_score + score.damage_score + risk
        }
        CombatPlanKind::ExhaustBadCards => {
            score.exhaust_value * 5 + score.block_score + score.setup_score + risk
        }
        CombatPlanKind::PreserveKeyCards => {
            score.block_score + score.damage_score + score.key_card_risk * 5 + score.random_risk
        }
    }
}

fn diagnose_sequence(
    start: &CombatState,
    final_state: &CombatState,
    accumulated: &AccumulatedSequenceEffects,
) -> PlanScoreBreakdown {
    let start_hp = start.entities.player.current_hp;
    let final_hp = final_state.entities.player.current_hp;
    let start_enemy_hp = total_alive_monster_hp(start);
    let final_enemy_hp = total_alive_monster_hp(final_state);
    let damage_delta = (start_enemy_hp - final_enemy_hp).max(0);
    let enemy_deaths =
        living_monster_count(start).saturating_sub(living_monster_count(final_state));
    let incoming = visible_incoming_damage(final_state);
    let unblocked = (incoming - final_state.entities.player.block).max(0);
    let block_score = ((visible_incoming_damage(start) - unblocked).max(0) * 8)
        + final_state.entities.player.block.min(60);
    let lethal_score = if living_monster_count(final_state) == 0 {
        1_000
    } else {
        0
    };
    let hp_loss_score = -(start_hp - final_hp).max(0) * 20;
    let enemy_death_score = enemy_deaths as i32 * 160;
    let damage_score = damage_delta * 6;
    let future_hand_penalty = accumulated.future_hand_penalty
        - start
            .zones
            .hand
            .len()
            .saturating_sub(final_state.zones.hand.len()) as i32
            * 3;

    let total_score = lethal_score
        + block_score
        + hp_loss_score
        + enemy_death_score
        + damage_score
        + accumulated.setup_score
        + accumulated.exhaust_value
        + accumulated.key_card_risk
        + accumulated.random_risk
        + future_hand_penalty
        + accumulated.strength_projection * 4
        + accumulated.dex_projection * 3
        + accumulated.vulnerable_projection * 3;

    PlanScoreBreakdown {
        total_score,
        lethal_score,
        block_score,
        hp_loss_score,
        enemy_death_score,
        damage_score,
        setup_score: accumulated.setup_score,
        exhaust_value: accumulated.exhaust_value,
        key_card_risk: accumulated.key_card_risk,
        random_risk: accumulated.random_risk,
        future_hand_penalty,
        strength_projection: accumulated.strength_projection,
        dex_projection: accumulated.dex_projection,
        vulnerable_projection: accumulated.vulnerable_projection,
        optimistic_bound_score: 0,
    }
}

fn build_sequence_outcome(
    start: &CombatState,
    final_state: &CombatState,
    accumulated: &AccumulatedSequenceEffects,
    ended_turn: bool,
) -> CombatPlanSequenceOutcome {
    let start_hp = start.entities.player.current_hp;
    let final_hp = final_state.entities.player.current_hp;
    let start_enemy_hp = total_alive_monster_hp(start);
    let final_enemy_hp = total_alive_monster_hp(final_state);
    let incoming = visible_incoming_damage(final_state);
    let enemy_deaths =
        living_monster_count(start).saturating_sub(living_monster_count(final_state));
    CombatPlanSequenceOutcome {
        damage_done: (start_enemy_hp - final_enemy_hp).max(0),
        block_after: final_state.entities.player.block,
        projected_unblocked_damage: (incoming - final_state.entities.player.block).max(0),
        hp_loss_actual: (start_hp - final_hp).max(0),
        remaining_energy: final_state.turn.energy as i32,
        remaining_hand_count: final_state.zones.hand.len(),
        enemy_deaths,
        living_monster_count: living_monster_count(final_state),
        total_monster_hp: final_enemy_hp,
        played_setup_or_scaling: accumulated.played_setup_or_scaling,
        played_kill_window_card: accumulated.played_kill_window_card,
        random_risk_present: accumulated.random_risk_present || accumulated.random_risk != 0,
        ended_turn,
        kill_window_target_count: kill_window_target_count(final_state),
    }
}

fn accumulate_action_effects(
    combat: &CombatState,
    action: &ClientInput,
    summary: &ActionEffectSummary,
    sequence_action_index: usize,
    action_key: &str,
    accumulated: &mut AccumulatedSequenceEffects,
    order_sensitive_reasons: &mut BTreeSet<String>,
    risk_notes: &mut Vec<CombatPlanRiskNote>,
) {
    match action {
        ClientInput::PlayCard { card_index, .. } => {
            let Some(card) = combat.zones.hand.get(*card_index) else {
                return;
            };
            let facts = card_facts::facts(card.id);
            let structure = card_structure::structure(card.id);
            if summary.draws_cards {
                order_sensitive_reasons.insert("draw_changes_future_action_space".to_string());
            }
            if facts.applies_vuln || facts.applies_weak {
                order_sensitive_reasons.insert("debuff_before_damage_can_change_value".to_string());
            }
            if summary.energy_gain {
                order_sensitive_reasons
                    .insert("energy_gain_changes_future_action_space".to_string());
            }
            if facts.exhausts_other_cards {
                order_sensitive_reasons.insert("exhaust_changes_hand_and_deck_state".to_string());
            }
            if facts.creates_cards {
                order_sensitive_reasons
                    .insert("card_creation_changes_future_card_zones".to_string());
            }
            if summary.immediate_action_space_change {
                accumulated.action_space_change_count += 1;
            }
            if facts.random_generation || card.id == CardId::TrueGrit && card.upgrades == 0 {
                order_sensitive_reasons.insert("random_effect_requires_risk_model".to_string());
                accumulated.random_risk_present = true;
            }
            if structure.is_setup_piece() || structure.is_scaling_piece() {
                accumulated.setup_score += 90;
                accumulated.played_setup_or_scaling = true;
            }
            if summary.strength_gain > 0 {
                let projection = (summary.strength_gain
                    * remaining_attack_hit_count(combat, Some(*card_index)))
                .min(total_alive_monster_hp(combat).max(0));
                accumulated.strength_projection += projection;
                accumulated.setup_score += projection * 4;
            }
            if summary.dex_gain > 0 {
                let projection =
                    summary.dex_gain * remaining_block_card_count(combat, Some(*card_index));
                accumulated.dex_projection += projection;
                accumulated.setup_score += projection * 3;
            }
            if summary.vulnerable_projection > 0 {
                accumulated.vulnerable_projection += summary.vulnerable_projection;
                accumulated.setup_score += summary.vulnerable_projection * 3;
            }
            if summary.attack_hit_count > 0 {
                let strength = combat.get_power(0, crate::runtime::combat::PowerId::Strength);
                if strength > 0 {
                    accumulated.strength_projection += strength * summary.attack_hit_count;
                }
            }
            if is_kill_window_card(card.id) {
                accumulated.played_kill_window_card = true;
            }
            if facts.exhausts_other_cards {
                accumulated.future_hand_penalty -= 12;
            }
            if card.id == CardId::TrueGrit && card.upgrades == 0 {
                add_true_grit_random_overlay(
                    combat,
                    *card_index,
                    sequence_action_index,
                    action_key,
                    accumulated,
                    risk_notes,
                );
            } else if card.id == CardId::TrueGrit {
                risk_notes.push(CombatPlanRiskNote {
                    sequence_action_index,
                    action_key: action_key.to_string(),
                    kind: "chosen_exhaust_pending".to_string(),
                    message:
                        "True Grit+ uses engine hand-select; selected card is exact, not random."
                            .to_string(),
                    chance_model: None,
                    exact_rng_branches: true,
                    risk_is_overlay_only: false,
                    bad_branch_probability_milli: None,
                    good_branch_probability_milli: None,
                    affected_cards: Vec::new(),
                });
            } else if card.id == CardId::SecondWind {
                accumulated.exhaust_value += exhaust_outlet_value(combat, Some(*card_index));
                accumulated.future_hand_penalty -= 30;
                risk_notes.push(CombatPlanRiskNote {
                    sequence_action_index,
                    action_key: action_key.to_string(),
                    kind: "second_wind_multi_plan_semantics".to_string(),
                    message:
                        "Second Wind can be block, deck cleanup, exhaust synergy, or hand-destruction risk depending on plan."
                            .to_string(),
                    chance_model: None,
                    exact_rng_branches: true,
                    risk_is_overlay_only: false,
                    bad_branch_probability_milli: None,
                    good_branch_probability_milli: None,
                    affected_cards: non_attack_hand_cards(combat, Some(*card_index)),
                });
            } else if card.id == CardId::FiendFire {
                accumulated.exhaust_value += exhaust_outlet_value(combat, Some(*card_index));
                accumulated.future_hand_penalty -= 60;
                risk_notes.push(CombatPlanRiskNote {
                    sequence_action_index,
                    action_key: action_key.to_string(),
                    kind: "fiend_fire_multi_plan_semantics".to_string(),
                    message:
                        "Fiend Fire can be lethal, exhaust payoff, or severe hand-destruction risk; V1 reports both sides."
                            .to_string(),
                    chance_model: None,
                    exact_rng_branches: true,
                    risk_is_overlay_only: false,
                    bad_branch_probability_milli: None,
                    good_branch_probability_milli: None,
                    affected_cards: hand_cards_except(combat, Some(*card_index)),
                });
            }
            if possible_kill_with_card(combat, card) {
                order_sensitive_reasons.insert("possible_kill_changes_incoming_damage".to_string());
            }
        }
        ClientInput::SubmitHandSelect(uuids) | ClientInput::SubmitGridSelect(uuids) => {
            let selected = uuids
                .iter()
                .filter_map(|uuid| hand_card_label_by_uuid(combat, *uuid))
                .collect::<Vec<_>>();
            accumulated.exhaust_value += uuids
                .iter()
                .map(|uuid| {
                    crate::bot::card_disposition::combat_exhaust_score_for_uuid(combat, *uuid) / 100
                })
                .sum::<i32>();
            risk_notes.push(CombatPlanRiskNote {
                sequence_action_index,
                action_key: action_key.to_string(),
                kind: "exact_selection_resolution".to_string(),
                message:
                    "Pending selection is an exact engine branch chosen by legal move enumeration."
                        .to_string(),
                chance_model: None,
                exact_rng_branches: true,
                risk_is_overlay_only: false,
                bad_branch_probability_milli: None,
                good_branch_probability_milli: None,
                affected_cards: selected,
            });
        }
        _ => {}
    }
}

fn action_sequence_residues(combat: &CombatState, action: &ClientInput) -> Vec<&'static str> {
    let ClientInput::PlayCard { card_index, .. } = action else {
        return Vec::new();
    };
    let Some(card) = combat.zones.hand.get(*card_index) else {
        return Vec::new();
    };
    let facts = card_facts::facts(card.id);
    if facts.self_replicating {
        return vec!["generated_card_copy"];
    }
    if facts.produces_status {
        return vec!["generated_status_card"];
    }
    if facts.creates_cards {
        return vec!["generated_card_zone_mutation"];
    }
    match card.id {
        CardId::Headbutt if !combat.zones.discard_pile.is_empty() => {
            vec!["future_draw_order"]
        }
        CardId::Warcry | CardId::ThinkingAhead => vec!["future_draw_order"],
        CardId::DeepBreath if !combat.zones.discard_pile.is_empty() => {
            vec!["shuffle_draw_order"]
        }
        CardId::Forethought if combat.zones.hand.len() > 1 => vec!["future_draw_order"],
        CardId::SecretTechnique | CardId::SecretWeapon if !combat.zones.draw_pile.is_empty() => {
            vec!["deck_search_to_hand"]
        }
        _ => Vec::new(),
    }
}

fn dropkick_payoff_active(combat: &CombatState, target: Option<usize>) -> bool {
    target.is_some_and(|target| combat.get_power(target, PowerId::Vulnerable) > 0)
}

fn action_changes_current_action_space(
    combat: &CombatState,
    action: &ClientInput,
    card: &CombatCard,
) -> bool {
    let target = match action {
        ClientInput::PlayCard { target, .. } => *target,
        _ => None,
    };
    match card.id {
        CardId::Dropkick => dropkick_payoff_active(combat, target),
        CardId::Impatience => !combat
            .zones
            .hand
            .iter()
            .any(|card| cards::get_card_definition(card.id).card_type == CardType::Attack),
        CardId::SecretTechnique => combat
            .zones
            .draw_pile
            .iter()
            .any(|card| cards::get_card_definition(card.id).card_type == CardType::Skill),
        CardId::SecretWeapon | CardId::Violence => combat
            .zones
            .draw_pile
            .iter()
            .any(|card| cards::get_card_definition(card.id).card_type == CardType::Attack),
        CardId::Forethought => combat.zones.hand.len() > 1,
        CardId::DeepBreath => {
            !combat.zones.draw_pile.is_empty() || !combat.zones.discard_pile.is_empty()
        }
        CardId::Acrobatics
        | CardId::Adrenaline
        | CardId::Backflip
        | CardId::BattleTrance
        | CardId::BurningPact
        | CardId::Discovery
        | CardId::DaggerThrow
        | CardId::Finesse
        | CardId::FlashOfSteel
        | CardId::InfernalBlade
        | CardId::JackOfAllTrades
        | CardId::MasterOfStrategy
        | CardId::Offering
        | CardId::PommelStrike
        | CardId::Prepared
        | CardId::ShrugItOff
        | CardId::ThinkingAhead
        | CardId::Warcry => true,
        _ => false,
    }
}

fn add_true_grit_random_overlay(
    combat: &CombatState,
    played_hand_index: usize,
    sequence_action_index: usize,
    action_key: &str,
    accumulated: &mut AccumulatedSequenceEffects,
    risk_notes: &mut Vec<CombatPlanRiskNote>,
) {
    let candidates = combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(idx, _)| *idx != played_hand_index)
        .map(|(_, card)| card)
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return;
    }

    let bad_cards = candidates
        .iter()
        .filter(|card| {
            crate::bot::card_disposition::combat_retention_score_for_uuid(combat, card.uuid)
                >= 4_000
        })
        .map(|card| hand_card_label(card))
        .collect::<Vec<_>>();
    let good_cards = candidates
        .iter()
        .filter(|card| {
            crate::bot::card_disposition::combat_exhaust_score_for_uuid(combat, card.uuid) >= 1_200
        })
        .map(|card| hand_card_label(card))
        .collect::<Vec<_>>();

    let bad_milli = bad_cards.len() as i32 * 1000 / candidates.len() as i32;
    let good_milli = good_cards.len() as i32 * 1000 / candidates.len() as i32;
    accumulated.random_risk -= bad_milli / 8;
    accumulated.key_card_risk -= bad_milli / 5;
    accumulated.exhaust_value += good_milli / 8;

    let mut affected = bad_cards.clone();
    for card in good_cards {
        if !affected.contains(&card) {
            affected.push(card);
        }
    }
    risk_notes.push(CombatPlanRiskNote {
        sequence_action_index,
        action_key: action_key.to_string(),
        kind: "true_grit_random_exhaust_overlay".to_string(),
        message:
            "Unupgraded True Grit uses a static remaining-hand distribution overlay; this is not exact RNG branch enumeration."
                .to_string(),
        chance_model: Some("static_hand_distribution".to_string()),
        exact_rng_branches: false,
        risk_is_overlay_only: true,
        bad_branch_probability_milli: Some(bad_milli),
        good_branch_probability_milli: Some(good_milli),
        affected_cards: affected,
    });
}

fn summarize_action_effect(combat: &CombatState, action: &ClientInput) -> ActionEffectSummary {
    let mut summary = ActionEffectSummary::default();
    let ClientInput::PlayCard { card_index, target } = action else {
        summary.target_sensitive = !matches!(action, ClientInput::EndTurn);
        return summary;
    };
    let Some(card) = combat.zones.hand.get(*card_index) else {
        summary.target_sensitive = true;
        return summary;
    };
    let def = cards::get_card_definition(card.id);
    let facts = card_facts::facts(card.id);
    let structure = card_structure::structure(card.id);

    summary.damage_estimate = estimate_card_damage(card, combat);
    summary.block_estimate = estimate_card_block(card);
    summary.attack_hit_count = attack_hit_count(card.id, card.upgrades, combat);
    let dropkick_payoff = card.id == CardId::Dropkick && dropkick_payoff_active(combat, *target);
    summary.draws_cards = facts.draws_cards && (card.id != CardId::Dropkick || dropkick_payoff);
    summary.energy_gain = facts.gains_energy || dropkick_payoff;
    summary.debuff = facts.applies_vuln || facts.applies_weak || facts.applies_frail;
    summary.setup_or_scaling = structure.is_setup_piece() || structure.is_scaling_piece();
    summary.exhaust_or_discard = facts.exhausts_other_cards;
    summary.creates_cards = facts.creates_cards;
    summary.immediate_action_space_change =
        action_changes_current_action_space(combat, action, card);
    summary.random_effect =
        facts.random_generation || card.id == CardId::TrueGrit && card.upgrades == 0;
    summary.possible_kill = possible_kill_with_card(combat, card);
    summary.target_sensitive = facts.target_sensitive || target.is_some() && summary.debuff;
    summary.pure_damage = def.card_type == CardType::Attack
        && summary.damage_estimate > 0
        && summary.block_estimate == 0
        && !summary.draws_cards
        && !summary.energy_gain
        && !summary.debuff
        && !summary.setup_or_scaling
        && !summary.exhaust_or_discard
        && !summary.creates_cards
        && !summary.random_effect
        && !summary.target_sensitive
        && !summary.possible_kill;
    summary.pure_block = def.card_type == CardType::Skill
        && summary.block_estimate > 0
        && summary.damage_estimate == 0
        && !summary.draws_cards
        && !summary.energy_gain
        && !summary.debuff
        && !summary.setup_or_scaling
        && !summary.exhaust_or_discard
        && !summary.creates_cards
        && !summary.random_effect
        && !summary.target_sensitive
        && !summary.possible_kill;
    summary.strength_gain = strength_gain_for_card(card);
    summary.dex_gain = dex_gain_for_card(card);
    if facts.applies_vuln {
        summary.vulnerable_projection =
            vulnerable_projection_for_action(combat, *card_index, *target);
    }
    summary
}

fn estimate_card_damage(card: &CombatCard, combat: &CombatState) -> i32 {
    let def = cards::get_card_definition(card.id);
    let base = if card.base_damage_mut > 0 {
        card.base_damage_mut
    } else {
        def.base_damage + def.upgrade_damage * card.upgrades as i32
    };
    (base * attack_hit_count(card.id, card.upgrades, combat)).max(0)
}

fn estimate_card_block(card: &CombatCard) -> i32 {
    let def = cards::get_card_definition(card.id);
    if card.base_block_mut > 0 {
        card.base_block_mut
    } else {
        def.base_block + def.upgrade_block * card.upgrades as i32
    }
    .max(0)
}

fn attack_hit_count(card_id: CardId, upgrades: u8, combat: &CombatState) -> i32 {
    match card_id {
        CardId::TwinStrike => 2,
        CardId::Pummel => {
            if upgrades > 0 {
                5
            } else {
                4
            }
        }
        CardId::SwordBoomerang => {
            if upgrades > 0 {
                4
            } else {
                3
            }
        }
        CardId::Cleave
        | CardId::ThunderClap
        | CardId::Immolate
        | CardId::Whirlwind
        | CardId::Reaper => living_monster_count(combat).max(1) as i32,
        _ => {
            let def = cards::get_card_definition(card_id);
            if def.card_type == CardType::Attack
                && def.base_damage + def.upgrade_damage * upgrades as i32 > 0
            {
                1
            } else {
                0
            }
        }
    }
}

fn strength_gain_for_card(card: &CombatCard) -> i32 {
    let def = cards::get_card_definition(card.id);
    match card.id {
        CardId::Inflame | CardId::Flex | CardId::SpotWeakness => {
            def.base_magic + def.upgrade_magic * card.upgrades as i32
        }
        _ => 0,
    }
}

fn dex_gain_for_card(card: &CombatCard) -> i32 {
    let def = cards::get_card_definition(card.id);
    match card.id {
        CardId::Footwork => def.base_magic + def.upgrade_magic * card.upgrades as i32,
        _ => 0,
    }
}

fn remaining_attack_hit_count(combat: &CombatState, exclude_hand_index: Option<usize>) -> i32 {
    combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(idx, card)| {
            Some(*idx) != exclude_hand_index && cards::can_play_card(card, combat).is_ok()
        })
        .map(|(_, card)| attack_hit_count(card.id, card.upgrades, combat))
        .sum()
}

fn remaining_block_card_count(combat: &CombatState, exclude_hand_index: Option<usize>) -> i32 {
    combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(idx, card)| {
            Some(*idx) != exclude_hand_index && cards::can_play_card(card, combat).is_ok()
        })
        .filter(|(_, card)| estimate_card_block(card) > 0)
        .count() as i32
}

fn vulnerable_projection_for_action(
    combat: &CombatState,
    played_hand_index: usize,
    target: Option<usize>,
) -> i32 {
    let remaining_damage = combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(idx, card)| {
            *idx != played_hand_index
                && cards::can_play_card(card, combat).is_ok()
                && cards::get_card_definition(card.id).card_type == CardType::Attack
        })
        .map(|(_, card)| estimate_card_damage(card, combat))
        .sum::<i32>();
    let target_hp = target
        .and_then(|entity_id| {
            combat
                .entities
                .monsters
                .iter()
                .find(|monster| monster.id == entity_id)
                .map(|monster| monster.current_hp.max(0))
        })
        .unwrap_or_else(|| total_alive_monster_hp(combat).max(0));
    (remaining_damage / 2).min(target_hp).max(0)
}

fn abstract_compression_decision(
    combat: &CombatState,
    engine: &EngineState,
    summary: &ActionEffectSummary,
    order_sensitive_reasons: &BTreeSet<String>,
    sequence_residues: &BTreeSet<String>,
) -> AbstractCompressionDecision {
    if !matches!(engine, EngineState::CombatPlayerTurn) {
        return AbstractCompressionDecision::BlockedByContext("not_player_turn".to_string());
    }
    if let Some(residue) = sequence_residues.iter().next() {
        return AbstractCompressionDecision::BlockedByContext(format!(
            "sequence_residue:{residue}"
        ));
    }
    if !order_sensitive_reasons.is_empty() {
        return AbstractCompressionDecision::BlockedByActionSemantics("order_sensitive_action");
    }
    if summary.creates_cards {
        return AbstractCompressionDecision::BlockedByActionSemantics("creates_cards");
    }
    if !summary.pure_damage && !summary.pure_block {
        return AbstractCompressionDecision::BlockedByActionSemantics("not_pure_damage_or_block");
    }
    if summary.draws_cards {
        return AbstractCompressionDecision::BlockedByActionSemantics("draw");
    }
    if summary.energy_gain {
        return AbstractCompressionDecision::BlockedByActionSemantics("energy_gain");
    }
    if summary.debuff {
        return AbstractCompressionDecision::BlockedByActionSemantics("debuff");
    }
    if summary.setup_or_scaling {
        return AbstractCompressionDecision::BlockedByActionSemantics("setup_or_scaling");
    }
    if summary.exhaust_or_discard {
        return AbstractCompressionDecision::BlockedByActionSemantics("exhaust_or_discard");
    }
    if summary.random_effect {
        return AbstractCompressionDecision::BlockedByActionSemantics("random_effect");
    }
    if summary.possible_kill {
        return AbstractCompressionDecision::BlockedByActionSemantics("possible_kill");
    }
    if summary.target_sensitive {
        return AbstractCompressionDecision::BlockedByActionSemantics("target_sensitive");
    }
    if let Some(reason) = known_order_sensitive_context(combat, summary) {
        return AbstractCompressionDecision::BlockedByContext(reason);
    }
    AbstractCompressionDecision::Candidate
}

fn generation_canonical_decision(
    combat: &CombatState,
    engine: &EngineState,
    action: &ClientInput,
    summary: &ActionEffectSummary,
    order_sensitive_reasons: &BTreeSet<String>,
    sequence_residues: &BTreeSet<String>,
) -> GenerationCanonicalDecision {
    if !matches!(engine, EngineState::CombatPlayerTurn) {
        return GenerationCanonicalDecision::BlockedByContext("not_player_turn".to_string());
    }
    if let Some(residue) = sequence_residues.iter().next() {
        return GenerationCanonicalDecision::BlockedByContext(format!(
            "sequence_residue:{residue}"
        ));
    }
    if !action_sequence_residues(combat, action).is_empty() {
        return GenerationCanonicalDecision::BlockedByActionSemantics(
            "action_leaves_sequence_residue",
        );
    }
    if !order_sensitive_reasons.is_empty() {
        return GenerationCanonicalDecision::BlockedByActionSemantics("order_sensitive_action");
    }
    if summary.creates_cards {
        return GenerationCanonicalDecision::BlockedByActionSemantics("creates_cards");
    }
    if !summary.pure_damage && !summary.pure_block {
        return GenerationCanonicalDecision::BlockedByActionSemantics("not_pure_damage_or_block");
    }
    if summary.draws_cards {
        return GenerationCanonicalDecision::BlockedByActionSemantics("draw");
    }
    if summary.energy_gain {
        return GenerationCanonicalDecision::BlockedByActionSemantics("energy_gain");
    }
    if summary.debuff {
        return GenerationCanonicalDecision::BlockedByActionSemantics("debuff");
    }
    if summary.setup_or_scaling {
        return GenerationCanonicalDecision::BlockedByActionSemantics("setup_or_scaling");
    }
    if summary.exhaust_or_discard {
        return GenerationCanonicalDecision::BlockedByActionSemantics("exhaust_or_discard");
    }
    if summary.random_effect {
        return GenerationCanonicalDecision::BlockedByActionSemantics("random_effect");
    }
    if summary.possible_kill {
        return GenerationCanonicalDecision::BlockedByActionSemantics("possible_kill");
    }
    if summary.target_sensitive {
        return GenerationCanonicalDecision::BlockedByActionSemantics("target_sensitive");
    }
    if let Some(reason) = known_order_sensitive_context(combat, summary) {
        return GenerationCanonicalDecision::BlockedByContext(reason);
    }
    match generation_canonical_action_keys(combat, action, summary) {
        Some(keys) => GenerationCanonicalDecision::Candidate(keys),
        None => GenerationCanonicalDecision::BlockedByActionSemantics("unsupported_action"),
    }
}

fn generation_canonical_action_keys(
    combat: &CombatState,
    action: &ClientInput,
    summary: &ActionEffectSummary,
) -> Option<GenerationCanonicalKeys> {
    let ClientInput::PlayCard { card_index, target } = action else {
        return None;
    };
    let card = combat.zones.hand.get(*card_index)?;
    let (lane_rank, lane_label) = if summary.pure_damage {
        ("0", "damage")
    } else if summary.pure_block {
        ("1", "block")
    } else {
        return None;
    };
    let target_label = if summary.pure_damage {
        probe_target_label(combat, *target)
    } else {
        "none".to_string()
    };
    let effect_key = generation_canonical_card_effect_key(card);
    let sort_key =
        format!("lane:{lane_rank}:{lane_label}|target:{target_label}|effect:{effect_key}");
    Some(GenerationCanonicalKeys {
        group_key: "pure_damage_block".to_string(),
        sort_key,
        effect_key,
        class_label: generation_canonical_class_label(summary, card, &target_label),
        lane_label: lane_label.to_string(),
        target_label,
    })
}

fn generation_canonical_card_effect_key(card: &CombatCard) -> String {
    format!(
        "card:{:?}|upg:{}|cost:{}|cost_mod:{}|cost_turn:{}|free:{}|energy_on_use:{}|misc:{}|base_dmg_override:{}|exhaust_override:{:?}|retain_override:{:?}|base_dmg_mut:{}|base_block_mut:{}|base_magic_mut:{}|multi_damage:{:?}",
        card.id,
        card.upgrades,
        card.get_cost(),
        card.cost_modifier,
        card.cost_for_turn
            .map(|cost| cost.to_string())
            .unwrap_or_else(|| "_".to_string()),
        card.free_to_play_once,
        card.energy_on_use,
        card.misc_value,
        card.base_damage_override
            .map(|damage| damage.to_string())
            .unwrap_or_else(|| "_".to_string()),
        card.exhaust_override,
        card.retain_override,
        card.base_damage_mut,
        card.base_block_mut,
        card.base_magic_num_mut,
        card.multi_damage
    )
}

fn generation_canonical_class_label(
    summary: &ActionEffectSummary,
    card: &CombatCard,
    target_label: &str,
) -> String {
    if summary.pure_block {
        format!(
            "block/effect:{}",
            generation_canonical_card_effect_key(card)
        )
    } else {
        format!(
            "damage/target:{target_label}/effect:{}",
            generation_canonical_card_effect_key(card)
        )
    }
}

fn classify_generation_prune_kind(
    previous_sort_key: &str,
    current_sort_key: &str,
) -> GenerationPruneKind {
    let previous = parse_generation_sort_key(previous_sort_key);
    let current = parse_generation_sort_key(current_sort_key);
    if previous.lane == current.lane
        && previous.target == current.target
        && previous.effect == current.effect
    {
        GenerationPruneKind::DuplicateCard
    } else if previous.lane == current.lane && previous.target == current.target {
        GenerationPruneKind::SameLaneOrder
    } else if previous.lane == current.lane {
        GenerationPruneKind::TargetOrder
    } else {
        GenerationPruneKind::LaneOrder
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct ParsedGenerationSortKey<'a> {
    lane: &'a str,
    target: &'a str,
    effect: &'a str,
}

fn parse_generation_sort_key(sort_key: &str) -> ParsedGenerationSortKey<'_> {
    let mut parsed = ParsedGenerationSortKey::default();
    for part in sort_key.split('|') {
        if let Some(value) = part.strip_prefix("lane:") {
            parsed.lane = value;
        } else if let Some(value) = part.strip_prefix("target:") {
            parsed.target = value;
        } else if let Some(value) = part.strip_prefix("effect:") {
            parsed.effect = value;
        }
    }
    parsed
}

fn increment_counter(counter: &mut BTreeMap<String, usize>, key: String) {
    *counter.entry(key).or_insert(0) += 1;
}

fn plan_expansion_gate_reason(
    engine: &EngineState,
    combat: &CombatState,
    action: &ClientInput,
    summary: &ActionEffectSummary,
    accumulated: &AccumulatedSequenceEffects,
    sequence_residues: &BTreeSet<String>,
) -> Option<&'static str> {
    if !matches!(engine, EngineState::CombatPlayerTurn) {
        return None;
    }
    if matches!(action, ClientInput::EndTurn) {
        return None;
    }
    if summary.pure_block
        && visible_incoming_damage(combat) <= combat.entities.player.block
        && kill_window_target_count(combat) == 0
        && known_order_sensitive_context(combat, summary).is_none()
    {
        return Some("surplus_block_without_pressure");
    }
    if summary.immediate_action_space_change && accumulated.action_space_change_count >= 1 {
        return Some("secondary_action_space_change_budget");
    }
    if summary.immediate_action_space_change && !sequence_residues.is_empty() {
        return Some("action_space_change_after_card_zone_residue");
    }
    None
}

fn plan_expansion_gate_example(
    reason: &str,
    node: &ProbeNode,
    action_key: &str,
    action: &ClientInput,
) -> PlanExpansionGateExample {
    PlanExpansionGateExample {
        reason: reason.to_string(),
        depth: node.depth,
        partial_action_keys: node.action_keys.clone(),
        partial_actions: node.actions.clone(),
        pruned_action_key: action_key.to_string(),
        pruned_action: format!("{action:?}"),
        current_energy: node.combat.turn.energy as i32,
        current_block: node.combat.entities.player.block,
        visible_incoming_damage: visible_incoming_damage(&node.combat),
        hand_size: node.combat.zones.hand.len(),
    }
}

fn known_order_sensitive_context(
    combat: &CombatState,
    summary: &ActionEffectSummary,
) -> Option<String> {
    if !combat.zones.limbo.is_empty() {
        return Some("limbo_not_empty".to_string());
    }
    if !combat.zones.queued_cards.is_empty() {
        return Some("queued_cards_not_empty".to_string());
    }
    if let Some(relic_id) = combat
        .entities
        .player
        .relics
        .iter()
        .map(|relic| relic.id)
        .find(|id| relic_has_current_turn_order_trigger(*id, summary))
    {
        return Some(format!("relic_order_trigger:{relic_id:?}"));
    }
    if let Some(power_id) = combat
        .entities
        .power_db
        .values()
        .flat_map(|powers| powers.iter().map(|power| power.power_type))
        .find(|id| power_has_current_turn_order_trigger(*id, summary))
    {
        return Some(format!("power_order_trigger:{power_id:?}"));
    }
    if let Some(enemy_id) = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .filter_map(|monster| EnemyId::from_id(monster.monster_type))
        .find(|enemy| enemy_has_current_turn_order_trigger(*enemy))
    {
        return Some(format!("enemy_order_trigger:{enemy_id:?}"));
    }
    None
}

fn relic_has_current_turn_order_trigger(id: RelicId, summary: &ActionEffectSummary) -> bool {
    let sub = get_relic_subscriptions(id);
    match id {
        RelicId::InkBottle | RelicId::OrangePellets => summary.pure_damage || summary.pure_block,
        RelicId::LetterOpener => summary.pure_block,
        RelicId::Duality
        | RelicId::Kunai
        | RelicId::Nunchaku
        | RelicId::OrnamentalFan
        | RelicId::PenNib
        | RelicId::Shuriken
        | RelicId::Necronomicon => summary.pure_damage,
        RelicId::MummifiedHand | RelicId::BirdFacedUrn => summary.setup_or_scaling,
        RelicId::BlueCandle | RelicId::MedicalKit => summary.exhaust_or_discard,
        RelicId::CharonsAshes | RelicId::DeadBranch => summary.exhaust_or_discard,
        RelicId::ToughBandages | RelicId::Tingsha | RelicId::HoveringKite => {
            summary.exhaust_or_discard
        }
        RelicId::GremlinHorn | RelicId::TheSpecimen => summary.possible_kill,
        RelicId::ChampionBelt | RelicId::SneckoSkull => summary.debuff,
        RelicId::Ginger | RelicId::Turnip | RelicId::OddMushroom | RelicId::PaperFrog => {
            summary.debuff
        }
        RelicId::ChemicalX => summary.energy_gain,
        // These mutate turn/relic counters but are not order-sensitive for pure
        // damage/block final states; the engine verification signature still
        // catches different counters if they matter.
        RelicId::ArtOfWar | RelicId::Pocketwatch => false,
        _ => {
            (sub.on_use_card && (summary.pure_damage || summary.pure_block))
                || (sub.on_exhaust && summary.exhaust_or_discard)
                || (sub.on_discard && summary.exhaust_or_discard)
                || (sub.on_monster_death && summary.possible_kill)
                || (sub.on_apply_power && (summary.debuff || summary.setup_or_scaling))
                || (sub.on_receive_power_modify && summary.debuff)
                || (sub.on_calculate_x_cost && summary.energy_gain)
                || (sub.on_calculate_vulnerable_multiplier && summary.debuff)
        }
    }
}

fn power_has_current_turn_order_trigger(id: PowerId, summary: &ActionEffectSummary) -> bool {
    match id {
        PowerId::Strength
        | PowerId::Dexterity
        | PowerId::Vulnerable
        | PowerId::Weak
        | PowerId::Frail
        | PowerId::Ritual
        | PowerId::Artifact
        | PowerId::Metallicize
        | PowerId::Barricade
        | PowerId::Minion
        | PowerId::Intangible
        | PowerId::IntangiblePlayer
        | PowerId::NoDraw
        | PowerId::NoBlock
        | PowerId::Entangle => false,
        PowerId::CurlUp
        | PowerId::FlameBarrier
        | PowerId::Malleable
        | PowerId::ModeShift
        | PowerId::SharpHide
        | PowerId::Thorns
        | PowerId::PenNibPower
        | PowerId::Flight
        | PowerId::Reactive
        | PowerId::PlatedArmor => summary.pure_damage,
        PowerId::Rage | PowerId::Juggernaut => summary.pure_block,
        PowerId::FeelNoPain | PowerId::DarkEmbrace => summary.exhaust_or_discard,
        PowerId::SadisticPower => summary.debuff,
        _ => true,
    }
}

fn conservative_order_sensitive_context(combat: &CombatState) -> Option<String> {
    if !combat.zones.limbo.is_empty() {
        return Some("limbo_not_empty".to_string());
    }
    if !combat.zones.queued_cards.is_empty() {
        return Some("queued_cards_not_empty".to_string());
    }
    if let Some(relic_id) = combat
        .entities
        .player
        .relics
        .iter()
        .map(|relic| relic.id)
        .find(|id| {
            let sub = get_relic_subscriptions(*id);
            sub.on_use_card
                || sub.on_exhaust
                || sub.on_discard
                || sub.on_monster_death
                || sub.on_apply_power
                || sub.on_lose_hp
                || sub.on_attacked_to_change_damage
                || sub.on_receive_power_modify
                || sub.on_calculate_x_cost
                || sub.on_calculate_vulnerable_multiplier
        })
    {
        return Some(format!("relic_order_trigger:{relic_id:?}"));
    }
    if let Some(power_id) = combat
        .entities
        .power_db
        .values()
        .flat_map(|powers| powers.iter().map(|power| power.power_type))
        .find(|id| {
            !matches!(
                id,
                PowerId::Strength
                    | PowerId::Dexterity
                    | PowerId::Vulnerable
                    | PowerId::Weak
                    | PowerId::Frail
                    | PowerId::Ritual
                    | PowerId::Artifact
                    | PowerId::Metallicize
                    | PowerId::Barricade
                    | PowerId::Minion
            )
        })
    {
        return Some(format!("power_order_trigger:{power_id:?}"));
    }
    if let Some(enemy_id) = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .filter_map(|monster| EnemyId::from_id(monster.monster_type))
        .find(|enemy| enemy_has_current_turn_order_trigger(*enemy))
    {
        return Some(format!("enemy_order_trigger:{enemy_id:?}"));
    }
    None
}

fn enemy_has_current_turn_order_trigger(id: EnemyId) -> bool {
    matches!(
        id,
        EnemyId::GremlinNob
            | EnemyId::Byrd
            | EnemyId::Spiker
            | EnemyId::TimeEater
            | EnemyId::AwakenedOne
            | EnemyId::CorruptHeart
            | EnemyId::GiantHead
            | EnemyId::TheGuardian
    )
}

fn abstract_sequence_key(engine: &EngineState, combat: &CombatState) -> String {
    let monster_hp = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_escaped && !monster.half_dead)
        .map(|monster| {
            format!(
                "{}:{}:{}",
                monster.id,
                monster.current_hp.max(0),
                monster.block
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let mut hand = combat
        .zones
        .hand
        .iter()
        .map(|card| format!("{:?}+{}", card.id, card.upgrades))
        .collect::<Vec<_>>();
    hand.sort();
    format!(
        "{engine:?}|hp:{}|block:{}|energy:{}|monsters:{monster_hp}|hand:{}|draw:{}|discard:{}|exhaust:{}",
        combat.entities.player.current_hp,
        combat.entities.player.block,
        combat.turn.energy,
        hand.join(","),
        combat.zones.draw_pile.len(),
        combat.zones.discard_pile.len(),
        combat.zones.exhaust_pile.len()
    )
}

fn abstract_rejection_reasons(
    previous_entries: &[AbstractSeen],
    current: &AbstractVerificationSnapshot,
) -> Vec<&'static str> {
    previous_entries
        .iter()
        .map(|entry| entry.verification_snapshot.diff_reasons(current))
        .min_by_key(|reasons| reasons.len())
        .unwrap_or_else(|| vec!["missing_previous_signature"])
}

fn abstract_verification_snapshot(
    engine: &EngineState,
    combat: &CombatState,
) -> AbstractVerificationSnapshot {
    #[derive(Clone, Debug)]
    struct ZoneSnapshot {
        state: String,
        eval_cache: String,
    }

    fn zone_signature(cards: &[CombatCard]) -> ZoneSnapshot {
        let mut state_parts = cards
            .iter()
            .map(|card| {
                format!(
                    "{:?}+{}:misc:{}:cost_mod:{}:cost_turn:{}:base_dmg_override:{}:exhaust:{:?}:retain:{:?}:free:{}:energy_on_use:{}",
                    card.id,
                    card.upgrades,
                    card.misc_value,
                    card.cost_modifier,
                    card.cost_for_turn
                        .map(|cost| cost.to_string())
                        .unwrap_or_else(|| "_".to_string()),
                    card.base_damage_override
                        .map(|damage| damage.to_string())
                        .unwrap_or_else(|| "_".to_string()),
                    card.exhaust_override,
                    card.retain_override,
                    card.free_to_play_once,
                    card.energy_on_use
                )
            })
            .collect::<Vec<_>>();
        state_parts.sort();
        let mut parts = cards
            .iter()
            .map(|card| {
                format!(
                    "{:?}+{}:base_dmg_mut:{}:base_block_mut:{}:base_magic_mut:{}:multi_damage:{:?}",
                    card.id,
                    card.upgrades,
                    card.base_damage_mut,
                    card.base_block_mut,
                    card.base_magic_num_mut,
                    card.multi_damage
                )
            })
            .collect::<Vec<_>>();
        parts.sort();
        ZoneSnapshot {
            state: state_parts.join(","),
            eval_cache: parts.join(","),
        }
    }

    let monsters = combat
        .entities
        .monsters
        .iter()
        .map(|monster| {
            format!(
                "{}:{}:{}:{}:{}:{}",
                monster.id,
                monster.current_hp,
                monster.block,
                monster.is_dying,
                monster.is_escaped,
                monster.half_dead
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let relics = combat
        .entities
        .player
        .relics
        .iter()
        .map(|relic| {
            format!(
                "{:?}:{}:{}:{}",
                relic.id, relic.counter, relic.used_up, relic.amount
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let powers = combat
        .entities
        .power_db
        .iter()
        .map(|(entity, powers)| {
            let mut parts = powers
                .iter()
                .map(|power| {
                    format!(
                        "{:?}:{}:{}:{}",
                        power.power_type, power.amount, power.extra_data, power.just_applied
                    )
                })
                .collect::<Vec<_>>();
            parts.sort();
            format!("{entity}:{}", parts.join("|"))
        })
        .collect::<Vec<_>>()
        .join(",");
    let mut played_card_ids = combat
        .turn
        .counters
        .card_ids_played_this_turn
        .iter()
        .map(|card_id| format!("{card_id:?}"))
        .collect::<Vec<_>>();
    played_card_ids.sort();
    let hand = zone_signature(&combat.zones.hand);
    let draw = zone_signature(&combat.zones.draw_pile);
    let discard = zone_signature(&combat.zones.discard_pile);
    let exhaust = zone_signature(&combat.zones.exhaust_pile);
    AbstractVerificationSnapshot {
        engine: format!("{engine:?}"),
        player: format!(
            "hp:{}|block:{}|energy:{}",
            combat.entities.player.current_hp, combat.entities.player.block, combat.turn.energy
        ),
        monsters,
        hand_state: hand.state,
        hand_eval_cache: hand.eval_cache,
        draw_state: draw.state,
        draw_eval_cache: draw.eval_cache,
        discard_state: discard.state,
        discard_eval_cache: discard.eval_cache,
        exhaust_state: exhaust.state,
        exhaust_eval_cache: exhaust.eval_cache,
        turn_counters: format!(
            "cards:{}|attacks:{}",
            combat.turn.counters.cards_played_this_turn,
            combat.turn.counters.attacks_played_this_turn
        ),
        played_card_ids: played_card_ids.join(","),
        relics,
        powers,
    }
}

fn can_bound_prune(
    engine: &EngineState,
    combat: &CombatState,
    order_sensitive_reasons: &BTreeSet<String>,
    sequence_residues: &BTreeSet<String>,
) -> bool {
    matches!(engine, EngineState::CombatPlayerTurn)
        && order_sensitive_reasons.is_empty()
        && sequence_residues.is_empty()
        && conservative_order_sensitive_context(combat).is_none()
}

fn optimistic_bound_score(
    combat: &CombatState,
    order_sensitive_reasons: &BTreeSet<String>,
    sequence_residues: &BTreeSet<String>,
) -> i32 {
    if !order_sensitive_reasons.is_empty()
        || !sequence_residues.is_empty()
        || conservative_order_sensitive_context(combat).is_some()
    {
        return 10_000;
    }
    let mut max_damage = 0;
    let mut max_block = 0;
    let mut max_setup = 0;
    for card in &combat.zones.hand {
        if cards::can_play_card(card, combat).is_err() {
            continue;
        }
        let structure = card_structure::structure(card.id);
        max_damage += estimate_card_damage(card, combat);
        max_block += estimate_card_block(card);
        if structure.is_setup_piece() || structure.is_scaling_piece() {
            max_setup += 90;
        }
    }
    let damage_score = max_damage.min(total_alive_monster_hp(combat).max(0)) * 6;
    let block_need = visible_incoming_damage(combat).max(0);
    let block_score = max_block.min(block_need + 20) * 8;
    damage_score + block_score + max_setup
}

fn allowed_probe_action(engine: &EngineState, action: &ClientInput) -> bool {
    match engine {
        EngineState::CombatPlayerTurn => {
            matches!(action, ClientInput::PlayCard { .. } | ClientInput::EndTurn)
        }
        EngineState::PendingChoice(_) => matches!(
            action,
            ClientInput::SubmitHandSelect(_)
                | ClientInput::SubmitGridSelect(_)
                | ClientInput::SubmitDiscoverChoice(_)
                | ClientInput::SubmitScryDiscard(_)
                | ClientInput::Cancel
                | ClientInput::Proceed
        ),
        _ => false,
    }
}

fn action_order_score(combat: &CombatState, action: &ClientInput) -> i32 {
    match action {
        ClientInput::EndTurn => -10_000,
        ClientInput::PlayCard { card_index, .. } => combat
            .zones
            .hand
            .get(*card_index)
            .map(|card| {
                let def = cards::get_card_definition(card.id);
                let structure = card_structure::structure(card.id);
                let mut score =
                    def.base_damage * 8 + def.base_block * 6 - card.get_cost() as i32 * 3;
                if structure.is_setup_piece() {
                    score += 80;
                }
                if structure.is_exhaust_outlet() {
                    score += 40;
                }
                if card.id == CardId::TrueGrit && card.upgrades == 0 {
                    score -= 20;
                }
                score
            })
            .unwrap_or(0),
        ClientInput::SubmitHandSelect(uuids) | ClientInput::SubmitGridSelect(uuids) => {
            uuids
                .iter()
                .map(|uuid| {
                    crate::bot::card_disposition::combat_exhaust_score_for_uuid(combat, *uuid)
                })
                .sum::<i32>()
                / 100
        }
        _ => 0,
    }
}

fn is_probe_frontier(engine: &EngineState) -> bool {
    matches!(
        engine,
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
    )
}

fn summarize_state(engine: &EngineState, combat: &CombatState) -> CombatPlanStateSummary {
    let incoming = visible_incoming_damage(combat);
    CombatPlanStateSummary {
        engine_state: format!("{engine:?}"),
        player_hp: combat.entities.player.current_hp,
        player_block: combat.entities.player.block,
        energy: combat.turn.energy as i32,
        turn_count: combat.turn.turn_count,
        visible_incoming_damage: incoming,
        unblocked_incoming_damage: (incoming - combat.entities.player.block).max(0),
        alive_monster_count: living_monster_count(combat),
        total_monster_hp: total_alive_monster_hp(combat),
        hand_count: combat.zones.hand.len(),
        draw_count: combat.zones.draw_pile.len(),
        discard_count: combat.zones.discard_pile.len(),
        exhaust_count: combat.zones.exhaust_pile.len(),
    }
}

fn build_probe_hand_cards(combat: &CombatState) -> Vec<CombatPlanHandCard> {
    combat
        .zones
        .hand
        .iter()
        .enumerate()
        .map(|(hand_index, card)| CombatPlanHandCard {
            hand_index,
            card_instance_id: card.uuid,
            card_id: format!("{:?}", card.id),
            upgraded: card.upgrades > 0,
            cost_for_turn: card.get_cost(),
            playable: cards::can_play_card(card, combat).is_ok(),
            base_semantics: semantics_for_card(card.id, card.upgrades),
            transient_tags: transient_tags_for_card(combat, hand_index, card),
        })
        .collect()
}

fn semantics_for_card(card_id: CardId, upgrades: u8) -> Vec<String> {
    let def = cards::get_card_definition(card_id);
    let facts = card_facts::facts(card_id);
    let structure = card_structure::structure(card_id);
    let mut tags = Vec::new();
    match def.card_type {
        CardType::Attack => tags.push("attack".to_string()),
        CardType::Skill => tags.push("skill".to_string()),
        CardType::Power => tags.push("power".to_string()),
        CardType::Status => tags.push("status".to_string()),
        CardType::Curse => tags.push("curse".to_string()),
    }
    if def.base_damage + def.upgrade_damage * upgrades as i32 > 0 {
        tags.push("damage".to_string());
    }
    if def.base_block + def.upgrade_block * upgrades as i32 > 0 || structure.is_block_core() {
        tags.push("block".to_string());
    }
    if facts.draws_cards {
        tags.push("draw".to_string());
    }
    if facts.gains_energy {
        tags.push("energy".to_string());
    }
    if facts.applies_weak {
        tags.push("apply_weak".to_string());
    }
    if facts.applies_vuln {
        tags.push("apply_vulnerable".to_string());
    }
    if structure.is_setup_piece() || structure.is_scaling_piece() {
        tags.push("setup_or_scaling".to_string());
    }
    if structure.is_exhaust_outlet() {
        tags.push("exhaust_outlet".to_string());
    }
    if facts.exhausts_self {
        tags.push("self_exhaust".to_string());
    }
    if facts.self_replicating {
        tags.push("self_replicating".to_string());
    }
    if facts.produces_status {
        tags.push("produces_status".to_string());
    }
    if facts.creates_cards {
        tags.push("creates_cards".to_string());
    }
    match card_id {
        CardId::TrueGrit if upgrades == 0 => {
            tags.push("random_exhaust".to_string());
            tags.push("risk_overlay_required".to_string());
        }
        CardId::TrueGrit => tags.push("chosen_exhaust".to_string()),
        CardId::SecondWind => tags.push("exhaust_non_attacks".to_string()),
        CardId::FiendFire => tags.push("exhaust_hand_for_damage".to_string()),
        _ => {}
    }
    tags
}

fn transient_tags_for_card(
    combat: &CombatState,
    hand_index: usize,
    card: &CombatCard,
) -> Vec<String> {
    let mut tags = Vec::new();
    tags.push(
        if cards::can_play_card(card, combat).is_ok() {
            "playable"
        } else {
            "unplayable"
        }
        .to_string(),
    );
    if possible_kill_with_card(combat, card) {
        tags.push("possible_kill".to_string());
    }
    if card.cost_for_turn.is_some() {
        tags.push("cost_for_turn_override".to_string());
    }
    if hand_index < combat.zones.hand.len() && card.id == CardId::TrueGrit && card.upgrades == 0 {
        tags.push("static_random_exhaust_overlay".to_string());
    }
    tags
}

fn sequence_equivalence_key(engine: &EngineState, combat: &CombatState) -> String {
    let monster_hp = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_escaped && !monster.half_dead)
        .map(|monster| {
            format!(
                "{}:{}:{}",
                monster.id,
                monster.current_hp.max(0),
                monster.block
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let hand = combat
        .zones
        .hand
        .iter()
        .map(|card| format!("{:?}+{}:{}", card.id, card.upgrades, card.uuid))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{engine:?}|hp:{}|block:{}|energy:{}|monsters:{monster_hp}|hand:{hand}|draw:{}|discard:{}|exhaust:{}",
        combat.entities.player.current_hp,
        combat.entities.player.block,
        combat.turn.energy,
        combat.zones.draw_pile.len(),
        combat.zones.discard_pile.len(),
        combat.zones.exhaust_pile.len()
    )
}

fn probe_action_key(combat: &CombatState, action: &ClientInput) -> String {
    match action {
        ClientInput::PlayCard { card_index, target } => combat
            .zones
            .hand
            .get(*card_index)
            .map(|card| {
                format!(
                    "combat/play_card/card:{:?}/hand:{card_index}/target:{}",
                    card.id,
                    probe_target_label(combat, *target)
                )
            })
            .unwrap_or_else(|| format!("{action:?}")),
        ClientInput::EndTurn => "combat/end_turn".to_string(),
        ClientInput::SubmitHandSelect(uuids) => {
            format!("combat/hand_select/uuids:{}", uuid_list_key(uuids))
        }
        ClientInput::SubmitGridSelect(uuids) => {
            format!("combat/grid_select/uuids:{}", uuid_list_key(uuids))
        }
        _ => format!("{action:?}"),
    }
}

fn probe_target_label(combat: &CombatState, target: Option<usize>) -> String {
    match target {
        None => "none".to_string(),
        Some(entity_id) => combat
            .entities
            .monsters
            .iter()
            .position(|monster| monster.id == entity_id)
            .map(|slot| format!("monster_slot:{slot}"))
            .unwrap_or_else(|| format!("entity:{entity_id}")),
    }
}

fn probe_action_label_from_key(action_key: &str) -> String {
    if action_key == "combat/end_turn" {
        return "EndTurn".to_string();
    }
    if let Some(rest) = action_key.strip_prefix("combat/play_card/card:") {
        let card = rest.split('/').next().unwrap_or(rest);
        let hand = rest
            .split("hand:")
            .nth(1)
            .and_then(|part| part.split('/').next())
            .filter(|hand| !hand.is_empty())
            .map(|hand| format!("[h{hand}]"))
            .unwrap_or_default();
        let target = rest
            .split("target:")
            .nth(1)
            .filter(|target| !target.is_empty())
            .unwrap_or("none");
        if target == "none" {
            format!("{card}{hand}")
        } else {
            format!("{card}{hand} -> {target}")
        }
    } else if let Some(rest) = action_key.strip_prefix("combat/hand_select/") {
        format!("HandSelect {rest}")
    } else if let Some(rest) = action_key.strip_prefix("combat/grid_select/") {
        format!("GridSelect {rest}")
    } else {
        action_key.to_string()
    }
}

fn uuid_list_key(uuids: &[u32]) -> String {
    uuids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn visible_incoming_damage(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            !monster.is_dying && !monster.is_escaped && !monster.half_dead && monster.current_hp > 0
        })
        .map(|monster| monster_preview_total_damage_in_combat(combat, monster))
        .sum()
}

fn total_alive_monster_hp(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            !monster.is_dying && !monster.is_escaped && !monster.half_dead && monster.current_hp > 0
        })
        .map(|monster| monster.current_hp.max(0))
        .sum()
}

fn living_monster_count(combat: &CombatState) -> usize {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            !monster.is_dying && !monster.is_escaped && !monster.half_dead && monster.current_hp > 0
        })
        .count()
}

fn possible_kill_with_card(combat: &CombatState, card: &CombatCard) -> bool {
    let def = cards::get_card_definition(card.id);
    if def.card_type != CardType::Attack || def.base_damage <= 0 {
        return false;
    }
    let rough_damage = (def.base_damage + def.upgrade_damage * card.upgrades as i32)
        .max(card.base_damage_mut)
        .max(0);
    combat
        .entities
        .monsters
        .iter()
        .any(|monster| monster.current_hp > 0 && monster.current_hp <= rough_damage)
}

fn has_setup_or_scaling_card_in_hand(combat: &CombatState) -> bool {
    combat.zones.hand.iter().any(|card| {
        let structure = card_structure::structure(card.id);
        structure.is_setup_piece() || structure.is_scaling_piece()
    })
}

fn is_kill_window_card(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Feed | CardId::HandOfGreed | CardId::RitualDagger
    )
}

fn kill_window_card_labels_in_hand(combat: &CombatState) -> Vec<String> {
    combat
        .zones
        .hand
        .iter()
        .filter(|card| is_kill_window_card(card.id))
        .map(hand_card_label)
        .collect()
}

fn kill_window_target_count(combat: &CombatState) -> usize {
    let kill_window_damages = combat
        .zones
        .hand
        .iter()
        .filter(|card| is_kill_window_card(card.id))
        .map(kill_window_card_damage)
        .collect::<Vec<_>>();
    if kill_window_damages.is_empty() {
        return 0;
    }
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            monster.current_hp > 0
                && !monster.is_dying
                && !monster.is_escaped
                && !monster.half_dead
                && kill_window_damages
                    .iter()
                    .any(|damage| monster.current_hp <= *damage)
        })
        .count()
}

fn kill_window_card_damage(card: &CombatCard) -> i32 {
    let def = cards::get_card_definition(card.id);
    (def.base_damage + def.upgrade_damage * card.upgrades as i32)
        .max(card.base_damage_mut)
        .max(0)
}

fn exhaust_outlet_value(combat: &CombatState, played_index: Option<usize>) -> i32 {
    combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(idx, _)| Some(*idx) != played_index)
        .map(|(_, card)| {
            crate::bot::card_disposition::combat_exhaust_score_for_uuid(combat, card.uuid) / 100
        })
        .sum()
}

fn hand_cards_except(combat: &CombatState, played_index: Option<usize>) -> Vec<String> {
    combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(idx, _)| Some(*idx) != played_index)
        .map(|(_, card)| hand_card_label(card))
        .collect()
}

fn non_attack_hand_cards(combat: &CombatState, played_index: Option<usize>) -> Vec<String> {
    combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(idx, card)| {
            Some(*idx) != played_index
                && cards::get_card_definition(card.id).card_type != CardType::Attack
        })
        .map(|(_, card)| hand_card_label(card))
        .collect()
}

fn hand_card_label_by_uuid(combat: &CombatState, uuid: u32) -> Option<String> {
    combat
        .zones
        .hand
        .iter()
        .find(|card| card.uuid == uuid)
        .map(hand_card_label)
}

fn hand_card_label(card: &CombatCard) -> String {
    format!("{:?}+{}#{}", card.id, card.upgrades, card.uuid)
}

fn plan_label(plan: CombatPlanKind) -> &'static str {
    match plan {
        CombatPlanKind::Lethal => "Lethal",
        CombatPlanKind::KillThreateningEnemy => "KillThreateningEnemy",
        CombatPlanKind::FullBlock => "FullBlock",
        CombatPlanKind::BlockEnoughThenDamage => "BlockEnoughThenDamage",
        CombatPlanKind::MaxDamage => "MaxDamage",
        CombatPlanKind::SetupPowerOrScaling => "SetupPowerOrScaling",
        CombatPlanKind::ExhaustBadCards => "ExhaustBadCards",
        CombatPlanKind::PreserveKeyCards => "PreserveKeyCards",
    }
}

fn plan_explanation(plan: CombatPlanKind) -> &'static str {
    match plan {
        CombatPlanKind::Lethal => "Find a current-turn sequence that ends combat.",
        CombatPlanKind::KillThreateningEnemy => {
            "Prefer killing an enemy that contributes to current incoming damage."
        }
        CombatPlanKind::FullBlock => "Prefer reducing visible unblocked damage this turn.",
        CombatPlanKind::BlockEnoughThenDamage => {
            "Prefer meeting the defensive requirement, then converting spare resources to damage."
        }
        CombatPlanKind::MaxDamage => "Prefer current-turn enemy HP progress.",
        CombatPlanKind::SetupPowerOrScaling => {
            "Prefer powers/scaling setup when it can be afforded under current risk."
        }
        CombatPlanKind::ExhaustBadCards => {
            "Prefer sequences that use exhaust as deck cleanup or exhaust-synergy activation."
        }
        CombatPlanKind::PreserveKeyCards => {
            "Penalize random or broad exhaust that can destroy high-retention cards."
        }
    }
}

#[cfg(test)]
include!("turn_plan_probe_tests.rs");
