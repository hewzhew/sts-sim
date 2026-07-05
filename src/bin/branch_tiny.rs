use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sts_simulator::ai::strategy::decision_pipeline::candidate_lane_label;
use sts_simulator::eval::run_control::{
    CombatSearchTraceSummary, RunControlAutoAppliedStepV1, RunControlSession,
};
use sts_simulator::state::events::EventId;

#[path = "branch_tiny/boundary_router.rs"]
mod boundary_router;
#[path = "branch_tiny/branch_frontier.rs"]
mod branch_frontier;
#[path = "branch_tiny/branch_generation.rs"]
mod branch_generation;
#[path = "branch_tiny/branch_observer.rs"]
mod branch_observer;
#[path = "branch_tiny/branch_scheduler.rs"]
mod branch_scheduler;
#[path = "branch_tiny/branch_status_view.rs"]
mod branch_status_view;
#[path = "branch_tiny/candidate_ir_adapter.rs"]
mod candidate_ir_adapter;
#[path = "branch_tiny/cli_args.rs"]
mod cli_args;
#[path = "branch_tiny/combat_gap_case.rs"]
mod combat_gap_case;
#[path = "branch_tiny/combat_search_lanes.rs"]
mod combat_search_lanes;
#[path = "branch_tiny/combat_search_orchestrator.rs"]
mod combat_search_orchestrator;
#[path = "branch_tiny/combat_search_report.rs"]
mod combat_search_report;
#[path = "branch_tiny/decision_delta.rs"]
mod decision_delta;
#[path = "branch_tiny/event_owner_probe.rs"]
mod event_owner_probe;
#[path = "branch_tiny/expansion_policy.rs"]
mod expansion_policy;
#[path = "branch_tiny/frontier_checkpoint.rs"]
mod frontier_checkpoint;
#[path = "branch_tiny/neow_owner.rs"]
mod neow_owner;
#[path = "branch_tiny/owner_model.rs"]
mod owner_model;
#[path = "branch_tiny/owner_orchestrator.rs"]
mod owner_orchestrator;
#[path = "branch_tiny/owners.rs"]
mod owners;
#[path = "branch_tiny/render.rs"]
mod render;
#[path = "branch_tiny/reward_shop_boss_owner.rs"]
mod reward_shop_boss_owner;
#[path = "branch_tiny/run_capsule.rs"]
mod run_capsule;
#[path = "branch_tiny/run_capsule_format.rs"]
mod run_capsule_format;
#[path = "branch_tiny/run_chain.rs"]
mod run_chain;
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
#[path = "branch_tiny/runner.rs"]
mod runner;
#[path = "branch_tiny/trace.rs"]
mod trace;

use cli_args::{Args, ArgsOverrides, ContinueCapsuleArgs, EventOwnerProbeArgs};
use owner_model::{ChoiceAnnotation, DecisionKey};

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
struct CombatSearchPortfolioReport {
    status: CombatSearchPortfolioStatus,
    max_nodes: usize,
    wall_ms: u64,
    action_keys: Vec<String>,
    attempts: Vec<CombatSearchLaneReport>,
}

#[derive(Clone)]
struct CombatSearchLaneReport {
    label: &'static str,
    status: CombatSearchPortfolioStatus,
    max_nodes: usize,
    wall_ms: u64,
    potion_policy: &'static str,
    max_potions_used: Option<u32>,
    action_keys: Vec<String>,
}

#[derive(Clone)]
enum CombatSearchPortfolioStatus {
    Failed(String),
    Advanced(String),
    Terminal(TerminalOutcome),
}

#[derive(Clone)]
struct BranchPathStep {
    key: Option<DecisionKey>,
    action_debug: String,
    label: String,
    annotation: ChoiceAnnotationSnapshot,
    state_before: Option<BranchPathState>,
    decision_delta: Option<decision_delta::DecisionDeltaSnapshot>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct BranchPathState {
    act: u8,
    floor: i32,
    hp: i32,
    max_hp: i32,
    gold: i32,
    deck_size: usize,
    boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ScoreComponentSnapshot {
    by: String,
    value: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ChoiceAnnotationSnapshot {
    None,
    Candidate {
        lane: String,
        score: i32,
        #[serde(default)]
        scores: Vec<ScoreComponentSnapshot>,
        candidate: Value,
        admission: Option<Value>,
        detail: String,
    },
    BossRelic {
        relic: Value,
        lane: String,
        class: String,
        detail: String,
    },
}

impl ChoiceAnnotationSnapshot {
    fn none() -> Self {
        Self::None
    }

    fn from_annotation(annotation: &ChoiceAnnotation) -> Self {
        match annotation {
            ChoiceAnnotation::None => Self::None,
            ChoiceAnnotation::Candidate(decision) => Self::Candidate {
                lane: candidate_lane_label(decision.evaluation.lane).to_string(),
                score: decision.evaluation.total_score(),
                scores: decision
                    .evaluation
                    .scores
                    .iter()
                    .map(|score| ScoreComponentSnapshot {
                        by: score.by.to_string(),
                        value: score.value,
                    })
                    .collect(),
                candidate: trace::candidate_kind_value(decision.evaluation.candidate.kind),
                admission: decision.admission.as_ref().map(|admission| {
                    json!({
                        "card": admission.card,
                        "class": format!("{:?}", admission.class),
                    })
                }),
                detail: render::render_candidate_decision_compact(decision),
            },
            ChoiceAnnotation::BossRelic(admission) => Self::BossRelic {
                relic: json!(admission.relic),
                lane: format!("{:?}", admission.lane),
                class: format!("{:?}", admission.class),
                detail: sts_simulator::ai::strategy::boss_relic_admission::render_boss_relic_admission_compact(admission),
            },
        }
    }

    fn detail(&self) -> Option<&str> {
        match self {
            Self::None => None,
            Self::Candidate { detail, .. } | Self::BossRelic { detail, .. } => Some(detail),
        }
    }
}

impl BranchPathState {
    fn from_branch(branch: &Branch) -> Self {
        let run = &branch.session.run_state;
        Self {
            act: run.act_num,
            floor: run.floor_num,
            hp: run.current_hp,
            max_hp: run.max_hp,
            gold: run.gold,
            deck_size: run.master_deck.len(),
            boundary: branch_status_view::status_boundary_label(&branch.status),
        }
    }
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
