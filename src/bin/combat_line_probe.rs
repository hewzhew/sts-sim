use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::Parser;
use serde::Deserialize;
use serde_json::json;
use sts_simulator::content::cards::{get_card_definition, CardType};
use sts_simulator::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal, EngineCombatStepper,
};
use sts_simulator::sim::combat_action::CombatActionChoice;
use sts_simulator::sim::combat_projection::monster_preview_total_damage_in_combat;
use sts_simulator::state::core::ClientInput;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    case: PathBuf,
    #[arg(long, default_value_t = 5_000)]
    nodes: usize,
    #[arg(long, default_value_t = 1_000)]
    ms: u64,
    #[arg(long, default_value_t = 96)]
    beam: usize,
    #[arg(long, default_value_t = 80)]
    max_actions: usize,
    #[arg(long, default_value_t = 0)]
    per_state_actions: usize,
    #[arg(long, default_value_t = 0)]
    repair_cuts: usize,
    #[arg(long, default_value_t = 2_000)]
    repair_nodes: usize,
    #[arg(long, default_value_t = 250)]
    repair_ms: u64,
    #[arg(long)]
    json: bool,
}

#[derive(Deserialize)]
struct CombatGapCase {
    schema: String,
    source: serde_json::Value,
    gap: serde_json::Value,
    run: serde_json::Value,
    combat: serde_json::Value,
    position: CombatPosition,
}

#[derive(Clone)]
struct Line {
    position: CombatPosition,
    inputs: Vec<ClientInput>,
    actions: Vec<String>,
    terminal: CombatTerminal,
    score: i64,
    lane: &'static str,
    setup_seen: bool,
}

#[derive(Clone, Copy)]
struct SearchConfig {
    nodes: usize,
    ms: u64,
    beam: usize,
    max_actions: usize,
    per_state_actions: usize,
}

struct SearchRun {
    best_win: Option<Line>,
    best_frontier: Option<Line>,
    nodes_expanded: usize,
    nodes_generated: usize,
    truncated: bool,
    elapsed_ms: u128,
}

#[derive(Default)]
struct RepairStats {
    attempts: usize,
    wins: usize,
    improvements: usize,
    best_cut: Option<usize>,
    elapsed_ms: u128,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let started = Instant::now();
    let case = load_case(&args.case)?;
    let initial_hp = case.position.combat.entities.player.current_hp;
    let stepper = EngineCombatStepper;
    let config = SearchConfig {
        nodes: args.nodes,
        ms: args.ms,
        beam: args.beam,
        max_actions: args.max_actions,
        per_state_actions: args.per_state_actions,
    };
    let run = line_search_from(case.position.clone(), initial_hp, config, &stepper);
    let (repaired_win, repair_stats) = match run.best_win.clone() {
        Some(best) => repair_line(&case.position, best, initial_hp, &args, &stepper),
        None => (None, RepairStats::default()),
    };
    let base_best_win = run.best_win.clone();
    let best_win = repaired_win.or(base_best_win.clone());

    let report = json!({
        "schema": "combat_line_probe",
        "case": case_header(&case),
        "budget": {
            "nodes": args.nodes,
            "ms": args.ms,
            "beam": args.beam,
            "max_actions": args.max_actions,
            "per_state_actions": args.per_state_actions,
            "repair_cuts": args.repair_cuts,
            "repair_nodes": args.repair_nodes,
            "repair_ms": args.repair_ms,
        },
        "stats": {
            "nodes_expanded": run.nodes_expanded,
            "nodes_generated": run.nodes_generated,
            "elapsed_ms": started.elapsed().as_millis(),
            "search_elapsed_ms": run.elapsed_ms,
            "truncated": run.truncated,
        },
        "repair": {
            "attempts": repair_stats.attempts,
            "wins": repair_stats.wins,
            "improvements": repair_stats.improvements,
            "best_cut": repair_stats.best_cut,
            "elapsed_ms": repair_stats.elapsed_ms,
        },
        "base_best_win": base_best_win.as_ref().map(|line| line_summary(line, initial_hp)),
        "best_win": best_win.as_ref().map(|line| line_summary(line, initial_hp)),
        "best_frontier": run.best_frontier.as_ref().map(|line| line_summary(line, initial_hp)),
    });

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_human(&report);
    }
    Ok(())
}

fn line_search_from(
    start_position: CombatPosition,
    initial_hp: i32,
    config: SearchConfig,
    stepper: &EngineCombatStepper,
) -> SearchRun {
    let started = Instant::now();
    let deadline = started + Duration::from_millis(config.ms);
    let mut frontier = vec![line_from(
        start_position,
        Vec::new(),
        Vec::new(),
        initial_hp,
        "root",
        false,
        &stepper,
    )];
    let mut best_win: Option<Line> = None;
    let mut nodes_expanded = 0usize;
    let mut nodes_generated = 0usize;
    let mut truncated = false;

    while !frontier.is_empty() && nodes_expanded < config.nodes && Instant::now() < deadline {
        let mut next = Vec::new();
        for line in frontier.drain(..) {
            if nodes_expanded >= config.nodes || Instant::now() >= deadline {
                truncated = true;
                break;
            }
            if line.terminal != CombatTerminal::Unresolved
                || line.actions.len() >= config.max_actions
            {
                remember_win(&mut best_win, line);
                continue;
            }
            nodes_expanded += 1;
            let mut choices = no_potion_actions(stepper.legal_action_choices(&line.position));
            order_choices(&mut choices);
            if config.per_state_actions > 0 {
                choices.truncate(config.per_state_actions);
            }
            for choice in choices {
                let input = choice.input.clone();
                let step = stepper.apply_to_stable(
                    &line.position,
                    input.clone(),
                    CombatStepLimits {
                        max_engine_steps: 250,
                        deadline: Some(deadline),
                    },
                );
                if step.truncated || step.timed_out {
                    truncated = true;
                    continue;
                }
                let mut inputs = line.inputs.clone();
                let mut actions = line.actions.clone();
                inputs.push(input.clone());
                actions.push(choice.action_key);
                let lane = classify_lane(&line.position, &step.position, &input);
                let setup_seen = line.setup_seen || lane == "setup";
                let child_lane = if setup_seen && lane != "win" {
                    "setup_path"
                } else {
                    lane
                };
                let child = line_from(
                    step.position,
                    inputs,
                    actions,
                    initial_hp,
                    child_lane,
                    setup_seen,
                    &stepper,
                );
                nodes_generated += 1;
                if child.terminal == CombatTerminal::Win {
                    remember_win(&mut best_win, child);
                } else {
                    next.push(child);
                }
                if Instant::now() >= deadline || nodes_generated >= config.nodes {
                    truncated = true;
                    break;
                }
            }
        }
        frontier = keep_lane_frontier(next, config.beam);
    }

    let best_frontier = frontier.into_iter().max_by_key(|line| line.score);

    SearchRun {
        best_win,
        best_frontier,
        nodes_expanded,
        nodes_generated,
        truncated,
        elapsed_ms: started.elapsed().as_millis(),
    }
}

fn load_case(path: &PathBuf) -> Result<CombatGapCase, String> {
    let payload = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let case: CombatGapCase = serde_json::from_str(&payload).map_err(|err| err.to_string())?;
    if case.schema != "combat_gap_case" {
        return Err(format!("expected combat_gap_case, got {}", case.schema));
    }
    Ok(case)
}

fn line_from(
    position: CombatPosition,
    inputs: Vec<ClientInput>,
    actions: Vec<String>,
    initial_hp: i32,
    lane: &'static str,
    setup_seen: bool,
    stepper: &EngineCombatStepper,
) -> Line {
    let terminal = stepper.terminal(&position);
    let score = score_position(&position, terminal, initial_hp, actions.len());
    Line {
        position,
        inputs,
        actions,
        terminal,
        score,
        lane,
        setup_seen,
    }
}

fn repair_line(
    root: &CombatPosition,
    mut best: Line,
    initial_hp: i32,
    args: &Args,
    stepper: &EngineCombatStepper,
) -> (Option<Line>, RepairStats) {
    let started = Instant::now();
    let mut stats = RepairStats::default();
    if args.repair_cuts == 0 || best.terminal != CombatTerminal::Win {
        return (Some(best), stats);
    }
    let config = SearchConfig {
        nodes: args.repair_nodes,
        ms: args.repair_ms,
        beam: args.beam,
        max_actions: args.max_actions,
        per_state_actions: args.per_state_actions,
    };
    for cut in repair_cut_points(best.inputs.len(), args.repair_cuts) {
        stats.attempts += 1;
        let Some(prefix_position) = replay_prefix(root, &best.inputs[..cut], stepper) else {
            continue;
        };
        let Some(suffix_win) =
            line_search_from(prefix_position, initial_hp, config, stepper).best_win
        else {
            continue;
        };
        stats.wins += 1;
        let candidate = splice_line(&best, cut, suffix_win, initial_hp, stepper);
        if candidate.score > best.score {
            best = candidate;
            stats.improvements += 1;
            stats.best_cut = Some(cut);
        }
    }
    stats.elapsed_ms = started.elapsed().as_millis();
    (Some(best), stats)
}

fn repair_cut_points(len: usize, limit: usize) -> Vec<usize> {
    let count = len.min(limit);
    if count == 0 {
        return Vec::new();
    }
    let mut points = Vec::new();
    for i in 0..count {
        let cut = i * len / count;
        if cut < len && !points.contains(&cut) {
            points.push(cut);
        }
    }
    points
}

fn replay_prefix(
    root: &CombatPosition,
    inputs: &[ClientInput],
    stepper: &EngineCombatStepper,
) -> Option<CombatPosition> {
    let mut position = root.clone();
    for input in inputs {
        let step = stepper.apply_to_stable(
            &position,
            input.clone(),
            CombatStepLimits {
                max_engine_steps: 250,
                deadline: None,
            },
        );
        if step.truncated || step.timed_out {
            return None;
        }
        position = step.position;
        if step.terminal != CombatTerminal::Unresolved {
            break;
        }
    }
    Some(position)
}

fn splice_line(
    prefix: &Line,
    cut: usize,
    suffix: Line,
    initial_hp: i32,
    stepper: &EngineCombatStepper,
) -> Line {
    let mut inputs = prefix.inputs[..cut].to_vec();
    let mut actions = prefix.actions[..cut].to_vec();
    inputs.extend(suffix.inputs);
    actions.extend(suffix.actions);
    line_from(
        suffix.position,
        inputs,
        actions,
        initial_hp,
        suffix.lane,
        prefix.setup_seen || suffix.setup_seen,
        stepper,
    )
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
    if line.terminal != CombatTerminal::Win {
        return;
    }
    let replace = best
        .as_ref()
        .map(|current| line.score > current.score)
        .unwrap_or(true);
    if replace {
        *best = Some(line);
    }
}

fn no_potion_actions(choices: Vec<CombatActionChoice>) -> Vec<CombatActionChoice> {
    choices
        .into_iter()
        .filter(|choice| {
            !matches!(
                choice.input,
                ClientInput::UsePotion { .. } | ClientInput::DiscardPotion(_)
            )
        })
        .collect()
}

fn order_choices(choices: &mut [CombatActionChoice]) {
    choices.sort_by_key(|choice| action_order_hint(&choice.input));
}

fn action_order_hint(input: &ClientInput) -> i32 {
    match input {
        ClientInput::PlayCard { .. } => 0,
        ClientInput::SubmitSelection(_) | ClientInput::SubmitDiscoverChoice(_) => 1,
        ClientInput::EndTurn => 2,
        _ => 3,
    }
}

fn classify_lane(
    before: &CombatPosition,
    after: &CombatPosition,
    input: &ClientInput,
) -> &'static str {
    match after.combat.are_monsters_basically_dead_java() {
        true => return "win",
        false => {}
    }
    if played_power(before, input) {
        return "setup";
    }
    if enemy_effort(&after.combat) < enemy_effort(&before.combat) {
        return "progress";
    }
    let before_pressure = visible_pressure(&before.combat);
    let after_pressure = visible_pressure(&after.combat);
    if after_pressure < before_pressure
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

fn visible_pressure(combat: &sts_simulator::runtime::combat::CombatState) -> i32 {
    visible_incoming(combat).saturating_sub(combat.entities.player.block)
}

fn enemy_effort(combat: &sts_simulator::runtime::combat::CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dead_or_escaped())
        .map(|monster| monster.current_hp.max(0) + monster.block.max(0))
        .sum()
}

fn visible_incoming(combat: &sts_simulator::runtime::combat::CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dead_or_escaped())
        .map(|monster| monster_preview_total_damage_in_combat(combat, monster))
        .sum()
}

fn case_header(case: &CombatGapCase) -> serde_json::Value {
    json!({
        "source": case.source,
        "gap": case.gap,
        "run": case.run,
        "combat": case.combat,
    })
}

fn line_summary(line: &Line, initial_hp: i32) -> serde_json::Value {
    let final_hp = line.position.combat.entities.player.current_hp;
    json!({
        "terminal": line.terminal,
        "final_hp": final_hp,
        "hp_loss": (initial_hp - final_hp).max(0),
        "score": line.score,
        "lane": line.lane,
        "action_count": line.actions.len(),
        "actions": line.actions.iter().take(32).collect::<Vec<_>>(),
    })
}

fn print_human(report: &serde_json::Value) {
    println!("combat_line_probe");
    println!("  budget: {}", one_line(&report["budget"]));
    println!("  stats: {}", one_line(&report["stats"]));
    println!("  repair: {}", one_line(&report["repair"]));
    print_line("base_best_win", &report["base_best_win"]);
    print_line("best_win", &report["best_win"]);
    if report["best_win"].is_null() {
        print_line("best_frontier", &report["best_frontier"]);
    }
}

fn print_line(label: &str, line: &serde_json::Value) {
    if line.is_null() {
        println!("  {label}: null");
        return;
    }
    println!(
        "  {label}: terminal={} final_hp={} hp_loss={} actions={} score={}",
        line["terminal"], line["final_hp"], line["hp_loss"], line["action_count"], line["score"]
    );
    println!("    path: {}", one_line(&line["actions"]));
}

fn one_line(value: &serde_json::Value) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "<json>".to_string())
        .chars()
        .take(4096)
        .collect()
}
