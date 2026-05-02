use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::bot::{card_facts, card_structure};
use crate::content::cards::{self, CardId, CardType};
use crate::projection::combat::monster_preview_total_damage_in_combat;
use crate::runtime::combat::{CombatCard, CombatState};
use crate::state::core::ClientInput;
use crate::state::EngineState;

use super::legal_moves::get_legal_moves;
use super::profile::SearchProfileBreakdown;
use super::stepping::simulate_input_bounded;

pub const COMBAT_TURN_PLAN_PROBE_SCHEMA_VERSION: &str = "combat_turn_plan_probe_v1_1";

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
    pub first_action_affordances: Vec<CombatFirstActionAffordance>,
    pub plans: Vec<CombatPlanReport>,
    pub sequence_classes: Vec<CombatPlanSequenceClass>,
    pub risk_notes: Vec<CombatPlanRiskNote>,
    pub probe_limits: CombatPlanProbeLimits,
    pub truth_warnings: Vec<String>,
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
pub struct CombatPlanSequenceClass {
    pub sequence_equivalence_key: String,
    pub actions: Vec<String>,
    pub action_keys: Vec<String>,
    pub order_sensitive_reasons: Vec<String>,
    pub diagnostics: PlanScoreBreakdown,
    pub pruned_as_equivalent: bool,
    pub pruned_by_budget: bool,
    pub pruned_by_dominated_state: bool,
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
    pub sequence_classes_kept: usize,
    pub pruned_as_equivalent: usize,
    pub pruned_by_budget: usize,
    pub pruned_by_dominated_state: usize,
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
    risk_notes: Vec<CombatPlanRiskNote>,
    accumulated: AccumulatedSequenceEffects,
    depth: usize,
    ended_turn: bool,
}

#[derive(Clone, Debug, Default)]
struct AccumulatedSequenceEffects {
    setup_score: i32,
    exhaust_value: i32,
    key_card_risk: i32,
    random_risk: i32,
    future_hand_penalty: i32,
}

pub fn probe_turn_plans(
    engine: &EngineState,
    combat: &CombatState,
    config: CombatTurnPlanProbeConfig,
) -> CombatTurnPlanProbeReport {
    let start_summary = summarize_state(engine, combat);
    let hand_cards = build_probe_hand_cards(combat);
    let (sequence_classes, limits, risk_notes) = explore_sequence_classes(engine, combat, config);
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
    let first_action_affordances =
        build_first_action_affordances(&plan_kinds, &sequence_classes, &risk_notes);

    CombatTurnPlanProbeReport {
        schema_version: COMBAT_TURN_PLAN_PROBE_SCHEMA_VERSION.to_string(),
        source_trace: serde_json::Value::Null,
        state_summary: start_summary,
        hand_cards,
        first_action_affordances,
        plans,
        sequence_classes,
        risk_notes,
        probe_limits: limits,
        truth_warnings: vec![
            "current_turn_only_horizon".to_string(),
            "no_future_seed_oracle".to_string(),
            "role_scores_are_heuristic_not_truth".to_string(),
            "static_random_risk_overlay_is_not_engine_rng_branch_enumeration".to_string(),
            "budget_pruning_can_hide_lower_ranked_sequences".to_string(),
        ],
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
        risk_notes: Vec::new(),
        accumulated: AccumulatedSequenceEffects::default(),
        depth: 0,
        ended_turn: false,
    });

    let mut seen = BTreeMap::<String, i32>::new();
    let mut kept = Vec::new();
    let mut all_risk_notes = Vec::new();
    let mut nodes_expanded = 0usize;
    let mut pruned_as_equivalent = 0usize;
    let mut pruned_by_budget = 0usize;
    let pruned_by_dominated_state = 0usize;
    let mut profile = SearchProfileBreakdown::default();

    while let Some(node) = queue.pop_front() {
        if nodes_expanded >= config.max_nodes {
            pruned_by_budget += queue.len() + 1;
            break;
        }
        nodes_expanded += 1;

        if !node.actions.is_empty() {
            let diagnostics = diagnose_sequence(combat, &node.combat, &node.accumulated);
            let key = sequence_equivalence_key(&node.engine, &node.combat);
            kept.push(CombatPlanSequenceClass {
                sequence_equivalence_key: key,
                actions: node.actions.clone(),
                action_keys: node.action_keys.clone(),
                order_sensitive_reasons: node
                    .order_sensitive_reasons
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>(),
                diagnostics,
                pruned_as_equivalent: false,
                pruned_by_budget: false,
                pruned_by_dominated_state: false,
            });
            all_risk_notes.extend(node.risk_notes.clone());
        }

        if node.depth >= config.max_depth || node.ended_turn || !is_probe_frontier(&node.engine) {
            continue;
        }

        let mut legal = get_legal_moves(&node.engine, &node.combat)
            .into_iter()
            .filter(|action| allowed_probe_action(&node.engine, action))
            .collect::<Vec<_>>();
        legal.sort_by_key(|action| -action_order_score(&node.combat, action));
        legal.truncate(config.beam_width);

        for action in legal {
            if nodes_expanded + queue.len() >= config.max_nodes {
                pruned_by_budget += 1;
                continue;
            }
            let action_key = probe_action_key(&node.combat, &action);
            let mut next_accumulated = node.accumulated.clone();
            let mut next_reasons = node.order_sensitive_reasons.clone();
            let mut next_notes = node.risk_notes.clone();
            accumulate_action_effects(
                &node.combat,
                &action,
                node.actions.len(),
                &action_key,
                &mut next_accumulated,
                &mut next_reasons,
                &mut next_notes,
            );

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
        sequence_classes_kept: kept.len(),
        pruned_as_equivalent,
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
        + future_hand_penalty;

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
    }
}

fn accumulate_action_effects(
    combat: &CombatState,
    action: &ClientInput,
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
            if facts.draws_cards {
                order_sensitive_reasons.insert("draw_changes_future_action_space".to_string());
            }
            if facts.applies_vuln || facts.applies_weak {
                order_sensitive_reasons.insert("debuff_before_damage_can_change_value".to_string());
            }
            if facts.gains_energy {
                order_sensitive_reasons
                    .insert("energy_gain_changes_future_action_space".to_string());
            }
            if facts.exhausts_other_cards {
                order_sensitive_reasons.insert("exhaust_changes_hand_and_deck_state".to_string());
            }
            if facts.random_generation || card.id == CardId::TrueGrit && card.upgrades == 0 {
                order_sensitive_reasons.insert("random_effect_requires_risk_model".to_string());
            }
            if structure.is_setup_piece() || structure.is_scaling_piece() {
                accumulated.setup_score += 90;
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
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::runtime::combat::{CombatCard, Power};
    use crate::test_support::{blank_test_combat, planned_monster};

    fn card(id: CardId, uuid: u32) -> CombatCard {
        CombatCard::new(id, uuid)
    }

    #[test]
    fn combat_turn_plan_probe_marks_true_grit_random_key_card_risk() {
        let mut combat = blank_test_combat();
        combat.zones.hand.push(card(CardId::TrueGrit, 1));
        combat.zones.hand.push(card(CardId::Bash, 2));
        combat.zones.hand.push(card(CardId::Wound, 3));
        combat.zones.hand.push(card(CardId::Strike, 4));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 1,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        let note = report
            .risk_notes
            .iter()
            .find(|note| note.kind == "true_grit_random_exhaust_overlay")
            .expect("unupgraded True Grit should emit static random exhaust overlay");
        assert_eq!(
            note.chance_model.as_deref(),
            Some("static_hand_distribution")
        );
        assert!(!note.exact_rng_branches);
        assert!(note.risk_is_overlay_only);
        assert!(note.bad_branch_probability_milli.unwrap_or_default() > 0);
        let affordance = report
            .first_action_affordances
            .iter()
            .find(|affordance| affordance.action_key.contains("card:TrueGrit"))
            .expect("True Grit should have a first-action affordance");
        assert!(affordance
            .risk_note_kinds
            .iter()
            .any(|kind| kind == "true_grit_random_exhaust_overlay"));
        assert!(affordance
            .major_tradeoffs
            .iter()
            .any(|tradeoff| tradeoff == "explicit_risk_note"));
    }

    #[test]
    fn combat_turn_plan_probe_marks_true_grit_bad_card_upside() {
        let mut combat = blank_test_combat();
        combat.zones.hand.push(card(CardId::TrueGrit, 1));
        combat.zones.hand.push(card(CardId::Wound, 2));
        combat.zones.hand.push(card(CardId::Slimed, 3));
        combat.zones.hand.push(card(CardId::Strike, 4));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 1,
                ..CombatTurnPlanProbeConfig::default()
            },
        );
        let note = report
            .risk_notes
            .iter()
            .find(|note| note.kind == "true_grit_random_exhaust_overlay")
            .expect("unupgraded True Grit should emit static random exhaust overlay");
        assert!(note.good_branch_probability_milli.unwrap_or_default() > 0);
    }

    #[test]
    fn combat_turn_plan_probe_marks_true_grit_plus_as_exact_selection() {
        let mut combat = blank_test_combat();
        let mut true_grit = card(CardId::TrueGrit, 1);
        true_grit.upgrades = 1;
        combat.zones.hand.push(true_grit);
        combat.zones.hand.push(card(CardId::Wound, 2));
        combat.zones.hand.push(card(CardId::Bash, 3));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 2,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert!(report
            .risk_notes
            .iter()
            .any(|note| note.kind == "chosen_exhaust_pending" && note.exact_rng_branches));
        assert!(report.risk_notes.iter().any(|note| {
            note.kind == "exact_selection_resolution"
                && note
                    .affected_cards
                    .iter()
                    .any(|card| card.contains("Wound"))
        }));
    }

    #[test]
    fn combat_turn_plan_probe_records_vulnerable_order_sensitivity() {
        let mut combat = blank_test_combat();
        combat
            .entities
            .monsters
            .push(planned_monster(EnemyId::Cultist, 1));
        combat.zones.hand.push(card(CardId::Bash, 1));
        combat.zones.hand.push(card(CardId::Strike, 2));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 2,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert!(report.sequence_classes.iter().any(|sequence| sequence
            .order_sensitive_reasons
            .iter()
            .any(|reason| reason == "debuff_before_damage_can_change_value")));
        let bash = report
            .first_action_affordances
            .iter()
            .find(|affordance| affordance.action_key.contains("card:Bash"))
            .expect("Bash should have first-action affordance rows");
        assert!(bash
            .supported_plans
            .iter()
            .any(|support| support.plan_name == "MaxDamage" && support.rank == 1));
        assert!(bash
            .order_sensitive_reasons
            .iter()
            .any(|reason| reason == "debuff_before_damage_can_change_value"));
    }

    #[test]
    fn combat_turn_plan_probe_marks_exhaust_engine_block_setup() {
        let mut combat = blank_test_combat();
        combat.entities.power_db.insert(
            0,
            vec![Power {
                power_type: crate::runtime::combat::PowerId::FeelNoPain,
                instance_id: None,
                amount: 3,
                extra_data: 0,
                just_applied: false,
            }],
        );
        combat.zones.hand.push(card(CardId::SecondWind, 1));
        combat.zones.hand.push(card(CardId::Wound, 2));
        combat.zones.hand.push(card(CardId::Defend, 3));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 1,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert!(report
            .risk_notes
            .iter()
            .any(|note| note.kind == "second_wind_multi_plan_semantics"));
        assert!(report.sequence_classes.iter().any(|sequence| {
            sequence.diagnostics.exhaust_value > 0 || sequence.diagnostics.block_score > 0
        }));
    }

    #[test]
    fn combat_turn_plan_probe_marks_fiend_fire_multi_plan_semantics() {
        let mut combat = blank_test_combat();
        combat
            .entities
            .monsters
            .push(planned_monster(EnemyId::Cultist, 1));
        combat.zones.hand.push(card(CardId::FiendFire, 1));
        combat.zones.hand.push(card(CardId::Bash, 2));
        combat.zones.hand.push(card(CardId::Strike, 3));

        let report = probe_turn_plans(
            &EngineState::CombatPlayerTurn,
            &combat,
            CombatTurnPlanProbeConfig {
                max_depth: 1,
                ..CombatTurnPlanProbeConfig::default()
            },
        );

        assert!(report
            .risk_notes
            .iter()
            .any(|note| note.kind == "fiend_fire_multi_plan_semantics"));
    }
}
