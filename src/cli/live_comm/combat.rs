use super::frame::LiveFrame;
use super::io::LiveCommIo;
use super::snapshot::write_failure_snapshot;
use super::LiveParityMode;
use crate::bot::branch_family_for_card;
use crate::bot::comm_mod;
use crate::bot::coverage::CoverageDb;
use crate::bot::monster_belief::{build_combat_belief_state, MonsterBeliefCertainty};
use crate::bot::search::{
    sequencing_assessment_for_input, SearchDiagnostics, SearchMoveStat, StatePressureFeatures,
};
use crate::bot::sidecar::CombatTopCandidateRecord;
use crate::runtime::combat::CombatState;
use crate::content::monsters::EnemyId;
use crate::diff::protocol::mapper::{card_id_from_java, monster_id_from_java, power_id_from_java};
use crate::diff::protocol::snapshot::build_live_combat_snapshot as build_protocol_live_combat_snapshot;
use crate::diff::replay::comparator::{ActionContext, DiffCategory, DiffResult};
use crate::state::core::{ClientInput, EngineState};
use serde::Serialize;
use serde_json::Value;
use std::io::Write;

const SEARCH_DIAG_TOP_K: usize = 5;
const SUSPICIOUS_SEQUENCE_THRESHOLD: f32 = 150_000.0;
const SUSPICIOUS_EXHAUST_THRESHOLD: f32 = 80_000.0;
const SUSPICIOUS_TOP_GAP_THRESHOLD: f32 = 3.0;

struct CombatDiffRecord {
    _frame: u64,
    field: String,
    category: DiffCategory,
    rust_val: String,
    java_val: String,
}

struct CombatStats {
    start_frame: u64,
    action_count: u32,
    diffs: Vec<CombatDiffRecord>,
    seen_content_gaps: std::collections::HashSet<String>,
    search_decisions: u32,
    search_elapsed_total_ms: u128,
    search_elapsed_max_ms: u128,
    diag_render_total_ms: u128,
    diag_render_max_ms: u128,
}

#[derive(Serialize)]
struct CombatSearchSuspectRecord {
    frame_count: u64,
    response_id: Option<i64>,
    state_frame_id: Option<i64>,
    chosen_move: String,
    heuristic_move: String,
    search_move: String,
    top_candidates: Vec<CombatTopCandidateRecord>,
    top_gap: Option<f32>,
    sequence_bonus: f32,
    sequence_frontload_bonus: f32,
    sequence_defer_bonus: f32,
    sequence_branch_bonus: f32,
    sequence_downside_penalty: f32,
    survival_window_delta: f32,
    exhaust_evidence_delta: f32,
    realized_exhaust_block: i32,
    realized_exhaust_draw: i32,
    branch_family: Option<String>,
    sequencing_rationale_key: Option<String>,
    branch_rationale_key: Option<String>,
    downside_rationale_key: Option<String>,
    hidden_intent_active: bool,
    visible_incoming: i32,
    visible_unblocked: i32,
    belief_expected_incoming: i32,
    belief_expected_unblocked: i32,
    belief_max_incoming: i32,
    belief_max_unblocked: i32,
    value_incoming: i32,
    value_unblocked: i32,
    survival_guard_incoming: i32,
    survival_guard_unblocked: i32,
    belief_attack_probability: f32,
    belief_lethal_probability: f32,
    belief_urgent_probability: f32,
    heuristic_search_gap: bool,
    large_sequence_bonus: bool,
    tight_root_gap: bool,
    reasons: Vec<String>,
}

fn log_potion_decision_trace(live_io: &mut LiveCommIo, combat: &CombatState) {
    let snapshot = crate::bot::potions::immediate_potion_snapshot(combat);
    writeln!(
        live_io.log,
        "  [POTION DIAG] min_priority={} {}",
        snapshot.minimum_priority, snapshot.context_summary
    )
    .unwrap();

    if let Some(chosen) = snapshot.chosen.as_ref() {
        writeln!(
            live_io.log,
            "  [POTION DIAG] chosen {}",
            chosen.debug_summary(snapshot.minimum_priority)
        )
        .unwrap();
        writeln!(
            live_io.focus_log,
            "[POTION] min_priority={} {}",
            snapshot.minimum_priority,
            chosen.debug_summary(snapshot.minimum_priority)
        )
        .unwrap();
    } else {
        writeln!(live_io.log, "  [POTION DIAG] chosen <none>").unwrap();
    }

    for (rank, candidate) in snapshot.candidates.iter().take(5).enumerate() {
        writeln!(
            live_io.log,
            "  [POTION DIAG] rank={} {}",
            rank + 1,
            candidate.debug_summary(snapshot.minimum_priority)
        )
        .unwrap();
    }
}

fn total_incoming_damage_for_log(combat: &CombatState) -> i32 {
    StatePressureFeatures::from_combat(combat).value_incoming
}

fn log_hidden_intent_belief(live_io: &mut LiveCommIo, combat: &CombatState) {
    let belief = build_combat_belief_state(combat);
    if !belief.hidden_intent_active {
        return;
    }

    let _ = writeln!(
        live_io.focus_log,
        "[BELIEF] expected_incoming={:.1} max_incoming={} attack_prob={:.2} lethal_prob={:.2} urgent_prob={:.2}",
        belief.expected_incoming_damage,
        belief.max_incoming_damage,
        belief.attack_probability,
        belief.lethal_probability,
        belief.urgent_probability
    );
    let _ = writeln!(
        live_io.log,
        "  [BELIEF] hidden_intent active expected_incoming={:.1} max_incoming={} attack_prob={:.2} lethal_prob={:.2} urgent_prob={:.2} public_complete={}",
        belief.expected_incoming_damage,
        belief.max_incoming_damage,
        belief.attack_probability,
        belief.lethal_probability,
        belief.urgent_probability,
        belief.public_state_complete
    );

    for monster in &belief.monsters {
        let certainty = match monster.certainty {
            MonsterBeliefCertainty::Exact => "exact",
            MonsterBeliefCertainty::Distribution => "distribution",
            MonsterBeliefCertainty::Unknown => "unknown",
        };
        let moves = if monster.predicted_moves.is_empty() {
            "unknown".to_string()
        } else {
            monster
                .predicted_moves
                .iter()
                .map(|predicted| {
                    format!(
                        "{}:{:?} p={:.2} dmg={}x{}",
                        predicted.move_id,
                        predicted.intent,
                        predicted.probability,
                        predicted.base_damage,
                        predicted.hits
                    )
                })
                .collect::<Vec<_>>()
                .join(" | ")
        };
        let _ = writeln!(
            live_io.log,
            "  [BELIEF] monster={} certainty={} source={:?} expected={:.1} max={} attack_prob={:.2} rationale={} moves=[{}]",
            monster.monster_name,
            certainty,
            monster.inference_source,
            monster.expected_incoming_damage,
            monster.max_incoming_damage,
            monster.attack_probability,
            monster.rationale_key.unwrap_or(""),
            moves
        );
    }
}

fn summarize_cached_candidate_outcome(combat: &CombatState, stat: &SearchMoveStat) -> String {
    format!(
        "cached_outcome hp {}->{}, block {}->{}, energy {}->{}, hand {}->{}, draw {}->{}, disc {}->{}, exhaust {}->{}, incoming {}->{}, enemy_total {}->{}",
        combat.entities.player.current_hp,
        stat.immediate_hp,
        combat.entities.player.block,
        stat.immediate_block,
        combat.turn.energy,
        stat.immediate_energy,
        combat.zones.hand.len(),
        stat.immediate_hand_len,
        combat.zones.draw_pile.len(),
        stat.immediate_draw_len,
        combat.zones.discard_pile.len(),
        stat.immediate_discard_len,
        combat.zones.exhaust_pile.len(),
        stat.immediate_exhaust_len,
        total_incoming_damage_for_log(combat),
        stat.immediate_incoming,
        total_enemy_hp_for_log(combat),
        stat.immediate_enemy_total,
    )
}

fn total_enemy_hp_for_log(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead && m.current_hp > 0)
        .map(|m| m.current_hp + m.block)
        .sum()
}

fn verbose_search_outcome_logging_enabled() -> bool {
    std::env::var("STS_LIVECOMM_VERBOSE_SEARCH_OUTCOME")
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn same_client_input(left: &ClientInput, right: &ClientInput) -> bool {
    match (left, right) {
        (
            ClientInput::PlayCard {
                card_index: left_card,
                target: left_target,
            },
            ClientInput::PlayCard {
                card_index: right_card,
                target: right_target,
            },
        ) => left_card == right_card && left_target == right_target,
        (
            ClientInput::UsePotion {
                potion_index: left_potion,
                target: left_target,
            },
            ClientInput::UsePotion {
                potion_index: right_potion,
                target: right_target,
            },
        ) => left_potion == right_potion && left_target == right_target,
        (ClientInput::EndTurn, ClientInput::EndTurn)
        | (ClientInput::Proceed, ClientInput::Proceed)
        | (ClientInput::Cancel, ClientInput::Cancel) => true,
        _ => false,
    }
}

fn describe_client_input(combat: &CombatState, input: &ClientInput) -> String {
    match input {
        ClientInput::PlayCard { card_index, target } => {
            let card = combat
                .zones
                .hand
                .get(*card_index)
                .map(format_card)
                .unwrap_or_else(|| format!("hand[{card_index}]"));
            match target {
                Some(target) => format!("Play #{} {card} @{target}", card_index + 1),
                None => format!("Play #{} {card}", card_index + 1),
            }
        }
        ClientInput::UsePotion {
            potion_index,
            target,
        } => match target {
            Some(target) => format!("UsePotion#{potion_index} @{target}"),
            None => format!("UsePotion#{potion_index}"),
        },
        ClientInput::EndTurn => "EndTurn".to_string(),
        other => format!("{other:?}"),
    }
}

fn format_card(card: &crate::runtime::combat::CombatCard) -> String {
    let mut label = crate::content::cards::get_card_definition(card.id)
        .name
        .to_string();
    if card.upgrades > 0 {
        label.push_str(&"+".repeat(card.upgrades as usize));
    }
    label
}

fn format_search_move_diag(combat: &CombatState, stat: &SearchMoveStat) -> String {
    let sequencing = sequencing_assessment_for_input(
        combat,
        &stat.input,
        total_incoming_damage_for_log(combat) <= combat.entities.player.block,
    );
    let branch_family = branch_family_for_input(combat, &stat.input)
        .map(|family| family.as_str())
        .unwrap_or("none");
    let cluster_suffix = if stat.cluster_size > 1 {
        format!(
            " cluster_id={} cluster_size={} reduced_kind={} collapsed=[{}]",
            stat.cluster_id,
            stat.cluster_size,
            stat.equivalence_kind
                .map(|kind| kind.as_str())
                .unwrap_or("none"),
            stat.collapsed_inputs
                .iter()
                .map(|input| describe_client_input(combat, input))
                .collect::<Vec<_>>()
                .join(" | ")
        )
    } else {
        String::new()
    };
    format!(
        "move={} visits={} avg_score={:.2} order={:.1} leaf={:.1} policy={:.1} sequence={:.1} frontload={:.1} defer={:.1} branch={:.1} downside={:.1} survival_window={:.1} exhaust_evidence={:.1} projected_hp={} projected_block={} projected_unblocked={} projected_enemy_total={} survives={} exhaust_block={} exhaust_draw={} branch_family={} rationale={} branch_rationale={} downside_rationale={}",
        describe_client_input(combat, &stat.input),
        stat.visits,
        stat.avg_score,
        stat.order_score,
        stat.leaf_score,
        stat.policy_bonus,
        stat.sequence_bonus,
        stat.sequence_frontload_bonus,
        stat.sequence_defer_bonus,
        stat.sequence_branch_bonus,
        stat.sequence_downside_penalty,
        stat.sequence_survival_bonus,
        stat.sequence_exhaust_bonus,
        stat.projected_hp,
        stat.projected_block,
        stat.projected_unblocked,
        stat.projected_enemy_total,
        stat.survives,
        stat.realized_exhaust_block,
        stat.realized_exhaust_draw,
        branch_family,
        sequencing
            .as_ref()
            .map(|assessment| assessment.rationale_key)
            .unwrap_or(""),
        sequencing
            .as_ref()
            .map(|assessment| assessment.branch_rationale_key)
            .unwrap_or(""),
        sequencing
            .as_ref()
            .map(|assessment| assessment.downside_rationale_key)
            .unwrap_or("")
    ) + &cluster_suffix
}

fn format_search_profile_summary(search_diag: &SearchDiagnostics) -> String {
    let profile = &search_diag.profile;
    format!(
        "search_ms={} render_ms={} root(legal_ms={} reduce_ms={} reduce={}=>{} clones={} leaf_ms={} leaf_calls={} avg_branch={:.1}->{:.1}) recursive(legal_ms={} reduce_ms={} reduce={}=>{} clones={} leaf_ms={} leaf_calls={} avg_branch={:.1}->{:.1}) advance(ms={} calls={} steps={} p50={} p95={} max={}) sequence_judge_ms={} nodes={} terminal_nodes={}",
        profile.search_total_ms,
        profile.root_diag_render_ms,
        profile.root.legal_move_gen_ms,
        profile.root.transition_reduce_ms,
        profile.root.transition_reduce_inputs,
        profile.root.transition_reduce_outputs,
        profile.root.clone_calls,
        profile.root.leaf_eval_ms,
        profile.root.leaf_eval_calls,
        profile.root.avg_branch_before_reduce,
        profile.root.avg_branch_after_reduce,
        profile.recursive.legal_move_gen_ms,
        profile.recursive.transition_reduce_ms,
        profile.recursive.transition_reduce_inputs,
        profile.recursive.transition_reduce_outputs,
        profile.recursive.clone_calls,
        profile.recursive.leaf_eval_ms,
        profile.recursive.leaf_eval_calls,
        profile.recursive.avg_branch_before_reduce,
        profile.recursive.avg_branch_after_reduce,
        profile.advance_ms,
        profile.advance_calls,
        profile.advance_engine_steps,
        profile.advance_steps_p50,
        profile.advance_steps_p95,
        profile.advance_steps_max,
        profile.sequence_judge_ms,
        profile.nodes.nodes_expanded,
        profile.nodes.terminal_nodes
    )
}

fn maybe_record_search_suspect(
    live_io: &mut LiveCommIo,
    frame_count: u64,
    frame: &LiveFrame,
    combat: &CombatState,
    heuristic_diag: &crate::bot::combat_heuristic::HeuristicDiagnostics,
    search_diag: &SearchDiagnostics,
) {
    let Some(chosen_stat) = search_diag.top_moves.first() else {
        return;
    };

    let top_gap = search_diag
        .top_moves
        .get(1)
        .map(|second| chosen_stat.avg_score - second.avg_score);
    let heuristic_search_gap =
        if same_client_input(&heuristic_diag.chosen_move, &search_diag.chosen_move) {
            false
        } else {
            let heuristic_rank_of_search = heuristic_diag
                .top_moves
                .iter()
                .position(|stat| same_client_input(&stat.input, &search_diag.chosen_move));
            let search_rank_of_heuristic = search_diag
                .top_moves
                .iter()
                .position(|stat| same_client_input(&stat.input, &heuristic_diag.chosen_move));
            heuristic_rank_of_search.is_none_or(|rank| rank >= 2)
                || search_rank_of_heuristic.is_none_or(|rank| rank >= 2)
        };
    let large_sequence_bonus = chosen_stat.sequence_bonus.abs() >= SUSPICIOUS_SEQUENCE_THRESHOLD
        || chosen_stat.sequence_survival_bonus.abs() >= SUSPICIOUS_SEQUENCE_THRESHOLD
        || chosen_stat.sequence_exhaust_bonus.abs() >= SUSPICIOUS_EXHAUST_THRESHOLD
        || chosen_stat.sequence_downside_penalty.abs() >= 8_000.0
        || chosen_stat.sequence_branch_bonus.abs() >= 8_000.0;
    let tight_root_gap = top_gap.is_some_and(|gap| gap.abs() <= SUSPICIOUS_TOP_GAP_THRESHOLD);
    let sequencing_conflict = heuristic_search_gap
        && (chosen_stat.sequence_frontload_bonus.abs() >= 3_000.0
            || chosen_stat.sequence_defer_bonus.abs() >= 3_000.0
            || chosen_stat.sequence_downside_penalty.abs() >= 3_000.0);
    let branch_opening_conflict = heuristic_search_gap
        && (chosen_stat.sequence_branch_bonus.abs() >= 3_500.0
            || chosen_stat.sequence_downside_penalty.abs() >= 3_500.0);

    let mut reasons = Vec::new();
    if heuristic_search_gap {
        reasons.push("heuristic_search_gap".to_string());
    }
    if large_sequence_bonus {
        reasons.push("large_sequence_bonus".to_string());
    }
    if tight_root_gap {
        reasons.push("tight_root_gap".to_string());
    }
    if sequencing_conflict {
        reasons.push("sequencing_conflict".to_string());
    }
    if branch_opening_conflict {
        reasons.push("branch_opening_conflict".to_string());
    }
    if reasons.is_empty() {
        return;
    }

    let sequencing = sequencing_assessment_for_input(
        combat,
        &search_diag.chosen_move,
        total_incoming_damage_for_log(combat) <= combat.entities.player.block,
    );
    let pressure = StatePressureFeatures::from_combat(combat);
    let top_candidates = search_diag
        .top_moves
        .iter()
        .take(SEARCH_DIAG_TOP_K)
        .map(|stat| combat_top_candidate_record(combat, stat))
        .collect::<Vec<_>>();
    let branch_family = branch_family_for_input(combat, &search_diag.chosen_move)
        .map(|family| family.as_str().to_string());

    let record = CombatSearchSuspectRecord {
        frame_count,
        response_id: frame.response_id(),
        state_frame_id: frame.state_frame_id(),
        chosen_move: describe_client_input(combat, &search_diag.chosen_move),
        heuristic_move: describe_client_input(combat, &heuristic_diag.chosen_move),
        search_move: describe_client_input(combat, &search_diag.chosen_move),
        top_candidates: top_candidates.clone(),
        top_gap,
        sequence_bonus: chosen_stat.sequence_bonus,
        sequence_frontload_bonus: chosen_stat.sequence_frontload_bonus,
        sequence_defer_bonus: chosen_stat.sequence_defer_bonus,
        sequence_branch_bonus: chosen_stat.sequence_branch_bonus,
        sequence_downside_penalty: chosen_stat.sequence_downside_penalty,
        survival_window_delta: chosen_stat.sequence_survival_bonus,
        exhaust_evidence_delta: chosen_stat.sequence_exhaust_bonus,
        realized_exhaust_block: chosen_stat.realized_exhaust_block,
        realized_exhaust_draw: chosen_stat.realized_exhaust_draw,
        branch_family,
        sequencing_rationale_key: sequencing
            .as_ref()
            .map(|assessment| assessment.rationale_key.to_string())
            .filter(|value| !value.is_empty()),
        branch_rationale_key: sequencing
            .as_ref()
            .map(|assessment| assessment.branch_rationale_key.to_string())
            .filter(|value| !value.is_empty()),
        downside_rationale_key: sequencing
            .as_ref()
            .map(|assessment| assessment.downside_rationale_key.to_string())
            .filter(|value| !value.is_empty()),
        hidden_intent_active: pressure.hidden_intent_active,
        visible_incoming: pressure.visible_incoming,
        visible_unblocked: pressure.visible_unblocked,
        belief_expected_incoming: pressure.belief_expected_incoming,
        belief_expected_unblocked: pressure.belief_expected_unblocked,
        belief_max_incoming: pressure.belief_max_incoming,
        belief_max_unblocked: pressure.belief_max_unblocked,
        value_incoming: pressure.value_incoming,
        value_unblocked: pressure.value_unblocked,
        survival_guard_incoming: pressure.survival_guard_incoming,
        survival_guard_unblocked: pressure.survival_guard_unblocked,
        belief_attack_probability: pressure.attack_probability,
        belief_lethal_probability: pressure.lethal_probability,
        belief_urgent_probability: pressure.urgent_probability,
        heuristic_search_gap,
        large_sequence_bonus,
        tight_root_gap,
        reasons: reasons.clone(),
    };
    if let Ok(encoded) = serde_json::to_string(&record) {
        let _ = writeln!(live_io.combat_suspects, "{}", encoded);
        let _ = live_io.combat_suspects.flush();
    }
    let _ = writeln!(
        live_io.focus_log,
        "[SUSPECT] frame={} state_frame_id={:?} reasons={} chosen={} heuristic={} top_gap={:?} sequence={:.1} frontload={:.1} defer={:.1} branch={:.1} downside={:.1} survival_window={:.1} exhaust_evidence={:.1} exhaust_block={} exhaust_draw={} branch_family={}",
        frame_count,
        frame.state_frame_id(),
        reasons.join(","),
        describe_client_input(combat, &search_diag.chosen_move),
        describe_client_input(combat, &heuristic_diag.chosen_move),
        top_gap,
        chosen_stat.sequence_bonus,
        chosen_stat.sequence_frontload_bonus,
        chosen_stat.sequence_defer_bonus,
        chosen_stat.sequence_branch_bonus,
        chosen_stat.sequence_downside_penalty,
        chosen_stat.sequence_survival_bonus,
        chosen_stat.sequence_exhaust_bonus,
        chosen_stat.realized_exhaust_block,
        chosen_stat.realized_exhaust_draw,
        branch_family_for_input(combat, &search_diag.chosen_move)
            .map(|family| family.as_str())
            .unwrap_or("none")
    );
    let belief = build_combat_belief_state(combat);
    let hidden_intent_high_risk = belief.hidden_intent_active
        && belief.urgent_probability >= 0.35
        && belief
            .monsters
            .iter()
            .any(|monster| monster.certainty != MonsterBeliefCertainty::Exact);
    let high_risk_snapshot = sequencing_conflict
        || branch_opening_conflict
        || chosen_stat.sequence_downside_penalty.abs() >= 8_000.0
        || top_gap.is_some_and(|gap| heuristic_search_gap && gap.abs() >= 12.0)
        || hidden_intent_high_risk;
    if high_risk_snapshot {
        let _ = write_failure_snapshot(
            live_io,
            frame_count,
            frame,
            "high_risk_suspect",
            reasons,
            serde_json::json!({
                "chosen_command": describe_client_input(combat, &search_diag.chosen_move),
                "heuristic_move": describe_client_input(combat, &heuristic_diag.chosen_move),
                "search_move": describe_client_input(combat, &search_diag.chosen_move),
                "top_gap": top_gap,
                "sequencing": {
                    "sequence_bonus": chosen_stat.sequence_bonus,
                    "frontload_bonus": chosen_stat.sequence_frontload_bonus,
                    "defer_bonus": chosen_stat.sequence_defer_bonus,
                    "branch_bonus": chosen_stat.sequence_branch_bonus,
                    "downside_penalty": chosen_stat.sequence_downside_penalty,
                    "survival_window_delta": chosen_stat.sequence_survival_bonus,
                    "exhaust_evidence_delta": chosen_stat.sequence_exhaust_bonus,
                    "sequencing_rationale_key": record.sequencing_rationale_key,
                    "branch_rationale_key": record.branch_rationale_key,
                    "downside_rationale_key": record.downside_rationale_key,
                },
                "belief_summary": {
                    "hidden_intent_active": belief.hidden_intent_active,
                    "expected_incoming": belief.expected_incoming_damage,
                    "max_incoming": belief.max_incoming_damage,
                    "attack_probability": belief.attack_probability,
                    "lethal_probability": belief.lethal_probability,
                    "urgent_probability": belief.urgent_probability,
                },
                "pressure_summary": {
                    "hidden_intent_active": pressure.hidden_intent_active,
                    "visible_incoming": pressure.visible_incoming,
                    "visible_unblocked": pressure.visible_unblocked,
                    "belief_expected_incoming": pressure.belief_expected_incoming,
                    "belief_expected_unblocked": pressure.belief_expected_unblocked,
                    "belief_max_incoming": pressure.belief_max_incoming,
                    "belief_max_unblocked": pressure.belief_max_unblocked,
                    "value_incoming": pressure.value_incoming,
                    "value_unblocked": pressure.value_unblocked,
                    "survival_guard_incoming": pressure.survival_guard_incoming,
                    "survival_guard_unblocked": pressure.survival_guard_unblocked,
                    "lethal_pressure": pressure.lethal_pressure,
                    "urgent_pressure": pressure.urgent_pressure,
                    "belief_attack_probability": pressure.attack_probability,
                    "belief_lethal_probability": pressure.lethal_probability,
                    "belief_urgent_probability": pressure.urgent_probability,
                },
                "top_candidates": top_candidates,
            }),
        );
    }
}

fn combat_top_candidate_record(
    combat: &CombatState,
    stat: &SearchMoveStat,
) -> CombatTopCandidateRecord {
    CombatTopCandidateRecord {
        move_label: describe_client_input(combat, &stat.input),
        avg_score: stat.avg_score,
        order_score: stat.order_score,
        leaf_score: stat.leaf_score,
        sequence_bonus: stat.sequence_bonus,
        sequence_frontload_bonus: stat.sequence_frontload_bonus,
        sequence_defer_bonus: stat.sequence_defer_bonus,
        sequence_branch_bonus: stat.sequence_branch_bonus,
        sequence_downside_penalty: stat.sequence_downside_penalty,
        projected_unblocked: stat.projected_unblocked,
        projected_enemy_total: stat.projected_enemy_total,
        survives: stat.survives,
        branch_family: branch_family_for_input(combat, &stat.input)
            .map(|family| family.as_str().to_string()),
        cluster_size: stat.cluster_size,
    }
}

fn branch_family_for_input(
    combat: &CombatState,
    input: &ClientInput,
) -> Option<crate::bot::BranchFamily> {
    let ClientInput::PlayCard { card_index, .. } = input else {
        return None;
    };
    let card = combat.zones.hand.get(*card_index)?;
    branch_family_for_card(card.id)
}

pub(super) enum CombatFrameOutcome {
    Continue,
    StopForParityFailure,
}

fn should_stop_for_combat_mismatch(
    parity_mode: LiveParityMode,
    has_parse_diff: bool,
    has_action_diff: bool,
) -> bool {
    parity_mode == LiveParityMode::Strict && (has_parse_diff || has_action_diff)
}

#[derive(Default)]
pub(super) struct CombatRuntime {
    pub(super) expected_combat_state: Option<CombatState>,
    pub(super) last_combat_truth: Option<CombatState>,
    pub(super) last_input: Option<ClientInput>,
    pub(super) action_context: ActionContext,
    combat_stats: Option<CombatStats>,
}

impl CombatStats {
    fn new(frame: u64) -> Self {
        Self {
            start_frame: frame,
            action_count: 0,
            diffs: Vec::new(),
            seen_content_gaps: std::collections::HashSet::new(),
            search_decisions: 0,
            search_elapsed_total_ms: 0,
            search_elapsed_max_ms: 0,
            diag_render_total_ms: 0,
            diag_render_max_ms: 0,
        }
    }

    fn has_findings(&self) -> bool {
        !self.diffs.is_empty()
    }

    fn write_summary<W: Write>(&self, log: &mut W, end_frame: u64) {
        let engine_bugs: Vec<_> = self
            .diffs
            .iter()
            .filter(|d| d.category == DiffCategory::EngineBug)
            .collect();
        let content_gaps: Vec<_> = self
            .diffs
            .iter()
            .filter(|d| d.category == DiffCategory::ContentGap)
            .collect();
        let timing: Vec<_> = self
            .diffs
            .iter()
            .filter(|d| d.category == DiffCategory::Timing)
            .collect();

        writeln!(
            log,
            "\n╔══════════════════════════════════════════════════════╗"
        )
        .unwrap();
        writeln!(
            log,
            "║  COMBAT SUMMARY (F{} ~ F{})                          ",
            self.start_frame, end_frame
        )
        .unwrap();
        writeln!(
            log,
            "╠══════════════════════════════════════════════════════╣"
        )
        .unwrap();
        writeln!(
            log,
            "║  Frames: {}  |  Actions: {}",
            end_frame - self.start_frame + 1,
            self.action_count
        )
        .unwrap();
        if self.search_decisions > 0 {
            let avg_search_ms = self.search_elapsed_total_ms as f64 / self.search_decisions as f64;
            let avg_render_ms = self.diag_render_total_ms as f64 / self.search_decisions as f64;
            writeln!(
                log,
                "║  SEARCH: decisions={} avg_ms={avg_search_ms:.1} max_ms={} diag_avg_ms={avg_render_ms:.1} diag_max_ms={}",
                self.search_decisions,
                self.search_elapsed_max_ms,
                self.diag_render_max_ms
            )
            .unwrap();
        }
        writeln!(log, "║  ENGINE BUGS:  {}", engine_bugs.len()).unwrap();
        writeln!(log, "║  CONTENT GAPS: {}", content_gaps.len()).unwrap();
        writeln!(log, "║  TIMING:       {}", timing.len()).unwrap();

        if !engine_bugs.is_empty() {
            writeln!(log, "║").unwrap();
            writeln!(log, "║  ⛔ Engine Bugs:").unwrap();
            let mut seen = std::collections::HashMap::<String, (usize, String, String)>::new();
            for d in &engine_bugs {
                let entry = seen.entry(d.field.clone()).or_insert((
                    0,
                    d.rust_val.clone(),
                    d.java_val.clone(),
                ));
                entry.0 += 1;
            }
            for (field, (count, rv, jv)) in &seen {
                writeln!(log, "║    - {} (×{}) Rust={} Java={}", field, count, rv, jv).unwrap();
            }
        }

        if !content_gaps.is_empty() {
            writeln!(log, "║").unwrap();
            writeln!(log, "║  ⚠ Content Gaps:").unwrap();
            let mut seen = std::collections::HashMap::<String, usize>::new();
            for d in &content_gaps {
                *seen.entry(d.field.clone()).or_insert(0) += 1;
            }
            for (field, count) in &seen {
                writeln!(log, "║    - {} (×{})", field, count).unwrap();
            }
        }

        let verdict = if !engine_bugs.is_empty() {
            "❌ Engine Bugs Found"
        } else if !content_gaps.is_empty() {
            "❌ Content Gaps Found"
        } else {
            "✅ Engine OK"
        };
        writeln!(log, "║").unwrap();
        writeln!(log, "║  VERDICT: {}", verdict).unwrap();
        writeln!(
            log,
            "╚══════════════════════════════════════════════════════╝"
        )
        .unwrap();
    }
}

impl CombatRuntime {
    pub(super) fn on_java_error(&mut self) {
        self.expected_combat_state = None;
    }

    pub(super) fn clear_after_combat_if_needed(
        &mut self,
        log: &mut std::fs::File,
        focus_log: &mut std::fs::File,
        frame_count: u64,
    ) {
        self.last_combat_truth = None;
        self.last_input = None;
        if let Some(stats) = self.combat_stats.take() {
            stats.write_summary(log, frame_count.saturating_sub(1));
            if stats.has_findings() {
                stats.write_summary(focus_log, frame_count.saturating_sub(1));
            }
        }
    }

    pub(super) fn flush_summary_on_game_over(
        &mut self,
        log: &mut std::fs::File,
        focus_log: &mut std::fs::File,
        frame_count: u64,
    ) {
        if let Some(stats) = self.combat_stats.take() {
            stats.write_summary(log, frame_count);
            if stats.has_findings() {
                stats.write_summary(focus_log, frame_count);
            }
        }
    }

    pub(super) fn ensure_combat_stats(&mut self, frame_count: u64) {
        if self.combat_stats.is_none() {
            self.combat_stats = Some(CombatStats::new(frame_count));
        }
    }

    pub(super) fn increment_action_count(&mut self) {
        if let Some(stats) = self.combat_stats.as_mut() {
            stats.action_count += 1;
        }
    }

    pub(super) fn record_search_timing(&mut self, search_elapsed_ms: u128, diag_render_ms: u128) {
        if let Some(stats) = self.combat_stats.as_mut() {
            stats.search_decisions += 1;
            stats.search_elapsed_total_ms += search_elapsed_ms;
            stats.search_elapsed_max_ms = stats.search_elapsed_max_ms.max(search_elapsed_ms);
            stats.diag_render_total_ms += diag_render_ms;
            stats.diag_render_max_ms = stats.diag_render_max_ms.max(diag_render_ms);
        }
    }

    pub(super) fn record_action_diffs(
        &mut self,
        action_diffs: &[DiffResult],
        frame_count: u64,
        log: &mut std::fs::File,
        focus_log: &mut std::fs::File,
        response_id: Option<i64>,
        state_frame_id: Option<i64>,
        predicted: &CombatState,
        java_snapshot: &Value,
        engine_bug_summary_interval: usize,
        engine_bug_total: &mut usize,
        content_gap_total: &mut usize,
    ) {
        let bugs: Vec<_> = action_diffs
            .iter()
            .filter(|d| d.category == DiffCategory::EngineBug)
            .collect();
        let gaps: Vec<_> = action_diffs
            .iter()
            .filter(|d| d.category == DiffCategory::ContentGap)
            .collect();
        let timing: Vec<_> = action_diffs
            .iter()
            .filter(|d| d.category == DiffCategory::Timing)
            .collect();

        *engine_bug_total += bugs.len();
        *content_gap_total += gaps.len();

        let parity_fail_line = format!(
            "  >>> PARITY FAIL ({} diffs: {} bugs, {} gaps, {} timing) <<<",
            action_diffs.len(),
            bugs.len(),
            gaps.len(),
            timing.len()
        );
        let cause_line = format!("  CAUSED BY: {}", self.action_context.describe());
        writeln!(log, "{}", parity_fail_line).unwrap();
        writeln!(log, "{}", cause_line).unwrap();
        writeln!(
            focus_log,
            "\n[F{}] PARITY FAIL response_id={:?} state_frame_id={:?}",
            frame_count, response_id, state_frame_id
        )
        .unwrap();
        writeln!(focus_log, "{}", parity_fail_line.trim_start()).unwrap();
        writeln!(focus_log, "{}", cause_line.trim_start()).unwrap();
        write_failure_context(focus_log, predicted, java_snapshot);
        write_failure_context(log, predicted, java_snapshot);

        let stats = self.combat_stats.as_mut().unwrap();
        for d in action_diffs {
            let is_repeat_gap = d.category == DiffCategory::ContentGap
                && stats.seen_content_gaps.contains(&d.field);

            if !is_repeat_gap {
                let diff_line = format!(
                    "    {} : Rust={}, Java={}  [{}]",
                    d.field, d.rust_val, d.java_val, d.category
                );
                writeln!(log, "{}", diff_line).unwrap();
                writeln!(focus_log, "{}", diff_line).unwrap();
            }

            if d.category == DiffCategory::ContentGap {
                stats.seen_content_gaps.insert(d.field.clone());
            }

            stats.diffs.push(CombatDiffRecord {
                _frame: frame_count,
                field: d.field.clone(),
                category: d.category,
                rust_val: d.rust_val.clone(),
                java_val: d.java_val.clone(),
            });
        }

        if *engine_bug_total > 0 && *engine_bug_total % engine_bug_summary_interval == 0 {
            writeln!(
                log,
                "  [SAMPLING] {} engine bugs observed so far; continuing collection.",
                *engine_bug_total
            )
            .unwrap();
            writeln!(
                focus_log,
                "  [SAMPLING] {} engine bugs observed so far; continuing collection.",
                *engine_bug_total
            )
            .unwrap();
        }

        writeln!(log, "  [HEALED] Prediction chain reset from Java truth").unwrap();
        writeln!(
            focus_log,
            "  [HEALED] Prediction chain reset from Java truth"
        )
        .unwrap();
    }
}

fn is_hexaghost_monster_type(monster_type: usize) -> bool {
    monster_type == crate::content::monsters::EnemyId::Hexaghost as usize
}

fn format_move_history(history: &std::collections::VecDeque<u8>) -> String {
    history
        .iter()
        .map(|b| b.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn monster_display_name(monster_type: usize) -> String {
    EnemyId::from_id(monster_type)
        .map(|enemy| enemy.get_name().to_string())
        .unwrap_or_else(|| format!("type_{}", monster_type))
}

fn summarize_names(names: &[String]) -> String {
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    let mut order = Vec::new();
    for name in names {
        if !counts.contains_key(name) {
            order.push(name.clone());
        }
        *counts.entry(name.clone()).or_insert(0) += 1;
    }
    order
        .into_iter()
        .map(|name| match counts.get(&name).copied().unwrap_or(0) {
            0 | 1 => name,
            count => format!("{} x{}", name, count),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_java_powers(monster: &Value) -> String {
    monster["powers"]
        .as_array()
        .map_or(String::new(), |powers| {
            powers
                .iter()
                .map(|power| {
                    format!(
                        "{}={}",
                        power["name"]
                            .as_str()
                            .or_else(|| power["id"].as_str())
                            .unwrap_or("?"),
                        power["amount"].as_i64().unwrap_or(0)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
}

fn format_rust_monster_line(
    cs: &CombatState,
    index: usize,
    monster: &crate::runtime::combat::MonsterEntity,
) -> String {
    let mut flags = Vec::new();
    if monster.is_dying || monster.is_escaped {
        flags.push("dead");
    }
    if monster.half_dead {
        flags.push("half_dead");
    }
    let powers = format_powers(cs, monster.id);
    let identity = format!(
        " inst={:?} spawn={:?} draw_x={:?}",
        monster.protocol_identity.instance_id,
        monster.protocol_identity.spawn_order,
        monster.protocol_identity.draw_x
    );
    format!(
        "    Rust M[{}] {} id={} hp={}/{} blk={} intent={:?}{}{}{}",
        index,
        monster_display_name(monster.monster_type),
        monster.id,
        monster.current_hp,
        monster.max_hp,
        monster.block,
        monster.current_intent,
        if powers.is_empty() {
            String::new()
        } else {
            format!(" pw=[{}]", powers)
        },
        identity,
        if flags.is_empty() {
            String::new()
        } else {
            format!(" flags={}", flags.join("|"))
        }
    )
}

fn format_java_monster_line(index: usize, monster: &Value) -> String {
    let mut flags = Vec::new();
    if monster["is_gone"].as_bool().unwrap_or(false)
        || monster["is_dying"].as_bool().unwrap_or(false)
    {
        flags.push("dead");
    }
    if monster["half_dead"].as_bool().unwrap_or(false) {
        flags.push("half_dead");
    }
    if monster["is_escaping"].as_bool().unwrap_or(false) {
        flags.push("escaping");
    }
    let powers = format_java_powers(monster);
    format!(
        "    Java M[{}] {} id={} hp={}/{} blk={} intent={} move_id={} inst={:?} spawn={:?} draw_x={:?}{}{}",
        index,
        monster["name"]
            .as_str()
            .or_else(|| monster["id"].as_str())
            .unwrap_or("?"),
        monster["id"].as_str().unwrap_or("?"),
        monster["current_hp"].as_i64().unwrap_or(-1),
        monster["max_hp"].as_i64().unwrap_or(-1),
        monster["block"].as_i64().unwrap_or(-1),
        monster["intent"].as_str().unwrap_or("?"),
        monster["move_id"].as_i64().unwrap_or(-1),
        monster["monster_instance_id"].as_u64(),
        monster["spawn_order"].as_u64(),
        monster
            .get("draw_x")
            .and_then(|v| v.as_i64())
            .or_else(|| monster.get("draw_x").and_then(|v| v.as_f64().map(|x| x.round() as i64))),
        if powers.is_empty() {
            String::new()
        } else {
            format!(" pw=[{}]", powers)
        },
        if flags.is_empty() {
            String::new()
        } else {
            format!(" flags={}", flags.join("|"))
        }
    )
}

fn write_failure_context<W: Write>(log: &mut W, predicted: &CombatState, java_snapshot: &Value) {
    let rust_names: Vec<String> = predicted
        .entities
        .monsters
        .iter()
        .map(|monster| monster_display_name(monster.monster_type))
        .collect();
    let java_names: Vec<String> = java_snapshot["monsters"]
        .as_array()
        .map_or(Vec::new(), |arr| {
            arr.iter()
                .map(|monster| {
                    monster["name"]
                        .as_str()
                        .or_else(|| monster["id"].as_str())
                        .unwrap_or("?")
                        .to_string()
                })
                .collect()
        });

    writeln!(
        log,
        "  [ENCOUNTER] rust={} | java={}",
        summarize_names(&rust_names),
        summarize_names(&java_names)
    )
    .unwrap();
    writeln!(
        log,
        "  [PILES] rust hand/draw/disc/exhaust={}/{}/{}/{} | java hand/draw/disc/exhaust={}/{}/{}/{}",
        predicted.zones.hand.len(),
        predicted.zones.draw_pile.len(),
        predicted.zones.discard_pile.len(),
        predicted.zones.exhaust_pile.len(),
        java_snapshot["hand"].as_array().map_or(0, |arr| arr.len()),
        java_snapshot["draw_pile"].as_array().map_or(0, |arr| arr.len()),
        java_snapshot["discard_pile"].as_array().map_or(0, |arr| arr.len()),
        java_snapshot["exhaust_pile"].as_array().map_or(0, |arr| arr.len()),
    )
    .unwrap();
    writeln!(log, "  [RUST PREDICTED]").unwrap();
    for (index, monster) in predicted.entities.monsters.iter().enumerate() {
        writeln!(
            log,
            "{}",
            format_rust_monster_line(predicted, index, monster)
        )
        .unwrap();
    }
    writeln!(log, "  [JAVA TRUTH]").unwrap();
    if let Some(monsters) = java_snapshot["monsters"].as_array() {
        for (index, monster) in monsters.iter().enumerate() {
            writeln!(log, "{}", format_java_monster_line(index, monster)).unwrap();
        }
    }
}

fn log_hexaghost_end_turn_debug(log: &mut std::fs::File, expected_cs: &CombatState, cv: &Value) {
    let rust_hex = expected_cs
        .entities
        .monsters
        .iter()
        .find(|m| is_hexaghost_monster_type(m.monster_type));
    let java_hex = cv
        .get("monsters")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter().find(|m| {
                m.get("id")
                    .and_then(|v| v.as_str())
                    .is_some_and(|id| id.eq_ignore_ascii_case("Hexaghost"))
            })
        });

    if rust_hex.is_none() && java_hex.is_none() {
        return;
    }

    writeln!(log, "  [HEXAGHOST END DEBUG]").unwrap();
    if let Some(rust_hex) = rust_hex {
        writeln!(
            log,
            "    rust_post_end: hp={}/{} blk={} next_move_byte={} intent={:?} move_history=[{}] intent_dmg={}",
            rust_hex.current_hp,
            rust_hex.max_hp,
            rust_hex.block,
            rust_hex.next_move_byte,
            rust_hex.current_intent,
            format_move_history(&rust_hex.move_history),
            rust_hex.intent_dmg
        )
        .unwrap();
    }
    if let Some(java_hex) = java_hex {
        writeln!(
            log,
            "    java_post_end: hp={}/{} blk={} move_id={} intent={} base_dmg={} adj_dmg={} hits={}",
            java_hex.get("current_hp").and_then(|v| v.as_i64()).unwrap_or(-1),
            java_hex.get("max_hp").and_then(|v| v.as_i64()).unwrap_or(-1),
            java_hex.get("block").and_then(|v| v.as_i64()).unwrap_or(-1),
            java_hex.get("move_id").and_then(|v| v.as_i64()).unwrap_or(-1),
            java_hex.get("intent").and_then(|v| v.as_str()).unwrap_or("?"),
            java_hex
                .get("move_base_damage")
                .and_then(|v| v.as_i64())
                .unwrap_or(-1),
            java_hex
                .get("move_adjusted_damage")
                .and_then(|v| v.as_i64())
                .unwrap_or(-1),
            java_hex.get("move_hits").and_then(|v| v.as_i64()).unwrap_or(-1)
        )
        .unwrap();
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_live_combat_frame<W: Write>(
    frame: &LiveFrame,
    gs: &Value,
    frame_count: u64,
    parity_mode: LiveParityMode,
    combat_search_budget: u32,
    last_sent_cmd: &mut String,
    cmd_failed_count: &mut u32,
    engine_bug_total: &mut usize,
    content_gap_total: &mut usize,
    coverage_db: &mut CoverageDb,
    combat_runtime: &mut CombatRuntime,
    live_io: &mut LiveCommIo,
    stdout: &mut W,
    engine_bug_summary_interval: usize,
    signature_source_file: &str,
) -> CombatFrameOutcome {
    let cv = frame
        .combat_state()
        .expect("combat branch requires combat_state");
    let rv = frame.relics();
    let combat_snapshot = build_live_combat_snapshot(gs);
    let truth = crate::diff::state_sync::build_combat_state(&combat_snapshot, rv);

    if let (Some(prev_truth), Some(prev_input)) = (
        &combat_runtime.last_combat_truth,
        &combat_runtime.last_input,
    ) {
        let after_engine = EngineState::CombatPlayerTurn;
        let signature = crate::bot::coverage_signatures::signature_from_transition_with_archetypes(
            &EngineState::CombatPlayerTurn,
            prev_truth,
            prev_input,
            &after_engine,
            &truth,
            crate::bot::coverage::archetype_tags_for_combat(prev_truth),
        );
        let signature_key = signature.canonical_key();
        let is_novel = !coverage_db.tested_signatures.contains(&signature_key);
        let novel_archetypes: Vec<String> = signature
            .archetype_tags
            .iter()
            .filter(|tag| !coverage_db.tested_archetypes.contains(*tag))
            .cloned()
            .collect();
        coverage_db.record_signature(&signature);
        coverage_db.save();
        let record = crate::bot::coverage_signatures::ObservedInteractionRecord {
            observed_from: "live_comm".to_string(),
            source_file: signature_source_file.to_string(),
            combat_idx: None,
            action_idx: Some(frame_count as usize),
            command: crate::bot::coverage_signatures::command_string(prev_input),
            signature_key,
            source_combo_key: signature.source_combo_key(),
            signature,
        };
        writeln!(
            live_io.signature_log,
            "{}",
            serde_json::to_string(&record).unwrap_or_else(|_| "{}".to_string())
        )
        .unwrap();
        if is_novel {
            writeln!(live_io.log, "  [NOVEL SIGNATURE] {}", record.signature_key).unwrap();
        }
        if !novel_archetypes.is_empty() {
            writeln!(
                live_io.log,
                "  [NOVEL ARCHETYPE] {} via {}",
                novel_archetypes.join(", "),
                record.signature.source_id
            )
            .unwrap();
        }
        if !record.signature.archetype_tags.is_empty() {
            writeln!(
                live_io.log,
                "  [ARCHETYPES] {}",
                record.signature.archetype_tags.join(", ")
            )
            .unwrap();
        }
    }

    combat_runtime.ensure_combat_stats(frame_count);
    log_combat_overview(&mut live_io.log, frame_count, &truth);

    let sync_diffs = validate_parse(&truth, cv);
    if !sync_diffs.is_empty() {
        writeln!(live_io.log, "  >>> PARSE DIFF ({}) <<<", sync_diffs.len()).unwrap();
        writeln!(
            live_io.focus_log,
            "\n[F{}] PARSE DIFF ({}) response_id={:?} state_frame_id={:?}",
            frame_count,
            sync_diffs.len(),
            frame.response_id(),
            frame.state_frame_id()
        )
        .unwrap();
        write_failure_context(&mut live_io.focus_log, &truth, cv);
        for d in &sync_diffs {
            writeln!(live_io.log, "    {}", d).unwrap();
            writeln!(live_io.focus_log, "    {}", d).unwrap();
        }
        let _ = write_failure_snapshot(
            live_io,
            frame_count,
            frame,
            "validation_failure",
            vec![
                "combat_parse_diff".to_string(),
                format!("count={}", sync_diffs.len()),
            ],
            serde_json::json!({
                "chosen_command": last_sent_cmd,
                "diffs": sync_diffs,
            }),
        );
        if should_stop_for_combat_mismatch(parity_mode, true, false) {
            writeln!(
                live_io.log,
                "  [STRICT] stopping on first combat parse diff"
            )
            .unwrap();
            writeln!(
                live_io.focus_log,
                "  [STRICT] stopping on first combat parse diff"
            )
            .unwrap();
            return CombatFrameOutcome::StopForParityFailure;
        }
    }

    let mut saw_action_diff = false;
    if let Some(expected_cs) = combat_runtime.expected_combat_state.take() {
        let action_diffs = crate::diff::replay::comparator::compare_states(
            &expected_cs,
            cv,
            combat_runtime.action_context.was_end_turn,
            &combat_runtime.action_context,
        );

        if !action_diffs.is_empty() {
            if combat_runtime.action_context.was_end_turn
                && (expected_cs
                    .entities
                    .monsters
                    .iter()
                    .any(|m| is_hexaghost_monster_type(m.monster_type))
                    || cv
                        .get("monsters")
                        .and_then(|v| v.as_array())
                        .is_some_and(|arr| {
                            arr.iter().any(|m| {
                                m.get("id")
                                    .and_then(|v| v.as_str())
                                    .is_some_and(|id| id.eq_ignore_ascii_case("Hexaghost"))
                            })
                        }))
            {
                log_hexaghost_end_turn_debug(&mut live_io.log, &expected_cs, cv);
            }
            combat_runtime.record_action_diffs(
                &action_diffs,
                frame_count,
                &mut live_io.log,
                &mut live_io.focus_log,
                frame.response_id(),
                frame.state_frame_id(),
                &expected_cs,
                cv,
                engine_bug_summary_interval,
                engine_bug_total,
                content_gap_total,
            );
            let _ = write_failure_snapshot(
                live_io,
                frame_count,
                frame,
                "engine_bug",
                vec![
                    "combat_action_diff".to_string(),
                    format!("count={}", action_diffs.len()),
                ],
                serde_json::json!({
                    "chosen_command": last_sent_cmd,
                    "diffs": action_diffs.iter().map(|diff| {
                        format!(
                            "{} [{}] Rust={} Java={}",
                            diff.field, diff.category, diff.rust_val, diff.java_val
                        )
                    }).collect::<Vec<_>>(),
                }),
            );
            saw_action_diff = true;
        } else {
            writeln!(live_io.log, "  >>> PARITY OK <<<").unwrap();
        }
    }
    if should_stop_for_combat_mismatch(parity_mode, false, saw_action_diff) {
        writeln!(
            live_io.log,
            "  [STRICT] stopping on first combat parity failure"
        )
        .unwrap();
        writeln!(
            live_io.focus_log,
            "  [STRICT] stopping on first combat parity failure"
        )
        .unwrap();
        return CombatFrameOutcome::StopForParityFailure;
    }

    log_hidden_intent_belief(live_io, &truth);

    let heuristic_diag = crate::bot::combat_heuristic::diagnose_decision(&truth);
    let mut search_diag = crate::bot::search::diagnose_root_search(
        &EngineState::CombatPlayerTurn,
        &truth,
        coverage_db,
        crate::bot::coverage::CoverageMode::Off,
        None,
        combat_search_budget,
    );
    let verbose_outcome_logging = verbose_search_outcome_logging_enabled();
    let render_started = std::time::Instant::now();
    writeln!(
        live_io.log,
        "  [SEARCH DIAG] budget={} depth_limit={} max_depth={} root_width={} branch_width={} max_engine_steps={} elapsed_ms={} legal_moves={} reduced_legal_moves={} equivalence_mode={} simulations={} chosen={}",
        combat_search_budget,
        search_diag.depth_limit,
        search_diag.max_decision_depth,
        search_diag.root_width,
        search_diag.branch_width,
        search_diag.max_engine_steps,
        search_diag.elapsed_ms,
        search_diag.legal_moves,
        search_diag.reduced_legal_moves,
        search_diag.equivalence_mode.as_str(),
        search_diag.simulations,
        describe_client_input(&truth, &search_diag.chosen_move)
    )
    .unwrap();
    for move_stat in search_diag.top_moves.iter().take(SEARCH_DIAG_TOP_K) {
        writeln!(
            live_io.log,
            "  [SEARCH DIAG] {}",
            format_search_move_diag(&truth, move_stat)
        )
        .unwrap();
        if verbose_outcome_logging {
            writeln!(
                live_io.log,
                "  [SEARCH DIAG] {}",
                summarize_cached_candidate_outcome(&truth, move_stat)
            )
            .unwrap();
        }
    }
    let render_elapsed_ms = render_started.elapsed().as_millis();
    search_diag.profile.root_diag_render_ms = render_elapsed_ms;
    combat_runtime.record_search_timing(search_diag.elapsed_ms, render_elapsed_ms);
    writeln!(
        live_io.log,
        "  [SEARCH PROFILE] {}",
        format_search_profile_summary(&search_diag)
    )
    .unwrap();
    maybe_record_search_suspect(
        live_io,
        frame_count,
        frame,
        &truth,
        &heuristic_diag,
        &search_diag,
    );
    if live_io.log.metadata().is_ok() {
        let top_candidates = search_diag
            .top_moves
            .iter()
            .take(SEARCH_DIAG_TOP_K)
            .map(|stat| combat_top_candidate_record(&truth, stat))
            .collect::<Vec<_>>();
        let shadow = crate::bot::sidecar::combat_shadow_json(
            frame_count,
            "live_comm_combat",
            describe_client_input(&truth, &search_diag.chosen_move),
            &search_diag,
            top_candidates,
            None,
            None,
        );
        crate::bot::sidecar::write_shadow_record(&mut live_io.sidecar_shadow, &shadow);
    }
    if !same_client_input(&heuristic_diag.chosen_move, &search_diag.chosen_move) {
        writeln!(
            live_io.log,
            "  [HEURISTIC DIAG] disagrees chosen={:?} baseline_score={}",
            heuristic_diag.chosen_move, heuristic_diag.baseline_score
        )
        .unwrap();
        for move_stat in heuristic_diag.top_moves.iter().take(3) {
            writeln!(
                live_io.log,
                "  [HEURISTIC DIAG] move={:?} score={} priority={}",
                move_stat.input, move_stat.score, move_stat.priority
            )
            .unwrap();
        }
    }
    let input = search_diag.chosen_move.clone();

    if matches!(input, crate::state::core::ClientInput::UsePotion { .. }) {
        log_potion_decision_trace(live_io, &truth);
    }

    writeln!(live_io.log, "  → {:?}", input).unwrap();
    if matches!(input, crate::state::core::ClientInput::EndTurn) {
        let end_diag_lines = crate::bot::combat_heuristic::describe_end_turn_options(&truth);
        let has_non_end_legal_play = end_diag_lines
            .first()
            .is_some_and(|line| line.contains("legal_plays="));
        for line in end_diag_lines {
            writeln!(live_io.log, "  [END DIAG] {}", line).unwrap();
        }
        if has_non_end_legal_play && search_diag.legal_moves > 1 {
            writeln!(
                live_io.log,
                "  [END DIAG] search_kept_end_turn chosen={:?}",
                search_diag.chosen_move
            )
            .unwrap();
        }
    }

    combat_runtime.increment_action_count();

    let mut engine_state = EngineState::CombatPlayerTurn;
    if let Some(cmd) = comm_mod::input_to_java_command(&input, &engine_state) {
        if cmd == *last_sent_cmd && *cmd_failed_count > 0 {
            writeln!(
                live_io.log,
                "  [!] AVOIDING REPEATED ERROR BY FORCING END TURN"
            )
            .unwrap();
            live_io.send_line(stdout, "END");
            *last_sent_cmd = "END".to_string();
        } else {
            writeln!(live_io.log, "  SEND: {}", cmd).unwrap();
            live_io.send_line(stdout, &cmd);
            *last_sent_cmd = cmd.clone();

            let is_end_turn = matches!(input, crate::state::core::ClientInput::EndTurn);
            combat_runtime.action_context = ActionContext {
                last_command: cmd,
                was_end_turn: is_end_turn,
                monster_intents: truth
                    .entities
                    .monsters
                    .iter()
                    .map(|m| format!("{:?}", m.current_intent))
                    .collect(),
                monster_names: truth
                    .entities
                    .monsters
                    .iter()
                    .map(|m| monster_display_name(m.monster_type))
                    .collect(),
                has_rng_state: cv.get("rng_state").is_some(),
            };

            let mut local_cs = truth.clone();
            crate::engine::core::tick_until_stable_turn(
                &mut engine_state,
                &mut local_cs,
                input.clone(),
            );
            combat_runtime.expected_combat_state = Some(local_cs);
        }
        *cmd_failed_count = 0;
        combat_runtime.last_combat_truth = Some(truth.clone());
        combat_runtime.last_input = Some(input.clone());
    } else {
        writeln!(live_io.log, "  SEND: END (fallback)").unwrap();
        live_io.send_line(stdout, "END");
        *last_sent_cmd = "END".to_string();
        *cmd_failed_count = 0;
    }

    CombatFrameOutcome::Continue
}

pub(super) fn build_live_combat_snapshot(gs: &Value) -> Value {
    build_protocol_live_combat_snapshot(gs)
}

pub(super) fn log_combat_overview(log: &mut std::fs::File, frame_count: u64, truth: &CombatState) {
    writeln!(
        log,
        "\n[F{}] COMBAT  HP={}/{}  E={}  Hand={}  Draw={}  Disc={}  Monsters={}",
        frame_count,
        truth.entities.player.current_hp,
        truth.entities.player.max_hp,
        truth.turn.energy,
        truth.zones.hand.len(),
        truth.zones.draw_pile.len(),
        truth.zones.discard_pile.len(),
        truth.entities.monsters.len()
    )
    .unwrap();

    for (i, m) in truth.entities.monsters.iter().enumerate() {
        let powers = format_powers(truth, m.id);
        let dead_str = if m.is_dying || m.is_escaped {
            " (DEAD)"
        } else {
            ""
        };
        writeln!(
            log,
            "  M[{}] id={} {} hp={}/{} blk={} intent={:?}{}{}{}",
            i,
            m.id,
            monster_display_name(m.monster_type),
            m.current_hp,
            m.max_hp,
            m.block,
            m.current_intent,
            if powers.is_empty() {
                String::new()
            } else {
                format!(" pw=[{}]", powers)
            },
            if m.half_dead { " half_dead" } else { "" },
            dead_str
        )
        .unwrap();
    }

    let hand_str: Vec<String> = truth
        .zones
        .hand
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let def = crate::content::cards::get_card_definition(c.id);
            let u = if c.upgrades > 0 { "+" } else { "" };
            format!("{}:{}{}", i, def.name, u)
        })
        .collect();
    writeln!(log, "  Hand: [{}]", hand_str.join(", ")).unwrap();

    let pp = format_powers(truth, 0);
    if !pp.is_empty() {
        writeln!(log, "  Player pw: [{}]", pp).unwrap();
    }
}

fn format_powers(cs: &CombatState, entity_id: usize) -> String {
    cs.entities
        .power_db
        .get(&entity_id)
        .map_or(String::new(), |powers| {
            powers
                .iter()
                .map(|p| {
                    let def = crate::content::powers::get_power_definition(p.power_type);
                    format!("{}={}", def.name, p.amount)
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
}

pub(super) fn validate_parse(cs: &CombatState, cv: &Value) -> Vec<String> {
    let mut diffs = Vec::new();
    let jp = &cv["player"];

    let j_energy = jp["energy"].as_u64().unwrap_or(0) as u8;
    if cs.turn.energy != j_energy {
        diffs.push(format!("energy: rust={} java={}", cs.turn.energy, j_energy));
    }

    let j_hp = jp["current_hp"].as_i64().unwrap_or(0) as i32;
    if cs.entities.player.current_hp != j_hp {
        diffs.push(format!(
            "player.hp: rust={} java={}",
            cs.entities.player.current_hp, j_hp
        ));
    }

    let j_block = jp["block"].as_i64().unwrap_or(0) as i32;
    if cs.entities.player.block != j_block {
        diffs.push(format!(
            "player.block: rust={} java={}",
            cs.entities.player.block, j_block
        ));
    }

    if let Some(j_hand) = cv["hand"].as_array() {
        if cs.zones.hand.len() != j_hand.len() {
            diffs.push(format!(
                "hand_size: rust={} java={}",
                cs.zones.hand.len(),
                j_hand.len()
            ));
        }
        for (i, jc) in j_hand.iter().enumerate() {
            let jid = jc["id"].as_str().unwrap_or("?");
            if card_id_from_java(jid).is_none() {
                let name = jc["name"].as_str().unwrap_or("?");
                diffs.push(format!(
                    "hand[{}]: UNMAPPED card java_id='{}' name='{}'",
                    i, jid, name
                ));
            }
        }
    }

    if let Some(j_draw) = cv["draw_pile"].as_array() {
        for jc in j_draw {
            let jid = jc["id"].as_str().unwrap_or("?");
            if card_id_from_java(jid).is_none() {
                let name = jc["name"].as_str().unwrap_or("?");
                diffs.push(format!(
                    "draw_pile: UNMAPPED card java_id='{}' name='{}'",
                    jid, name
                ));
            }
        }
    }

    if let Some(j_monsters) = cv["monsters"].as_array() {
        let aligned_indices = align_rust_monsters_for_parse(cs, j_monsters);
        if cs.entities.monsters.len() != j_monsters.len() {
            diffs.push(format!(
                "monster_count: rust={} java={}",
                cs.entities.monsters.len(),
                j_monsters.len()
            ));
        }
        for (i, jm) in j_monsters.iter().enumerate() {
            let Some(rust_idx) = aligned_indices.get(i).and_then(|idx| *idx) else {
                diffs.push(format!("monster[{}]: MISSING in Rust", i));
                continue;
            };
            let rm = &cs.entities.monsters[rust_idx];
            let j_mhp = jm["current_hp"].as_i64().unwrap_or(0) as i32;
            if rm.current_hp != j_mhp {
                diffs.push(format!(
                    "monster[{}].hp: rust={} java={}",
                    i, rm.current_hp, j_mhp
                ));
            }
            if let Some(j_powers) = jm["powers"].as_array() {
                for jp in j_powers {
                    let pid = jp["id"].as_str().unwrap_or("?");
                    if power_id_from_java(pid).is_none() {
                        diffs.push(format!(
                            "monster[{}].power: UNMAPPED '{}' amount={}",
                            i,
                            pid,
                            jp["amount"].as_i64().unwrap_or(0)
                        ));
                    }
                }
            }
        }
    }

    if let Some(j_powers) = jp["powers"].as_array() {
        for jp_item in j_powers {
            let pid = jp_item["id"].as_str().unwrap_or("?");
            if power_id_from_java(pid).is_none() {
                diffs.push(format!(
                    "player.power: UNMAPPED '{}' amount={}",
                    pid,
                    jp_item["amount"].as_i64().unwrap_or(0)
                ));
            }
        }
    }

    diffs
}

fn java_monster_instance_id_for_parse(monster: &Value) -> Option<u64> {
    monster.get("monster_instance_id").and_then(|v| v.as_u64())
}

fn java_monster_draw_x_for_parse(monster: &Value) -> Option<i32> {
    monster
        .get("draw_x")
        .and_then(|v| v.as_i64().map(|value| value as i32))
        .or_else(|| {
            monster
                .get("draw_x")
                .and_then(|v| v.as_f64().map(|value| value.round() as i32))
        })
}

fn align_rust_monsters_for_parse(cs: &CombatState, java_ms: &[Value]) -> Vec<Option<usize>> {
    let mut used = std::collections::HashSet::new();
    let mut aligned = Vec::with_capacity(java_ms.len());

    for (java_index, java_monster) in java_ms.iter().enumerate() {
        let matched = java_monster_instance_id_for_parse(java_monster)
            .and_then(|instance_id| {
                cs.entities
                    .monsters
                    .iter()
                    .enumerate()
                    .find(|(idx, monster)| {
                        !used.contains(idx)
                            && monster.protocol_identity.instance_id == Some(instance_id)
                    })
                    .map(|(idx, _)| idx)
            })
            .or_else(|| {
                let java_type = monster_id_from_java(java_monster["id"].as_str().unwrap_or(""));
                let java_draw_x = java_monster_draw_x_for_parse(java_monster);
                cs.entities
                    .monsters
                    .iter()
                    .enumerate()
                    .find(|(idx, monster)| {
                        !used.contains(idx)
                            && Some(monster.monster_type) == java_type.map(|id| id as usize)
                            && monster.protocol_identity.draw_x == java_draw_x
                    })
                    .map(|(idx, _)| idx)
            })
            .or_else(|| {
                (java_index < cs.entities.monsters.len() && !used.contains(&java_index))
                    .then_some(java_index)
            });

        if let Some(idx) = matched {
            used.insert(idx);
        }
        aligned.push(matched);
    }

    aligned
}
