use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::ai::combat_search_v2::{
    filter_combat_search_legal_actions, CombatSearchV2ActionTrace, CombatSearchV2Config,
};
use crate::content::cards::{get_card_definition, CardType};
use crate::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal, EngineCombatStepper,
};
use crate::sim::combat_action::CombatActionChoice;
use crate::sim::combat_projection::monster_preview_total_damage_in_combat;
use crate::state::core::ClientInput;

use super::combat_candidate_line::{CombatCandidateLine, CombatCandidateLineSource};

const LINE_BEAM: usize = 128;
const REPAIR_CUTS: usize = 4;
const REPAIR_NODES: usize = 8_000;
const REPAIR_MS: u64 = 500;
const MIN_REPAIR_HP_LOSS: i32 = 8;
const MIN_REPAIR_ACTIONS: usize = 24;

pub(super) struct CompleteLineSolverOutcome {
    pub line: CombatCandidateLine,
    pub base_hp_loss: i32,
    pub base_action_count: usize,
    pub final_hp_loss: i32,
    pub final_action_count: usize,
    pub repair_hp_loss_saved: i32,
    pub repair_action_count_delta: isize,
    pub base_node_budget: usize,
    pub base_ms_budget: u64,
    pub repair_node_budget_per_cut: usize,
    pub repair_ms_budget_per_cut: u64,
    pub repair_cut_budget: usize,
    pub base_stop_reason: &'static str,
    pub last_repair_stop_reason: Option<&'static str>,
    pub repair_attempts: usize,
    pub repair_wins: usize,
    pub repair_improvements: usize,
    pub base_nodes_expanded: usize,
    pub base_nodes_generated: usize,
    pub repair_nodes_expanded: usize,
    pub repair_nodes_generated: usize,
    pub nodes_expanded: usize,
    pub nodes_generated: usize,
    pub elapsed_ms: u128,
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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum LineLane {
    Root,
    Setup,
    SetupPath,
    Progress,
    Survival,
    Other,
}

#[derive(Clone)]
struct Line {
    position: CombatPosition,
    actions: Vec<CombatSearchV2ActionTrace>,
    terminal: CombatTerminal,
    score: i64,
    lane: LineLane,
    setup_seen: bool,
}

#[derive(Clone, Copy)]
struct LineSearchConfig {
    nodes: usize,
    ms: u64,
    beam: usize,
    max_actions: usize,
}

struct LineSearchRun {
    best_win: Option<Line>,
    nodes_expanded: usize,
    nodes_generated: usize,
    stop_reason: LineSearchStopReason,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LineSearchStopReason {
    FrontierEmpty,
    NodeBudget,
    GeneratedBudget,
    Deadline,
}

impl LineSearchStopReason {
    fn label(self) -> &'static str {
        match self {
            LineSearchStopReason::FrontierEmpty => "frontier_empty",
            LineSearchStopReason::NodeBudget => "node_budget",
            LineSearchStopReason::GeneratedBudget => "generated_budget",
            LineSearchStopReason::Deadline => "deadline",
        }
    }
}

#[derive(Clone, Copy)]
struct LineSearchSeed {
    lane: LineLane,
    setup_seen: bool,
}

impl LineSearchSeed {
    fn root() -> Self {
        Self {
            lane: LineLane::Root,
            setup_seen: false,
        }
    }

    fn from_prefix(setup_seen: bool) -> Self {
        Self {
            lane: if setup_seen {
                LineLane::SetupPath
            } else {
                LineLane::Root
            },
            setup_seen,
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

fn line_search_from(
    start_position: CombatPosition,
    initial_hp: i32,
    search: LineSearchConfig,
    config: &CombatSearchV2Config,
    stepper: &EngineCombatStepper,
    seed: LineSearchSeed,
) -> LineSearchRun {
    let deadline = Instant::now() + Duration::from_millis(search.ms);
    let mut frontier = vec![line_from(
        start_position,
        Vec::new(),
        initial_hp,
        seed.lane,
        seed.setup_seen,
        stepper,
    )];
    let mut best_win = None;
    let mut nodes_expanded = 0usize;
    let mut nodes_generated = 0usize;

    while !frontier.is_empty()
        && nodes_expanded < search.nodes
        && nodes_generated < search.nodes
        && Instant::now() < deadline
    {
        let mut next = Vec::new();
        for line in frontier.drain(..) {
            if nodes_expanded >= search.nodes
                || nodes_generated >= search.nodes
                || Instant::now() >= deadline
            {
                break;
            }
            if line.terminal != CombatTerminal::Unresolved
                || line.actions.len() >= search.max_actions
            {
                remember_win(&mut best_win, line);
                continue;
            }
            nodes_expanded += 1;
            let mut choices = legal_non_potion_choices(&line.position, config, stepper);
            order_choices(&mut choices);
            for (action_id, choice) in choices.into_iter().enumerate() {
                let step = stepper.apply_to_stable(
                    &line.position,
                    choice.input.clone(),
                    CombatStepLimits {
                        max_engine_steps: config.max_engine_steps_per_action,
                        deadline: Some(deadline),
                    },
                );
                if step.truncated || step.timed_out {
                    continue;
                }
                let mut actions = line.actions.clone();
                actions.push(CombatSearchV2ActionTrace {
                    step_index: actions.len(),
                    action_id,
                    action_key: choice.action_key,
                    action_debug: choice.action_debug,
                    input: choice.input.clone(),
                });
                let lane = classify_lane(&line.position, &step.position, &choice.input);
                let setup_seen = line.setup_seen || lane == LineLane::Setup;
                let child_lane = if lane == LineLane::Setup {
                    LineLane::Setup
                } else if line.setup_seen {
                    LineLane::SetupPath
                } else {
                    lane
                };
                let child = line_from(
                    step.position,
                    actions,
                    initial_hp,
                    child_lane,
                    setup_seen,
                    stepper,
                );
                nodes_generated += 1;
                match child.terminal {
                    CombatTerminal::Win => remember_win(&mut best_win, child),
                    CombatTerminal::Unresolved => next.push(child),
                    CombatTerminal::Loss => {}
                }
                if nodes_generated >= search.nodes || Instant::now() >= deadline {
                    break;
                }
            }
        }
        frontier = keep_lane_frontier(next, search.beam);
    }

    LineSearchRun {
        best_win,
        nodes_expanded,
        nodes_generated,
        stop_reason: line_search_stop_reason(&frontier, nodes_expanded, nodes_generated, search),
    }
}

fn line_search_stop_reason(
    frontier: &[Line],
    nodes_expanded: usize,
    nodes_generated: usize,
    search: LineSearchConfig,
) -> LineSearchStopReason {
    if frontier.is_empty() {
        LineSearchStopReason::FrontierEmpty
    } else if nodes_generated >= search.nodes {
        LineSearchStopReason::GeneratedBudget
    } else if nodes_expanded >= search.nodes {
        LineSearchStopReason::NodeBudget
    } else {
        LineSearchStopReason::Deadline
    }
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

fn line_from(
    position: CombatPosition,
    actions: Vec<CombatSearchV2ActionTrace>,
    initial_hp: i32,
    lane: LineLane,
    setup_seen: bool,
    stepper: &EngineCombatStepper,
) -> Line {
    let terminal = stepper.terminal(&position);
    let score = score_position(&position, terminal, initial_hp, actions.len());
    Line {
        position,
        actions,
        terminal,
        score,
        lane,
        setup_seen,
    }
}

fn legal_non_potion_choices(
    position: &CombatPosition,
    config: &CombatSearchV2Config,
    stepper: &EngineCombatStepper,
) -> Vec<CombatActionChoice> {
    filter_combat_search_legal_actions(
        stepper.legal_action_choices(position),
        config.potion_policy,
        &position.combat,
    )
    .into_iter()
    .filter(|choice| {
        !matches!(
            choice.input,
            ClientInput::UsePotion { .. } | ClientInput::DiscardPotion(_)
        )
    })
    .collect()
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

fn keep_lane_frontier(mut lines: Vec<Line>, beam: usize) -> Vec<Line> {
    lines.sort_by(|a, b| b.score.cmp(&a.score));
    let per_lane = (beam / 5).max(4);
    let mut kept = Vec::new();
    let mut counts: HashMap<LineLane, usize> = HashMap::new();
    let mut rest = Vec::new();
    for line in lines {
        let count = counts.entry(line.lane).or_default();
        if *count < per_lane && kept.len() < beam {
            *count += 1;
            kept.push(line);
        } else {
            rest.push(line);
        }
    }
    kept.extend(rest.into_iter().take(beam.saturating_sub(kept.len())));
    kept.sort_by(|a, b| b.score.cmp(&a.score));
    kept
}

fn remember_win(best: &mut Option<Line>, line: Line) {
    if line.terminal == CombatTerminal::Win
        && best
            .as_ref()
            .map(|current| line.score > current.score)
            .unwrap_or(true)
    {
        *best = Some(line);
    }
}

fn order_choices(choices: &mut [CombatActionChoice]) {
    choices.sort_by_key(|choice| match choice.input {
        ClientInput::PlayCard { .. } => 0,
        ClientInput::SubmitSelection(_) | ClientInput::SubmitDiscoverChoice(_) => 1,
        ClientInput::EndTurn => 2,
        _ => 3,
    });
}

fn classify_lane(before: &CombatPosition, after: &CombatPosition, input: &ClientInput) -> LineLane {
    if after.combat.are_monsters_basically_dead_java() {
        return LineLane::Progress;
    }
    if played_power(before, input) {
        return LineLane::Setup;
    }
    if enemy_effort(&after.combat) < enemy_effort(&before.combat) {
        return LineLane::Progress;
    }
    if net_visible_pressure(&after.combat) < net_visible_pressure(&before.combat)
        || after.combat.entities.player.block > before.combat.entities.player.block
    {
        return LineLane::Survival;
    }
    LineLane::Other
}

fn played_power(position: &CombatPosition, input: &ClientInput) -> bool {
    let ClientInput::PlayCard { card_index, .. } = input else {
        return false;
    };
    position
        .combat
        .zones
        .hand
        .get(*card_index)
        .is_some_and(|card| get_card_definition(card.id).card_type == CardType::Power)
}

fn score_position(
    position: &CombatPosition,
    terminal: CombatTerminal,
    initial_hp: i32,
    action_count: usize,
) -> i64 {
    let hp_loss = (initial_hp - position.combat.entities.player.current_hp).max(0) as i64;
    let enemy_effort = enemy_effort(&position.combat) as i64;
    let net_pressure = net_visible_pressure(&position.combat) as i64;
    match terminal {
        CombatTerminal::Win => 1_000_000 - hp_loss * 10_000 - action_count as i64,
        CombatTerminal::Loss => -1_000_000 - action_count as i64,
        CombatTerminal::Unresolved => {
            -hp_loss * 2_000 - enemy_effort * 450 - net_pressure * 700 - action_count as i64
        }
    }
}

fn net_visible_pressure(combat: &crate::runtime::combat::CombatState) -> i32 {
    (visible_incoming(combat) - combat.entities.player.block).max(0)
}

fn enemy_effort(combat: &crate::runtime::combat::CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dead_or_escaped())
        .map(|monster| monster.current_hp.max(0) + monster.block.max(0))
        .sum()
}

fn visible_incoming(combat: &crate::runtime::combat::CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dead_or_escaped())
        .map(|monster| monster_preview_total_damage_in_combat(combat, monster))
        .sum()
}
