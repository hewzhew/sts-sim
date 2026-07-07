#![allow(dead_code)]

use std::path::PathBuf;

#[path = "../../bin/branch_tiny/boss_relic_owner.rs"]
mod boss_relic_owner;
#[path = "../../bin/branch_tiny/boundary_router.rs"]
mod boundary_router;
#[path = "../../bin/branch_tiny/branch_frontier.rs"]
mod branch_frontier;
#[path = "../../bin/branch_tiny/branch_generation.rs"]
mod branch_generation;
#[path = "../../bin/branch_tiny/branch_generation_step.rs"]
mod branch_generation_step;
#[path = "../../bin/branch_tiny/branch_model.rs"]
mod branch_model;
#[path = "../../bin/branch_tiny/branch_observer.rs"]
mod branch_observer;
#[path = "../../bin/branch_tiny/branch_path.rs"]
mod branch_path;
#[path = "../../bin/branch_tiny/branch_runtime.rs"]
mod branch_runtime;
#[path = "../../bin/branch_tiny/branch_scheduler.rs"]
mod branch_scheduler;
#[path = "../../bin/branch_tiny/branch_status_view.rs"]
mod branch_status_view;
#[path = "../../bin/branch_tiny/campfire_owner.rs"]
mod campfire_owner;
#[path = "../../bin/branch_tiny/candidate_ir_adapter.rs"]
mod candidate_ir_adapter;
#[path = "../../bin/branch_tiny/card_reward_owner.rs"]
mod card_reward_owner;
#[path = "../../bin/branch_tiny/cli_args.rs"]
mod cli_args;
#[path = "../../bin/branch_tiny/combat_gap_case.rs"]
mod combat_gap_case;
#[path = "../../bin/branch_tiny/combat_portfolio_json.rs"]
mod combat_portfolio_json;
#[path = "../../bin/branch_tiny/combat_search_dirty_win.rs"]
mod combat_search_dirty_win;
#[path = "../../bin/branch_tiny/combat_search_lane_commit.rs"]
mod combat_search_lane_commit;
#[path = "../../bin/branch_tiny/combat_search_lane_options.rs"]
mod combat_search_lane_options;
#[path = "../../bin/branch_tiny/combat_search_lane_runner.rs"]
mod combat_search_lane_runner;
#[path = "../../bin/branch_tiny/combat_search_lane_spec.rs"]
mod combat_search_lane_spec;
#[path = "../../bin/branch_tiny/combat_search_lanes.rs"]
mod combat_search_lanes;
#[path = "../../bin/branch_tiny/combat_search_orchestrator.rs"]
mod combat_search_orchestrator;
#[path = "../../bin/branch_tiny/combat_search_portfolio_context.rs"]
mod combat_search_portfolio_context;
#[path = "../../bin/branch_tiny/combat_search_portfolio_output.rs"]
mod combat_search_portfolio_output;
#[path = "../../bin/branch_tiny/combat_search_portfolio_plan.rs"]
mod combat_search_portfolio_plan;
#[path = "../../bin/branch_tiny/combat_search_portfolio_result.rs"]
mod combat_search_portfolio_result;
#[path = "../../bin/branch_tiny/combat_search_recipe.rs"]
mod combat_search_recipe;
#[path = "../../bin/branch_tiny/combat_search_report.rs"]
mod combat_search_report;
#[path = "../../bin/branch_tiny/combat_search_trace_actions.rs"]
mod combat_search_trace_actions;
#[path = "../../bin/branch_tiny/decision_delta.rs"]
mod decision_delta;
#[path = "../../bin/branch_tiny/event_owner_bridge.rs"]
mod event_owner_bridge;
#[path = "../../bin/branch_tiny/event_owner_probe.rs"]
mod event_owner_probe;
#[path = "../../bin/branch_tiny/expansion_policy.rs"]
mod expansion_policy;
#[path = "../../bin/branch_tiny/frontier_checkpoint.rs"]
mod frontier_checkpoint;
#[path = "../../bin/branch_tiny/neow_owner.rs"]
mod neow_owner;
#[path = "../../bin/branch_tiny/owner_candidate_eval.rs"]
mod owner_candidate_eval;
#[path = "../../bin/branch_tiny/owner_choice_expander.rs"]
mod owner_choice_expander;
#[path = "../../bin/branch_tiny/owner_commands.rs"]
mod owner_commands;
#[path = "../../bin/branch_tiny/owner_model.rs"]
mod owner_model;
#[path = "../../bin/branch_tiny/owner_orchestrator.rs"]
mod owner_orchestrator;
#[path = "../../bin/branch_tiny/owner_routines.rs"]
mod owner_routines;
#[path = "../../bin/branch_tiny/owners.rs"]
mod owners;
#[path = "../../bin/branch_tiny/render.rs"]
mod render;
#[path = "../../bin/branch_tiny/render_choice.rs"]
mod render_choice;
#[path = "../../bin/branch_tiny/reward_tiny_owner.rs"]
mod reward_tiny_owner;
#[path = "../../bin/branch_tiny/run_capsule.rs"]
mod run_capsule;
#[path = "../../bin/branch_tiny/run_capsule_format.rs"]
mod run_capsule_format;
#[path = "../../bin/branch_tiny/run_capsule_io.rs"]
mod run_capsule_io;
#[path = "../../bin/branch_tiny/run_chain.rs"]
mod run_chain;
#[path = "../../bin/branch_tiny/run_chain_state.rs"]
mod run_chain_state;
#[path = "../../bin/branch_tiny/run_choice_owner.rs"]
mod run_choice_owner;
#[path = "../../bin/branch_tiny/run_contract.rs"]
mod run_contract;
#[path = "../../bin/branch_tiny/run_deadline.rs"]
mod run_deadline;
#[path = "../../bin/branch_tiny/run_identity.rs"]
mod run_identity;
#[path = "../../bin/branch_tiny/run_loop.rs"]
mod run_loop;
#[path = "../../bin/branch_tiny/run_persistence.rs"]
mod run_persistence;
#[path = "../../bin/branch_tiny/run_slice_request.rs"]
mod run_slice_request;
#[path = "../../bin/branch_tiny/run_slice_result.rs"]
mod run_slice_result;
#[path = "../../bin/branch_tiny/run_startup.rs"]
mod run_startup;
#[path = "../../bin/branch_tiny/run_state_json.rs"]
mod run_state_json;
#[path = "../../bin/branch_tiny/run_stop_recorder.rs"]
mod run_stop_recorder;
#[path = "../../bin/branch_tiny/runner.rs"]
mod runner;
#[path = "../../bin/branch_tiny/shop_investment.rs"]
mod shop_investment;
#[path = "../../bin/branch_tiny/shop_route_evidence.rs"]
mod shop_route_evidence;
#[path = "../../bin/branch_tiny/shop_tiny_owner.rs"]
mod shop_tiny_owner;
#[path = "../../bin/branch_tiny/trace.rs"]
mod trace;
#[path = "../../bin/branch_tiny/trace_format.rs"]
mod trace_format;

use branch_model::{BoundarySite, Branch, BranchStatus, Owner, TerminalOutcome};
use cli_args::{Args, ArgsOverrides, ContinueCapsuleArgs, EventOwnerProbeArgs};
use run_slice_request::ContinueSliceRequest;

use super::RunSliceResult;

pub struct OwnerAuditRuntime;

pub struct OwnerAuditSliceRequest {
    pub args: super::Args,
    pub capsule_path: PathBuf,
    pub resume: bool,
    pub human_output: bool,
}

impl OwnerAuditRuntime {
    pub fn run_capsule_slice(request: OwnerAuditSliceRequest) -> Result<RunSliceResult, String> {
        let slice = ContinueSliceRequest {
            args: request.args,
            overrides: ArgsOverrides::default(),
            capsule_path: request.capsule_path,
            resume: request.resume,
            human_output: request.human_output,
        }
        .prepare()?;
        branch_runtime::BranchRuntime::run_slice(slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::branch::{default_branch_args, RunSliceRequestKind};

    #[test]
    fn owner_audit_runtime_runs_one_capsule_slice_in_process() {
        let root = std::env::temp_dir().join("owner_audit_runtime_start_slice");
        let _ = std::fs::remove_dir_all(&root);
        let mut args = default_branch_args(123);
        args.generations = 0;
        args.max_branches = 1;
        args.search_nodes = 1;
        args.search_ms = 1;
        args.rescue_search_nodes = 1;
        args.rescue_search_ms = 1;
        args.boss_search_nodes = 1;
        args.boss_search_ms = 1;
        args.wall_ms = Some(1_000);

        let result = OwnerAuditRuntime::run_capsule_slice(OwnerAuditSliceRequest {
            args,
            capsule_path: root.clone(),
            resume: false,
            human_output: false,
        })
        .unwrap();

        assert_eq!(result.request_kind, RunSliceRequestKind::Start);
        assert!(root.join("manifest.json").exists());

        let _ = std::fs::remove_dir_all(root);
    }
}
