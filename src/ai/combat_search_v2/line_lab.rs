use std::time::Duration;

use serde::Serialize;

use crate::content::cards::{get_card_definition, CardType};
use crate::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal, EngineCombatStepper,
};
use crate::state::core::ClientInput;

pub use super::turn_pool_rescue::{
    CombatTurnPoolRescueLineSummary as CombatLineLabTurnPoolLineSummary,
    CombatTurnPoolRescueReport as CombatLineLabTurnPoolReport,
};
use super::{
    run_combat_search_v2, turn_pool_rescue::run_combat_turn_pool_rescue_report_v0,
    CombatSearchV2ActionTrace, CombatSearchV2Config, CombatSearchV2TrajectoryReport,
    SearchTerminalLabel,
};

#[derive(Clone, Debug, Serialize)]
pub struct CombatLineLabReport {
    pub schema: &'static str,
    pub baseline: Option<CombatLineLabLineSummary>,
    pub cuts: Vec<CombatLineLabCutReport>,
    pub best_repair: Option<CombatLineLabCutReport>,
    pub turn_pool: Option<CombatLineLabTurnPoolReport>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatLineLabLineSummary {
    pub source: &'static str,
    pub terminal: SearchTerminalLabel,
    pub final_hp: i32,
    pub total_enemy_hp: i32,
    pub living_enemy_count: usize,
    pub turns: u32,
    pub actions: usize,
    pub potions_used: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatLineLabCutReport {
    pub cut_kind: &'static str,
    pub cut_action_index: usize,
    pub prefix_replayed_actions: usize,
    pub terminal: Option<SearchTerminalLabel>,
    pub final_hp: Option<i32>,
    pub total_enemy_hp: Option<i32>,
    pub living_enemy_count: Option<usize>,
    pub turns: Option<u32>,
    pub suffix_actions: Option<usize>,
    pub total_potions_used: Option<u32>,
    pub baseline_suffix_replay_ok: Option<bool>,
    pub baseline_suffix_terminal: Option<SearchTerminalLabel>,
    pub baseline_suffix_final_hp: Option<i32>,
    pub baseline_suffix_total_enemy_hp: Option<i32>,
    pub repair_action_edit_distance: Option<usize>,
    pub delta_enemy_hp: Option<i32>,
    pub delta_player_hp: Option<i32>,
    pub nodes_expanded: Option<u64>,
    pub deadline_hit: Option<bool>,
    pub failed_reason: Option<&'static str>,
}

pub fn run_combat_line_lab_v0(
    start: &CombatPosition,
    mut config: CombatSearchV2Config,
    total_budget_ms: u64,
    max_cuts: usize,
) -> CombatLineLabReport {
    let baseline_ms = (total_budget_ms / 2).max(500);
    let repair_budget_ms = total_budget_ms.saturating_sub(baseline_ms).max(500);
    config.wall_time = Some(Duration::from_millis(baseline_ms));
    let baseline_report = run_combat_search_v2(&start.engine, &start.combat, config.clone());
    let Some(parent) = baseline_report.best_complete_trajectory.as_ref() else {
        return CombatLineLabReport {
            schema: "combat_line_lab_v1",
            baseline: None,
            cuts: Vec::new(),
            best_repair: None,
            turn_pool: Some(run_combat_turn_pool_rescue_report_v0(
                start,
                repair_budget_ms,
                Some(&config),
            )),
        };
    };
    run_combat_line_lab_from_parent_v0(start, parent, config, repair_budget_ms, max_cuts)
}

pub fn run_combat_line_lab_from_parent_v0(
    start: &CombatPosition,
    parent: &CombatSearchV2TrajectoryReport,
    config: CombatSearchV2Config,
    repair_budget_ms: u64,
    max_cuts: usize,
) -> CombatLineLabReport {
    let suffix_budget_ms = (repair_budget_ms / 2).max(500);
    let turn_pool_budget_ms = repair_budget_ms.saturating_sub(suffix_budget_ms).max(500);
    let baseline = line_summary("baseline_best_complete", parent);
    let cut_points = collect_cut_points(start, parent, max_cuts);
    let per_cut_ms = per_cut_budget_ms(suffix_budget_ms, cut_points.len());
    let cuts = cut_points
        .into_iter()
        .map(|cut| repair_cut(start, parent, cut, &config, per_cut_ms))
        .collect::<Vec<_>>();
    let best_repair = cuts
        .iter()
        .filter(|cut| cut.failed_reason.is_none())
        .max_by_key(|cut| repair_rank(cut))
        .cloned();

    CombatLineLabReport {
        schema: "combat_line_lab_v1",
        baseline: Some(baseline),
        cuts,
        best_repair,
        turn_pool: Some(run_combat_turn_pool_rescue_report_v0(
            start,
            turn_pool_budget_ms,
            Some(&config),
        )),
    }
}

fn per_cut_budget_ms(total_ms: u64, cut_count: usize) -> u64 {
    if cut_count == 0 {
        return total_ms;
    }
    (total_ms / cut_count as u64).clamp(500, 8_000)
}

fn collect_cut_points(
    start: &CombatPosition,
    parent: &CombatSearchV2TrajectoryReport,
    max_cuts: usize,
) -> Vec<CutPoint> {
    let mut cuts = Vec::new();
    let mut position = start.clone();
    let stepper = EngineCombatStepper;
    for (index, action) in parent.actions.iter().enumerate() {
        match action.input {
            ClientInput::UsePotion { .. } => add_cut(&mut cuts, "before_potion", index),
            ClientInput::PlayCard { card_index, .. } if is_power_in_hand(&position, card_index) => {
                add_cut(&mut cuts, "before_power", index);
            }
            ClientInput::EndTurn => add_cut(&mut cuts, "turn_boundary", index + 1),
            _ => {}
        }
        let Some(next) = replay_one(&position, action, &stepper) else {
            break;
        };
        position = next;
    }

    if !parent.actions.is_empty() {
        add_cut(
            &mut cuts,
            "late_suffix_12",
            parent.actions.len().saturating_sub(12),
        );
        add_cut(
            &mut cuts,
            "late_suffix_6",
            parent.actions.len().saturating_sub(6),
        );
    }
    select_cut_points(cuts, max_cuts)
}

fn add_cut(cuts: &mut Vec<CutPoint>, kind: &'static str, action_index: usize) {
    if cuts.iter().any(|cut| cut.action_index == action_index) {
        return;
    }
    cuts.push(CutPoint { kind, action_index });
}

fn select_cut_points(cuts: Vec<CutPoint>, max_cuts: usize) -> Vec<CutPoint> {
    if cuts.len() <= max_cuts {
        return cuts;
    }
    let mut selected = Vec::new();
    for kind in [
        "before_potion",
        "before_power",
        "late_suffix_12",
        "late_suffix_6",
    ] {
        for cut in cuts.iter().filter(|cut| cut.kind == kind) {
            add_cut(&mut selected, cut.kind, cut.action_index);
            if selected.len() >= max_cuts {
                return selected;
            }
        }
    }
    for cut in cuts.iter().rev().filter(|cut| cut.kind == "turn_boundary") {
        add_cut(&mut selected, cut.kind, cut.action_index);
        if selected.len() >= max_cuts {
            return selected;
        }
    }
    for cut in cuts {
        add_cut(&mut selected, cut.kind, cut.action_index);
        if selected.len() >= max_cuts {
            break;
        }
    }
    selected
}

fn repair_cut(
    start: &CombatPosition,
    parent: &CombatSearchV2TrajectoryReport,
    cut: CutPoint,
    config: &CombatSearchV2Config,
    per_cut_ms: u64,
) -> CombatLineLabCutReport {
    let stepper = EngineCombatStepper;
    let Some(prefix) = replay_prefix(start, &parent.actions[..cut.action_index], &stepper) else {
        return failed_cut(cut, "prefix_replay_failed");
    };

    let original_suffix = &parent.actions[cut.action_index..];
    let baseline_suffix = replay_actions(&prefix.position, original_suffix, &stepper);
    let baseline_suffix_replay_ok = baseline_suffix.as_ref().map(|summary| {
        summary.terminal == parent.terminal
            && summary.final_hp == parent.final_hp
            && summary.total_enemy_hp == parent.final_state.total_enemy_hp
            && summary.living_enemy_count == parent.final_state.living_enemy_count
    });

    let mut suffix_config = config.clone();
    suffix_config.wall_time = Some(Duration::from_millis(per_cut_ms));
    let suffix = run_combat_search_v2(
        &prefix.position.engine,
        &prefix.position.combat,
        suffix_config,
    );
    let Some(trajectory) = suffix.best_complete_trajectory.as_ref() else {
        return CombatLineLabCutReport {
            cut_kind: cut.kind,
            cut_action_index: cut.action_index,
            prefix_replayed_actions: prefix.replayed_actions,
            terminal: None,
            final_hp: None,
            total_enemy_hp: None,
            living_enemy_count: None,
            turns: None,
            suffix_actions: None,
            total_potions_used: None,
            baseline_suffix_replay_ok,
            baseline_suffix_terminal: baseline_suffix.as_ref().map(|summary| summary.terminal),
            baseline_suffix_final_hp: baseline_suffix.as_ref().map(|summary| summary.final_hp),
            baseline_suffix_total_enemy_hp: baseline_suffix
                .as_ref()
                .map(|summary| summary.total_enemy_hp),
            repair_action_edit_distance: None,
            delta_enemy_hp: None,
            delta_player_hp: None,
            nodes_expanded: Some(suffix.stats.nodes_expanded),
            deadline_hit: Some(suffix.stats.deadline_hit),
            failed_reason: Some("no_suffix_complete"),
        };
    };

    CombatLineLabCutReport {
        cut_kind: cut.kind,
        cut_action_index: cut.action_index,
        prefix_replayed_actions: prefix.replayed_actions,
        terminal: Some(trajectory.terminal),
        final_hp: Some(trajectory.final_hp),
        total_enemy_hp: Some(trajectory.final_state.total_enemy_hp),
        living_enemy_count: Some(trajectory.final_state.living_enemy_count),
        turns: Some(trajectory.turns),
        suffix_actions: Some(trajectory.actions.len()),
        total_potions_used: Some(prefix.potions_used + trajectory.potions_used),
        baseline_suffix_replay_ok,
        baseline_suffix_terminal: baseline_suffix.as_ref().map(|summary| summary.terminal),
        baseline_suffix_final_hp: baseline_suffix.as_ref().map(|summary| summary.final_hp),
        baseline_suffix_total_enemy_hp: baseline_suffix
            .as_ref()
            .map(|summary| summary.total_enemy_hp),
        repair_action_edit_distance: Some(action_edit_distance(
            original_suffix,
            &trajectory.actions,
        )),
        delta_enemy_hp: Some(
            trajectory.final_state.total_enemy_hp - parent.final_state.total_enemy_hp,
        ),
        delta_player_hp: Some(trajectory.final_hp - parent.final_hp),
        nodes_expanded: Some(suffix.stats.nodes_expanded),
        deadline_hit: Some(suffix.stats.deadline_hit),
        failed_reason: None,
    }
}

fn failed_cut(cut: CutPoint, reason: &'static str) -> CombatLineLabCutReport {
    CombatLineLabCutReport {
        cut_kind: cut.kind,
        cut_action_index: cut.action_index,
        prefix_replayed_actions: 0,
        terminal: None,
        final_hp: None,
        total_enemy_hp: None,
        living_enemy_count: None,
        turns: None,
        suffix_actions: None,
        total_potions_used: None,
        baseline_suffix_replay_ok: None,
        baseline_suffix_terminal: None,
        baseline_suffix_final_hp: None,
        baseline_suffix_total_enemy_hp: None,
        repair_action_edit_distance: None,
        delta_enemy_hp: None,
        delta_player_hp: None,
        nodes_expanded: None,
        deadline_hit: None,
        failed_reason: Some(reason),
    }
}

fn replay_actions(
    start: &CombatPosition,
    actions: &[CombatSearchV2ActionTrace],
    stepper: &EngineCombatStepper,
) -> Option<ReplaySummary> {
    let mut position = start.clone();
    for action in actions {
        position = replay_one(&position, action, stepper)?;
    }
    Some(ReplaySummary {
        terminal: search_terminal(stepper.terminal(&position)),
        final_hp: position.combat.entities.player.current_hp,
        total_enemy_hp: total_enemy_hp(&position),
        living_enemy_count: living_enemy_count(&position),
    })
}

fn replay_prefix(
    start: &CombatPosition,
    actions: &[CombatSearchV2ActionTrace],
    stepper: &EngineCombatStepper,
) -> Option<PrefixReplay> {
    let mut position = start.clone();
    let mut potions_used = 0;
    for action in actions {
        if matches!(action.input, ClientInput::UsePotion { .. }) {
            potions_used += 1;
        }
        position = replay_one(&position, action, stepper)?;
    }
    Some(PrefixReplay {
        position,
        replayed_actions: actions.len(),
        potions_used,
    })
}

fn replay_one(
    position: &CombatPosition,
    action: &CombatSearchV2ActionTrace,
    stepper: &EngineCombatStepper,
) -> Option<CombatPosition> {
    let choice = stepper
        .legal_action_choices(position)
        .into_iter()
        .find(|choice| choice.input == action.input && choice.action_key == action.action_key)?;
    let step = stepper.apply_to_stable(
        position,
        choice.input,
        CombatStepLimits {
            max_engine_steps: 250,
            deadline: None,
        },
    );
    if step.truncated || step.timed_out {
        return None;
    }
    Some(step.position)
}

fn is_power_in_hand(position: &CombatPosition, card_index: usize) -> bool {
    position
        .combat
        .zones
        .hand
        .get(card_index)
        .is_some_and(|card| get_card_definition(card.id).card_type == CardType::Power)
}

fn line_summary(
    source: &'static str,
    trajectory: &CombatSearchV2TrajectoryReport,
) -> CombatLineLabLineSummary {
    CombatLineLabLineSummary {
        source,
        terminal: trajectory.terminal,
        final_hp: trajectory.final_hp,
        total_enemy_hp: trajectory.final_state.total_enemy_hp,
        living_enemy_count: trajectory.final_state.living_enemy_count,
        turns: trajectory.turns,
        actions: trajectory.actions.len(),
        potions_used: trajectory.potions_used,
    }
}

fn search_terminal(terminal: CombatTerminal) -> SearchTerminalLabel {
    match terminal {
        CombatTerminal::Win => SearchTerminalLabel::Win,
        CombatTerminal::Loss => SearchTerminalLabel::Loss,
        CombatTerminal::Unresolved => SearchTerminalLabel::Unresolved,
    }
}

fn living_enemy_count(position: &CombatPosition) -> usize {
    position
        .combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .count()
}

fn total_enemy_hp(position: &CombatPosition) -> i32 {
    position
        .combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .map(|monster| monster.current_hp.max(0) + monster.block.max(0))
        .sum()
}

fn action_edit_distance(
    left: &[CombatSearchV2ActionTrace],
    right: &[CombatSearchV2ActionTrace],
) -> usize {
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    let mut current = vec![0; right.len() + 1];
    for (i, left_action) in left.iter().enumerate() {
        current[0] = i + 1;
        for (j, right_action) in right.iter().enumerate() {
            let substitution = usize::from(left_action.action_key != right_action.action_key);
            current[j + 1] = (previous[j + 1] + 1)
                .min(current[j] + 1)
                .min(previous[j] + substitution);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right.len()]
}

fn repair_rank(cut: &CombatLineLabCutReport) -> (i32, i32, i32) {
    let terminal_rank = match cut.terminal {
        Some(SearchTerminalLabel::Win) => 2,
        Some(SearchTerminalLabel::Unresolved) => 1,
        _ => 0,
    };
    (
        terminal_rank,
        -cut.total_enemy_hp.unwrap_or(i32::MAX),
        cut.final_hp.unwrap_or(i32::MIN),
    )
}

#[derive(Clone, Copy)]
struct CutPoint {
    kind: &'static str,
    action_index: usize,
}

struct PrefixReplay {
    position: CombatPosition,
    replayed_actions: usize,
    potions_used: u32,
}

struct ReplaySummary {
    terminal: SearchTerminalLabel,
    final_hp: i32,
    total_enemy_hp: i32,
    living_enemy_count: usize,
}
