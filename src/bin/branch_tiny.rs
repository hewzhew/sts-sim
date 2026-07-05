use serde::{Deserialize, Serialize};
use sts_simulator::eval::run_control::{
    CombatSearchTraceSummary, RunControlAutoAppliedStepV1, RunControlSession,
};
use sts_simulator::state::events::EventId;

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
#[path = "branch_tiny/branch_observer.rs"]
mod branch_observer;
#[path = "branch_tiny/branch_path.rs"]
mod branch_path;
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
#[path = "branch_tiny/combat_search_lane_options.rs"]
mod combat_search_lane_options;
#[path = "branch_tiny/combat_search_lane_runner.rs"]
mod combat_search_lane_runner;
#[path = "branch_tiny/combat_search_lanes.rs"]
mod combat_search_lanes;
#[path = "branch_tiny/combat_search_orchestrator.rs"]
mod combat_search_orchestrator;
#[path = "branch_tiny/combat_search_report.rs"]
mod combat_search_report;
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

use branch_path::BranchPathStep;
use cli_args::{Args, ArgsOverrides, ContinueCapsuleArgs, EventOwnerProbeArgs};
use combat_search_report::CombatSearchPortfolioReport;

#[derive(Clone)]
struct Branch {
    id: usize,
    parent_id: Option<usize>,
    path: Vec<BranchPathStep>,
    session: RunControlSession,
    status: BranchStatus,
    combat_portfolio: Option<CombatSearchPortfolioReport>,
    auto_steps: Vec<RunControlAutoAppliedStepV1>,
    combat_search: Vec<CombatSearchTraceSummary>,
}

#[derive(Clone)]
enum BranchStatus {
    Running {
        boundary: String,
        owner: Owner,
    },
    AwaitingAuto {
        boundary: String,
        reason: String,
    },
    Terminal(TerminalOutcome),
    AutomationGap {
        boundary: String,
        site: BoundarySite,
    },
    CombatGap {
        boundary: String,
        reason: String,
    },
    OperationBudgetExhausted {
        boundary: String,
        reason: String,
    },
    BudgetGap {
        boundary: String,
        reason: String,
    },
    ApplyFailed(String),
    AdvanceFailed(String),
}

impl BranchStatus {
    fn is_resumable(&self) -> bool {
        matches!(
            self,
            BranchStatus::Running { .. } | BranchStatus::AwaitingAuto { .. }
        )
    }

    fn is_expandable_now(&self) -> bool {
        matches!(self, BranchStatus::Running { .. })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
enum TerminalOutcome {
    Victory,
    Defeat,
}

impl TerminalOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Victory => "victory",
            Self::Defeat => "defeat",
        }
    }
}

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
enum Owner {
    NeowStart,
    CardReward,
    BossRelic,
    Event(EventId),
    RewardTiny,
    ShopTiny,
    Campfire,
    RunChoice,
}

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
enum BoundarySite {
    Event(EventId),
    Reward,
    Shop,
    Route,
    Campfire,
    BossRelic,
    RunChoice,
    Treasure,
    Terminal,
    Unknown,
}

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
    run_loop::run(context)
}
