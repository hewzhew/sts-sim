use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

use super::run_contract::RunObjective;
use super::{Args, ContinueCapsuleArgs};

pub(super) fn run(args: Args, chain: ContinueCapsuleArgs) -> Result<(), String> {
    if args.wall_ms.is_none() {
        return Err("--continue-capsule requires --wall-ms".to_string());
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
        push_args(&mut command, args);
        if resume {
            command.arg("--resume-capsule").arg(&chain.capsule);
        } else {
            command.arg("--run-capsule").arg(&chain.capsule);
        }
        let status = command
            .status()
            .map_err(|err| format!("failed to run continuation slice {index}: {err}"))?;
        let after = capsule_state(&chain.capsule)?;
        let should_continue = after.is_wall_pause();
        let success = status.success();
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

fn push_args(command: &mut Command, args: Args) {
    command
        .arg("--seed")
        .arg(args.seed.to_string())
        .arg("--ascension")
        .arg(args.ascension.to_string())
        .arg("--objective")
        .arg(objective_arg(args.objective))
        .arg("--generations")
        .arg(args.generations.to_string())
        .arg("--max-branches")
        .arg(args.max_branches.to_string())
        .arg("--auto-ops")
        .arg(args.auto_ops.to_string())
        .arg("--search-nodes")
        .arg(args.search_nodes.to_string())
        .arg("--search-ms")
        .arg(args.search_ms.to_string())
        .arg("--rescue-search-nodes")
        .arg(args.rescue_search_nodes.to_string())
        .arg("--rescue-search-ms")
        .arg(args.rescue_search_ms.to_string())
        .arg("--boss-search-nodes")
        .arg(args.boss_search_nodes.to_string())
        .arg("--boss-search-ms")
        .arg(args.boss_search_ms.to_string());
    if let Some(wall_ms) = args.wall_ms {
        command.arg("--wall-ms").arg(wall_ms.to_string());
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
    Ok(CapsuleState {
        manifest_exists: value.is_some(),
        frontier_exists: capsule.join("frontier.json").exists(),
        result_exists: capsule.join("result.json").exists(),
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
    })
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&text).map_err(|err| format!("failed to parse {}: {err}", path.display()))
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
