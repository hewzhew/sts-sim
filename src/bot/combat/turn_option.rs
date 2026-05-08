use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};

use serde::Serialize;
use serde_json::json;

use crate::runtime::combat::{CombatState, Power};
use crate::state::core::ClientInput;
use crate::state::EngineState;

use super::decision::{outcome_from_end_state, DecisionOutcome, TerminalForecast};
use super::dominance::TurnResourceSummary;
use super::exact_turn_solver::{ExactTurnSolution, TurnEndState};

#[derive(Clone, Debug)]
pub(crate) struct TurnOption {
    pub index: usize,
    pub line: Vec<ClientInput>,
    pub first_input: Option<ClientInput>,
    pub outcome: DecisionOutcome,
    pub resources: TurnResourceSummary,
    pub effect: TurnEffectVector,
    pub signature: String,
}

#[derive(Clone, Debug)]
pub(crate) struct TurnOptionPack {
    pub options: Vec<TurnOption>,
    pub groups: Vec<TurnPlanGroup>,
    pub exact_truncated: bool,
    pub exact_elapsed_ms: u128,
    pub exact_explored_nodes: u32,
    pub exact_dominance_prunes: u32,
    pub exact_cycle_cuts: u32,
    pub exact_cache_hits: u32,
    pub exact_cache_misses: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct TurnOptionEvidence {
    pub status_reason: Option<&'static str>,
    pub audit: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct TurnEffectVector {
    pub line_close: LineCloseEffect,
    pub post_turn_frontier: PostTurnFrontierEffect,
    pub resource: ResourceEffect,
    pub risk: RiskEffect,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct LineCloseEffect {
    pub boundary: LineCloseBoundary,
    pub energy: i32,
    pub hand_count: usize,
    pub draw_count: usize,
    pub discard_count: usize,
    pub exhaust_count: usize,
    pub hand_delta: i32,
    pub draw_delta: i32,
    pub discard_delta: i32,
    pub exhaust_delta: i32,
    pub enemy_total_after: i32,
    pub enemy_total_delta: i32,
    pub living_after: usize,
    pub killed_count: usize,
    pub near_lethal_enemies: usize,
    pub player_buff_delta: i32,
    pub player_debuff_delta: i32,
    pub enemy_buff_delta: i32,
    pub enemy_debuff_delta: i32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LineCloseBoundary {
    PlayerTurn,
    PendingChoice,
    Other,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct PostTurnFrontierEffect {
    pub final_hp: i32,
    pub final_block: i32,
    pub combat_cleared: bool,
    pub dies_in_window: bool,
    pub enemy_total_after: i32,
    pub enemy_total_delta: i32,
    pub living_after: usize,
    pub note: &'static str,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ResourceEffect {
    pub spent_potions: u8,
    pub hp_lost: i32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct RiskEffect {
    pub exact_truncated: bool,
    pub pending_unresolved: bool,
    pub random_branch_assessment: RandomBranchAssessment,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RandomBranchAssessment {
    NotAssessed,
}

#[derive(Clone, Debug)]
pub(crate) struct TurnPlanGroup {
    pub group_id: usize,
    pub signature: String,
    pub size: usize,
    pub representative: TurnOption,
    pub member_first_inputs: Vec<String>,
}

pub(crate) fn build_turn_option_pack(
    root_combat: &CombatState,
    solution: &ExactTurnSolution,
) -> TurnOptionPack {
    let options = solution
        .nondominated_end_states
        .iter()
        .enumerate()
        .map(|(index, end_state)| {
            option_from_end_state(index, root_combat, end_state, solution.truncated)
        })
        .collect::<Vec<_>>();
    let groups = build_turn_plan_groups(&options);

    TurnOptionPack {
        options,
        groups,
        exact_truncated: solution.truncated,
        exact_elapsed_ms: solution.elapsed_ms,
        exact_explored_nodes: solution.explored_nodes,
        exact_dominance_prunes: solution.dominance_prunes,
        exact_cycle_cuts: solution.cycle_cuts,
        exact_cache_hits: solution.cache_hits,
        exact_cache_misses: solution.cache_misses,
    }
}

pub(crate) fn build_turn_option_evidence(
    root_combat: &CombatState,
    solution: &ExactTurnSolution,
    legacy_frontier_choice: &ClientInput,
    audit_budget: usize,
) -> TurnOptionEvidence {
    let pack = build_turn_option_pack(root_combat, solution);
    let status_reason = if pack.exact_truncated {
        Some("exact_turn_truncated")
    } else if pack.groups.is_empty() {
        Some("no_turn_options")
    } else {
        Some("evidence_only")
    };

    let audit = evidence_audit(&pack, status_reason, legacy_frontier_choice, audit_budget);

    TurnOptionEvidence {
        status_reason,
        audit,
    }
}

pub(crate) fn unavailable_turn_option_evidence(
    status_reason: &'static str,
    legacy_frontier_choice: &ClientInput,
) -> TurnOptionEvidence {
    TurnOptionEvidence {
        status_reason: Some(status_reason),
        audit: json!({
            "kind": "turn_option_evidence_v0",
            "available": false,
            "decision_role": "evidence_only",
            "status_reason": status_reason,
            "legacy_frontier_choice": format!("{legacy_frontier_choice:?}"),
            "option_count": 0,
            "group_count": 0,
            "compression_ratio": serde_json::Value::Null,
            "sample_groups": [],
            "sample_options_debug": [],
        }),
    }
}

fn option_from_end_state(
    index: usize,
    root_combat: &CombatState,
    end_state: &TurnEndState,
    exact_truncated: bool,
) -> TurnOption {
    let outcome = outcome_from_end_state(end_state);
    let effect = turn_effect_vector(
        root_combat,
        &end_state.line_close_engine,
        &end_state.line_close_combat,
        &end_state.frontier_engine,
        &end_state.frontier_combat,
        &end_state.resources,
        &outcome,
        exact_truncated,
    );
    let signature = effect_signature(&effect, &outcome);
    TurnOption {
        index,
        line: end_state.line.clone(),
        first_input: end_state.line.first().cloned(),
        outcome,
        resources: end_state.resources,
        effect,
        signature,
    }
}

fn evidence_audit(
    pack: &TurnOptionPack,
    status_reason: Option<&'static str>,
    legacy_frontier_choice: &ClientInput,
    audit_budget: usize,
) -> serde_json::Value {
    let group_count = pack.groups.len();
    let compression_ratio = if pack.options.is_empty() {
        None
    } else {
        Some(group_count as f64 / pack.options.len() as f64)
    };
    json!({
        "kind": "turn_option_evidence_v0",
        "available": true,
        "decision_role": "evidence_only",
        "status_reason": status_reason,
        "exact_truncated": pack.exact_truncated,
        "exact_elapsed_ms": pack.exact_elapsed_ms,
        "exact_explored_nodes": pack.exact_explored_nodes,
        "exact_dominance_prunes": pack.exact_dominance_prunes,
        "exact_cycle_cuts": pack.exact_cycle_cuts,
        "exact_cache_hits": pack.exact_cache_hits,
        "exact_cache_misses": pack.exact_cache_misses,
        "option_count": pack.options.len(),
        "group_count": group_count,
        "compression_ratio": compression_ratio,
        "boundary_model": {
            "line_close": "state after the chosen same-turn line before forced EndTurn projection; current-turn plan effects may use this boundary",
            "post_turn_frontier": "state after EndTurn projection to a stable boundary; survival and settlement may use this boundary",
        },
        "legacy_frontier_choice": format!("{legacy_frontier_choice:?}"),
        "sample_groups": pack
            .groups
            .iter()
            .take(audit_budget.max(1))
            .map(group_audit)
            .collect::<Vec<_>>(),
        "sample_options_debug": pack
            .options
            .iter()
            .take(audit_budget.min(3).max(1))
            .map(option_audit)
            .collect::<Vec<_>>(),
    })
}

fn build_turn_plan_groups(options: &[TurnOption]) -> Vec<TurnPlanGroup> {
    let mut by_signature: HashMap<String, TurnPlanGroup> = HashMap::new();
    for option in options {
        let entry = by_signature
            .entry(option.signature.clone())
            .or_insert_with(|| TurnPlanGroup {
                group_id: 0,
                signature: option.signature.clone(),
                size: 0,
                representative: option.clone(),
                member_first_inputs: Vec::new(),
            });
        entry.size += 1;
        if option.line.len() < entry.representative.line.len()
            || (option.line.len() == entry.representative.line.len()
                && option.index < entry.representative.index)
        {
            entry.representative = option.clone();
        }
    }

    let mut groups = by_signature.into_values().collect::<Vec<_>>();
    for group in &mut groups {
        let mut members = BTreeSet::new();
        for option in options
            .iter()
            .filter(|option| option.signature == group.signature)
        {
            if let Some(input) = option.first_input.as_ref() {
                members.insert(format!("{input:?}"));
            }
        }
        group.member_first_inputs = members.into_iter().collect();
    }
    groups.sort_by(compare_turn_plan_groups);
    for (index, group) in groups.iter_mut().enumerate() {
        group.group_id = index;
    }
    groups
}

fn compare_turn_plan_groups(left: &TurnPlanGroup, right: &TurnPlanGroup) -> Ordering {
    right
        .size
        .cmp(&left.size)
        .then_with(|| left.signature.cmp(&right.signature))
        .then_with(|| {
            left.representative
                .line
                .len()
                .cmp(&right.representative.line.len())
        })
        .then_with(|| left.representative.index.cmp(&right.representative.index))
}

fn group_audit(group: &TurnPlanGroup) -> serde_json::Value {
    json!({
        "group_id": group.group_id,
        "size": group.size,
        "representative_index": group.representative.index,
        "representative_first_input": group
            .representative
            .first_input
            .as_ref()
            .map(|input| format!("{input:?}")),
        "representative_line": line_strings(&group.representative.line),
        "representative_outcome": group.representative.outcome,
        "effect_summary": effect_summary(&group.representative.effect),
        "member_first_inputs": group.member_first_inputs,
    })
}

fn option_audit(option: &TurnOption) -> serde_json::Value {
    json!({
        "index": option.index,
        "first_input": option.first_input.as_ref().map(|input| format!("{input:?}")),
        "line": line_strings(&option.line),
        "line_len": option.line.len(),
        "outcome": option.outcome,
        "resources": {
            "spent_potions": option.resources.spent_potions,
            "hp_lost": option.resources.hp_lost,
            "exhausted_cards": option.resources.exhausted_cards,
            "enemy_buff_delta": option.resources.enemy_buff_delta,
            "final_hp": option.resources.final_hp,
            "final_block": option.resources.final_block,
        },
        "effect_summary": effect_summary(&option.effect),
    })
}

fn line_strings(line: &[ClientInput]) -> Vec<String> {
    line.iter().map(|input| format!("{input:?}")).collect()
}

fn effect_summary(effect: &TurnEffectVector) -> serde_json::Value {
    json!({
        "line_close": {
            "boundary": effect.line_close.boundary,
            "energy": effect.line_close.energy,
            "hand_count": effect.line_close.hand_count,
            "draw_count": effect.line_close.draw_count,
            "discard_count": effect.line_close.discard_count,
            "exhaust_count": effect.line_close.exhaust_count,
            "enemy_total_after": effect.line_close.enemy_total_after,
            "enemy_total_delta": effect.line_close.enemy_total_delta,
            "living_after": effect.line_close.living_after,
            "killed_count": effect.line_close.killed_count,
            "near_lethal_enemies": effect.line_close.near_lethal_enemies,
            "player_buff_delta": effect.line_close.player_buff_delta,
            "player_debuff_delta": effect.line_close.player_debuff_delta,
            "enemy_buff_delta": effect.line_close.enemy_buff_delta,
            "enemy_debuff_delta": effect.line_close.enemy_debuff_delta,
        },
        "post_turn_frontier": {
            "final_hp": effect.post_turn_frontier.final_hp,
            "final_block": effect.post_turn_frontier.final_block,
            "combat_cleared": effect.post_turn_frontier.combat_cleared,
            "dies_in_window": effect.post_turn_frontier.dies_in_window,
            "enemy_total_after": effect.post_turn_frontier.enemy_total_after,
            "enemy_total_delta": effect.post_turn_frontier.enemy_total_delta,
            "living_after": effect.post_turn_frontier.living_after,
            "note": effect.post_turn_frontier.note,
        },
        "hp_lost": effect.resource.hp_lost,
        "spent_potions": effect.resource.spent_potions,
        "pending_unresolved": effect.risk.pending_unresolved,
        "random_branch_assessment": effect.risk.random_branch_assessment,
    })
}

fn turn_effect_vector(
    root: &CombatState,
    line_close_engine: &EngineState,
    line_close: &CombatState,
    frontier_engine: &EngineState,
    frontier: &CombatState,
    resources: &TurnResourceSummary,
    outcome: &DecisionOutcome,
    exact_truncated: bool,
) -> TurnEffectVector {
    let enemy_total_before = total_enemy_hp(root);
    let line_close_enemy_total_after = total_enemy_hp(line_close);
    let enemy_total_after = total_enemy_hp(frontier);
    let living_before = living_monster_count(root);
    let line_close_living_after = living_monster_count(line_close);
    let living_after = living_monster_count(frontier);
    TurnEffectVector {
        line_close: LineCloseEffect {
            boundary: line_close_boundary(line_close_engine),
            energy: i32::from(line_close.turn.energy),
            hand_count: line_close.zones.hand.len(),
            draw_count: line_close.zones.draw_pile.len(),
            discard_count: line_close.zones.discard_pile.len(),
            exhaust_count: line_close.zones.exhaust_pile.len(),
            hand_delta: len_delta(line_close.zones.hand.len(), root.zones.hand.len()),
            draw_delta: len_delta(line_close.zones.draw_pile.len(), root.zones.draw_pile.len()),
            discard_delta: len_delta(
                line_close.zones.discard_pile.len(),
                root.zones.discard_pile.len(),
            ),
            exhaust_delta: len_delta(
                line_close.zones.exhaust_pile.len(),
                root.zones.exhaust_pile.len(),
            ),
            enemy_total_after: line_close_enemy_total_after,
            enemy_total_delta: enemy_total_before - line_close_enemy_total_after,
            living_after: line_close_living_after,
            killed_count: living_before.saturating_sub(line_close_living_after),
            near_lethal_enemies: near_lethal_enemy_count(line_close),
            player_buff_delta: player_buff_score(line_close) - player_buff_score(root),
            player_debuff_delta: player_debuff_score(line_close) - player_debuff_score(root),
            enemy_buff_delta: enemy_buff_score(line_close) - enemy_buff_score(root),
            enemy_debuff_delta: enemy_debuff_score(line_close) - enemy_debuff_score(root),
        },
        post_turn_frontier: PostTurnFrontierEffect {
            final_hp: resources.final_hp,
            final_block: resources.final_block,
            combat_cleared: matches!(outcome.terminality, TerminalForecast::LethalWin)
                || living_after == 0,
            dies_in_window: matches!(outcome.terminality, TerminalForecast::DiesInWindow),
            enemy_total_after,
            enemy_total_delta: enemy_total_before - enemy_total_after,
            living_after,
            note: "post end-turn stable boundary; use for survival/settlement, not current-turn access",
        },
        resource: ResourceEffect {
            spent_potions: resources.spent_potions,
            hp_lost: resources.hp_lost,
        },
        risk: RiskEffect {
            exact_truncated,
            pending_unresolved: matches!(frontier_engine, EngineState::PendingChoice(_)),
            random_branch_assessment: RandomBranchAssessment::NotAssessed,
        },
    }
}

fn effect_signature(effect: &TurnEffectVector, outcome: &DecisionOutcome) -> String {
    format!(
        "surv={:?}|pos={:?}|term={:?}|fhp{}|fblk{}|lc={:?}|lcet{}|lced{}|lclive{}|lckill{}|lce{}|lch{}|lcd{}|lcdisc{}|lcexh{}|pot{}|hpl{}|pb{}|pd{}|eb{}|edb{}|pending{}",
        outcome.survival,
        outcome.position,
        outcome.terminality,
        bucket(effect.post_turn_frontier.final_hp, 5),
        bucket(effect.post_turn_frontier.final_block, 5),
        effect.line_close.boundary,
        bucket(effect.line_close.enemy_total_after, 5),
        bucket(effect.line_close.enemy_total_delta, 5),
        effect.line_close.living_after,
        effect.line_close.killed_count,
        effect.line_close.energy,
        sign_bucket(effect.line_close.hand_delta),
        sign_bucket(effect.line_close.draw_delta),
        sign_bucket(effect.line_close.discard_delta),
        sign_bucket(effect.line_close.exhaust_delta),
        effect.resource.spent_potions,
        bucket(effect.resource.hp_lost, 5),
        bucket(effect.line_close.player_buff_delta, 2),
        bucket(effect.line_close.player_debuff_delta, 2),
        bucket(effect.line_close.enemy_buff_delta, 2),
        bucket(effect.line_close.enemy_debuff_delta, 2),
        effect.risk.pending_unresolved,
    )
}

fn bucket(value: i32, step: i32) -> i32 {
    if step <= 1 {
        value
    } else {
        value.div_euclid(step)
    }
}

fn sign_bucket(value: i32) -> i32 {
    value.signum()
}

fn len_delta(after: usize, before: usize) -> i32 {
    after as i32 - before as i32
}

fn line_close_boundary(engine: &EngineState) -> LineCloseBoundary {
    match engine {
        EngineState::CombatPlayerTurn => LineCloseBoundary::PlayerTurn,
        EngineState::PendingChoice(_) => LineCloseBoundary::PendingChoice,
        _ => LineCloseBoundary::Other,
    }
}

fn total_enemy_hp(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && monster.current_hp > 0)
        .map(|monster| monster.current_hp + monster.block)
        .sum()
}

fn living_monster_count(combat: &CombatState) -> usize {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            !monster.is_dying && !monster.half_dead && !monster.is_escaped && monster.current_hp > 0
        })
        .count()
}

fn near_lethal_enemy_count(combat: &CombatState) -> usize {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            !monster.is_dying
                && !monster.half_dead
                && !monster.is_escaped
                && monster.current_hp > 0
                && monster.current_hp + monster.block <= 6
        })
        .count()
}

fn player_buff_score(combat: &CombatState) -> i32 {
    combat.entities.power_db.get(&0).map_or(0, |powers| {
        powers
            .iter()
            .filter(|power| !crate::content::powers::is_debuff(power.power_type, power.amount))
            .map(generic_power_magnitude)
            .sum()
    })
}

fn player_debuff_score(combat: &CombatState) -> i32 {
    combat.entities.power_db.get(&0).map_or(0, |powers| {
        powers
            .iter()
            .filter(|power| crate::content::powers::is_debuff(power.power_type, power.amount))
            .map(generic_power_magnitude)
            .sum()
    })
}

fn enemy_buff_score(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .map(|monster| {
            combat
                .entities
                .power_db
                .get(&monster.id)
                .map_or(0, |powers| {
                    powers
                        .iter()
                        .filter(|power| {
                            !crate::content::powers::is_debuff(power.power_type, power.amount)
                        })
                        .map(generic_power_magnitude)
                        .sum::<i32>()
                })
        })
        .sum()
}

fn enemy_debuff_score(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .map(|monster| {
            combat
                .entities
                .power_db
                .get(&monster.id)
                .map_or(0, |powers| {
                    powers
                        .iter()
                        .filter(|power| {
                            crate::content::powers::is_debuff(power.power_type, power.amount)
                        })
                        .map(generic_power_magnitude)
                        .sum::<i32>()
                })
        })
        .sum()
}

fn generic_power_magnitude(power: &Power) -> i32 {
    power.amount.abs().clamp(1, 8)
}
