use std::collections::VecDeque;

use sts_simulator::eval::run_control::{
    build_decision_surface, RunControlAutoStepOptions, RunControlCommand, RunControlConfig,
    RunControlHpLossLimit, RunControlRouteAutomationMode, RunControlSearchCombatOptions,
    RunControlSession,
};
use sts_simulator::state::core::{EngineState, RunResult};

#[derive(Clone)]
struct Branch {
    id: String,
    path: Vec<String>,
    session: RunControlSession,
    stop: String,
}

#[derive(Clone, Copy)]
struct Args {
    seed: u64,
    ascension: u8,
    layers: usize,
    max_branches: usize,
    auto_ops: usize,
    search_nodes: usize,
    search_ms: u64,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let mut session = RunControlSession::new(RunControlConfig {
        seed: args.seed,
        ascension_level: args.ascension,
        reward_automation: sts_simulator::eval::run_control::RewardAutomationConfig {
            claim_gold: false,
            claim_potion_with_empty_slot: false,
            claim_safe_relic_without_sapphire_key: false,
        },
        ..Default::default()
    });
    let root_stop = advance(&mut session, args);
    let mut frontier = VecDeque::from([Branch {
        id: "root".to_string(),
        path: Vec::new(),
        session,
        stop: root_stop,
    }]);
    println!(
        "branch_tiny seed={} ascension={} layers={} max_branches={}",
        args.seed, args.ascension, args.layers, args.max_branches
    );
    for layer in 0..=args.layers {
        println!("layer {layer} branches={}", frontier.len());
        let mut next = VecDeque::new();
        let mut truncated = false;
        while let Some(branch) = frontier.pop_front() {
            print_branch(&branch);
            if layer == args.layers || !can_expand(&branch) {
                continue;
            }
            for child in expand(&branch, args) {
                if next.len() >= args.max_branches {
                    truncated = true;
                    break;
                }
                next.push_back(child);
            }
        }
        if truncated {
            println!("  layer_truncated cap={}", args.max_branches);
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
    }
    Ok(())
}

fn expand(branch: &Branch, args: Args) -> Vec<Branch> {
    let surface = build_decision_surface(&branch.session);
    let candidates = surface
        .view
        .candidates
        .iter()
        .filter(|candidate| !is_navigation_only_candidate(&candidate.id))
        .filter(|candidate| candidate.action.executable_input().is_some())
        .map(|candidate| (candidate.id.clone(), candidate.label.clone()))
        .collect::<Vec<_>>();
    let mut children = Vec::new();
    for (index, (candidate_id, label)) in candidates.into_iter().enumerate() {
        let mut session = branch.session.clone();
        let apply = session.apply_command(RunControlCommand::Candidate(candidate_id.clone()));
        let stop = match apply {
            Ok(_) => advance(&mut session, args),
            Err(err) => format!("apply_failed: {err}"),
        };
        let mut path = branch.path.clone();
        path.push(format!("{candidate_id}:{label}"));
        children.push(Branch {
            id: format!("{}.{}", branch.id, index),
            path,
            session,
            stop,
        });
    }
    children
}

fn advance(session: &mut RunControlSession, args: Args) -> String {
    let options = RunControlAutoStepOptions {
        search: RunControlSearchCombatOptions {
            max_nodes: Some(args.search_nodes),
            wall_ms: Some(args.search_ms),
            max_hp_loss: Some(RunControlHpLossLimit::Unlimited),
            ..Default::default()
        },
        max_operations: Some(args.auto_ops),
        route: RunControlRouteAutomationMode::Manual,
    };
    match session.apply_command(RunControlCommand::AutoRun(options)) {
        Ok(outcome) => first_reason(&outcome.message).unwrap_or_else(|| "boundary".to_string()),
        Err(err) => format!("advance_failed: {err}"),
    }
}

fn print_branch(branch: &Branch) {
    let surface = build_decision_surface(&branch.session);
    let terminal = terminal_label(&branch.session).unwrap_or("-");
    let candidates = visible_action_labels(&surface);
    println!(
        "  {} A{}F{} hp={}/{} deck={} terminal={} choices={} boundary=\"{}\" stop=\"{}\" path=\"{}\"",
        branch.id,
        branch.session.run_state.act_num,
        branch.session.run_state.floor_num,
        branch.session.run_state.current_hp,
        branch.session.run_state.max_hp,
        branch.session.run_state.master_deck.len(),
        terminal,
        candidates.len(),
        surface.view.header.title,
        branch.stop,
        if branch.path.is_empty() {
            "-".to_string()
        } else {
            branch.path.join(" -> ")
        }
    );
    if !candidates.is_empty() {
        println!("    candidates: {}", candidates.join(" | "));
    }
}

fn visible_action_labels(
    surface: &sts_simulator::eval::run_control::DecisionSurface,
) -> Vec<String> {
    surface
        .view
        .candidates
        .iter()
        .filter(|candidate| !is_navigation_only_candidate(&candidate.id))
        .filter(|candidate| candidate.action.executable_input().is_some())
        .map(|candidate| format!("{}:{}", candidate.id, candidate.label))
        .collect()
}

fn terminal_label(session: &RunControlSession) -> Option<&'static str> {
    match session.engine_state {
        EngineState::GameOver(RunResult::Victory) => Some("victory"),
        EngineState::GameOver(RunResult::Defeat) => Some("defeat"),
        _ => None,
    }
}

fn first_reason(message: &str) -> Option<String> {
    message
        .lines()
        .find_map(|line| line.strip_prefix("Reason: ").map(str::to_string))
}

fn is_navigation_only_candidate(id: &str) -> bool {
    matches!(id, "back" | "cancel")
}

fn can_expand(branch: &Branch) -> bool {
    terminal_label(&branch.session).is_none()
        && !branch.stop.starts_with("apply_failed:")
        && !branch.stop.starts_with("advance_failed:")
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        seed: 1,
        ascension: 0,
        layers: 2,
        max_branches: 24,
        auto_ops: 64,
        search_nodes: 20_000,
        search_ms: 300,
    };
    let raw = std::env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;
    while index < raw.len() {
        let key = raw[index].as_str();
        if matches!(key, "--help" | "-h") {
            println!("branch_tiny --seed N --layers N --max-branches N");
            std::process::exit(0);
        }
        if !matches!(
            key,
            "--seed"
                | "--ascension"
                | "--a"
                | "--layers"
                | "--max-branches"
                | "--auto-ops"
                | "--search-nodes"
                | "--search-ms"
        ) {
            return Err(format!("unknown argument {key}"));
        }
        let value = raw
            .get(index + 1)
            .ok_or_else(|| format!("{key} requires a value"))?;
        match key {
            "--seed" => args.seed = parse(value, key)?,
            "--ascension" | "--a" => args.ascension = parse(value, key)?,
            "--layers" => args.layers = parse(value, key)?,
            "--max-branches" => args.max_branches = parse(value, key)?,
            "--auto-ops" => args.auto_ops = parse(value, key)?,
            "--search-nodes" => args.search_nodes = parse(value, key)?,
            "--search-ms" => args.search_ms = parse(value, key)?,
            _ => unreachable!("argument key was validated before value parsing"),
        }
        index += 2;
    }
    Ok(args)
}

fn parse<T: std::str::FromStr>(value: &str, key: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("invalid value for {key}: {value}"))
}
