use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::ai::combat_search_v2::pending_choice_action_prefix::canonical_pending_choice_inputs;
use crate::ai::combat_search_v2::{
    filter_combat_search_legal_actions, CombatSearchV2ActionTrace, CombatSearchV2Config,
    CombatSearchV2TrajectoryReport,
};
use crate::content::cards::{get_card_definition, CardType};
use crate::sim::combat::{
    combat_terminal, CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal,
    EngineCombatStepper,
};
use crate::sim::combat_action::CombatActionChoice;
use crate::sim::combat_projection::monster_preview_total_damage_in_combat;
use crate::state::core::ClientInput;

use super::combat_candidate_line::{CombatCandidateLine, CombatCandidateLineSource};

const MIN_REPAIR_HP_LOSS: i32 = 8;
const REPAIR_CUTS: usize = 4;
const REPAIR_NODES: usize = 2_000;
const REPAIR_MS: u64 = 250;
const REPAIR_BEAM: usize = 96;

pub(super) struct CombatLineRepairOutcome {
    pub line: CombatCandidateLine,
    pub attempts: usize,
    pub wins: usize,
    pub improvements: usize,
    pub elapsed_ms: u128,
}

#[derive(Clone)]
struct Line {
    position: CombatPosition,
    actions: Vec<CombatSearchV2ActionTrace>,
    terminal: CombatTerminal,
    score: i64,
    lane: &'static str,
    setup_seen: bool,
}

#[derive(Clone, Copy)]
struct LineSearchConfig {
    nodes: usize,
    ms: u64,
    beam: usize,
    max_actions: usize,
}

struct RepairStats {
    attempts: usize,
    wins: usize,
    improvements: usize,
    elapsed_ms: u128,
}

pub(super) fn try_repair_winning_trajectory(
    start: &CombatPosition,
    trajectory: &CombatSearchV2TrajectoryReport,
    config: &CombatSearchV2Config,
) -> Option<CombatLineRepairOutcome> {
    if trajectory.hp_loss < MIN_REPAIR_HP_LOSS
        || trajectory.potions_used > 0
        || trajectory.potions_discarded > 0
    {
        return None;
    }

    let stepper = EngineCombatStepper;
    let initial_hp = start.combat.entities.player.current_hp;
    let base = line_from_trajectory(start, &trajectory.actions, initial_hp, config, &stepper)?;
    if base.terminal != CombatTerminal::Win {
        return None;
    }

    let (repaired, stats) = repair_line(start, base.clone(), initial_hp, config, &stepper);
    let repaired = repaired?;
    if repaired.position.combat.entities.player.current_hp
        <= base.position.combat.entities.player.current_hp
    {
        return None;
    }
    Some(CombatLineRepairOutcome {
        line: CombatCandidateLine::from_position(
            CombatCandidateLineSource::LineRepair,
            reindex_actions(repaired.actions),
            initial_hp,
            &repaired.position,
        ),
        attempts: stats.attempts,
        wins: stats.wins,
        improvements: stats.improvements,
        elapsed_ms: stats.elapsed_ms,
    })
}

fn line_from_trajectory(
    start: &CombatPosition,
    actions: &[CombatSearchV2ActionTrace],
    initial_hp: i32,
    config: &CombatSearchV2Config,
    stepper: &EngineCombatStepper,
) -> Option<Line> {
    let mut position = start.clone();
    let mut replayed = Vec::new();
    for action in actions {
        let choice = replay_choice(&position, action, config, stepper)?;
        let step = stepper.apply_to_stable(
            &position,
            choice.input.clone(),
            CombatStepLimits {
                max_engine_steps: config.max_engine_steps_per_action,
                deadline: None,
            },
        );
        if step.truncated || step.timed_out {
            return None;
        }
        position = step.position;
        replayed.push(action.clone());
    }
    Some(line_from(
        position, replayed, initial_hp, "root", false, stepper,
    ))
}

fn repair_line(
    root: &CombatPosition,
    mut best: Line,
    initial_hp: i32,
    config: &CombatSearchV2Config,
    stepper: &EngineCombatStepper,
) -> (Option<Line>, RepairStats) {
    let started = Instant::now();
    let mut stats = RepairStats {
        attempts: 0,
        wins: 0,
        improvements: 0,
        elapsed_ms: 0,
    };
    let repair_config = LineSearchConfig {
        nodes: REPAIR_NODES,
        ms: REPAIR_MS,
        beam: REPAIR_BEAM,
        max_actions: config
            .max_actions_per_line
            .min(best.actions.len().saturating_add(20)),
    };
    for cut in repair_cut_points(best.actions.len(), REPAIR_CUTS) {
        let cut = cut.min(best.actions.len());
        stats.attempts += 1;
        let Some(prefix_position) = replay_prefix(root, &best.actions[..cut], config, stepper)
        else {
            continue;
        };
        let Some(suffix_win) =
            line_search_from(prefix_position, initial_hp, repair_config, config, stepper).best_win
        else {
            continue;
        };
        stats.wins += 1;
        let candidate = splice_line(&best, cut, suffix_win, initial_hp, stepper);
        if candidate.score > best.score {
            best = candidate;
            stats.improvements += 1;
        }
    }
    stats.elapsed_ms = started.elapsed().as_millis();
    (Some(best), stats)
}

fn line_search_from(
    start_position: CombatPosition,
    initial_hp: i32,
    search: LineSearchConfig,
    config: &CombatSearchV2Config,
    stepper: &EngineCombatStepper,
) -> LineSearchRun {
    let deadline = Instant::now() + Duration::from_millis(search.ms);
    let mut frontier = vec![line_from(
        start_position,
        Vec::new(),
        initial_hp,
        "root",
        false,
        stepper,
    )];
    let mut best_win = None;
    let mut nodes = 0usize;

    while !frontier.is_empty() && nodes < search.nodes && Instant::now() < deadline {
        let mut next = Vec::new();
        for line in frontier.drain(..) {
            if nodes >= search.nodes || Instant::now() >= deadline {
                break;
            }
            if line.terminal != CombatTerminal::Unresolved
                || line.actions.len() >= search.max_actions
            {
                remember_win(&mut best_win, line);
                continue;
            }
            nodes += 1;
            for (action_id, choice) in
                ordered_repair_choices(&line.position, config, stepper).enumerate()
            {
                if nodes + next.len() >= search.nodes || Instant::now() >= deadline {
                    break;
                }
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
                let setup_seen = line.setup_seen || lane == "setup";
                let child_lane = if setup_seen && lane != "win" {
                    "setup_path"
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
                if child.terminal == CombatTerminal::Win {
                    remember_win(&mut best_win, child);
                } else {
                    next.push(child);
                }
                if nodes + next.len() >= search.nodes || Instant::now() >= deadline {
                    break;
                }
            }
        }
        frontier = keep_lane_frontier(next, search.beam);
    }

    LineSearchRun { best_win }
}

struct LineSearchRun {
    best_win: Option<Line>,
}

fn line_from(
    position: CombatPosition,
    actions: Vec<CombatSearchV2ActionTrace>,
    initial_hp: i32,
    lane: &'static str,
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

fn legal_choices(
    position: &CombatPosition,
    config: &CombatSearchV2Config,
    stepper: &EngineCombatStepper,
) -> Vec<CombatActionChoice> {
    filter_combat_search_legal_actions(
        stepper.atomic_action_choices(position),
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

fn ordered_repair_choices<'a>(
    position: &'a CombatPosition,
    config: &CombatSearchV2Config,
    stepper: &'a EngineCombatStepper,
) -> Box<dyn Iterator<Item = CombatActionChoice> + 'a> {
    if let crate::state::core::EngineState::PendingChoice(choice) = &position.engine {
        if let Some(inputs) = canonical_pending_choice_inputs(choice) {
            return Box::new(
                inputs.filter_map(move |input| stepper.choice_for_legal_input(position, &input)),
            );
        }
    }

    let mut choices = legal_choices(position, config, stepper);
    order_choices(&mut choices);
    Box::new(choices.into_iter())
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
) -> Option<CombatPosition> {
    let mut position = root.clone();
    for action in actions {
        let choice = replay_choice(&position, action, config, stepper)?;
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
        if combat_terminal(&position.engine, &position.combat) != CombatTerminal::Unresolved {
            break;
        }
    }
    Some(position)
}

fn replay_choice(
    position: &CombatPosition,
    action: &CombatSearchV2ActionTrace,
    config: &CombatSearchV2Config,
    stepper: &EngineCombatStepper,
) -> Option<CombatActionChoice> {
    let candidate = stepper.choice_for_legal_input(position, &action.input)?;
    filter_combat_search_legal_actions(vec![candidate], config.potion_policy, &position.combat)
        .into_iter()
        .filter(|choice| {
            !matches!(
                choice.input,
                ClientInput::UsePotion { .. } | ClientInput::DiscardPotion(_)
            )
        })
        .find(|choice| choice.action_key == action.action_key)
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
    let mut counts: HashMap<&'static str, usize> = HashMap::new();
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

fn classify_lane(
    before: &CombatPosition,
    after: &CombatPosition,
    input: &ClientInput,
) -> &'static str {
    if after.combat.are_monsters_basically_dead_java() {
        return "win";
    }
    if played_power(before, input) {
        return "setup";
    }
    if enemy_effort(&after.combat) < enemy_effort(&before.combat) {
        return "progress";
    }
    if visible_pressure(&after.combat) < visible_pressure(&before.combat)
        || after.combat.entities.player.block > before.combat.entities.player.block
    {
        return "survival";
    }
    "other"
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
    let hp = position.combat.entities.player.current_hp;
    let hp_loss = (initial_hp - hp).max(0) as i64;
    let enemy_effort = enemy_effort(&position.combat) as i64;
    let incoming = visible_incoming(&position.combat) as i64;
    match terminal {
        CombatTerminal::Win => 1_000_000 - hp_loss * 10_000 - action_count as i64,
        CombatTerminal::Loss => -1_000_000 - action_count as i64,
        CombatTerminal::Unresolved => {
            hp as i64 * 1_000
                - hp_loss * 1_000
                - enemy_effort * 450
                - incoming.saturating_sub(position.combat.entities.player.block as i64) * 700
                - action_count as i64
        }
    }
}

fn visible_pressure(combat: &crate::runtime::combat::CombatState) -> i32 {
    visible_incoming(combat).saturating_sub(combat.entities.player.block)
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
