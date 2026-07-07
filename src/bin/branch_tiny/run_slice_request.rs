use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;

use super::cli_args::ArgsOverrides;
use super::run_capsule::RunCapsule;
use super::run_slice_result::RunSliceRequestKind;
use super::{branch_runtime, frontier_checkpoint, Args, Branch};

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

pub(super) struct ContinueSliceRequest {
    pub(super) args: Args,
    pub(super) overrides: ArgsOverrides,
    pub(super) capsule_path: PathBuf,
    pub(super) resume: bool,
    pub(super) human_output: bool,
}

impl ContinueSliceRequest {
    pub(super) fn prepare(self) -> Result<RunSliceRequest, String> {
        let started = Instant::now();
        let run_capsule = RunCapsule::new(self.capsule_path.clone());
        let mut effective_args = self.args;
        let mut generation_start = 0usize;
        let resume_frontier = self.resume.then(|| self.capsule_path.join("frontier.json"));
        let (frontier, next_branch_id) = if let Some(path) = resume_frontier.as_ref() {
            let checkpoint = frontier_checkpoint::load(path)?;
            effective_args = checkpoint.args;
            self.overrides.apply_to(&mut effective_args);
            if effective_args.wall_ms.is_none() {
                effective_args.wall_ms = self.args.wall_ms;
            }
            generation_start = checkpoint.generation;
            checkpoint.into_frontier()?
        } else {
            self.overrides.apply_to(&mut effective_args);
            branch_runtime::BranchRuntime::initial_frontier(effective_args, started)
        };
        run_capsule.write_running_manifest(effective_args)?;
        let combat_gap_case_dir = Some(run_capsule.combat_cases_dir());
        Ok(RunSliceRequest {
            args: effective_args,
            request_kind: if self.resume {
                RunSliceRequestKind::ResumeFrontier
            } else {
                RunSliceRequestKind::Start
            },
            human_output: self.human_output,
            trace_path: None,
            combat_gap_case_dir,
            frontier_checkpoint_path: None,
            resume_frontier,
            run_capsule: Some(run_capsule),
            generation_start,
            frontier,
            next_branch_id,
            started,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::cli_args::ArgsOverrides;
    use crate::run_contract::RunObjective;

    fn sample_args(seed: u64) -> Args {
        Args {
            seed,
            ascension: 0,
            objective: RunObjective::FirstVictory,
            generations: 0,
            max_branches: 1,
            auto_ops: 64,
            search_nodes: 1,
            search_ms: 1,
            rescue_search_nodes: 1,
            rescue_search_ms: 1,
            boss_search_nodes: 1,
            boss_search_ms: 1,
            wall_ms: Some(5_000),
            checkpoint_before_combat_portfolio: false,
            wall_capped_search_budget: false,
            wall_capped_boss_budget: false,
        }
    }

    #[test]
    fn continue_slice_request_without_frontier_prepares_start_request() {
        let capsule = std::env::temp_dir().join("branch_tiny_continue_slice_request_start");
        let _ = fs::remove_dir_all(&capsule);

        let request = ContinueSliceRequest {
            args: sample_args(123),
            overrides: ArgsOverrides::default(),
            capsule_path: capsule.clone(),
            resume: false,
            human_output: false,
        }
        .prepare()
        .unwrap();

        assert_eq!(request.request_kind, RunSliceRequestKind::Start);
        assert!(capsule.join("manifest.json").exists());

        let _ = fs::remove_dir_all(capsule);
    }
}
