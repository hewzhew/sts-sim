use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::{json, Value};

use super::run_chain_state::{capsule_state, manifest_wall_ms, CapsuleState};
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
