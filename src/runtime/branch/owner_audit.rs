#![allow(dead_code)]

use std::path::PathBuf;

#[path = "owner_audit/accepted_combat_attrition.rs"]
mod accepted_combat_attrition;
#[path = "owner_audit/accepted_high_loss_diagnostic.rs"]
mod accepted_high_loss_diagnostic;
#[path = "owner_audit/boss_relic_owner.rs"]
mod boss_relic_owner;
#[path = "owner_audit/boundary_router.rs"]
mod boundary_router;
#[path = "owner_audit/branch_frontier.rs"]
mod branch_frontier;
#[path = "owner_audit/branch_generation.rs"]
mod branch_generation;
#[path = "owner_audit/branch_generation_step.rs"]
mod branch_generation_step;
#[path = "owner_audit/branch_model.rs"]
mod branch_model;
#[path = "owner_audit/branch_observer.rs"]
mod branch_observer;
#[path = "owner_audit/branch_path.rs"]
mod branch_path;
#[path = "owner_audit/branch_policy_lane.rs"]
mod branch_policy_lane;
#[path = "owner_audit/branch_runtime.rs"]
mod branch_runtime;
#[path = "owner_audit/branch_scheduler.rs"]
mod branch_scheduler;
#[path = "owner_audit/branch_status_view.rs"]
mod branch_status_view;
#[path = "owner_audit/campfire_owner.rs"]
mod campfire_owner;
#[path = "owner_audit/candidate_ir_adapter.rs"]
mod candidate_ir_adapter;
#[path = "owner_audit/capsule_artifact_store.rs"]
mod capsule_artifact_store;
#[path = "owner_audit/card_reward_owner.rs"]
mod card_reward_owner;
#[path = "owner_audit/cli_args.rs"]
mod cli_args;
#[path = "owner_audit/combat_gap_case.rs"]
mod combat_gap_case;
#[path = "owner_audit/combat_portfolio_json.rs"]
mod combat_portfolio_json;
#[path = "owner_audit/combat_search_dirty_win.rs"]
mod combat_search_dirty_win;
#[path = "owner_audit/combat_search_lane_commit.rs"]
mod combat_search_lane_commit;
#[path = "owner_audit/combat_search_lane_options.rs"]
mod combat_search_lane_options;
#[path = "owner_audit/combat_search_lane_runner.rs"]
mod combat_search_lane_runner;
#[path = "owner_audit/combat_search_lane_spec.rs"]
mod combat_search_lane_spec;
#[path = "owner_audit/combat_search_lanes.rs"]
mod combat_search_lanes;
#[path = "owner_audit/combat_search_orchestrator.rs"]
mod combat_search_orchestrator;
#[path = "owner_audit/combat_search_portfolio_context.rs"]
mod combat_search_portfolio_context;
#[path = "owner_audit/combat_search_portfolio_output.rs"]
mod combat_search_portfolio_output;
#[path = "owner_audit/combat_search_portfolio_plan.rs"]
mod combat_search_portfolio_plan;
#[path = "owner_audit/combat_search_portfolio_result.rs"]
mod combat_search_portfolio_result;
#[path = "owner_audit/combat_search_recipe.rs"]
mod combat_search_recipe;
#[path = "owner_audit/combat_search_report.rs"]
mod combat_search_report;
#[path = "owner_audit/combat_search_survival.rs"]
mod combat_search_survival;
#[path = "owner_audit/combat_search_trace_actions.rs"]
mod combat_search_trace_actions;
#[path = "owner_audit/decision_delta.rs"]
mod decision_delta;
#[cfg(test)]
#[path = "owner_audit/event_owner_boundaries.rs"]
mod event_owner_boundaries;
#[path = "owner_audit/event_owner_bridge.rs"]
mod event_owner_bridge;
#[path = "owner_audit/event_owner_probe.rs"]
mod event_owner_probe;
#[path = "owner_audit/expansion_policy.rs"]
mod expansion_policy;
#[path = "owner_audit/frontier_checkpoint.rs"]
mod frontier_checkpoint;
#[path = "owner_audit/neow_owner.rs"]
mod neow_owner;
#[path = "owner_audit/owner_candidate_eval.rs"]
mod owner_candidate_eval;
#[path = "owner_audit/owner_choice_expander.rs"]
mod owner_choice_expander;
#[path = "owner_audit/owner_commands.rs"]
mod owner_commands;
#[path = "owner_audit/owner_model.rs"]
mod owner_model;
#[path = "owner_audit/owner_orchestrator.rs"]
mod owner_orchestrator;
#[path = "owner_audit/owner_routines.rs"]
mod owner_routines;
#[path = "owner_audit/owners.rs"]
mod owners;
#[path = "owner_audit/policy_expansion_plan.rs"]
mod policy_expansion_plan;
#[path = "owner_audit/primary_search_outcome.rs"]
mod primary_search_outcome;
#[path = "owner_audit/render.rs"]
mod render;
#[path = "owner_audit/render_choice.rs"]
mod render_choice;
#[path = "owner_audit/reward_tiny_owner.rs"]
mod reward_tiny_owner;
#[path = "owner_audit/run_capsule.rs"]
mod run_capsule;
#[path = "owner_audit/run_capsule_format.rs"]
mod run_capsule_format;
#[path = "owner_audit/run_capsule_io.rs"]
mod run_capsule_io;
#[path = "owner_audit/run_chain.rs"]
mod run_chain;
#[path = "owner_audit/run_chain_state.rs"]
mod run_chain_state;
#[path = "owner_audit/run_choice_owner.rs"]
mod run_choice_owner;
#[path = "owner_audit/run_contract.rs"]
mod run_contract;
#[path = "owner_audit/run_deadline.rs"]
mod run_deadline;
#[path = "owner_audit/run_identity.rs"]
mod run_identity;
#[path = "owner_audit/run_loop.rs"]
mod run_loop;
#[path = "owner_audit/run_persistence.rs"]
mod run_persistence;
#[path = "owner_audit/run_slice_request.rs"]
mod run_slice_request;
#[path = "owner_audit/run_slice_result.rs"]
mod run_slice_result;
#[path = "owner_audit/run_startup.rs"]
mod run_startup;
#[path = "owner_audit/run_state_json.rs"]
mod run_state_json;
#[path = "owner_audit/run_stop_recorder.rs"]
mod run_stop_recorder;
#[path = "owner_audit/runner.rs"]
mod runner;
#[path = "owner_audit/shop_boss_preview_bundle_expansion.rs"]
mod shop_boss_preview_bundle_expansion;
#[path = "owner_audit/shop_investment.rs"]
mod shop_investment;
#[path = "owner_audit/shop_route_evidence.rs"]
mod shop_route_evidence;
#[path = "owner_audit/shop_tiny_owner.rs"]
mod shop_tiny_owner;
#[path = "owner_audit/trace.rs"]
mod trace;
#[path = "owner_audit/trace_format.rs"]
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
    pub fn run_cli() -> Result<(), String> {
        let context = match run_startup::prepare()? {
            run_startup::RunStartup::Delegated => return Ok(()),
            run_startup::RunStartup::Ready(context) => context,
        };
        branch_runtime::BranchRuntime::run_slice(context).map(|_| ())
    }

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
    use crate::runtime::branch::{default_branch_args, ArtifactKind, RunSliceRequestKind};

    #[test]
    fn owner_audit_runtime_exposes_cli_entrypoint() {
        let _entrypoint: fn() -> Result<(), String> = OwnerAuditRuntime::run_cli;
    }

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
        assert!(result.artifacts.manifest_written);
        assert!(result.artifacts.frontier_written);
        assert!(result.artifacts.summary_written);
        assert!(!result.artifacts.result_written);
        assert_eq!(
            result
                .artifacts
                .manifest_ref
                .as_ref()
                .map(|artifact| artifact.kind),
            Some(ArtifactKind::Manifest)
        );
        assert_eq!(
            result
                .artifacts
                .frontier_ref
                .as_ref()
                .map(|artifact| artifact.kind),
            Some(ArtifactKind::Frontier)
        );
        assert_eq!(
            result
                .artifacts
                .summary_ref
                .as_ref()
                .map(|artifact| artifact.kind),
            Some(ArtifactKind::Summary)
        );
        assert!(result
            .artifacts
            .frontier_ref
            .as_ref()
            .unwrap()
            .path
            .ends_with("frontier.json"));
        assert!(root.join("manifest.json").exists());
        assert!(root.join("frontier.json").exists());
        assert!(root.join("summary.json").exists());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn owner_audit_runtime_records_capsule_ledger_slice_event() {
        let root = std::env::temp_dir().join("owner_audit_runtime_capsule_ledger");
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

        OwnerAuditRuntime::run_capsule_slice(OwnerAuditSliceRequest {
            args,
            capsule_path: root.clone(),
            resume: false,
            human_output: false,
        })
        .unwrap();

        let ledger = std::fs::read_to_string(root.join("capsule_ledger.jsonl")).unwrap();
        let rows = ledger
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["schema"], "branch_tiny_capsule_ledger_event_v0");
        assert_eq!(rows[0]["event"], "slice_started");
        assert_eq!(rows[0]["request_kind"], "start");
        assert_eq!(rows[0]["seed"], 123);
        assert_eq!(rows[0]["generation_start"], 0);
        assert!(rows[0]["generation_end"].is_null());
        assert!(rows[0]["artifact_refs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|artifact| artifact["kind"] == "manifest"));
        assert_eq!(rows[1]["event"], "slice_finished");
        assert_eq!(rows[1]["seed"], 123);
        assert_eq!(rows[1]["generation_start"], 0);
        assert_eq!(rows[1]["generation_end"], 0);
        assert!(rows[1]["artifact_refs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|artifact| {
                artifact["kind"] == "frontier"
                    && artifact["schema"] == "branch_tiny_frontier_checkpoint"
            }));

        let _ = std::fs::remove_dir_all(root);
    }
}
