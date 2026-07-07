#[path = "branch_tiny/boss_relic_owner.rs"]
mod boss_relic_owner;
#[path = "branch_tiny/boundary_router.rs"]
mod boundary_router;
#[path = "branch_tiny/branch_frontier.rs"]
mod branch_frontier;
#[path = "branch_tiny/branch_generation.rs"]
mod branch_generation;
#[path = "branch_tiny/branch_generation_step.rs"]
mod branch_generation_step;
#[path = "branch_tiny/branch_model.rs"]
mod branch_model;
#[path = "branch_tiny/branch_observer.rs"]
mod branch_observer;
#[path = "branch_tiny/branch_path.rs"]
mod branch_path;
#[path = "branch_tiny/branch_runtime.rs"]
mod branch_runtime;
#[path = "branch_tiny/branch_scheduler.rs"]
mod branch_scheduler;
#[path = "branch_tiny/branch_status_view.rs"]
mod branch_status_view;
#[path = "branch_tiny/campfire_owner.rs"]
mod campfire_owner;
#[path = "branch_tiny/candidate_ir_adapter.rs"]
mod candidate_ir_adapter;
#[path = "branch_tiny/card_reward_owner.rs"]
mod card_reward_owner;
#[path = "branch_tiny/cli_args.rs"]
mod cli_args;
#[path = "branch_tiny/combat_gap_case.rs"]
mod combat_gap_case;
#[path = "branch_tiny/combat_portfolio_json.rs"]
mod combat_portfolio_json;
#[path = "branch_tiny/combat_search_dirty_win.rs"]
mod combat_search_dirty_win;
#[path = "branch_tiny/combat_search_lane_commit.rs"]
mod combat_search_lane_commit;
#[path = "branch_tiny/combat_search_lane_options.rs"]
mod combat_search_lane_options;
#[path = "branch_tiny/combat_search_lane_runner.rs"]
mod combat_search_lane_runner;
#[path = "branch_tiny/combat_search_lane_spec.rs"]
mod combat_search_lane_spec;
#[path = "branch_tiny/combat_search_lanes.rs"]
mod combat_search_lanes;
#[path = "branch_tiny/combat_search_orchestrator.rs"]
mod combat_search_orchestrator;
#[path = "branch_tiny/combat_search_portfolio_context.rs"]
mod combat_search_portfolio_context;
#[path = "branch_tiny/combat_search_portfolio_output.rs"]
mod combat_search_portfolio_output;
#[path = "branch_tiny/combat_search_portfolio_plan.rs"]
mod combat_search_portfolio_plan;
#[path = "branch_tiny/combat_search_portfolio_result.rs"]
mod combat_search_portfolio_result;
#[path = "branch_tiny/combat_search_recipe.rs"]
mod combat_search_recipe;
#[path = "branch_tiny/combat_search_report.rs"]
mod combat_search_report;
#[path = "branch_tiny/combat_search_trace_actions.rs"]
mod combat_search_trace_actions;
#[path = "branch_tiny/decision_delta.rs"]
mod decision_delta;
#[path = "branch_tiny/event_owner_bridge.rs"]
mod event_owner_bridge;
#[path = "branch_tiny/event_owner_probe.rs"]
mod event_owner_probe;
#[path = "branch_tiny/expansion_policy.rs"]
mod expansion_policy;
#[path = "branch_tiny/frontier_checkpoint.rs"]
mod frontier_checkpoint;
#[path = "branch_tiny/neow_owner.rs"]
mod neow_owner;
#[path = "branch_tiny/owner_candidate_eval.rs"]
mod owner_candidate_eval;
#[path = "branch_tiny/owner_choice_expander.rs"]
mod owner_choice_expander;
#[path = "branch_tiny/owner_commands.rs"]
mod owner_commands;
#[path = "branch_tiny/owner_model.rs"]
mod owner_model;
#[path = "branch_tiny/owner_orchestrator.rs"]
mod owner_orchestrator;
#[path = "branch_tiny/owner_routines.rs"]
mod owner_routines;
#[path = "branch_tiny/owners.rs"]
mod owners;
#[path = "branch_tiny/render.rs"]
mod render;
#[path = "branch_tiny/render_choice.rs"]
mod render_choice;
#[path = "branch_tiny/reward_tiny_owner.rs"]
mod reward_tiny_owner;
#[path = "branch_tiny/run_capsule.rs"]
mod run_capsule;
#[path = "branch_tiny/run_capsule_format.rs"]
mod run_capsule_format;
#[path = "branch_tiny/run_capsule_io.rs"]
mod run_capsule_io;
#[path = "branch_tiny/run_chain.rs"]
mod run_chain;
#[path = "branch_tiny/run_chain_state.rs"]
mod run_chain_state;
#[path = "branch_tiny/run_choice_owner.rs"]
mod run_choice_owner;
#[path = "branch_tiny/run_contract.rs"]
mod run_contract;
#[path = "branch_tiny/run_deadline.rs"]
mod run_deadline;
#[path = "branch_tiny/run_loop.rs"]
mod run_loop;
#[path = "branch_tiny/run_persistence.rs"]
mod run_persistence;
#[path = "branch_tiny/run_slice_result.rs"]
mod run_slice_result;
#[path = "branch_tiny/run_startup.rs"]
mod run_startup;
#[path = "branch_tiny/run_state_json.rs"]
mod run_state_json;
#[path = "branch_tiny/run_stop_recorder.rs"]
mod run_stop_recorder;
#[path = "branch_tiny/runner.rs"]
mod runner;
#[path = "branch_tiny/shop_investment.rs"]
mod shop_investment;
#[path = "branch_tiny/shop_route_evidence.rs"]
mod shop_route_evidence;
#[path = "branch_tiny/shop_tiny_owner.rs"]
mod shop_tiny_owner;
#[path = "branch_tiny/trace.rs"]
mod trace;
#[path = "branch_tiny/trace_format.rs"]
mod trace_format;

use branch_model::{BoundarySite, Branch, BranchStatus, Owner, TerminalOutcome};
use cli_args::{Args, ArgsOverrides, ContinueCapsuleArgs, EventOwnerProbeArgs};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let context = match run_startup::prepare()? {
        run_startup::RunStartup::Delegated => return Ok(()),
        run_startup::RunStartup::Ready(context) => context,
    };
    branch_runtime::BranchRuntime::run_slice(context).map(|_| ())
}
