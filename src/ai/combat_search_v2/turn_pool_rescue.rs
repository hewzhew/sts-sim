use std::time::{Duration, Instant};

use serde::Serialize;

use crate::content::cards::{get_card_definition, CardType};
use crate::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal, EngineCombatStepper,
};
use crate::sim::combat_action::CombatActionChoice;
use crate::state::core::ClientInput;

use super::{
    filter_combat_search_legal_actions, CombatSearchV2ActionTrace, CombatSearchV2Config,
    SearchTerminalLabel,
};

#[derive(Clone, Debug, Serialize)]
pub struct CombatTurnPoolRescueReport {
    pub schema: &'static str,
    pub lanes: Vec<CombatTurnPoolRescueLineSummary>,
    pub best: Option<CombatTurnPoolRescueLineSummary>,
    pub nodes_expanded: u64,
    pub deadline_hit: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatTurnPoolRescueLineSummary {
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
pub struct CombatTurnPoolRescueWin {
    pub summary: CombatTurnPoolRescueLineSummary,
    pub actions: Vec<CombatSearchV2ActionTrace>,
    pub nodes_expanded: u64,
    pub nodes_generated: u64,
    pub deadline_hit: bool,
}

impl CombatTurnPoolRescueWin {
    pub fn transition_summary(&self) -> String {
        format!(
            "turn_pool_rescue lane={} actions={} final_hp={} turns={} potions_used={} powers_played={} nodes={}/{} deadline_hit={}",
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

pub fn find_combat_turn_pool_rescue_win_v0(
    start: &CombatPosition,
    config: &CombatSearchV2Config,
    budget_ms: u64,
) -> Option<CombatTurnPoolRescueWin> {
    let run = run_turn_pool_nodes_v0(start, budget_ms, Some(config));
    let best = run
        .lanes
        .into_iter()
        .filter(|candidate| candidate.node.terminal == SearchTerminalLabel::Win)
        .max_by_key(|candidate| {
            turn_pool_summary_rank(&turn_pool_summary(candidate.lane, &candidate.node))
        })?;
    let summary = turn_pool_summary(best.lane, &best.node);
    Some(CombatTurnPoolRescueWin {
        summary,
        actions: best.node.actions,
        nodes_expanded: run.nodes_expanded,
        nodes_generated: run.nodes_generated,
        deadline_hit: run.deadline_hit,
    })
}

pub fn run_combat_turn_pool_rescue_report_v0(
    start: &CombatPosition,
    budget_ms: u64,
    config: Option<&CombatSearchV2Config>,
) -> CombatTurnPoolRescueReport {
    let run = run_turn_pool_nodes_v0(start, budget_ms, config);
    let lanes = run
        .lanes
        .iter()
        .map(|candidate| turn_pool_summary(candidate.lane, &candidate.node))
        .collect::<Vec<_>>();
    let best = lanes
        .iter()
        .max_by_key(|line| turn_pool_summary_rank(line))
        .cloned();
    CombatTurnPoolRescueReport {
        schema: "combat_turn_pool_rescue_v0",
        lanes,
        best,
        nodes_expanded: run.nodes_expanded,
        deadline_hit: run.deadline_hit,
    }
}

fn run_turn_pool_nodes_v0(
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
                let mut child = node.child(step.position, stepper);
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

fn turn_pool_summary(lane: TurnPoolLane, node: &TurnPoolNode) -> CombatTurnPoolRescueLineSummary {
    CombatTurnPoolRescueLineSummary {
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

fn turn_pool_summary_rank(line: &CombatTurnPoolRescueLineSummary) -> (i32, i32, i32, i32) {
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

fn turn_pool_summary_tier(line: &CombatTurnPoolRescueLineSummary) -> i32 {
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

fn is_power_in_hand(position: &CombatPosition, card_index: usize) -> bool {
    position
        .combat
        .zones
        .hand
        .get(card_index)
        .is_some_and(|card| get_card_definition(card.id).card_type == CardType::Power)
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
