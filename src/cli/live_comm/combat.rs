use super::frame::LiveFrame;
use super::io::LiveCommIo;
use super::snapshot::write_failure_snapshot;
use super::{LiveCombatMode, LiveExactTurnMode, LiveParityMode};
use crate::bot::combat::legal_moves::protocol_root_moves;
use crate::bot::combat::monster_belief::build_combat_belief_state;
use crate::bot::combat::pressure::StatePressureFeatures;
use crate::bot::combat::{
    branch_family_for_card, describe_end_turn_options,
    diagnose_root_search_with_depth_and_runtime_and_root_inputs,
    diagnose_root_search_with_runtime_and_root_inputs, BranchFamily, CombatDiagnostics,
    CombatMoveStat, SearchExactTurnMode, SearchExperimentFlags, SearchRuntimeBudget,
};
use crate::bot::infra::comm as comm_mod;
use crate::bot::infra::coverage_signatures::{
    command_string, signature_from_transition_with_archetypes, ObservedInteractionRecord,
};
use crate::bot::infra::sidecar::{self, CombatTopCandidateRecord};
use crate::bot::CoverageDb;
use crate::content::monsters::{resolve_monster_turn_plan, EnemyId};
use crate::protocol::java::{
    build_combat_affordance_snapshot,
    build_live_observation_snapshot as build_protocol_live_observation_snapshot,
    build_live_truth_snapshot as build_protocol_live_truth_snapshot, card_id_from_java,
    monster_id_from_java, power_id_from_java,
};
use crate::runtime::combat::CombatState;
use crate::state::core::{ClientInput, EngineState};
use crate::verification::combat::{
    build_combat_state_from_snapshots, compare_combat_states_from_snapshots, ActionContext,
    DiffCategory, DiffResult,
};
use serde_json::Value;
use std::io::Write;
use std::time::{Duration, Instant};

const SEARCH_DIAG_TOP_K: usize = 5;
const LIVE_BASELINE_SEARCH_TIMEOUT_MS: u64 = 250;
const LIVE_ROOT_SEARCH_TIMEOUT_MS: u64 = 2_500;
const LIVE_ROOT_EXACT_TURN_MAX_NODES: usize = 1_200;
const LIVE_SAMPLED_AUDIT_INTERVAL: u64 = 8;

fn log_combat_stage_enter(
    live_io: &mut LiveCommIo,
    frame_count: u64,
    stage: &'static str,
    detail: impl AsRef<str>,
) -> Instant {
    let detail = detail.as_ref();
    writeln!(live_io.log, "  [STAGE] enter {stage} {detail}").unwrap();
    writeln!(
        live_io.focus_log,
        "[STAGE] frame={frame_count} enter {stage} {detail}"
    )
    .unwrap();
    let _ = live_io.log.flush();
    let _ = live_io.focus_log.flush();
    Instant::now()
}

fn log_combat_stage_exit(
    live_io: &mut LiveCommIo,
    frame_count: u64,
    stage: &'static str,
    started: Instant,
    detail: impl AsRef<str>,
) {
    let elapsed_ms = started.elapsed().as_millis();
    let detail = detail.as_ref();
    writeln!(
        live_io.log,
        "  [STAGE] exit {stage} elapsed_ms={elapsed_ms} {detail}"
    )
    .unwrap();
    if elapsed_ms >= 100 || matches!(stage, "baseline_search" | "root_search") {
        writeln!(
            live_io.focus_log,
            "[STAGE] frame={frame_count} exit {stage} elapsed_ms={elapsed_ms} {detail}"
        )
        .unwrap();
    }
    let _ = live_io.log.flush();
    let _ = live_io.focus_log.flush();
}

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
}

fn summarize_cached_candidate_outcome(combat: &CombatState, stat: &CombatMoveStat) -> String {
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

fn log_slow_search_summary(
    live_io: &mut LiveCommIo,
    frame_count: u64,
    combat: &CombatState,
    baseline_diag: Option<&CombatDiagnostics>,
    search_diag: &CombatDiagnostics,
) {
    let baseline_ms = baseline_diag.map(|diag| diag.elapsed_ms).unwrap_or(0);
    let root_ms = search_diag.elapsed_ms;
    if baseline_ms < 500 && root_ms < 500 {
        return;
    }

    let exact_turn_summary = decision_audit_exact_turn_summary(&search_diag.decision_audit)
        .unwrap_or_else(|| "none".to_string());
    let hand = combat
        .zones
        .hand
        .iter()
        .map(format_card)
        .collect::<Vec<_>>()
        .join(", ");
    let monsters = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && monster.current_hp > 0)
        .map(|monster| {
            format!(
                "{:?}:hp{}/{}:intent={:?}",
                monster.monster_type,
                monster.current_hp,
                monster.max_hp,
                monster.move_state.planned_visible_spec
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");

    writeln!(
        live_io.focus_log,
        "[SLOW SEARCH] frame={frame_count} baseline_ms={baseline_ms} root_ms={root_ms} legal_moves={} chosen={} hand=[{}] monsters=[{}] exact_turn={} audit={}",
        search_diag.legal_moves,
        describe_client_input(combat, &search_diag.chosen_move),
        hand,
        monsters,
        exact_turn_summary,
        if baseline_diag.is_some() { "sync" } else { "off" }
    )
    .unwrap();
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

fn root_node_budget_for_legacy_budget(legacy_budget: u32) -> usize {
    match legacy_budget {
        0..=300 => 24,
        301..=900 => 48,
        901..=2_000 => 72,
        _ => 96,
    }
}

fn engine_step_budget_for_legacy_budget(legacy_budget: u32) -> usize {
    match legacy_budget {
        0..=300 => 120,
        301..=900 => 160,
        901..=2_000 => 220,
        _ => 260,
    }
}

fn audit_node_budget_for_legacy_budget(legacy_budget: u32) -> usize {
    match legacy_budget {
        0..=300 => 8,
        301..=900 => 12,
        _ => 16,
    }
}

fn exact_turn_mode_for_live(mode: LiveExactTurnMode) -> SearchExactTurnMode {
    match mode {
        LiveExactTurnMode::Off => SearchExactTurnMode::Off,
        LiveExactTurnMode::Auto => SearchExactTurnMode::Auto,
        LiveExactTurnMode::Force => SearchExactTurnMode::Force,
    }
}

fn live_root_search_budget(
    legacy_budget: u32,
    exact_turn_mode: LiveExactTurnMode,
) -> SearchRuntimeBudget {
    SearchRuntimeBudget {
        wall_clock_deadline: Some(
            Instant::now() + Duration::from_millis(LIVE_ROOT_SEARCH_TIMEOUT_MS),
        ),
        root_node_budget: root_node_budget_for_legacy_budget(legacy_budget),
        engine_step_budget: engine_step_budget_for_legacy_budget(legacy_budget),
        exact_turn_node_budget: LIVE_ROOT_EXACT_TURN_MAX_NODES
            .min(root_node_budget_for_legacy_budget(legacy_budget) * 25),
        audit_budget: audit_node_budget_for_legacy_budget(legacy_budget),
        exact_turn_mode: exact_turn_mode_for_live(exact_turn_mode),
        experiment_flags: SearchExperimentFlags::default(),
    }
}

fn live_baseline_search_budget(legacy_budget: u32) -> SearchRuntimeBudget {
    SearchRuntimeBudget {
        wall_clock_deadline: Some(
            Instant::now() + Duration::from_millis(LIVE_BASELINE_SEARCH_TIMEOUT_MS),
        ),
        root_node_budget: audit_node_budget_for_legacy_budget(legacy_budget),
        engine_step_budget: engine_step_budget_for_legacy_budget(legacy_budget).min(120),
        exact_turn_node_budget: 0,
        audit_budget: audit_node_budget_for_legacy_budget(legacy_budget),
        exact_turn_mode: SearchExactTurnMode::Off,
        experiment_flags: SearchExperimentFlags::default(),
    }
}

fn should_run_sync_audit(
    mode: LiveCombatMode,
    frame_count: u64,
    search_diag: &CombatDiagnostics,
) -> bool {
    match mode {
        LiveCombatMode::ChooserOnly => false,
        LiveCombatMode::ChooserPlusSampledAudit => {
            frame_count % LIVE_SAMPLED_AUDIT_INTERVAL == 0 || search_diag.timed_out
        }
        LiveCombatMode::FullDebug => true,
    }
}

fn same_card_play_signature(
    left: &crate::runtime::combat::CombatCard,
    right: &crate::runtime::combat::CombatCard,
) -> bool {
    left.id == right.id
        && left.upgrades == right.upgrades
        && left.misc_value == right.misc_value
        && left.base_damage_override == right.base_damage_override
        && left.cost_modifier == right.cost_modifier
        && left.cost_for_turn == right.cost_for_turn
        && left.base_damage_mut == right.base_damage_mut
        && left.base_block_mut == right.base_block_mut
        && left.base_magic_num_mut == right.base_magic_num_mut
        && left.multi_damage == right.multi_damage
        && left.exhaust_override == right.exhaust_override
        && left.retain_override == right.retain_override
        && left.free_to_play_once == right.free_to_play_once
        && left.energy_on_use == right.energy_on_use
}

fn same_or_equivalent_client_input_with_state(
    hand: &[crate::runtime::combat::CombatCard],
    potions: &[Option<crate::content::potions::Potion>],
    left: &ClientInput,
    right: &ClientInput,
) -> bool {
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
        ) => {
            if left_target != right_target {
                return false;
            }
            match (hand.get(*left_card), hand.get(*right_card)) {
                (Some(left_card), Some(right_card)) => {
                    left_card == right_card || same_card_play_signature(left_card, right_card)
                }
                _ => left_card == right_card,
            }
        }
        (
            ClientInput::UsePotion {
                potion_index: left_potion,
                target: left_target,
            },
            ClientInput::UsePotion {
                potion_index: right_potion,
                target: right_target,
            },
        ) => {
            if left_target != right_target {
                return false;
            }
            match (potions.get(*left_potion), potions.get(*right_potion)) {
                (Some(Some(left_potion)), Some(Some(right_potion))) => {
                    left_potion == right_potion || left_potion.id == right_potion.id
                }
                _ => left_potion == right_potion,
            }
        }
        (ClientInput::EndTurn, ClientInput::EndTurn)
        | (ClientInput::Proceed, ClientInput::Proceed)
        | (ClientInput::Cancel, ClientInput::Cancel) => true,
        _ => false,
    }
}

fn same_or_equivalent_client_input(
    combat: &CombatState,
    left: &ClientInput,
    right: &ClientInput,
) -> bool {
    same_or_equivalent_client_input_with_state(
        &combat.zones.hand,
        &combat.entities.potions,
        left,
        right,
    )
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

fn format_search_move_diag(combat: &CombatState, stat: &CombatMoveStat) -> String {
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
        "move={} visits={} avg_score={:.2} order={:.1} leaf={:.1} policy={:.1} sequence={:.1} frontload={:.1} defer={:.1} branch={:.1} downside={:.1} survival_window={:.1} exhaust_evidence={:.1} projected_hp={} projected_block={} projected_unblocked={} projected_enemy_total={} survives={} exhaust_block={} exhaust_draw={} branch_family={}",
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
        branch_family
    ) + &cluster_suffix
}

fn format_search_profile_summary(search_diag: &CombatDiagnostics) -> String {
    let profile = &search_diag.profile;
    format!(
        "search_ms={} chooser_ms={} audit_ms={} render_ms={} root(legal_ms={} reduce_ms={} reduce={}=>{} clones={} leaf_ms={} leaf_calls={} avg_branch={:.1}->{:.1}) recursive(legal_ms={} reduce_ms={} reduce={}=>{} clones={} leaf_ms={} leaf_calls={} avg_branch={:.1}->{:.1}) planner(ms={} calls={}) projection(ms={} calls={}) exact_turn(ms={} calls={}) engine_steps(ms={} calls={} steps={} p50={} p95={} max={}) cache(hits={} misses={}) timeout_source={} sequence_judge_ms={} nodes={} terminal_nodes={}",
        profile.search_total_ms,
        profile.chooser_ms,
        profile.audit_ms,
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
        profile.planner.elapsed_ms,
        profile.planner.calls,
        profile.turn_close_projection.elapsed_ms,
        profile.turn_close_projection.calls,
        profile.exact_turn.elapsed_ms,
        profile.exact_turn.calls,
        profile.advance_ms,
        profile.advance_calls,
        profile.advance_engine_steps,
        profile.advance_steps_p50,
        profile.advance_steps_p95,
        profile.advance_steps_max,
        profile.cache_hits,
        profile.cache_misses,
        profile.timeout_source.as_deref().unwrap_or("none"),
        profile.sequence_judge_ms,
        profile.nodes.nodes_expanded,
        profile.nodes.terminal_nodes
    )
}

fn log_combat_decision_audit_summary(live_io: &mut LiveCommIo, search_diag: &CombatDiagnostics) {
    let root_summary = decision_audit_root_summary(&search_diag.decision_audit);
    let tactical_summary = decision_audit_tactical_summary(&search_diag.decision_audit);
    let hand_select_summary = decision_audit_hand_select_summary(&search_diag.decision_audit);
    let exact_turn_summary = decision_audit_exact_turn_summary(&search_diag.decision_audit);

    if let Some(summary) = root_summary.as_deref() {
        writeln!(live_io.log, "  [AUDIT] root {}", summary).unwrap();
    }
    if let Some(summary) = tactical_summary.as_deref() {
        writeln!(live_io.log, "  [AUDIT] tactical {}", summary).unwrap();
    }
    if let Some(summary) = hand_select_summary.as_deref() {
        writeln!(live_io.log, "  [AUDIT] hand_select {}", summary).unwrap();
    }
    if let Some(summary) = exact_turn_summary.as_deref() {
        writeln!(live_io.log, "  [AUDIT] exact_turn {}", summary).unwrap();
    }

    let mut focus_parts = Vec::new();
    if let Some(summary) = root_summary {
        focus_parts.push(format!("root {}", summary));
    }
    if let Some(summary) = tactical_summary {
        focus_parts.push(format!("tactical {}", summary));
    }
    if let Some(summary) = hand_select_summary {
        focus_parts.push(format!("hand_select {}", summary));
    }
    if let Some(summary) = exact_turn_summary {
        focus_parts.push(format!("exact_turn {}", summary));
    }
    if !focus_parts.is_empty() {
        writeln!(live_io.focus_log, "[AUDIT] {}", focus_parts.join(" | ")).unwrap();
    }
}

fn decision_audit_root_summary(audit: &Value) -> Option<String> {
    let sequencing = audit.get("root_policy")?.get("sequencing")?;
    if sequencing.is_null() {
        return None;
    }

    let mut parts = Vec::new();
    if let Some(total_delta) = json_number_as_i64(sequencing.get("total_delta")) {
        parts.push(format!("delta={total_delta}"));
    }
    if let Some(rationale_key) = json_str(sequencing.get("rationale_key")) {
        parts.push(format!("rationale={rationale_key}"));
    }
    if let Some(branch_rationale_key) = json_str(sequencing.get("branch_rationale_key")) {
        parts.push(format!("branch={branch_rationale_key}"));
    }
    if let Some(downside_rationale_key) = json_str(sequencing.get("downside_rationale_key")) {
        parts.push(format!("downside={downside_rationale_key}"));
    }

    let branch_opening = audit.get("root_policy")?.get("branch_opening")?;
    if !branch_opening.is_null() {
        if let Some(branch_family) = json_str(branch_opening.get("branch_family")) {
            parts.push(format!("family={branch_family}"));
        }
        if let Some(continuation_value) =
            json_number_as_i64(branch_opening.get("continuation_value"))
        {
            if continuation_value != 0 {
                parts.push(format!("continue={continuation_value}"));
            }
        }
        if let Some(downside_value) = json_number_as_i64(branch_opening.get("downside_value")) {
            if downside_value != 0 {
                parts.push(format!("risk={downside_value}"));
            }
        }
    }

    (!parts.is_empty()).then(|| parts.join(" "))
}

fn decision_audit_tactical_summary(audit: &Value) -> Option<String> {
    let tactical = audit.get("tactical_bonus")?;
    let total = json_number_as_f64(tactical.get("total"))?;
    let components = tactical.get("components")?.as_array()?;

    let mut ranked = components
        .iter()
        .filter_map(|component| {
            let name = json_str(component.get("name"))?;
            let value = json_number_as_f64(component.get("value"))?;
            (value != 0.0).then_some((name.to_string(), value))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|lhs, rhs| {
        rhs.1
            .abs()
            .partial_cmp(&lhs.1.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let top_components = ranked
        .iter()
        .take(3)
        .map(|(name, value)| format!("{name}={value:.0}"))
        .collect::<Vec<_>>();

    if total == 0.0 && top_components.is_empty() {
        return None;
    }

    Some(if top_components.is_empty() {
        format!("total={total:.0}")
    } else {
        format!("total={total:.0} top=[{}]", top_components.join(", "))
    })
}

fn decision_audit_hand_select_summary(audit: &Value) -> Option<String> {
    let hand_select = audit.get("hand_select")?;
    if hand_select.is_null() {
        return None;
    }

    let selection_kind = json_str(hand_select.get("selection_kind"))?;
    let reason = json_str(hand_select.get("reason")).unwrap_or("unknown");
    let chosen_count = hand_select
        .get("chosen_uuids")
        .and_then(|value| value.as_array())
        .map_or(0, |entries| entries.len());
    let top_candidate = hand_select
        .get("top_candidates")
        .and_then(|value| value.as_array())
        .and_then(|candidates| candidates.first())
        .map(|candidate| {
            let label = json_str(candidate.get("label")).unwrap_or("?");
            let score = json_number_as_i64(candidate.get("score")).unwrap_or(0);
            format!("{label}:{score}")
        });

    Some(match top_candidate {
        Some(candidate) => {
            format!("kind={selection_kind} reason={reason} chosen={chosen_count} top={candidate}")
        }
        None => format!("kind={selection_kind} reason={reason} chosen={chosen_count}"),
    })
}

fn decision_audit_exact_turn_summary(audit: &Value) -> Option<String> {
    let exact_turn = audit.get("exact_turn_shadow")?;
    if exact_turn.is_null() {
        return None;
    }

    let regime = audit
        .get("regime")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let frontier_outcome = audit.get("frontier_outcome");
    let frontier_survival = frontier_outcome
        .and_then(|value| value.get("survival"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let exact_turn_verdict = audit.get("exact_turn_verdict");
    let exact_survival = exact_turn_verdict
        .and_then(|value| value.get("survival"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let dominance = exact_turn_verdict
        .and_then(|value| value.get("dominance"))
        .and_then(Value::as_str)
        .unwrap_or("incomparable");
    let confidence = exact_turn_verdict
        .and_then(|value| value.get("confidence"))
        .and_then(Value::as_str)
        .unwrap_or("unavailable");
    let takeover_policy = audit.get("takeover_policy");
    let takeover = takeover_policy
        .and_then(|value| value.get("takeover_applied"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let takeover_reason = takeover_policy
        .and_then(|value| value.get("takeover_reason"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let decision_trace = audit.get("decision_trace");
    let root_pipeline = audit.get("root_pipeline");
    let chosen_by = decision_trace
        .and_then(|value| value.get("chosen_by"))
        .and_then(Value::as_str)
        .unwrap_or("frontier");
    let frontier_class = decision_trace
        .and_then(|value| value.get("frontier_proposal_class"))
        .and_then(Value::as_str)
        .unwrap_or("other");
    let screened_out = root_pipeline
        .and_then(|value| value.get("screened_out"))
        .and_then(Value::as_array)
        .map(|entries| entries.len())
        .unwrap_or(0);
    let alternatives = decision_trace
        .and_then(|value| value.get("why_not_others"))
        .and_then(Value::as_array)
        .map(|entries| entries.len())
        .unwrap_or(0);
    let rejection_reasons = decision_trace
        .and_then(|value| value.get("rejection_reasons"))
        .and_then(Value::as_array)
        .map(|reasons| {
            reasons
                .iter()
                .filter_map(Value::as_str)
                .take(4)
                .collect::<Vec<_>>()
                .join(",")
        })
        .filter(|reasons| !reasons.is_empty())
        .unwrap_or_else(|| "none".to_string());

    if exact_turn
        .get("skipped")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
    {
        let reason = json_str(exact_turn.get("skip_reason")).unwrap_or("unknown");
        let legal_moves = json_number_as_i64(exact_turn.get("legal_moves")).unwrap_or(0);
        let living_monsters = json_number_as_i64(exact_turn.get("living_monsters")).unwrap_or(0);
        let filled_potions = json_number_as_i64(exact_turn.get("filled_potions")).unwrap_or(0);
        return Some(format!(
            "skipped=true reason={reason} legal_moves={legal_moves} living_monsters={living_monsters} filled_potions={filled_potions} regime={regime} frontier_class={frontier_class} screened_out={screened_out} alternatives={alternatives} dominance={dominance} confidence={confidence} takeover={takeover} takeover_reason={takeover_reason} chosen_by={chosen_by} frontier_survival={frontier_survival} exact_survival={exact_survival} rejection_reasons={rejection_reasons}"
        ));
    }

    let best = json_str(exact_turn.get("best_first_input")).unwrap_or("?");
    let line_len = json_number_as_i64(exact_turn.get("best_line_len")).unwrap_or(0);
    let end_states = json_number_as_i64(exact_turn.get("nondominated_end_states")).unwrap_or(0);
    let nodes = json_number_as_i64(exact_turn.get("explored_nodes")).unwrap_or(0);
    let prunes = json_number_as_i64(exact_turn.get("dominance_prunes")).unwrap_or(0);
    let cycle_cuts = json_number_as_i64(exact_turn.get("cycle_cuts")).unwrap_or(0);
    let truncated = exact_turn
        .get("truncated")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let agrees = exact_turn
        .get("agrees_with_frontier")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);

    let mut parts = vec![
        format!("best={best}"),
        format!("line_len={line_len}"),
        format!("ends={end_states}"),
        format!("nodes={nodes}"),
        format!("prunes={prunes}"),
        format!("cycles={cycle_cuts}"),
        format!("truncated={truncated}"),
        format!("agrees={agrees}"),
        format!("regime={regime}"),
        format!("frontier_class={frontier_class}"),
        format!("screened_out={screened_out}"),
        format!("alternatives={alternatives}"),
        format!("dominance={dominance}"),
        format!("confidence={confidence}"),
        format!("takeover={takeover}"),
        format!("takeover_reason={takeover_reason}"),
        format!("chosen_by={chosen_by}"),
        format!("frontier_survival={frontier_survival}"),
        format!("exact_survival={exact_survival}"),
        format!("rejection_reasons={rejection_reasons}"),
    ];

    if let Some(resources) = exact_turn.get("best_resources") {
        let final_hp = json_number_as_i64(resources.get("final_hp")).unwrap_or(0);
        let final_block = json_number_as_i64(resources.get("final_block")).unwrap_or(0);
        let spent_potions = json_number_as_i64(resources.get("spent_potions")).unwrap_or(0);
        let hp_lost = json_number_as_i64(resources.get("hp_lost")).unwrap_or(0);
        let exhausted_cards = json_number_as_i64(resources.get("exhausted_cards")).unwrap_or(0);
        parts.push(format!(
            "resources=hp{final_hp}/blk{final_block}/pots{spent_potions}/lost{hp_lost}/exh{exhausted_cards}"
        ));
    }

    Some(parts.join(" "))
}

fn json_str(value: Option<&Value>) -> Option<&str> {
    value
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
}

fn json_number_as_i64(value: Option<&Value>) -> Option<i64> {
    value.and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_u64().map(|number| number as i64))
            .or_else(|| value.as_f64().map(|number| number.round() as i64))
    })
}

fn json_number_as_f64(value: Option<&Value>) -> Option<f64> {
    value.and_then(|value| {
        value
            .as_f64()
            .or_else(|| value.as_i64().map(|number| number as f64))
            .or_else(|| value.as_u64().map(|number| number as f64))
    })
}

fn combat_top_candidate_record(
    combat: &CombatState,
    stat: &CombatMoveStat,
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

fn branch_family_for_input(combat: &CombatState, input: &ClientInput) -> Option<BranchFamily> {
    let ClientInput::PlayCard { card_index, .. } = input else {
        return None;
    };
    let card = combat.zones.hand.get(*card_index)?;
    branch_family_for_card(card.id)
}

pub(super) enum CombatFrameOutcome {
    Continue,
    StopForParityFailure,
    StopForFailFast,
}

fn should_fail_fast_on_combat_snapshot(trigger_kind: &str, reasons: &[String]) -> bool {
    let _ = reasons;
    match trigger_kind {
        "validation_failure" | "engine_bug" => true,
        _ => false,
    }
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
    pub(super) last_root_action_source: Option<String>,
    pub(super) last_root_action_id: Option<String>,
    pub(super) last_root_action_command: Option<String>,
    pub(super) last_protocol_root_action_count: Option<usize>,
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
        self.last_root_action_source = None;
        self.last_root_action_id = None;
        self.last_root_action_command = None;
        self.last_protocol_root_action_count = None;
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
        truth_snapshot: &Value,
        observation_snapshot: &Value,
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
        write_failure_context(focus_log, predicted, truth_snapshot, observation_snapshot);
        write_failure_context(log, predicted, truth_snapshot, observation_snapshot);

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
    let identity = cs.monster_protocol_identity(monster.id);
    let identity = format!(
        " inst={:?} spawn={:?} draw_x={:?}",
        identity.and_then(|state| state.instance_id),
        identity.and_then(|state| state.spawn_order),
        identity.and_then(|state| state.draw_x)
    );
    format!(
        "    Rust M[{}] {} id={} hp={}/{} blk={} intent={:?}{}{}{}",
        index,
        monster_display_name(monster.monster_type),
        monster.id,
        monster.current_hp,
        monster.max_hp,
        monster.block,
        resolve_monster_turn_plan(cs, monster).summary_spec(),
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

fn format_java_monster_line(
    index: usize,
    truth_monster: &Value,
    observation_monster: Option<&Value>,
) -> String {
    let bool_field = |key: &str| {
        observation_monster
            .and_then(|monster| monster.get(key))
            .or_else(|| truth_monster.get(key))
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    };
    let str_field = |key: &str| {
        observation_monster
            .and_then(|monster| monster.get(key))
            .or_else(|| truth_monster.get(key))
            .and_then(|value| value.as_str())
            .unwrap_or("?")
    };
    let mut flags = Vec::new();
    if bool_field("is_gone") || bool_field("is_dying") {
        flags.push("dead");
    }
    if bool_field("half_dead") {
        flags.push("half_dead");
    }
    if bool_field("is_escaping") {
        flags.push("escaping");
    }
    let powers = format_java_powers(truth_monster);
    format!(
        "    Java M[{}] {} id={} hp={}/{} blk={} intent={} move_id={} inst={:?} spawn={:?} draw_x={:?}{}{}",
        index,
        str_field("name"),
        truth_monster["id"].as_str().unwrap_or("?"),
        truth_monster["current_hp"].as_i64().unwrap_or(-1),
        truth_monster["max_hp"].as_i64().unwrap_or(-1),
        truth_monster["block"].as_i64().unwrap_or(-1),
        str_field("intent"),
        truth_monster["move_id"].as_i64().unwrap_or(-1),
        truth_monster["monster_instance_id"].as_u64(),
        truth_monster["spawn_order"].as_u64(),
        observation_monster
            .and_then(|monster| monster.get("draw_x"))
            .and_then(|v| v.as_i64())
            .or_else(|| {
                observation_monster
                    .and_then(|monster| monster.get("draw_x"))
                    .and_then(|v| v.as_f64().map(|x| x.round() as i64))
            }),
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

fn write_failure_context<W: Write>(
    log: &mut W,
    predicted: &CombatState,
    truth_snapshot: &Value,
    observation_snapshot: &Value,
) {
    let rust_names: Vec<String> = predicted
        .entities
        .monsters
        .iter()
        .map(|monster| monster_display_name(monster.monster_type))
        .collect();
    let truth_monsters = truth_snapshot["monsters"].as_array();
    let observation_monsters = observation_snapshot["monsters"].as_array();
    let java_names: Vec<String> = truth_snapshot["monsters"]
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
        truth_snapshot["hand"].as_array().map_or(0, |arr| arr.len()),
        truth_snapshot["draw_pile"].as_array().map_or(0, |arr| arr.len()),
        truth_snapshot["discard_pile"].as_array().map_or(0, |arr| arr.len()),
        truth_snapshot["exhaust_pile"].as_array().map_or(0, |arr| arr.len()),
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
    if let Some(monsters) = truth_monsters {
        for (index, monster) in monsters.iter().enumerate() {
            let observation_monster = observation_monsters.and_then(|entries| entries.get(index));
            writeln!(
                log,
                "{}",
                format_java_monster_line(index, monster, observation_monster)
            )
            .unwrap();
        }
    }
}

fn is_hexaghost_protocol_monster(monster: &Value) -> bool {
    monster
        .get("id")
        .and_then(|v| v.as_str())
        .is_some_and(|id| id.eq_ignore_ascii_case("Hexaghost"))
}

fn log_hexaghost_end_turn_debug(
    log: &mut std::fs::File,
    expected_cs: &CombatState,
    truth_snapshot: &Value,
    observation_snapshot: &Value,
) {
    let rust_hex = expected_cs
        .entities
        .monsters
        .iter()
        .find(|m| is_hexaghost_monster_type(m.monster_type));
    let truth_hex_index = truth_snapshot
        .get("monsters")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.iter().position(is_hexaghost_protocol_monster));
    let truth_hex = truth_hex_index.and_then(|index| {
        truth_snapshot
            .get("monsters")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.get(index))
    });
    let observation_hex = truth_hex_index
        .and_then(|index| {
            observation_snapshot
                .get("monsters")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.get(index))
        })
        .or_else(|| {
            observation_snapshot
                .get("monsters")
                .and_then(|v| v.as_array())
                .and_then(|arr| {
                    arr.iter()
                        .find(|monster| is_hexaghost_protocol_monster(monster))
                })
        });

    if rust_hex.is_none() && truth_hex.is_none() && observation_hex.is_none() {
        return;
    }

    writeln!(log, "  [HEXAGHOST END DEBUG]").unwrap();
    if let Some(rust_hex) = rust_hex {
        writeln!(
            log,
            "    rust_post_end: hp={}/{} blk={} next_move_byte={} intent={:?} move_history=[{}] intent_preview_damage={}",
            rust_hex.current_hp,
            rust_hex.max_hp,
            rust_hex.block,
            rust_hex.planned_move_id(),
            resolve_monster_turn_plan(expected_cs, rust_hex).summary_spec(),
            format_move_history(rust_hex.move_history()),
            crate::projection::combat::project_monster_move_preview_in_combat(expected_cs, rust_hex)
                .damage_per_hit
                .unwrap_or(0)
        )
        .unwrap();
    }
    if truth_hex.is_some() || observation_hex.is_some() {
        writeln!(
            log,
            "    java_post_end: hp={}/{} blk={} move_id={} intent={} base_dmg={} adj_dmg={} hits={}",
            truth_hex
                .and_then(|monster| monster.get("current_hp"))
                .and_then(|v| v.as_i64())
                .unwrap_or(-1),
            truth_hex
                .and_then(|monster| monster.get("max_hp"))
                .and_then(|v| v.as_i64())
                .unwrap_or(-1),
            truth_hex
                .and_then(|monster| monster.get("block"))
                .and_then(|v| v.as_i64())
                .unwrap_or(-1),
            truth_hex
                .and_then(|monster| monster.get("move_id"))
                .and_then(|v| v.as_i64())
                .unwrap_or(-1),
            observation_hex
                .and_then(|monster| monster.get("intent"))
                .and_then(|v| v.as_str())
                .unwrap_or("?"),
            truth_hex
                .and_then(|monster| monster.get("move_base_damage"))
                .and_then(|v| v.as_i64())
                .unwrap_or(-1),
            observation_hex
                .and_then(|monster| monster.get("move_adjusted_damage"))
                .and_then(|v| v.as_i64())
                .unwrap_or(-1),
            truth_hex
                .and_then(|monster| monster.get("move_hits"))
                .or_else(|| {
                    observation_hex
                        .and_then(|monster| monster.get("move_hits"))
                })
                .and_then(|v| v.as_i64())
                .unwrap_or(-1)
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
    combat_mode: LiveCombatMode,
    exact_turn_mode: LiveExactTurnMode,
    fail_fast_debug: bool,
    combat_search_budget: u32,
    legacy_root_legal_moves: bool,
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
    let rv = frame.relics();
    let rebuild_started = log_combat_stage_enter(live_io, frame_count, "rebuild_truth", "");
    let combat_truth_snapshot = build_live_truth_snapshot(gs);
    let combat_observation_snapshot = build_live_observation_snapshot(gs);
    let truth =
        build_combat_state_from_snapshots(&combat_truth_snapshot, &combat_observation_snapshot, rv);
    log_combat_stage_exit(
        live_io,
        frame_count,
        "rebuild_truth",
        rebuild_started,
        format!(
            "monsters={} hand={}",
            truth.entities.monsters.len(),
            truth.zones.hand.len()
        ),
    );

    if let (Some(prev_truth), Some(prev_input)) = (
        &combat_runtime.last_combat_truth,
        &combat_runtime.last_input,
    ) {
        let after_engine = EngineState::CombatPlayerTurn;
        let signature = signature_from_transition_with_archetypes(
            &EngineState::CombatPlayerTurn,
            prev_truth,
            prev_input,
            &after_engine,
            &truth,
            crate::bot::archetype_tags_for_combat(prev_truth),
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
        let record = ObservedInteractionRecord {
            observed_from: "live_comm".to_string(),
            source_file: signature_source_file.to_string(),
            combat_idx: None,
            action_idx: Some(frame_count as usize),
            command: command_string(prev_input),
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

    let validate_started = log_combat_stage_enter(live_io, frame_count, "validate_parse", "");
    let sync_diffs = validate_parse(&truth, &combat_truth_snapshot, &combat_observation_snapshot);
    log_combat_stage_exit(
        live_io,
        frame_count,
        "validate_parse",
        validate_started,
        format!("diffs={}", sync_diffs.len()),
    );
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
        write_failure_context(
            &mut live_io.focus_log,
            &truth,
            &combat_truth_snapshot,
            &combat_observation_snapshot,
        );
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
        if fail_fast_debug
            && should_fail_fast_on_combat_snapshot(
                "validation_failure",
                &[
                    "combat_parse_diff".to_string(),
                    format!("count={}", sync_diffs.len()),
                ],
            )
        {
            writeln!(live_io.log, "  [FAIL_FAST] stopping on combat parse diff").unwrap();
            writeln!(
                live_io.focus_log,
                "  [FAIL_FAST] stopping on combat parse diff"
            )
            .unwrap();
            return CombatFrameOutcome::StopForFailFast;
        }
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
        let parity_started = log_combat_stage_enter(live_io, frame_count, "action_parity", "");
        let action_diffs = compare_combat_states_from_snapshots(
            &expected_cs,
            &combat_truth_snapshot,
            &combat_observation_snapshot,
            combat_runtime.action_context.was_end_turn,
            &combat_runtime.action_context,
        );
        log_combat_stage_exit(
            live_io,
            frame_count,
            "action_parity",
            parity_started,
            format!("diffs={}", action_diffs.len()),
        );

        if !action_diffs.is_empty() {
            if combat_runtime.action_context.was_end_turn
                && (expected_cs
                    .entities
                    .monsters
                    .iter()
                    .any(|m| is_hexaghost_monster_type(m.monster_type))
                    || combat_truth_snapshot
                        .get("monsters")
                        .and_then(|v| v.as_array())
                        .is_some_and(|arr| arr.iter().any(is_hexaghost_protocol_monster)))
            {
                log_hexaghost_end_turn_debug(
                    &mut live_io.log,
                    &expected_cs,
                    &combat_truth_snapshot,
                    &combat_observation_snapshot,
                );
            }
            combat_runtime.record_action_diffs(
                &action_diffs,
                frame_count,
                &mut live_io.log,
                &mut live_io.focus_log,
                frame.response_id(),
                frame.state_frame_id(),
                &expected_cs,
                &combat_truth_snapshot,
                &combat_observation_snapshot,
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
            if fail_fast_debug
                && should_fail_fast_on_combat_snapshot(
                    "engine_bug",
                    &[
                        "combat_action_diff".to_string(),
                        format!("count={}", action_diffs.len()),
                    ],
                )
            {
                writeln!(live_io.log, "  [FAIL_FAST] stopping on combat action diff").unwrap();
                writeln!(
                    live_io.focus_log,
                    "  [FAIL_FAST] stopping on combat action diff"
                )
                .unwrap();
                return CombatFrameOutcome::StopForFailFast;
            }
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

    let protocol_meta = frame.protocol_meta().unwrap_or(&Value::Null);
    let protocol_affordance = match build_combat_affordance_snapshot(protocol_meta, &truth) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            writeln!(
                live_io.log,
                "  [PROTO ROOT BUG] invalid combat_action_space: {}",
                err
            )
            .unwrap();
            let _ = write_failure_snapshot(
                live_io,
                frame_count,
                frame,
                "protocol_root_action_space",
                vec!["invalid_combat_action_space".to_string(), err.clone()],
                serde_json::json!({
                    "chosen_command": last_sent_cmd,
                    "root_action_source": "protocol",
                }),
            );
            if !legacy_root_legal_moves {
                return CombatFrameOutcome::StopForFailFast;
            }
            None
        }
    };
    let protocol_root_action_count = protocol_affordance
        .as_ref()
        .map(|snapshot| snapshot.len())
        .unwrap_or(0);
    if frame.has_combat_action_space_capability()
        && protocol_affordance
            .as_ref()
            .is_some_and(|snapshot| snapshot.is_empty())
    {
        writeln!(
            live_io.log,
            "  [PROTO ROOT BUG] combat_action_space exported zero root actions"
        )
        .unwrap();
        let _ = write_failure_snapshot(
            live_io,
            frame_count,
            frame,
            "protocol_root_action_space",
            vec!["empty_combat_action_space".to_string()],
            serde_json::json!({
                "chosen_command": last_sent_cmd,
                "root_action_source": "protocol",
            }),
        );
        if !legacy_root_legal_moves {
            return CombatFrameOutcome::StopForFailFast;
        }
    }
    let root_inputs = protocol_affordance
        .as_ref()
        .filter(|snapshot| !snapshot.is_empty())
        .map(protocol_root_moves);
    let root_action_source = if root_inputs.is_some() {
        "protocol"
    } else if legacy_root_legal_moves {
        "legacy"
    } else if frame.has_combat_action_space_capability() {
        writeln!(
            live_io.log,
            "  [PROTO ROOT BUG] combat_action_space missing while capability=true"
        )
        .unwrap();
        let _ = write_failure_snapshot(
            live_io,
            frame_count,
            frame,
            "protocol_root_action_space",
            vec!["missing_combat_action_space".to_string()],
            serde_json::json!({
                "chosen_command": last_sent_cmd,
                "root_action_source": "protocol",
            }),
        );
        return CombatFrameOutcome::StopForFailFast;
    } else {
        "legacy"
    };

    let root_started = log_combat_stage_enter(
        live_io,
        frame_count,
        "root_search",
        format!(
            "legacy_budget={combat_search_budget} mode={:?} exact_turn={:?} root_source={} protocol_root_action_count={}",
            combat_mode, exact_turn_mode, root_action_source, protocol_root_action_count
        ),
    );
    let root_runtime = live_root_search_budget(combat_search_budget, exact_turn_mode);
    let mut search_diag = diagnose_root_search_with_runtime_and_root_inputs(
        &EngineState::CombatPlayerTurn,
        &truth,
        combat_search_budget,
        root_runtime,
        root_inputs.clone(),
    );
    log_combat_stage_exit(
        live_io,
        frame_count,
        "root_search",
        root_started,
        format!(
            "chosen={} legal_moves={} elapsed_ms={} timed_out={}",
            describe_client_input(&truth, &search_diag.chosen_move),
            search_diag.legal_moves,
            search_diag.elapsed_ms,
            search_diag.timed_out,
        ),
    );
    if search_diag.timed_out {
        writeln!(
            live_io.log,
            "  [SEARCH TIMEOUT] root_search hit live timeout/max-nodes; using best partial result"
        )
        .unwrap();
        writeln!(
            live_io.focus_log,
            "[SEARCH TIMEOUT] frame={frame_count} root_search partial_result budget={combat_search_budget} elapsed_ms={}",
            search_diag.elapsed_ms
        )
        .unwrap();
    }
    let baseline_diag = if should_run_sync_audit(combat_mode, frame_count, &search_diag) {
        let baseline_started = log_combat_stage_enter(live_io, frame_count, "baseline_search", "");
        let baseline_runtime = live_baseline_search_budget(combat_search_budget);
        let mut baseline_diag = diagnose_root_search_with_depth_and_runtime_and_root_inputs(
            &EngineState::CombatPlayerTurn,
            &truth,
            2,
            0,
            baseline_runtime,
            root_inputs.clone(),
        );
        baseline_diag.profile.audit_ms = baseline_diag.elapsed_ms;
        log_combat_stage_exit(
            live_io,
            frame_count,
            "baseline_search",
            baseline_started,
            format!(
                "chosen={} legal_moves={} timed_out={}",
                describe_client_input(&truth, &baseline_diag.chosen_move),
                baseline_diag.legal_moves,
                baseline_diag.timed_out,
            ),
        );
        if baseline_diag.timed_out {
            writeln!(
                live_io.log,
                "  [SEARCH TIMEOUT] baseline_search fell back to partial frontier results"
            )
            .unwrap();
        }
        Some(baseline_diag)
    } else {
        None
    };
    if let Some(baseline_diag) = baseline_diag.as_ref() {
        search_diag.profile.audit_ms = baseline_diag.elapsed_ms;
    }
    let verbose_outcome_logging = verbose_search_outcome_logging_enabled();
    let render_started = log_combat_stage_enter(live_io, frame_count, "search_render", "");
    writeln!(
        live_io.log,
        "  [SEARCH DIAG] budget={} root_node_budget={} engine_step_budget={} exact_turn_node_budget={} audit_budget={} depth_limit={} max_depth={} root_width={} branch_width={} max_engine_steps={} elapsed_ms={} timed_out={} legal_moves={} reduced_legal_moves={} equivalence_mode={} simulations={} chosen={}",
        combat_search_budget,
        root_runtime.root_node_budget,
        root_runtime.engine_step_budget,
        root_runtime.exact_turn_node_budget,
        root_runtime.audit_budget,
        search_diag.depth_limit,
        search_diag.max_decision_depth,
        search_diag.root_width,
        search_diag.branch_width,
        search_diag.max_engine_steps,
        search_diag.elapsed_ms,
        search_diag.timed_out,
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
    log_combat_stage_exit(
        live_io,
        frame_count,
        "search_render",
        render_started,
        format!("top_moves={}", search_diag.top_moves.len()),
    );
    writeln!(
        live_io.log,
        "  [SEARCH PROFILE] {}",
        format_search_profile_summary(&search_diag)
    )
    .unwrap();
    writeln!(
        live_io.log,
        "  [ROOT ACTION] source={} protocol_root_action_count={}",
        root_action_source, protocol_root_action_count
    )
    .unwrap();
    writeln!(
        live_io.focus_log,
        "[ROOT ACTION] frame={} source={} protocol_root_action_count={}",
        frame_count, root_action_source, protocol_root_action_count
    )
    .unwrap();
    log_combat_decision_audit_summary(live_io, &search_diag);
    if live_io.log.metadata().is_ok() {
        let top_candidates = search_diag
            .top_moves
            .iter()
            .take(SEARCH_DIAG_TOP_K)
            .map(|stat| combat_top_candidate_record(&truth, stat))
            .collect::<Vec<_>>();
        let meta = crate::bot::DecisionMetadata::new(
            crate::bot::DecisionDomain::Combat,
            "combat_search",
            Some("search_root_policy"),
            None,
            false,
        );
        let shadow = sidecar::combat_shadow_json(
            frame_count,
            "live_comm_combat",
            &meta,
            describe_client_input(&truth, &search_diag.chosen_move),
            &search_diag,
            top_candidates,
            None,
            None,
        );
        sidecar::write_shadow_record(&mut live_io.combat_decision_audit, &shadow);
        sidecar::write_shadow_record(&mut live_io.sidecar_shadow, &shadow);
    }
    if let Some(baseline_diag) = baseline_diag.as_ref() {
        if !same_or_equivalent_client_input(
            &truth,
            &baseline_diag.chosen_move,
            &search_diag.chosen_move,
        ) {
            writeln!(
                live_io.log,
                "  [BASELINE DIAG] disagrees chosen={:?} baseline_score={:.1}",
                baseline_diag.chosen_move,
                baseline_diag
                    .top_moves
                    .first()
                    .map(|stat| stat.avg_score)
                    .unwrap_or(0.0)
            )
            .unwrap();
            for move_stat in baseline_diag.top_moves.iter().take(3) {
                writeln!(
                    live_io.log,
                    "  [BASELINE DIAG] move={:?} score={:.1} visits={}",
                    move_stat.input, move_stat.avg_score, move_stat.visits
                )
                .unwrap();
            }
        }
    }
    log_slow_search_summary(
        live_io,
        frame_count,
        &truth,
        baseline_diag.as_ref(),
        &search_diag,
    );
    let input = search_diag.chosen_move.clone();
    let chosen_protocol_action = protocol_affordance
        .as_ref()
        .and_then(|snapshot| snapshot.find_by_input(&input));
    let chosen_action_id = chosen_protocol_action.map(|action| action.action_id.clone());
    let chosen_command = chosen_protocol_action.map(|action| action.command.clone());
    if root_action_source == "protocol" && chosen_command.is_none() {
        writeln!(
            live_io.log,
            "  [PROTO ROOT BUG] chosen move {:?} missing action_id/command mapping",
            input
        )
        .unwrap();
        let _ = write_failure_snapshot(
            live_io,
            frame_count,
            frame,
            "protocol_root_action_space",
            vec!["missing_chosen_protocol_action".to_string()],
            serde_json::json!({
                "chosen_move": format!("{:?}", input),
                "root_action_source": "protocol",
                "protocol_root_action_count": protocol_root_action_count,
            }),
        );
        return CombatFrameOutcome::StopForFailFast;
    }
    if let Some(action_id) = chosen_action_id.as_ref() {
        writeln!(
            live_io.log,
            "  [ROOT ACTION] chosen_action_id={} chosen_command={}",
            action_id,
            chosen_command.as_deref().unwrap_or("?")
        )
        .unwrap();
        writeln!(
            live_io.focus_log,
            "[ROOT ACTION] frame={} chosen_action_id={} chosen_command={}",
            frame_count,
            action_id,
            chosen_command.as_deref().unwrap_or("?")
        )
        .unwrap();
    }

    if matches!(input, crate::state::core::ClientInput::UsePotion { .. }) {
        log_potion_decision_trace(live_io, &truth);
    }

    writeln!(live_io.log, "  → {:?}", input).unwrap();
    if matches!(input, crate::state::core::ClientInput::EndTurn) {
        let end_diag_lines = describe_end_turn_options(&truth);
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
    combat_runtime.last_root_action_source = Some(root_action_source.to_string());
    combat_runtime.last_root_action_id = chosen_action_id.clone();
    combat_runtime.last_root_action_command = chosen_command.clone();
    combat_runtime.last_protocol_root_action_count = Some(protocol_root_action_count);

    let mut engine_state = EngineState::CombatPlayerTurn;
    let submit_command = if root_action_source == "protocol" {
        chosen_command.clone()
    } else {
        comm_mod::input_to_java_command(&input, &engine_state)
    };
    if let Some(cmd) = submit_command {
        if cmd == *last_sent_cmd && *cmd_failed_count > 0 {
            writeln!(
                live_io.log,
                "  [!] AVOIDING REPEATED ERROR BY FORCING END TURN root_source={} action_id={}",
                root_action_source,
                chosen_action_id.as_deref().unwrap_or("<none>")
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
                    .map(|m| format!("{:?}", resolve_monster_turn_plan(&truth, m).summary_spec()))
                    .collect(),
                monster_names: truth
                    .entities
                    .monsters
                    .iter()
                    .map(|m| monster_display_name(m.monster_type))
                    .collect(),
                has_rng_state: combat_truth_snapshot.get("rng_state").is_some(),
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
        combat_runtime.last_root_action_command = Some("END".to_string());
        live_io.send_line(stdout, "END");
        *last_sent_cmd = "END".to_string();
        *cmd_failed_count = 0;
    }

    CombatFrameOutcome::Continue
}

pub(super) fn build_live_truth_snapshot(gs: &Value) -> Value {
    build_protocol_live_truth_snapshot(gs)
}

pub(super) fn build_live_observation_snapshot(gs: &Value) -> Value {
    build_protocol_live_observation_snapshot(gs)
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
            resolve_monster_turn_plan(truth, m).summary_spec(),
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

pub(super) fn validate_parse(
    cs: &CombatState,
    truth_snapshot: &Value,
    observation_snapshot: &Value,
) -> Vec<String> {
    let mut diffs = Vec::new();
    let jp = &truth_snapshot["player"];

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

    if let Some(j_hand) = truth_snapshot["hand"].as_array() {
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

    if let Some(j_draw) = truth_snapshot["draw_pile"].as_array() {
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

    if let Some(j_monsters) = truth_snapshot["monsters"].as_array() {
        let alignment_monsters = observation_snapshot["monsters"]
            .as_array()
            .unwrap_or(j_monsters);
        let aligned_indices = align_rust_monsters_for_parse(cs, alignment_monsters);
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
                            && cs
                                .monster_protocol_identity(monster.id)
                                .and_then(|identity| identity.instance_id)
                                == Some(instance_id)
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
                            && cs
                                .monster_protocol_identity(monster.id)
                                .and_then(|identity| identity.draw_x)
                                == java_draw_x
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

#[cfg(test)]
mod tests {
    use super::same_or_equivalent_client_input_with_state;
    use crate::content::cards::CardId;
    use crate::runtime::combat::CombatCard;
    use crate::state::core::ClientInput;

    #[test]
    fn identical_hand_cards_with_different_indices_count_as_equivalent_actions() {
        let hand = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Defend, 2),
            CombatCard::new(CardId::Defend, 3),
        ];
        let potions = Vec::new();

        assert!(same_or_equivalent_client_input_with_state(
            &hand,
            &potions,
            &ClientInput::PlayCard {
                card_index: 1,
                target: None,
            },
            &ClientInput::PlayCard {
                card_index: 2,
                target: None,
            },
        ));
    }

    #[test]
    fn different_hand_cards_do_not_count_as_equivalent_actions() {
        let hand = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Defend, 2),
        ];
        let potions = Vec::new();

        assert!(!same_or_equivalent_client_input_with_state(
            &hand,
            &potions,
            &ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
            &ClientInput::PlayCard {
                card_index: 1,
                target: Some(1),
            },
        ));
    }
}
