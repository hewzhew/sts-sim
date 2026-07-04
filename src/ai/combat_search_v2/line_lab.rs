use std::time::{Duration, Instant};

use serde::Serialize;

use crate::content::cards::{get_card_definition, CardType};
use crate::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal, EngineCombatStepper,
};
use crate::sim::combat_action::CombatActionChoice;
use crate::state::core::ClientInput;

use super::{
    filter_combat_search_legal_actions, run_combat_search_v2, CombatSearchV2ActionTrace,
    CombatSearchV2Config, CombatSearchV2TrajectoryReport, SearchTerminalLabel,
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

#[derive(Clone, Debug, Serialize)]
pub struct CombatLineLabTurnPoolReport {
    pub schema: &'static str,
    pub lanes: Vec<CombatLineLabTurnPoolLineSummary>,
    pub best: Option<CombatLineLabTurnPoolLineSummary>,
    pub nodes_expanded: u64,
    pub deadline_hit: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatLineLabTurnPoolLineSummary {
    pub lane: &'static str,
    pub terminal: SearchTerminalLabel,
    pub final_hp: i32,
    pub total_enemy_hp: i32,
    pub living_enemy_count: usize,
    pub turns: u32,
    pub actions: usize,
    pub potions_used: u32,
    pub powers_played: u32,
}

#[derive(Clone, Debug)]
pub struct CombatLineLabTurnPoolWin {
    pub summary: CombatLineLabTurnPoolLineSummary,
    pub actions: Vec<CombatSearchV2ActionTrace>,
    pub nodes_expanded: u64,
    pub nodes_generated: u64,
    pub deadline_hit: bool,
}

impl CombatLineLabTurnPoolWin {
    pub fn transition_summary(&self) -> String {
        format!(
            "line_lab_turn_pool_rescue lane={} actions={} final_hp={} turns={} potions_used={} powers_played={} nodes={}/{} deadline_hit={}",
            self.summary.lane,
            self.actions.len(),
            self.summary.final_hp,
            self.summary.turns,
            self.summary.potions_used,
            self.summary.powers_played,
            self.nodes_expanded,
            self.nodes_generated,
            self.deadline_hit
        )
    }
}

pub fn find_combat_line_lab_turn_pool_win_v0(
    start: &CombatPosition,
    config: &CombatSearchV2Config,
    budget_ms: u64,
) -> Option<CombatLineLabTurnPoolWin> {
    let run = run_turn_pool_nodes_v1(start, budget_ms, Some(config));
    let best = run
        .lanes
        .into_iter()
        .filter(|candidate| candidate.node.terminal == SearchTerminalLabel::Win)
        .max_by_key(|candidate| {
            turn_pool_summary_rank(&turn_pool_summary(candidate.lane, &candidate.node))
        })?;
    let summary = turn_pool_summary(best.lane, &best.node);
    Some(CombatLineLabTurnPoolWin {
        summary,
        actions: best.node.actions,
        nodes_expanded: run.nodes_expanded,
        nodes_generated: run.nodes_generated,
        deadline_hit: run.deadline_hit,
    })
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
            turn_pool: Some(run_turn_pool_v1(start, repair_budget_ms, Some(&config))),
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
        turn_pool: Some(run_turn_pool_v1(start, turn_pool_budget_ms, Some(&config))),
    }
}

fn per_cut_budget_ms(total_ms: u64, cut_count: usize) -> u64 {
    if cut_count == 0 {
        return total_ms;
    }
    (total_ms / cut_count as u64).clamp(500, 8_000)
}

fn run_turn_pool_v1(
    start: &CombatPosition,
    budget_ms: u64,
    config: Option<&CombatSearchV2Config>,
) -> CombatLineLabTurnPoolReport {
    let run = run_turn_pool_nodes_v1(start, budget_ms, config);
    let lanes = run
        .lanes
        .iter()
        .map(|candidate| turn_pool_summary(candidate.lane, &candidate.node))
        .collect::<Vec<_>>();
    let best = lanes
        .iter()
        .max_by_key(|line| turn_pool_summary_rank(line))
        .cloned();
    CombatLineLabTurnPoolReport {
        schema: "combat_line_lab_turn_pool_v1",
        lanes,
        best,
        nodes_expanded: run.nodes_expanded,
        deadline_hit: run.deadline_hit,
    }
}

fn run_turn_pool_nodes_v1(
    start: &CombatPosition,
    budget_ms: u64,
    config: Option<&CombatSearchV2Config>,
) -> TurnPoolRun {
    const LANES: [TurnPoolLane; 5] = [
        TurnPoolLane::Damage,
        TurnPoolLane::Survival,
        TurnPoolLane::Setup,
        TurnPoolLane::PowerDelay,
        TurnPoolLane::PotionBurst,
    ];
    const LANE_KEEP: usize = 3;
    const INNER_BEAM: usize = 12;
    const MAX_TURNS: usize = 12;
    const MAX_INNER_NODES: usize = 160;

    let stepper = EngineCombatStepper;
    let per_lane_ms = (budget_ms / LANES.len() as u64).max(500);
    let mut total_nodes = 0u64;
    let mut total_generated = 0u64;
    let mut any_deadline_hit = false;
    let mut lane_results = Vec::new();

    for lane in LANES {
        let deadline = Instant::now() + Duration::from_millis(per_lane_ms);
        let mut frontier = vec![TurnPoolNode::root(start.clone(), &stepper)];
        let mut lane_deadline_hit = false;
        for _ in 0..MAX_TURNS {
            if Instant::now() >= deadline {
                lane_deadline_hit = true;
                break;
            }
            let mut next_turn = Vec::new();
            for node in std::mem::take(&mut frontier) {
                if node.terminal != SearchTerminalLabel::Unresolved {
                    next_turn.push(node);
                    continue;
                }
                let outcome = expand_one_turn(
                    node,
                    lane,
                    &stepper,
                    deadline,
                    INNER_BEAM,
                    MAX_INNER_NODES,
                    config,
                );
                total_nodes = total_nodes.saturating_add(outcome.nodes_expanded);
                total_generated = total_generated.saturating_add(outcome.nodes_generated);
                lane_deadline_hit |= outcome.deadline_hit;
                next_turn.extend(outcome.nodes);
                if lane_deadline_hit {
                    break;
                }
            }
            if next_turn.is_empty() {
                break;
            }
            keep_lane_nodes(&mut next_turn, lane, LANE_KEEP);
            let all_terminal = next_turn
                .iter()
                .all(|node| node.terminal != SearchTerminalLabel::Unresolved);
            frontier = next_turn;
            if all_terminal || lane_deadline_hit {
                break;
            }
        }
        any_deadline_hit |= lane_deadline_hit;
        if let Some(best) = frontier
            .into_iter()
            .max_by_key(|node| lane_rank(node, lane))
        {
            lane_results.push(TurnPoolLaneNode { lane, node: best });
        }
    }

    TurnPoolRun {
        lanes: lane_results,
        nodes_expanded: total_nodes,
        nodes_generated: total_generated,
        deadline_hit: any_deadline_hit,
    }
}

fn expand_one_turn(
    root: TurnPoolNode,
    lane: TurnPoolLane,
    stepper: &EngineCombatStepper,
    deadline: Instant,
    beam: usize,
    max_nodes: usize,
    config: Option<&CombatSearchV2Config>,
) -> TurnPoolExpandOutcome {
    let start_turn = root.position.combat.turn.turn_count;
    let mut frontier = vec![root];
    let mut boundary = Vec::new();
    let mut nodes_expanded = 0u64;
    let mut nodes_generated = 0u64;
    let mut deadline_hit = false;
    let boundary_limit = beam.saturating_mul(4).max(beam);

    while !frontier.is_empty() && nodes_expanded < max_nodes as u64 {
        if Instant::now() >= deadline {
            deadline_hit = true;
            break;
        }
        let mut next = Vec::new();
        for node in std::mem::take(&mut frontier) {
            if node.terminal != SearchTerminalLabel::Unresolved
                || node.position.combat.turn.turn_count > start_turn
            {
                boundary.push(node);
                continue;
            }
            nodes_expanded = nodes_expanded.saturating_add(1);
            let choices = match config {
                Some(config) => filter_combat_search_legal_actions(
                    stepper.legal_action_choices(&node.position),
                    config.potion_policy,
                    &node.position.combat,
                ),
                None => stepper.legal_action_choices(&node.position),
            };
            let choices = filter_turn_pool_potion_budget(choices, config, node.potions_used);
            for (action_id, choice) in choices.into_iter().enumerate() {
                if Instant::now() >= deadline {
                    deadline_hit = true;
                    break;
                }
                let played_power = match choice.input {
                    ClientInput::PlayCard { card_index, .. } => {
                        is_power_in_hand(&node.position, card_index)
                    }
                    _ => false,
                };
                let step = stepper.apply_to_stable(
                    &node.position,
                    choice.input.clone(),
                    CombatStepLimits {
                        max_engine_steps: config
                            .map(|config| config.max_engine_steps_per_action)
                            .unwrap_or(250),
                        deadline: Some(deadline),
                    },
                );
                if step.truncated || step.timed_out {
                    deadline_hit |= step.timed_out;
                    continue;
                }
                let mut child = node.child(step.position, &stepper);
                child.note_action(action_id, choice, played_power);
                nodes_generated = nodes_generated.saturating_add(1);
                if child.terminal != SearchTerminalLabel::Unresolved
                    || child.position.combat.turn.turn_count > start_turn
                {
                    boundary.push(child);
                    if boundary.len() >= boundary_limit {
                        break;
                    }
                } else {
                    next.push(child);
                }
            }
            if deadline_hit
                || nodes_expanded >= max_nodes as u64
                || boundary.len() >= boundary_limit
            {
                break;
            }
        }
        if boundary.len() >= boundary_limit {
            break;
        }
        if !next.is_empty() {
            keep_lane_nodes(&mut next, lane, beam);
        }
        frontier = next;
        if deadline_hit {
            break;
        }
    }

    if boundary.is_empty() {
        boundary = frontier;
    }
    keep_lane_nodes(&mut boundary, lane, beam);
    TurnPoolExpandOutcome {
        nodes: boundary,
        nodes_expanded,
        nodes_generated,
        deadline_hit,
    }
}

fn filter_turn_pool_potion_budget(
    choices: Vec<CombatActionChoice>,
    config: Option<&CombatSearchV2Config>,
    potions_used: u32,
) -> Vec<CombatActionChoice> {
    let Some(max_potions) = config.and_then(|config| config.max_potions_used) else {
        return choices;
    };
    if potions_used < max_potions {
        return choices;
    }
    choices
        .into_iter()
        .filter(|choice| !matches!(choice.input, ClientInput::UsePotion { .. }))
        .collect()
}

fn keep_lane_nodes(nodes: &mut Vec<TurnPoolNode>, lane: TurnPoolLane, limit: usize) {
    nodes.sort_by_key(|node| std::cmp::Reverse(lane_rank(node, lane)));
    nodes.truncate(limit);
}

fn lane_rank(node: &TurnPoolNode, lane: TurnPoolLane) -> (i32, i32, i32, i32, i32, i32) {
    let terminal = terminal_rank_for_line(node.terminal);
    let hp = node.position.combat.entities.player.current_hp;
    let enemy_hp = total_enemy_hp(&node.position);
    match lane {
        TurnPoolLane::Damage => (
            terminal,
            -enemy_hp,
            hp,
            -(node.actions.len() as i32),
            -(node.potions_used as i32),
            0,
        ),
        TurnPoolLane::Survival => (
            terminal,
            hp,
            -visible_pressure(&node.position),
            -enemy_hp,
            -(node.potions_used as i32),
            0,
        ),
        TurnPoolLane::Setup => (
            terminal,
            node.powers_played as i32,
            hp,
            -enemy_hp,
            -(node.actions.len() as i32),
            0,
        ),
        TurnPoolLane::PowerDelay => (
            terminal,
            -(node.powers_played as i32),
            -enemy_hp,
            hp,
            -(node.potions_used as i32),
            0,
        ),
        TurnPoolLane::PotionBurst => (
            terminal,
            node.potions_used as i32,
            -enemy_hp,
            hp,
            -(node.actions.len() as i32),
            0,
        ),
    }
}

fn turn_pool_summary(lane: TurnPoolLane, node: &TurnPoolNode) -> CombatLineLabTurnPoolLineSummary {
    CombatLineLabTurnPoolLineSummary {
        lane: lane.label(),
        terminal: node.terminal,
        final_hp: node.position.combat.entities.player.current_hp,
        total_enemy_hp: total_enemy_hp(&node.position),
        living_enemy_count: living_enemy_count(&node.position),
        turns: node.position.combat.turn.turn_count,
        actions: node.actions.len(),
        potions_used: node.potions_used,
        powers_played: node.powers_played,
    }
}

fn turn_pool_summary_rank(line: &CombatLineLabTurnPoolLineSummary) -> (i32, i32, i32, i32) {
    let loss = line.terminal == SearchTerminalLabel::Loss;
    (
        turn_pool_summary_tier(line),
        if loss {
            -line.total_enemy_hp
        } else {
            line.turns as i32
        },
        if loss {
            line.turns as i32
        } else {
            -line.total_enemy_hp
        },
        line.final_hp,
    )
}

fn turn_pool_summary_tier(line: &CombatLineLabTurnPoolLineSummary) -> i32 {
    match line.terminal {
        SearchTerminalLabel::Win => 4,
        SearchTerminalLabel::Loss if line.living_enemy_count == 1 && line.total_enemy_hp <= 50 => 3,
        SearchTerminalLabel::Unresolved => 2,
        SearchTerminalLabel::Loss => 1,
    }
}

fn terminal_rank_for_line(terminal: SearchTerminalLabel) -> i32 {
    match terminal {
        SearchTerminalLabel::Win => 2,
        SearchTerminalLabel::Unresolved => 1,
        SearchTerminalLabel::Loss => 0,
    }
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

fn visible_pressure(position: &CombatPosition) -> i32 {
    position
        .combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .map(|monster| {
            crate::sim::combat_projection::monster_preview_total_damage_in_combat(
                &position.combat,
                monster,
            )
        })
        .sum::<i32>()
        .saturating_sub(position.combat.entities.player.block)
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

struct TurnPoolRun {
    lanes: Vec<TurnPoolLaneNode>,
    nodes_expanded: u64,
    nodes_generated: u64,
    deadline_hit: bool,
}

struct TurnPoolLaneNode {
    lane: TurnPoolLane,
    node: TurnPoolNode,
}

#[derive(Clone, Copy)]
enum TurnPoolLane {
    Damage,
    Survival,
    Setup,
    PowerDelay,
    PotionBurst,
}

impl TurnPoolLane {
    fn label(self) -> &'static str {
        match self {
            Self::Damage => "damage",
            Self::Survival => "survival",
            Self::Setup => "setup",
            Self::PowerDelay => "power_delay",
            Self::PotionBurst => "potion_burst",
        }
    }
}

#[derive(Clone)]
struct TurnPoolNode {
    position: CombatPosition,
    actions: Vec<CombatSearchV2ActionTrace>,
    terminal: SearchTerminalLabel,
    potions_used: u32,
    powers_played: u32,
}

impl TurnPoolNode {
    fn root(position: CombatPosition, stepper: &EngineCombatStepper) -> Self {
        let terminal = search_terminal(stepper.terminal(&position));
        Self {
            position,
            actions: Vec::new(),
            terminal,
            potions_used: 0,
            powers_played: 0,
        }
    }

    fn child(&self, position: CombatPosition, stepper: &EngineCombatStepper) -> Self {
        Self {
            terminal: search_terminal(stepper.terminal(&position)),
            position,
            actions: self.actions.clone(),
            potions_used: self.potions_used,
            powers_played: self.powers_played,
        }
    }

    fn note_action(&mut self, action_id: usize, choice: CombatActionChoice, played_power: bool) {
        if matches!(choice.input, ClientInput::UsePotion { .. }) {
            self.potions_used = self.potions_used.saturating_add(1);
        }
        if played_power {
            self.powers_played = self.powers_played.saturating_add(1);
        }
        self.actions.push(CombatSearchV2ActionTrace {
            step_index: self.actions.len(),
            action_id,
            action_key: choice.action_key,
            action_debug: choice.action_debug,
            input: choice.input,
        });
    }
}

struct TurnPoolExpandOutcome {
    nodes: Vec<TurnPoolNode>,
    nodes_expanded: u64,
    nodes_generated: u64,
    deadline_hit: bool,
}
