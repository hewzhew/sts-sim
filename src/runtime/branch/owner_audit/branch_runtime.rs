use std::collections::VecDeque;
use std::time::Instant;

use sts_simulator::eval::run_control::{
    RewardAutomationConfig, RunControlConfig, RunControlSession,
};

use super::run_deadline::RunDeadline;
use super::run_slice_request::RunSliceRequest;
use super::run_slice_result::RunSliceResult;
use super::{run_loop, runner, Args, Branch};

pub(super) struct BranchRuntime;

impl BranchRuntime {
    pub(super) fn run_slice(request: RunSliceRequest) -> Result<RunSliceResult, String> {
        run_loop::run(request)
    }

    pub(super) fn initial_frontier(args: Args, started: Instant) -> (VecDeque<Branch>, usize) {
        let mut session = RunControlSession::new(RunControlConfig {
            seed: args.seed,
            ascension_level: args.ascension,
            reward_automation: RewardAutomationConfig {
                claim_gold: true,
                claim_potion_with_empty_slot: true,
                claim_safe_relic_without_sapphire_key: true,
            },
            ..Default::default()
        });
        let deadline = RunDeadline::new(started, args.wall_ms);
        let advance =
            runner::advance_to_owner_or_gap(&mut session, deadline.cap_args(args, 1), deadline);
        let combat_search = advance.combat_search;
        (
            VecDeque::from([Branch {
                id: 0,
                parent_id: None,
                path: Vec::new(),
                session,
                status: advance.status,
                combat_portfolio: advance.combat_portfolio,
                auto_steps: advance.auto_steps,
                combat_search: combat_search.clone(),
                combat_search_history: combat_search,
            }]),
            1usize,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::super::run_slice_request::RunSliceRequest;
    use super::super::run_slice_result::{ArtifactWriteSummary, RunSliceRequestKind};
    use super::super::{run_contract::RunObjective, Args};
    use super::*;

    fn sample_args() -> Args {
        Args {
            seed: 1,
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
            wall_ms: None,
            checkpoint_before_combat_portfolio: false,
            shop_boss_preview_bundle_limit: 0,
            shop_boss_preview_target_floor: None,
            wall_capped_search_budget: false,
            wall_capped_boss_budget: false,
        }
    }

    #[test]
    fn runtime_initial_frontier_starts_one_root_branch() {
        let (frontier, next_branch_id) =
            BranchRuntime::initial_frontier(sample_args(), Instant::now());

        assert_eq!(frontier.len(), 1);
        assert_eq!(frontier.front().unwrap().id, 0);
        assert_eq!(next_branch_id, 1);
    }

    #[test]
    fn runtime_slice_result_uses_explicit_request_kind() {
        let args = sample_args();
        let started = Instant::now();
        let (frontier, next_branch_id) = BranchRuntime::initial_frontier(args, started);
        let request = RunSliceRequest {
            args,
            capsule_args: args,
            request_kind: RunSliceRequestKind::ResumeFrontier,
            human_output: false,
            trace_path: None,
            combat_gap_case_dir: None,
            frontier_checkpoint_path: None,
            resume_frontier: None,
            run_capsule: None,
            artifact_writes: ArtifactWriteSummary::default(),
            generation_start: 0,
            frontier,
            next_branch_id,
            started,
        };

        let result = BranchRuntime::run_slice(request).unwrap();

        assert_eq!(result.request_kind, RunSliceRequestKind::ResumeFrontier);
    }
}
