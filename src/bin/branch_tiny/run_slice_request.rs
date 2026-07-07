use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;

use super::run_capsule::RunCapsule;
use super::run_slice_result::RunSliceRequestKind;
use super::{Args, Branch};

pub(super) struct RunSliceRequest {
    pub(super) args: Args,
    pub(super) request_kind: RunSliceRequestKind,
    pub(super) human_output: bool,
    pub(super) trace_path: Option<PathBuf>,
    pub(super) combat_gap_case_dir: Option<PathBuf>,
    pub(super) frontier_checkpoint_path: Option<PathBuf>,
    pub(super) resume_frontier: Option<PathBuf>,
    pub(super) run_capsule: Option<RunCapsule>,
    pub(super) generation_start: usize,
    pub(super) frontier: VecDeque<Branch>,
    pub(super) next_branch_id: usize,
    pub(super) started: Instant,
}
