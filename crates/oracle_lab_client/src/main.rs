use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use clap::{Parser, Subcommand};
use oracle_lab_protocol::{
    call_oracle_analysis_tcp_v1, OracleAnalysisServiceCommandV1, OracleAnalysisServiceEndpointV1,
};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Parser)]
#[command(
    name = "oracle_lab",
    about = "Fast client for a resident oracle laboratory",
    after_help = "Use `start` and `live` for routine work. Deliberately invoke heavyweight offline commands with `offline`, for example `cargo ol offline combat-case ...`."
)]
struct Cli {
    #[arg(long, hide = true, global = true)]
    canonical_fast_run: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Start or reconnect to a named resident workspace service.
    Start {
        #[arg(long)]
        session: String,
        #[arg(long)]
        workspace: PathBuf,
    },
    /// Send one raw JSON command to a resident loopback service.
    Call {
        #[arg(long)]
        endpoint: PathBuf,
        #[arg(long)]
        request: String,
    },
    /// Use typed commands against a resident oracle workspace.
    Live {
        #[arg(long, conflicts_with = "session")]
        endpoint: Option<PathBuf>,
        #[arg(long, conflicts_with = "endpoint")]
        session: Option<String>,
        #[command(subcommand)]
        command: LiveCommand,
    },
    /// Deliberately invoke a heavyweight offline oracle-lab command.
    Offline {
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        arguments: Vec<OsString>,
    },
}

#[derive(Debug, Subcommand)]
enum LiveCommand {
    /// Show the current node, choices, and tactical progress.
    Status {
        #[arg(long)]
        node: Option<usize>,
        #[arg(long, default_value_t = 8)]
        limit: usize,
    },
    /// Continue tactical search and return its new compact status.
    Advance {
        #[arg(long, default_value_t = 100_000)]
        max_quanta: usize,
        #[arg(long, default_value_t = 4_096)]
        quantum_nodes: usize,
        #[arg(long, default_value_t = 100)]
        quantum_ms: u64,
        #[arg(long, default_value_t = 10_000)]
        wall_ms: u64,
    },
    /// Choose an owner-ranked decision at the current node.
    Choose {
        #[arg(long)]
        owner_rank: u64,
        #[arg(long)]
        node: Option<usize>,
    },
    /// Move the resident cursor to an existing retained node without replaying it.
    Focus {
        #[arg(long)]
        node: usize,
    },
    /// Show the retained run inventory at the current or selected node.
    Inspect {
        #[arg(long)]
        node: Option<usize>,
    },
    /// Apply the owner's first choice for a bounded number of decisions.
    Owner {
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=64))]
        steps: u8,
    },
    /// Accept the current verified combat incumbent.
    Accept,
    /// Restart tactical search at the unchanged exact combat state.
    Restart,
    /// Print a compact timeline for the current or selected node.
    Timeline {
        #[arg(long)]
        node: Option<usize>,
        #[arg(long, default_value_t = 30)]
        tail: usize,
    },
    /// Export the current or selected exact combat case.
    ExportCase {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        node: Option<usize>,
    },
    /// Show the exact combat root and replayed deepest trajectories.
    /// Heavy replay executes inside the resident workspace service.
    Combat {
        #[arg(long)]
        node: Option<usize>,
        #[arg(long, default_value_t = 512)]
        max_engine_steps_per_transition: usize,
    },
    /// Show a compact, turn-by-turn trace of the trajectories retained by combat search.
    Trace {
        #[arg(long)]
        node: Option<usize>,
        #[arg(long, default_value_t = 512)]
        max_engine_steps_per_transition: usize,
    },
    /// Compare generation, admission, and downstream service by root action.
    RootActions {
        #[arg(long)]
        node: Option<usize>,
        #[arg(long, default_value_t = 512)]
        max_engine_steps_per_transition: usize,
    },
    /// Save the resident workspace immediately.
    Save,
    /// Save and stop the resident workspace service.
    Shutdown,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    validate_canonical_launch(cli.canonical_fast_run)?;
    match cli.command {
        Command::Start { session, workspace } => start_session(&session, &workspace),
        Command::Call { endpoint, request } => {
            print_json(&call_oracle_analysis_tcp_v1(&endpoint, &request)?)
        }
        Command::Live {
            endpoint,
            session,
            command,
        } => {
            let endpoint = resolve_endpoint_argument(endpoint, session)?;
            run_live_command(&endpoint, command)
        }
        Command::Offline { arguments } => delegate_heavy(arguments),
    }
}

fn run_live_command(endpoint: &Path, command: LiveCommand) -> Result<(), String> {
    match command {
        LiveCommand::Status { node, limit } => {
            let result = live_call(endpoint, OracleAnalysisServiceCommandV1::Status { node })?;
            print_json(&compact_live_node(&result, limit))
        }
        LiveCommand::Advance {
            max_quanta,
            quantum_nodes,
            quantum_ms,
            wall_ms,
        } => {
            let before = live_call(
                endpoint,
                OracleAnalysisServiceCommandV1::Status { node: None },
            )?;
            let result = live_call(
                endpoint,
                OracleAnalysisServiceCommandV1::Advance {
                    max_quanta,
                    quantum_nodes,
                    quantum_ms,
                    wall_ms: Some(wall_ms),
                },
            )?;
            print_json(&compact_live_advance(&before, &result))
        }
        LiveCommand::Choose { owner_rank, node } => {
            let node = resolve_live_node(endpoint, node)?;
            let result = live_call(
                endpoint,
                OracleAnalysisServiceCommandV1::Choose { node, owner_rank },
            )?;
            print_json(&compact_live_node(&result, 8))
        }
        LiveCommand::Focus { node } => print_json(&compact_live_node(
            &live_call(endpoint, OracleAnalysisServiceCommandV1::Focus { node })?,
            8,
        )),
        LiveCommand::Inspect { node } => {
            let node = resolve_live_node(endpoint, node)?;
            let view = live_call(
                endpoint,
                OracleAnalysisServiceCommandV1::View { node: Some(node) },
            )?;
            print_json(&compact_live_inventory(&view))
        }
        LiveCommand::Owner { steps } => print_json(&run_live_owner(endpoint, steps)?),
        LiveCommand::Accept => print_json(&compact_live_node(
            &live_call(endpoint, OracleAnalysisServiceCommandV1::AcceptCombat)?,
            8,
        )),
        LiveCommand::Restart => print_json(&compact_live_node(
            &live_call(endpoint, OracleAnalysisServiceCommandV1::RestartCombat)?,
            8,
        )),
        LiveCommand::Timeline { node, tail } => {
            let node = resolve_live_node(endpoint, node)?;
            print_json(&live_call(
                endpoint,
                OracleAnalysisServiceCommandV1::Timeline { node, tail },
            )?)
        }
        LiveCommand::ExportCase { path, node } => {
            let node = resolve_live_node(endpoint, node)?;
            print_json(&live_call(
                endpoint,
                OracleAnalysisServiceCommandV1::ExportCombatCase { node, path },
            )?)
        }
        LiveCommand::Save => {
            print_json(&live_call(endpoint, OracleAnalysisServiceCommandV1::Save)?)
        }
        LiveCommand::Shutdown => print_json(&live_call(
            endpoint,
            OracleAnalysisServiceCommandV1::Shutdown,
        )?),
        LiveCommand::Combat {
            node,
            max_engine_steps_per_transition,
        } => {
            let node = resolve_live_node(endpoint, node)?;
            print_json(&live_call(
                endpoint,
                OracleAnalysisServiceCommandV1::CombatDiagnostic {
                    node,
                    max_engine_steps_per_transition,
                },
            )?)
        }
        LiveCommand::Trace {
            node,
            max_engine_steps_per_transition,
        } => {
            let node = resolve_live_node(endpoint, node)?;
            let diagnostic = live_call(
                endpoint,
                OracleAnalysisServiceCommandV1::CombatDiagnostic {
                    node,
                    max_engine_steps_per_transition,
                },
            )?;
            print_json(&compact_live_combat_trace(&diagnostic))
        }
        LiveCommand::RootActions {
            node,
            max_engine_steps_per_transition,
        } => {
            let node = resolve_live_node(endpoint, node)?;
            let diagnostic = live_call(
                endpoint,
                OracleAnalysisServiceCommandV1::CombatDiagnostic {
                    node,
                    max_engine_steps_per_transition,
                },
            )?;
            print_json(&compact_root_action_report(&diagnostic))
        }
    }
}

fn live_call(endpoint: &Path, command: OracleAnalysisServiceCommandV1) -> Result<Value, String> {
    let request = serde_json::to_string(&command)
        .map_err(|error| format!("failed to encode typed oracle command: {error}"))?;
    let response = call_oracle_analysis_tcp_v1(endpoint, &request)?;
    if !response.ok {
        return Err(response
            .error
            .unwrap_or_else(|| format!("oracle service returned event '{}'", response.event)));
    }
    response.result.ok_or_else(|| {
        format!(
            "oracle service event '{}' returned no result",
            response.event
        )
    })
}

fn resolve_live_node(endpoint: &Path, node: Option<usize>) -> Result<usize, String> {
    if let Some(node) = node {
        return Ok(node);
    }
    live_call(
        endpoint,
        OracleAnalysisServiceCommandV1::Status { node: None },
    )?
    .get("node_id")
    .and_then(Value::as_u64)
    .and_then(|node| usize::try_from(node).ok())
    .ok_or_else(|| "oracle status did not contain a valid current node_id".to_string())
}

fn run_live_owner(endpoint: &Path, steps: u8) -> Result<Value, String> {
    let mut node = live_call(
        endpoint,
        OracleAnalysisServiceCommandV1::Status { node: None },
    )?;
    let mut applied = Vec::new();
    let mut stopped = "step_limit";
    for _ in 0..steps {
        let node_id = node
            .get("node_id")
            .and_then(Value::as_u64)
            .ok_or_else(|| "live owner status omitted node_id".to_string())?
            as usize;
        let owner_choices = node
            .get("choices")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|choice| choice.get("owner_rank").and_then(Value::as_u64) == Some(0))
            .collect::<Vec<_>>();
        let [choice] = owner_choices.as_slice() else {
            stopped = if owner_choices.is_empty() {
                "no_owner_choice"
            } else {
                "ambiguous_owner_choice"
            };
            break;
        };
        applied.push(json!({
            "node": node_id,
            "candidate_id": choice.get("candidate_id"),
            "label": choice.get("label"),
        }));
        node = live_call(
            endpoint,
            OracleAnalysisServiceCommandV1::Choose {
                node: node_id,
                owner_rank: 0,
            },
        )?;
    }
    Ok(json!({
        "requested_steps": steps,
        "applied_count": applied.len(),
        "applied": applied,
        "stopped": stopped,
        "status": compact_live_node(&node, 8),
    }))
}

fn compact_live_node(node: &Value, limit: usize) -> Value {
    let choices = limited_values(node.get("choices"), limit);
    let children = limited_values(node.get("children"), limit);
    let choice_count = node
        .get("choice_count")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| {
            node.get("choices")
                .and_then(Value::as_array)
                .map_or(0, |values| values.len() as u64)
        });
    let child_count = node
        .get("child_count")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| {
            node.get("children")
                .and_then(Value::as_array)
                .map_or(0, |values| values.len() as u64)
        });
    json!({
        "node": node.get("node_id"),
        "parent": node.get("canonical_parent_node_id"),
        "act": node.get("act"),
        "floor": node.get("floor"),
        "hp": node.get("current_hp"),
        "max_hp": node.get("max_hp"),
        "gold": node.get("gold"),
        "boundary": node.get("boundary"),
        "event": node.get("event"),
        "choice_count": choice_count,
        "choices_shown": choices.len(),
        "choices_truncated": choice_count > choices.len() as u64,
        "choices": choices,
        "child_count": child_count,
        "children_shown": children.len(),
        "children_truncated": child_count > children.len() as u64,
        "children": children,
        "encounter": compact_encounter(node.get("encounter")),
        "combat": compact_combat_progress(node.get("combat")),
    })
}

fn compact_live_inventory(node: &Value) -> Value {
    json!({
        "node": node.get("node_id"),
        "act": node.get("act"),
        "floor": node.get("floor"),
        "hp": node.get("current_hp"),
        "max_hp": node.get("max_hp"),
        "gold": node.get("gold"),
        "deck": node.get("deck"),
        "relics": node.get("relics"),
        "potions": node.get("potions"),
    })
}

fn limited_values(values: Option<&Value>, limit: usize) -> Vec<Value> {
    values
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(limit)
        .cloned()
        .collect()
}

fn compact_live_advance(before: &Value, result: &Value) -> Value {
    let report = result.get("report");
    let before_combat = before.get("combat");
    let after_combat = report.and_then(|report| report.get("combat"));
    json!({
        "status": report.and_then(|report| report.get("status")),
        "elapsed_ms": report.and_then(|report| report.get("elapsed_ms")),
        "quanta": report.and_then(|report| report.get("quanta_served")),
        "work_delta": {
            "generation_work": value_u64(after_combat, "generation_work").saturating_sub(value_u64(before_combat, "generation_work")),
            "exact_states": value_u64(after_combat, "exact_states").saturating_sub(value_u64(before_combat, "exact_states")),
            "completed_turn_options": value_u64(after_combat, "completed_turn_options").saturating_sub(value_u64(before_combat, "completed_turn_options")),
        },
        "combat": compact_combat_progress(after_combat),
        "node": result.get("node"),
    })
}

fn value_u64(value: Option<&Value>, field: &str) -> u64 {
    value
        .and_then(|value| value.get(field))
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

fn compact_encounter(encounter: Option<&Value>) -> Value {
    let Some(encounter) = encounter.filter(|value| !value.is_null()) else {
        return Value::Null;
    };
    json!({
        "turn": encounter.get("turn"),
        "phase": encounter.get("phase"),
        "energy": encounter.get("energy"),
        "player_block": encounter.get("player_block"),
        "hand": encounter.get("hand").and_then(Value::as_array).map(|cards| cards.iter().map(card_value_label).collect::<Vec<_>>()),
        "draw": encounter.get("draw_pile_count"),
        "discard": encounter.get("discard_pile_count"),
        "exhaust": encounter.get("exhaust_pile_count"),
        "monsters": encounter.get("monsters"),
    })
}

fn compact_combat_progress(combat: Option<&Value>) -> Value {
    let Some(combat) = combat.filter(|value| !value.is_null()) else {
        return Value::Null;
    };
    json!({
        "generation_work": combat.get("generation_work"),
        "exact_states": combat.get("exact_states"),
        "completed_turn_options": combat.get("completed_turn_options"),
        "max_player_turn": combat.get("max_player_turn"),
        "deepest_progress": combat.get("deepest_progress_state"),
        "deepest_survival": combat.get("deepest_survival_state"),
        "incumbent_final_hp": combat.get("incumbent_final_hp"),
        "incumbent_hp_loss": combat.get("incumbent_hp_loss"),
        "incumbent_actions": combat.get("incumbent_action_count"),
        "last_status": combat.get("last_status"),
        "quantum_count": combat.get("quantum_count"),
        "remaining_nodes": combat.get("remaining_nodes"),
        "remaining_wall_ms": combat.get("remaining_wall_ms"),
        "resume_kind": combat.get("resume_kind"),
        "restart_count": combat.get("restart_count"),
    })
}

fn compact_live_combat_trace(diagnostic: &Value) -> Value {
    json!({
        "node": diagnostic.get("node"),
        "root": diagnostic.get("root"),
        "search": diagnostic.get("search"),
        "root_policy_top": compact_policy_top(diagnostic.get("root_policy"), 8),
        "root_action_families": diagnostic.get("root_action_families"),
        "progress_trace": compact_replayed_trace(diagnostic.get("deepest_progress_trace")),
        "survival_trace": compact_replayed_trace(diagnostic.get("deepest_survival_trace")),
    })
}

fn compact_root_action_report(diagnostic: &Value) -> Value {
    let policy_actions = diagnostic
        .get("root_policy")
        .and_then(|policy| policy.get("actions"))
        .and_then(Value::as_array);
    let families = diagnostic
        .get("root_action_families")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let total_work = families
        .iter()
        .map(|family| value_u64(Some(family), "descendant_generation_work"))
        .sum::<u64>();
    let total_downstream_options = families
        .iter()
        .map(|family| value_u64(Some(family), "descendant_completed_turn_options"))
        .sum::<u64>();
    let mut compact_families = families
        .iter()
        .map(|family| {
            let action = family.get("action").and_then(Value::as_str);
            let policy = policy_actions
                .into_iter()
                .flatten()
                .find(|candidate| candidate.get("action").and_then(Value::as_str) == action);
            let work = value_u64(Some(family), "descendant_generation_work");
            json!({
                "action": family.get("action"),
                "prior_rank": policy.and_then(|value| value.get("rank")),
                "prior_probability": policy.and_then(|value| value.get("probability")),
                "root_turn_options": family.get("completed_root_turn_options"),
                "unique_root_successors": family.get("unique_root_successors"),
                "accepted_root_successors": family.get("accepted_root_successors"),
                "retained_root_successors": family.get("retained_root_successors"),
                "accepted_descendants": family.get("accepted_descendants"),
                "retained_descendants": family.get("retained_descendants"),
                "downstream_work": work,
                "work_share_percent": if total_work == 0 { 0.0 } else { work as f64 * 100.0 / total_work as f64 },
                "downstream_turn_options": family.get("descendant_completed_turn_options"),
                "max_player_turn": family.get("max_player_turn"),
                "best_hp_at_max_turn": family.get("best_hp_at_max_turn"),
                "lowest_enemy_hp_at_max_turn": family.get("lowest_enemy_hp_at_max_turn"),
            })
        })
        .collect::<Vec<_>>();
    compact_families.sort_by_key(|family| {
        std::cmp::Reverse(
            family
                .get("downstream_work")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        )
    });

    let node = diagnostic.get("node");
    let root = diagnostic.get("root");
    let search = diagnostic.get("search");
    json!({
        "node": {
            "node_id": node.and_then(|value| value.get("node_id")),
            "act": node.and_then(|value| value.get("act")),
            "floor": node.and_then(|value| value.get("floor")),
            "hp": node.and_then(|value| value.get("hp")),
            "max_hp": node.and_then(|value| value.get("max_hp")),
        },
        "root": {
            "turn": root.and_then(|value| value.get("turn")),
            "phase": root.and_then(|value| value.get("phase")),
            "player": root.and_then(|value| value.get("player")),
            "hand": root.and_then(|value| value.get("hand")),
            "monsters": root.and_then(|value| value.get("monsters")),
        },
        "search": {
            "generation_work": search.and_then(|value| value.get("generation_work")),
            "exact_states": search.and_then(|value| value.get("exact_states")),
            "completed_turn_options": search.and_then(|value| value.get("completed_turn_options")),
            "max_player_turn": search.and_then(|value| value.get("max_player_turn")),
            "incumbent_final_hp": search.and_then(|value| value.get("incumbent_final_hp")),
            "last_status": search.and_then(|value| value.get("last_status")),
        },
        "totals": {
            "root_action_families": compact_families.len(),
            "attributed_downstream_work": total_work,
            "attributed_downstream_turn_options": total_downstream_options,
        },
        "families_by_downstream_work": compact_families,
    })
}

fn compact_replayed_trace(trace: Option<&Value>) -> Value {
    let Some(trace) = trace.filter(|value| !value.is_null()) else {
        return Value::Null;
    };
    if let Some(same_as) = trace.get("same_as") {
        return json!({"same_as": same_as});
    }
    let turns = trace
        .get("turns")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|turn| {
            let end = turn.get("end");
            json!({
                "turn": turn.get("turn"),
                "start_hp": turn.get("start_hp"),
                "actions": turn.get("actions"),
                "policy_top": compact_policy_top(turn.get("start_policy"), 5),
                "end": {
                    "hp": end.and_then(|value| value.get("hp")),
                    "block": end.and_then(|value| value.get("block")),
                    "energy": end.and_then(|value| value.get("energy")),
                    "hand": end.and_then(|value| value.get("hand")),
                    "monsters": end.and_then(|value| value.get("monsters")),
                    "player_powers": end.and_then(|value| value.get("player_powers")),
                    "piles": end.and_then(|value| value.get("piles")),
                },
            })
        })
        .collect::<Vec<_>>();
    json!({
        "action_count": trace.get("action_count"),
        "terminal": trace.get("terminal"),
        "turns": turns,
    })
}

fn compact_policy_top(policy: Option<&Value>, limit: usize) -> Vec<Value> {
    policy
        .and_then(|value| value.get("actions"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(limit)
        .map(|action| {
            json!({
                "rank": action.get("rank"),
                "action": action.get("action"),
                "probability": action.get("probability"),
            })
        })
        .collect()
}

fn card_value_label(card: &Value) -> String {
    let id = card
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("UnknownCard");
    let upgrades = card.get("upgrades").and_then(Value::as_u64).unwrap_or(0);
    if upgrades == 0 {
        id.to_string()
    } else {
        format!("{id}+{upgrades}")
    }
}

fn start_session(session: &str, workspace: &Path) -> Result<(), String> {
    validate_session_name(session)?;
    let workspace = workspace.canonicalize().map_err(|error| {
        format!(
            "failed to resolve oracle workspace '{}': {error}",
            workspace.display()
        )
    })?;
    if !workspace.is_file() {
        return Err(format!(
            "oracle workspace is not a file: {}",
            workspace.display()
        ));
    }
    let endpoint_path = session_endpoint_path(session)?;
    if let Some(endpoint) = active_endpoint(&endpoint_path) {
        ensure_endpoint_workspace(session, &endpoint, &workspace)?;
        return print_json(&json!({
            "session": session,
            "status": "already_running",
            "process_id": endpoint.process_id,
            "workspace": endpoint.workspace,
            "endpoint": endpoint_path,
        }));
    }

    let executable = heavy_executable()?;
    let mut command = ProcessCommand::new(&executable);
    command
        .current_dir(repository_root())
        .arg("--canonical-fast-run")
        .arg("serve")
        .arg("--workspace")
        .arg(&workspace)
        .arg("--session")
        .arg(session)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);
    let mut child = command.spawn().map_err(|error| {
        format!(
            "failed to start resident oracle service '{}': {error}",
            executable.display()
        )
    })?;

    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(endpoint) = active_endpoint(&endpoint_path) {
            ensure_endpoint_workspace(session, &endpoint, &workspace)?;
            return print_json(&json!({
                "session": session,
                "status": "started",
                "process_id": endpoint.process_id,
                "workspace": endpoint.workspace,
                "endpoint": endpoint_path,
            }));
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("failed to inspect resident oracle service: {error}"))?
        {
            return Err(format!(
                "resident oracle service exited before publishing its endpoint: {status}"
            ));
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "resident oracle service did not become ready within 10 seconds; process {}, endpoint {}",
                child.id(),
                endpoint_path.display()
            ));
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn active_endpoint(endpoint_path: &Path) -> Option<OracleAnalysisServiceEndpointV1> {
    let response = call_oracle_analysis_tcp_v1(endpoint_path, r#"{"command":"ping"}"#).ok()?;
    if !response.ok {
        return None;
    }
    let bytes = fs::read(endpoint_path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn ensure_endpoint_workspace(
    session: &str,
    endpoint: &OracleAnalysisServiceEndpointV1,
    requested_workspace: &Path,
) -> Result<(), String> {
    let active_workspace = endpoint.workspace.canonicalize().map_err(|error| {
        format!("failed to resolve workspace for active oracle session `{session}`: {error}")
    })?;
    if active_workspace != requested_workspace {
        return Err(format!(
            "oracle session `{session}` already serves '{}', not requested '{}'",
            active_workspace.display(),
            requested_workspace.display()
        ));
    }
    Ok(())
}

fn delegate_heavy(arguments: Vec<OsString>) -> Result<(), String> {
    let executable = heavy_executable()?;
    let status = ProcessCommand::new(&executable)
        .arg("--canonical-fast-run")
        .args(arguments)
        .status()
        .map_err(|error| {
            format!(
                "failed to start heavy oracle laboratory '{}': {error}",
                executable.display()
            )
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("heavy oracle laboratory exited with {status}"))
    }
}

fn heavy_executable() -> Result<PathBuf, String> {
    let executable = repository_root()
        .join("target")
        .join("fast-run")
        .join(if cfg!(windows) {
            "oracle_lab.exe"
        } else {
            "oracle_lab"
        });
    if !executable.is_file() {
        return Err(format!(
            "heavy oracle laboratory is missing at {}; build it once with `cargo build --profile fast-run -p sts_simulator_control --bin oracle_lab`",
            executable.display()
        ));
    }
    ensure_heavy_artifact_fresh(&executable)?;
    Ok(executable)
}

fn ensure_heavy_artifact_fresh(executable: &Path) -> Result<(), String> {
    let executable_modified = fs::metadata(executable)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| {
            format!(
                "failed to inspect heavy oracle laboratory '{}': {error}",
                executable.display()
            )
        })?;
    let depfile = executable.with_extension("d");
    let depfile_text = fs::read_to_string(&depfile).map_err(|error| {
        format!(
            "heavy oracle dependency manifest is missing at '{}': {error}; rebuild the canonical artifact",
            depfile.display()
        )
    })?;
    let root = repository_root();
    let mut dependencies = depfile_dependencies(&depfile_text);
    dependencies.extend([
        root.join("Cargo.toml"),
        root.join("Cargo.lock"),
        root.join(".cargo/config.toml"),
        root.join("crates/oracle_lab_protocol/Cargo.toml"),
        root.join("crates/sts_combat_planner/Cargo.toml"),
        root.join("crates/sts_simulator_control/Cargo.toml"),
    ]);
    let stale_dependency = dependencies.into_iter().find(|dependency| {
        fs::metadata(dependency)
            .and_then(|metadata| metadata.modified())
            .is_ok_and(|modified| modified > executable_modified)
    });
    if let Some(dependency) = stale_dependency {
        return Err(format!(
            "heavy oracle laboratory is stale: '{}' is newer than '{}'. Rebuild once with `cargo build --profile fast-run -p sts_simulator_control --bin oracle_lab`; refusing to run stale search code",
            dependency.display(),
            executable.display()
        ));
    }
    Ok(())
}

fn depfile_dependencies(depfile: &str) -> Vec<PathBuf> {
    depfile
        .lines()
        .filter_map(|line| line.split_once(": ").map(|(_, dependencies)| dependencies))
        .flat_map(str::split_whitespace)
        .filter(|dependency| !dependency.ends_with(':'))
        .map(PathBuf::from)
        .collect()
}

fn resolve_endpoint_argument(
    endpoint: Option<PathBuf>,
    session: Option<String>,
) -> Result<PathBuf, String> {
    match (endpoint, session) {
        (Some(endpoint), None) => Ok(endpoint),
        (None, Some(session)) => session_endpoint_path(&session),
        (None, None) => Err("oracle_lab live requires --session or --endpoint".to_string()),
        (Some(_), Some(_)) => {
            Err("oracle_lab live accepts only one of --session or --endpoint".to_string())
        }
    }
}

fn session_endpoint_path(session: &str) -> Result<PathBuf, String> {
    validate_session_name(session)?;
    Ok(repository_root()
        .join("target")
        .join("oracle-lab")
        .join("sessions")
        .join(format!("{session}.endpoint.json")))
}

fn validate_session_name(session: &str) -> Result<(), String> {
    if session.is_empty()
        || session.len() > 64
        || matches!(session, "." | "..")
        || !session.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
        || !session
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric())
    {
        return Err(format!("invalid oracle session name `{session}`; use 1-64 ASCII letters, digits, '.', '_' or '-', starting with a letter or digit"));
    }
    Ok(())
}

fn validate_canonical_launch(canonical_fast_run: bool) -> Result<(), String> {
    if !canonical_fast_run {
        return Err(
            "oracle_lab client refuses direct execution; run `cargo ol <command> ...`".to_string(),
        );
    }
    let expected = repository_root()
        .join("target")
        .join("fast-run")
        .join(if cfg!(windows) {
            "oracle_lab_client.exe"
        } else {
            "oracle_lab_client"
        });
    let current = std::env::current_exe()
        .and_then(|path| path.canonicalize())
        .map_err(|error| format!("failed to identify running oracle_lab client: {error}"))?;
    let expected = expected.canonicalize().map_err(|error| {
        format!(
            "canonical oracle_lab client is missing at {}: {error}; run `cargo ol --help`",
            expected.display()
        )
    })?;
    if current != expected {
        return Err(format!(
            "oracle_lab client refuses non-canonical artifact {}; expected {}",
            current.display(),
            expected.display()
        ));
    }
    Ok(())
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn print_json<T: Serialize>(value: &T) -> Result<(), String> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    serde_json::to_writer_pretty(&mut stdout, value)
        .map_err(|error| format!("failed to print JSON: {error}"))?;
    writeln!(stdout).map_err(|error| format!("failed to finish JSON output: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn typed_live_status_parses_without_loading_heavy_commands() {
        let cli = Cli::try_parse_from([
            "oracle_lab_client",
            "--canonical-fast-run",
            "live",
            "--session",
            "seed008",
            "status",
            "--limit",
            "3",
        ])
        .expect("parse typed live status");
        assert!(cli.canonical_fast_run);
        assert!(matches!(
            cli.command,
            Command::Live {
                command: LiveCommand::Status { limit: 3, .. },
                ..
            }
        ));
    }

    #[test]
    fn typed_live_combat_stays_on_the_resident_service_path() {
        let cli = Cli::try_parse_from([
            "oracle_lab_client",
            "--canonical-fast-run",
            "live",
            "--session",
            "seed008",
            "combat",
            "--node",
            "69",
        ])
        .expect("parse typed live combat");
        assert!(matches!(
            cli.command,
            Command::Live {
                command: LiveCommand::Combat {
                    node: Some(69),
                    max_engine_steps_per_transition: 512,
                },
                ..
            }
        ));
    }

    #[test]
    fn typed_live_trace_stays_on_the_resident_service_path() {
        let cli = Cli::try_parse_from([
            "oracle_lab_client",
            "--canonical-fast-run",
            "live",
            "--session",
            "seed008",
            "trace",
            "--node",
            "69",
        ])
        .expect("parse typed live trace");
        assert!(matches!(
            cli.command,
            Command::Live {
                command: LiveCommand::Trace {
                    node: Some(69),
                    max_engine_steps_per_transition: 512,
                },
                ..
            }
        ));
    }

    #[test]
    fn typed_live_root_actions_stays_on_the_resident_service_path() {
        let cli = Cli::try_parse_from([
            "oracle_lab_client",
            "--canonical-fast-run",
            "live",
            "--session",
            "seed008",
            "root-actions",
        ])
        .expect("parse typed live root actions");
        assert!(matches!(
            cli.command,
            Command::Live {
                command: LiveCommand::RootActions {
                    node: None,
                    max_engine_steps_per_transition: 512,
                },
                ..
            }
        ));
    }

    #[test]
    fn typed_start_owns_named_session_launch() {
        let cli = Cli::try_parse_from([
            "oracle_lab_client",
            "--canonical-fast-run",
            "start",
            "--session",
            "seed008",
            "--workspace",
            "seed008.workspace.json",
        ])
        .expect("parse typed session start");
        assert!(matches!(
            cli.command,
            Command::Start { session, workspace }
                if session == "seed008" && workspace == PathBuf::from("seed008.workspace.json")
        ));
    }

    #[test]
    fn unknown_commands_do_not_silently_launch_the_heavy_tool() {
        assert!(
            Cli::try_parse_from(["oracle_lab_client", "--canonical-fast-run", "combat-csae",])
                .is_err()
        );
        let cli = Cli::try_parse_from([
            "oracle_lab_client",
            "--canonical-fast-run",
            "offline",
            "combat-case",
            "--case",
            "fight.json",
        ])
        .expect("parse deliberate heavyweight invocation");
        assert!(matches!(
            cli.command,
            Command::Offline { arguments }
                if arguments == ["combat-case", "--case", "fight.json"]
        ));
    }

    #[test]
    fn windows_depfile_parser_does_not_split_the_drive_prefix() {
        let dependencies = depfile_dependencies(
            r#"D:\rust\target\oracle_lab.exe: D:\rust\src\lib.rs D:\rust\src\bin\oracle_lab.rs
D:\rust\src\lib.rs:
D:\rust\src\bin\oracle_lab.rs:
"#,
        );
        assert_eq!(
            dependencies,
            [
                PathBuf::from(r"D:\rust\src\lib.rs"),
                PathBuf::from(r"D:\rust\src\bin\oracle_lab.rs"),
            ]
        );
    }

    #[test]
    fn stale_heavy_artifact_is_rejected_before_execution() {
        let directory = std::env::temp_dir().join(format!(
            "oracle-lab-client-stale-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("create stale-artifact fixture");
        let executable = directory.join("oracle_lab.exe");
        let dependency = directory.join("search_source.rs");
        fs::write(&executable, b"old artifact").expect("write fake artifact");
        thread::sleep(Duration::from_millis(20));
        fs::write(&dependency, b"new source").expect("write newer dependency");
        fs::write(
            executable.with_extension("d"),
            format!("{}: {}\n", executable.display(), dependency.display()),
        )
        .expect("write fake depfile");

        let error = ensure_heavy_artifact_fresh(&executable)
            .expect_err("newer source must reject stale artifact");
        assert!(error.contains("refusing to run stale search code"));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn compact_status_reports_truncation_instead_of_hiding_candidates() {
        let node = json!({
            "node_id": 7,
            "choices": [
                {"label": "a"},
                {"label": "b"},
                {"label": "c"}
            ],
            "children": [],
            "event": {"title": "test"}
        });
        let compact = compact_live_node(&node, 2);
        assert_eq!(compact.get("choice_count"), Some(&json!(3)));
        assert_eq!(compact.get("choices_shown"), Some(&json!(2)));
        assert_eq!(compact.get("choices_truncated"), Some(&json!(true)));
        assert_eq!(compact.get("event"), node.get("event"));
    }

    #[test]
    fn compact_trace_keeps_chosen_actions_and_small_policy_context() {
        let diagnostic = json!({
            "node": {"node_id": 7},
            "root": {"hand": "Offering | Defend"},
            "root_policy": {
                "actions": [
                    {"rank": 1, "action": "use BlockPotion", "probability": 0.5},
                    {"rank": 2, "action": "play Offering", "probability": 0.25}
                ]
            },
            "deepest_progress_trace": {
                "action_count": 2,
                "terminal": "Unresolved",
                "turns": [{
                    "turn": 0,
                    "start_hp": 45,
                    "actions": ["use BlockPotion", "end turn"],
                    "start_policy": {"actions": [
                        {"rank": 1, "action": "use BlockPotion", "probability": 0.5}
                    ]},
                    "end": {
                        "hp": 45,
                        "block": 0,
                        "energy": 4,
                        "hand": "Defend",
                        "monsters": ["Donu 250/250"],
                        "player_powers": [],
                        "piles": "draw 1 / discard 1 / exhaust 0"
                    }
                }]
            },
            "deepest_survival_trace": {"same_as": "deepest_progress_trace"}
        });
        let compact = compact_live_combat_trace(&diagnostic);
        assert_eq!(compact["root_policy_top"][1]["action"], "play Offering");
        assert_eq!(
            compact["progress_trace"]["turns"][0]["actions"],
            json!(["use BlockPotion", "end turn"])
        );
        assert_eq!(
            compact["survival_trace"]["same_as"],
            "deepest_progress_trace"
        );
    }

    #[test]
    fn root_action_report_attributes_work_without_copying_deep_search_state() {
        let diagnostic = json!({
            "node": {"node_id": 134, "act": 3, "floor": 48, "hp": 45, "max_hp": 46},
            "root": {"turn": 0, "phase": "PlayerTurn", "player": {"hp": 45}, "hand": "Offering", "monsters": []},
            "root_policy": {"actions": [
                {"rank": 1, "action": "use BlockPotion", "probability": 0.34},
                {"rank": 7, "action": "play Offering", "probability": 0.046}
            ]},
            "search": {
                "generation_work": 100,
                "exact_states": 20,
                "completed_turn_options": 30,
                "max_player_turn": 4,
                "deepest_progress": {"large": [1, 2, 3]}
            },
            "root_action_families": [
                {"action": "play Offering", "completed_root_turn_options": 2, "unique_root_successors": 2, "accepted_root_successors": 2, "retained_root_successors": 2, "accepted_descendants": 3, "retained_descendants": 2, "descendant_generation_work": 1, "descendant_completed_turn_options": 1, "max_player_turn": 2, "best_hp_at_max_turn": 19, "lowest_enemy_hp_at_max_turn": 476},
                {"action": "use BlockPotion", "completed_root_turn_options": 8, "unique_root_successors": 8, "accepted_root_successors": 8, "retained_root_successors": 7, "accepted_descendants": 9, "retained_descendants": 7, "descendant_generation_work": 9, "descendant_completed_turn_options": 6, "max_player_turn": 4, "best_hp_at_max_turn": 3, "lowest_enemy_hp_at_max_turn": 387}
            ]
        });

        let report = compact_root_action_report(&diagnostic);
        assert_eq!(
            report["families_by_downstream_work"][0]["action"],
            "use BlockPotion"
        );
        assert_eq!(report["families_by_downstream_work"][1]["prior_rank"], 7);
        assert_eq!(
            report["families_by_downstream_work"][1]["work_share_percent"],
            10.0
        );
        assert!(report["search"].get("deepest_progress").is_none());
    }

    #[test]
    fn session_names_cannot_escape_the_session_directory() {
        assert!(validate_session_name("seed008").is_ok());
        assert!(validate_session_name("../seed008").is_err());
        assert!(validate_session_name("seed/008").is_err());
    }
}
