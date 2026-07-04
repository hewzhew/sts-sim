use std::time::{Duration, Instant};

use crate::ai::combat_search_v2::{CombatSearchV2ActionTrace, CombatSearchV2Config};
use crate::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal, EngineCombatStepper,
};

use super::combat_candidate_line::{CombatCandidateLine, CombatCandidateLineSource};
use super::combat_complete_line_scoring::played_power;
use super::combat_complete_line_search::{
    legal_non_potion_choices, line_from, line_search_from, Line, LineSearchConfig, LineSearchSeed,
    LineSearchStopReason,
};

const LINE_BEAM: usize = 128;
const REPAIR_CUTS: usize = 4;
const REPAIR_NODES: usize = 8_000;
const REPAIR_MS: u64 = 500;
const MIN_REPAIR_HP_LOSS: i32 = 8;
const MIN_REPAIR_ACTIONS: usize = 24;

pub(super) struct CompleteLineSolverOutcome {
    pub line: CombatCandidateLine,
    base_hp_loss: i32,
    base_action_count: usize,
    final_hp_loss: i32,
    final_action_count: usize,
    repair_hp_loss_saved: i32,
    repair_action_count_delta: isize,
    base_node_budget: usize,
    base_ms_budget: u64,
    repair_node_budget_per_cut: usize,
    repair_ms_budget_per_cut: u64,
    repair_cut_budget: usize,
    base_stop_reason: &'static str,
    last_repair_stop_reason: Option<&'static str>,
    repair_attempts: usize,
    repair_wins: usize,
    repair_improvements: usize,
    base_nodes_expanded: usize,
    base_nodes_generated: usize,
    repair_nodes_expanded: usize,
    repair_nodes_generated: usize,
    pub nodes_expanded: usize,
    pub nodes_generated: usize,
    pub elapsed_ms: u128,
}

impl CompleteLineSolverOutcome {
    pub(super) fn transition_summary(&self) -> String {
        format!(
            "complete_line_solver actions={}/{} delta={} hp_loss={}/{} saved={} budget=base:{}/{}ms repair:{}x{}/{}ms stops={}/{} nodes={} generated={} base_nodes={}/{} repair_nodes={}/{} repair={}/{}/{} elapsed_ms={}",
            self.final_action_count,
            self.base_action_count,
            self.repair_action_count_delta,
            self.final_hp_loss,
            self.base_hp_loss,
            self.repair_hp_loss_saved,
            self.base_node_budget,
            self.base_ms_budget,
            self.repair_cut_budget,
            self.repair_node_budget_per_cut,
            self.repair_ms_budget_per_cut,
            self.base_stop_reason,
            self.last_repair_stop_reason.unwrap_or("none"),
            self.nodes_expanded,
            self.nodes_generated,
            self.base_nodes_expanded,
            self.base_nodes_generated,
            self.repair_nodes_expanded,
            self.repair_nodes_generated,
            self.repair_attempts,
            self.repair_wins,
            self.repair_improvements,
            self.elapsed_ms
        )
    }
}

#[derive(Clone, Copy)]
struct CompleteLineSolverBudget {
    base_nodes: usize,
    base_ms: u64,
    repair_nodes_per_cut: usize,
    repair_ms_per_cut: u64,
    repair_cuts: usize,
}

impl CompleteLineSolverBudget {
    fn from_search_config(config: &CombatSearchV2Config) -> Self {
        Self {
            base_nodes: config.max_nodes,
            base_ms: config
                .wall_time
                .unwrap_or_else(|| Duration::from_millis(2_000))
                .as_millis()
                .min(u128::from(u64::MAX)) as u64,
            repair_nodes_per_cut: REPAIR_NODES,
            repair_ms_per_cut: REPAIR_MS,
            repair_cuts: REPAIR_CUTS,
        }
    }

    fn base_search(self, max_actions: usize) -> LineSearchConfig {
        LineSearchConfig {
            nodes: self.base_nodes,
            ms: self.base_ms,
            beam: LINE_BEAM,
            max_actions,
        }
    }

    fn repair_search(self, max_actions: usize) -> LineSearchConfig {
        LineSearchConfig {
            nodes: self.repair_nodes_per_cut,
            ms: self.repair_ms_per_cut,
            beam: LINE_BEAM,
            max_actions,
        }
    }
}

#[derive(Default)]
struct RepairStats {
    attempts: usize,
    wins: usize,
    improvements: usize,
    nodes_expanded: usize,
    nodes_generated: usize,
    last_stop_reason: Option<LineSearchStopReason>,
}

struct ReplayedPrefix {
    position: CombatPosition,
    setup_seen: bool,
}

pub(super) fn try_solve_complete_line(
    start: &CombatPosition,
    config: &CombatSearchV2Config,
) -> Option<CompleteLineSolverOutcome> {
    let started = Instant::now();
    let stepper = EngineCombatStepper;
    let initial_hp = start.combat.entities.player.current_hp;
    let budget = CompleteLineSolverBudget::from_search_config(config);
    let run = line_search_from(
        start.clone(),
        initial_hp,
        budget.base_search(config.max_actions_per_line),
        config,
        &stepper,
        LineSearchSeed::root(),
    );
    let base = run.best_win?;
    let base_hp_loss = (initial_hp - base.position.combat.entities.player.current_hp).max(0);
    let base_action_count = base.actions.len();
    let (best, repair) = repair_line_if_useful(start, base, initial_hp, budget, config, &stepper);
    let final_hp_loss = (initial_hp - best.position.combat.entities.player.current_hp).max(0);
    let final_action_count = best.actions.len();
    Some(CompleteLineSolverOutcome {
        line: CombatCandidateLine::from_position(
            CombatCandidateLineSource::CompleteLineSolver,
            reindex_actions(best.actions),
            initial_hp,
            &best.position,
        ),
        base_hp_loss,
        base_action_count,
        final_hp_loss,
        final_action_count,
        repair_hp_loss_saved: base_hp_loss - final_hp_loss,
        repair_action_count_delta: final_action_count as isize - base_action_count as isize,
        base_node_budget: budget.base_nodes,
        base_ms_budget: budget.base_ms,
        repair_node_budget_per_cut: budget.repair_nodes_per_cut,
        repair_ms_budget_per_cut: budget.repair_ms_per_cut,
        repair_cut_budget: budget.repair_cuts,
        base_stop_reason: run.stop_reason.label(),
        last_repair_stop_reason: repair.last_stop_reason.map(LineSearchStopReason::label),
        repair_attempts: repair.attempts,
        repair_wins: repair.wins,
        repair_improvements: repair.improvements,
        base_nodes_expanded: run.nodes_expanded,
        base_nodes_generated: run.nodes_generated,
        repair_nodes_expanded: repair.nodes_expanded,
        repair_nodes_generated: repair.nodes_generated,
        nodes_expanded: run.nodes_expanded + repair.nodes_expanded,
        nodes_generated: run.nodes_generated + repair.nodes_generated,
        elapsed_ms: started.elapsed().as_millis(),
    })
}

fn repair_line_if_useful(
    root: &CombatPosition,
    mut best: Line,
    initial_hp: i32,
    budget: CompleteLineSolverBudget,
    config: &CombatSearchV2Config,
    stepper: &EngineCombatStepper,
) -> (Line, RepairStats) {
    let mut stats = RepairStats::default();
    if !should_repair_line(&best, initial_hp) {
        return (best, stats);
    }
    let repair_base = best.clone();
    for cut in repair_cut_points(repair_base.actions.len(), budget.repair_cuts) {
        let cut = cut.min(repair_base.actions.len());
        let remaining_actions = config.max_actions_per_line.saturating_sub(cut);
        if remaining_actions == 0 {
            continue;
        }
        stats.attempts += 1;
        let Some(prefix) = replay_prefix(root, &repair_base.actions[..cut], config, stepper) else {
            continue;
        };
        let repair_run = line_search_from(
            prefix.position,
            initial_hp,
            budget.repair_search(remaining_actions),
            config,
            stepper,
            LineSearchSeed::from_prefix(prefix.setup_seen),
        );
        stats.nodes_expanded += repair_run.nodes_expanded;
        stats.nodes_generated += repair_run.nodes_generated;
        stats.last_stop_reason = Some(repair_run.stop_reason);
        let Some(suffix_win) = repair_run.best_win else {
            continue;
        };
        stats.wins += 1;
        let candidate = splice_line(&repair_base, cut, suffix_win, initial_hp, stepper);
        if candidate.score > best.score {
            best = candidate;
            stats.improvements += 1;
        }
    }
    (best, stats)
}

fn should_repair_line(line: &Line, initial_hp: i32) -> bool {
    if line.terminal != CombatTerminal::Win {
        return false;
    }
    let hp_loss = (initial_hp - line.position.combat.entities.player.current_hp).max(0);
    hp_loss >= MIN_REPAIR_HP_LOSS || line.actions.len() >= MIN_REPAIR_ACTIONS
}

fn repair_cut_points(len: usize, limit: usize) -> Vec<usize> {
    let count = len.min(limit);
    (0..count).map(|index| index * len / count).collect()
}

fn replay_prefix(
    root: &CombatPosition,
    actions: &[CombatSearchV2ActionTrace],
    config: &CombatSearchV2Config,
    stepper: &EngineCombatStepper,
) -> Option<ReplayedPrefix> {
    let mut position = root.clone();
    let mut setup_seen = false;
    for action in actions {
        let choices = legal_non_potion_choices(&position, config, stepper);
        let choice = choices.into_iter().find(|choice| {
            choice.input == action.input && choice.action_key == action.action_key
        })?;
        setup_seen |= played_power(&position, &choice.input);
        let step = stepper.apply_to_stable(
            &position,
            choice.input,
            CombatStepLimits {
                max_engine_steps: config.max_engine_steps_per_action,
                deadline: None,
            },
        );
        if step.truncated || step.timed_out {
            return None;
        }
        position = step.position;
        if stepper.terminal(&position) != CombatTerminal::Unresolved {
            break;
        }
    }
    Some(ReplayedPrefix {
        position,
        setup_seen,
    })
}

fn splice_line(
    prefix: &Line,
    cut: usize,
    suffix: Line,
    initial_hp: i32,
    stepper: &EngineCombatStepper,
) -> Line {
    let mut actions = prefix.actions[..cut].to_vec();
    actions.extend(suffix.actions);
    line_from(
        suffix.position,
        reindex_actions(actions),
        initial_hp,
        suffix.lane,
        prefix.setup_seen || suffix.setup_seen,
        stepper,
    )
}

fn reindex_actions(mut actions: Vec<CombatSearchV2ActionTrace>) -> Vec<CombatSearchV2ActionTrace> {
    for (index, action) in actions.iter_mut().enumerate() {
        action.step_index = index;
    }
    actions
}
