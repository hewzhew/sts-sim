use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

use super::owner_model::OwnerChoice;
pub(super) use super::trace_format::candidate_kind_value;
use super::{trace_format, Args, Branch};
use serde_json::Value;

pub(super) struct TraceWriter {
    out: BufWriter<File>,
}

impl TraceWriter {
    pub(super) fn create(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)
            .map_err(|err| format!("failed to create trace {}: {err}", path.display()))?;
        Ok(Self {
            out: BufWriter::new(file),
        })
    }

    pub(super) fn record_run(&mut self, args: Args) -> Result<(), String> {
        self.write(trace_format::run_start_event(args))
    }

    pub(super) fn record_node(
        &mut self,
        generation: usize,
        branch: &Branch,
        choices: &[OwnerChoice],
        expanded: &[bool],
    ) -> Result<(), String> {
        self.write(trace_format::node_event(
            generation, branch, choices, expanded,
        ))
    }

    pub(super) fn record_branch_snapshot(
        &mut self,
        generation: usize,
        reason: &'static str,
        branch: &Branch,
    ) -> Result<(), String> {
        self.write(trace_format::branch_snapshot_event(
            generation, reason, branch,
        ))
    }

    pub(super) fn record_frontier_snapshot(
        &mut self,
        generation: usize,
        frontier: &VecDeque<Branch>,
    ) -> Result<(), String> {
        self.write(trace_format::frontier_snapshot_event(generation, frontier))
    }

    fn write(&mut self, value: Value) -> Result<(), String> {
        serde_json::to_writer(&mut self.out, &value).map_err(|err| err.to_string())?;
        self.out.write_all(b"\n").map_err(|err| err.to_string())?;
        self.out.flush().map_err(|err| err.to_string())
    }
}
