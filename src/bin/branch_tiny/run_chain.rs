use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{json, Value};

use super::run_contract::RunObjective;
use super::{Args, ArgsOverrides, ContinueCapsuleArgs};

pub(super) fn run(
    mut args: Args,
    overrides: ArgsOverrides,
    chain: ContinueCapsuleArgs,
) -> Result<(), String> {
    if args.wall_ms.is_none() {
        args.wall_ms = manifest_wall_ms(&chain.capsule)?;
    }
    if args.wall_ms.is_none() {
        return Err(format!(
            "--continue-capsule requires --wall-ms; no previous wall_ms found in {}",
            chain.capsule.join("manifest.json").display()
        ));
    }
    let exe = std::env::current_exe().map_err(|err| err.to_string())?;
    let mut slices = Vec::new();
    for index in 0..chain.max_slices {
        let before = capsule_state(&chain.capsule)?;
        if index == 0 && before.manifest_exists && !before.frontier_exists {
            slices.push(before.into_value(index, false, None));
            break;
        }
        let resume = before.frontier_exists;
        let mut command = Command::new(&exe);
        push_overrides(&mut command, args, overrides);
        if resume {
            command.arg("--resume-capsule").arg(&chain.capsule);
        } else {
            command.arg("--run-capsule").arg(&chain.capsule);
        }
        command.stdout(Stdio::null());
        let status = command
            .status()
            .map_err(|err| format!("failed to run continuation slice {index}: {err}"))?;
        let after = capsule_state(&chain.capsule)?;
        let should_continue = after.is_wall_pause();
        let success = status.success();
        print_slice_summary(index, chain.max_slices, resume, success, &after);
        slices.push(after.into_value(index, resume, Some(success)));
        write_chain(&chain.capsule, chain.max_slices, &slices)?;
        if !success {
            return Err(format!("continuation slice {index} exited with {status}"));
        }
        if !should_continue {
            break;
        }
    }
    write_chain(&chain.capsule, chain.max_slices, &slices)
}

fn push_overrides(command: &mut Command, args: Args, overrides: ArgsOverrides) {
    push_optional_arg(
        command,
        "--objective",
        overrides.objective.map(objective_arg),
    );
    push_optional_arg(command, "--generations", overrides.generations);
    push_optional_arg(command, "--max-branches", overrides.max_branches);
    push_optional_arg(command, "--auto-ops", overrides.auto_ops);
    push_optional_arg(command, "--search-nodes", overrides.search_nodes);
    push_optional_arg(command, "--search-ms", overrides.search_ms);
    push_optional_arg(
        command,
        "--rescue-search-nodes",
        overrides.rescue_search_nodes,
    );
    push_optional_arg(command, "--rescue-search-ms", overrides.rescue_search_ms);
    push_optional_arg(command, "--boss-search-nodes", overrides.boss_search_nodes);
    push_optional_arg(command, "--boss-search-ms", overrides.boss_search_ms);
    if let Some(wall_ms) = args.wall_ms {
        command.arg("--wall-ms").arg(wall_ms.to_string());
    }
    if overrides.checkpoint_before_combat_portfolio {
        command.arg("--checkpoint-before-combat-portfolio");
    }
}

fn push_optional_arg<T: ToString>(command: &mut Command, key: &str, value: Option<T>) {
    if let Some(value) = value {
        command.arg(key).arg(value.to_string());
    }
}

fn objective_arg(objective: RunObjective) -> &'static str {
    match objective {
        RunObjective::FirstVictory => "first-victory",
        RunObjective::FirstTerminal => "first-terminal",
        RunObjective::ExhaustFrontier => "exhaust-frontier",
    }
}

struct CapsuleState {
    manifest_exists: bool,
    frontier_exists: bool,
    result_exists: bool,
    terminal_exists: bool,
    status: Option<String>,
    reason: Option<String>,
    generation: Option<u64>,
    branch_id: Option<u64>,
    boundary: Option<String>,
    owner: Option<String>,
}

impl CapsuleState {
    fn is_wall_pause(&self) -> bool {
        self.status.as_deref() == Some("paused")
            && self.reason.as_deref() == Some("wall_deadline")
            && self.frontier_exists
    }

    fn into_value(self, slice: usize, resumed: bool, process_success: Option<bool>) -> Value {
        json!({
            "slice": slice,
            "resumed": resumed,
            "process_success": process_success,
            "manifest": self.manifest_exists,
            "frontier": self.frontier_exists,
            "result": self.result_exists,
            "terminal": self.terminal_exists,
            "status": self.status,
            "reason": self.reason,
            "generation": self.generation,
            "branch_id": self.branch_id,
            "boundary": self.boundary,
            "owner": self.owner,
        })
    }
}

fn capsule_state(capsule: &Path) -> Result<CapsuleState, String> {
    let manifest = capsule.join("manifest.json");
    let value = if manifest.exists() {
        Some(read_json(&manifest)?)
    } else {
        None
    };
    let result = capsule.join("result.json");
    let frontier = capsule.join("frontier.json");
    let mut generation = None;
    let mut branch_id = None;
    let mut boundary = None;
    let mut owner = None;
    if result.exists() {
        let value = read_json(&result)?;
        generation = value.get("generation").and_then(Value::as_u64);
        branch_id = value.get("branch_id").and_then(Value::as_u64);
        boundary = value
            .get("status")
            .and_then(|status| status.get("boundary"))
            .and_then(Value::as_str)
            .map(str::to_string);
    } else if frontier.exists() {
        let value = read_json(&frontier)?;
        generation = value.get("generation").and_then(Value::as_u64);
        if let Some(branch) = value
            .get("frontier")
            .and_then(Value::as_array)
            .and_then(|frontier| frontier.first())
        {
            branch_id = branch.get("id").and_then(Value::as_u64);
            if let Some(running) = branch
                .get("status")
                .and_then(|status| status.get("Running"))
            {
                boundary = running
                    .get("boundary")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                owner = running
                    .get("owner")
                    .and_then(Value::as_str)
                    .map(str::to_string);
            } else if let Some(awaiting) = branch
                .get("status")
                .and_then(|status| status.get("AwaitingAuto"))
            {
                boundary = awaiting
                    .get("boundary")
                    .and_then(Value::as_str)
                    .map(str::to_string);
            }
        }
    }
    Ok(CapsuleState {
        manifest_exists: value.is_some(),
        frontier_exists: frontier.exists(),
        result_exists: result.exists(),
        terminal_exists: capsule.join("terminal.json").exists(),
        status: value
            .as_ref()
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str)
            .map(str::to_string),
        reason: value
            .as_ref()
            .and_then(|value| value.get("reason"))
            .and_then(Value::as_str)
            .map(str::to_string),
        generation,
        branch_id,
        boundary,
        owner,
    })
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&text).map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn manifest_wall_ms(capsule: &Path) -> Result<Option<u64>, String> {
    let manifest = capsule.join("manifest.json");
    if !manifest.exists() {
        return Ok(None);
    }
    Ok(read_json(&manifest)?
        .get("args")
        .and_then(|args| args.get("wall_ms"))
        .and_then(Value::as_u64))
}

fn write_chain(capsule: &PathBuf, max_slices: usize, slices: &[Value]) -> Result<(), String> {
    fs::create_dir_all(capsule).map_err(|err| err.to_string())?;
    let path = capsule.join("chain.json");
    let payload = json!({
        "schema": "branch_tiny_run_chain",
        "capsule": capsule.display().to_string(),
        "max_slices": max_slices,
        "slices": slices,
    });
    fs::write(
        &path,
        serde_json::to_string_pretty(&payload).map_err(|err| err.to_string())?,
    )
    .map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn print_slice_summary(
    index: usize,
    max_slices: usize,
    resumed: bool,
    success: bool,
    state: &CapsuleState,
) {
    println!(
        "continue_slice {}/{} resumed={} success={} status={} reason={} generation={} branch={} boundary={} owner={} frontier={} result={}",
        index + 1,
        max_slices,
        resumed,
        success,
        state.status.as_deref().unwrap_or("-"),
        state.reason.as_deref().unwrap_or("-"),
        state
            .generation
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        state
            .branch_id
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        state.boundary.as_deref().unwrap_or("-"),
        state.owner.as_deref().unwrap_or("-"),
        state.frontier_exists,
        state.result_exists
    );
}
