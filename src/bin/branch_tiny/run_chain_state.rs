use std::fs;
use std::path::Path;

use serde_json::{json, Value};

pub(super) struct CapsuleState {
    pub(super) manifest_exists: bool,
    pub(super) frontier_exists: bool,
    pub(super) result_exists: bool,
    pub(super) terminal_exists: bool,
    pub(super) status: Option<String>,
    pub(super) reason: Option<String>,
    pub(super) generation: Option<u64>,
    pub(super) branch_id: Option<u64>,
    pub(super) boundary: Option<String>,
    pub(super) owner: Option<String>,
}

impl CapsuleState {
    pub(super) fn is_wall_pause(&self) -> bool {
        self.status.as_deref() == Some("paused")
            && self.reason.as_deref() == Some("wall_deadline")
            && self.frontier_exists
    }

    pub(super) fn into_value(
        self,
        slice: usize,
        resumed: bool,
        process_success: Option<bool>,
    ) -> Value {
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

pub(super) fn capsule_state(capsule: &Path) -> Result<CapsuleState, String> {
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

pub(super) fn manifest_wall_ms(capsule: &Path) -> Result<Option<u64>, String> {
    let manifest = capsule.join("manifest.json");
    if !manifest.exists() {
        return Ok(None);
    }
    Ok(read_json(&manifest)?
        .get("args")
        .and_then(|args| args.get("wall_ms"))
        .and_then(Value::as_u64))
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&text).map_err(|err| format!("failed to parse {}: {err}", path.display()))
}
