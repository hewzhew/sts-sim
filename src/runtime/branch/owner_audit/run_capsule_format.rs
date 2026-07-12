use std::path::Path;

use serde_json::{json, Value};
use sts_simulator::eval::combat_case::combat_summary;
use sts_simulator::eval::run_control::RunControlAutoAppliedKindV1;
use sts_simulator::sim::combat::CombatPosition;

use super::branch_path::BranchPathStep;
use super::run_contract::RunContract;
use super::run_identity::{RunIdentity, SourceIdentity};
use super::trajectory_snapshot::FrontierTrajectoryEvaluation;
use super::{
    combat_portfolio_json, run_state_json, trajectory_snapshot, Args, Branch, BranchStatus,
};

pub(super) fn manifest_value(
    args: Args,
    status: &'static str,
    reason: Option<&'static str>,
    created_at_ms: u128,
    updated_at_ms: u128,
    git_commit: &Option<String>,
    source_identity: &SourceIdentity,
) -> Value {
    json!({
        "schema": "branch_tiny_run_capsule",
        "seed": args.seed,
        "ascension": args.ascension,
        "status": status,
        "reason": reason,
        "created_at_epoch_ms": created_at_ms,
        "updated_at_epoch_ms": updated_at_ms,
        "git_commit": git_commit,
        "run_contract": RunContract::from_args(args),
        "run_identity": RunIdentity::from_args(args),
        "source_identity": source_identity,
        "args_schema": "legacy_args_projection_v1",
        "args": args,
    })
}

pub(super) fn branch_summary_value(
    capsule_path: &Path,
    args: Args,
    generation: usize,
    branch: &Branch,
    combat_case: &Value,
    accepted_high_loss_combat_diagnostics: &Value,
    trajectory_evaluation: &FrontierTrajectoryEvaluation,
    capsule_status: &'static str,
    reason: Option<&'static str>,
    frontier: Option<Value>,
) -> Value {
    let run = &branch.session.run_state;
    let status = status_value(&branch.status);
    let status_kind = status
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or(capsule_status);
    let blocker_kind = if capsule_status == "paused" {
        reason.unwrap_or("paused")
    } else {
        status_kind
    };
    let combat = active_combat_value(branch);
    let enemies = combat
        .as_ref()
        .and_then(|value| value.get("enemies"))
        .cloned()
        .unwrap_or(Value::Null);
    let subject = enemies
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        })
        .filter(|text| !text.is_empty());
    let (next_recommended_command, next_recommended_reason) =
        next_recommendation(capsule_path, args, &branch.status, combat_case);
    let mut value = json!({
        "schema": "branch_tiny_capsule_summary",
        "seed": args.seed,
        "ascension": args.ascension,
        "capsule_status": capsule_status,
        "reason": reason,
        "blocker_kind": blocker_kind,
        "generation": generation,
        "branch_id": branch.id,
        "parent_id": branch.parent_id,
        "policy_lane": &branch.policy_lane,
        "trajectory_snapshot": trajectory_snapshot::trajectory_snapshot(branch),
        "trajectory_evaluation": trajectory_evaluation,
        "status": status,
        "act": run.act_num,
        "floor": run.floor_num,
        "hp": run.current_hp,
        "max_hp": run.max_hp,
        "gold": run.gold,
        "deck_size": run.master_deck.len(),
        "subject": subject,
        "enemies": enemies,
        "capsule_path": capsule_path.display().to_string(),
        "combat_case": combat_case,
        "accepted_high_loss_combat_diagnostics": accepted_high_loss_combat_diagnostics,
        "combat_search": combat_search_telemetry_value(branch),
        "primary_search": super::primary_search_outcome::primary_search_outcome_value(
            &branch.combat_search,
            branch.combat_portfolio.as_ref()
        ),
        "next_recommended_command": next_recommended_command,
        "next_recommended_reason": next_recommended_reason,
    });
    if let Some(frontier) = frontier {
        value["frontier"] = frontier;
    }
    value
}

fn combat_search_telemetry_value(branch: &Branch) -> Value {
    let mut summary = sts_simulator::runtime::branch::CombatSearchTelemetrySummary::default();
    for attempt in &branch.combat_search {
        summary.record_attempt_with_timing(
            combat_search_telemetry_source(attempt),
            attempt.complete_win_found,
            attempt.terminal_wins,
            attempt.nodes_expanded,
            attempt.total_us,
            sts_simulator::runtime::branch::CombatSearchTimingSummary {
                rollout_us: attempt.rollout_us,
                expansion_us: attempt.expansion_us,
                engine_step_us: attempt.engine_step_us,
                pre_expand_us: attempt.pre_expand_us,
                frontier_pop_us: attempt.frontier_pop_us,
                child_bookkeeping_us: attempt.child_bookkeeping_us,
                turn_plan_seed_us: attempt.turn_plan_seed_us,
                shadow_audit_us: attempt.shadow_audit_us,
                root_turn_plan_diag_us: attempt.root_turn_plan_diag_us,
                unattributed_us: attempt.unattributed_us,
            },
        );
    }
    serde_json::to_value(summary).unwrap_or(Value::Null)
}

fn combat_search_telemetry_source(
    attempt: &sts_simulator::eval::run_control::CombatSearchTraceSummary,
) -> String {
    match attempt.lane.as_ref() {
        Some(lane) if lane != &attempt.source => format!("{lane}/{}", attempt.source),
        Some(lane) => lane.clone(),
        None => attempt.source.clone(),
    }
}

pub(super) fn frontier_trajectory_summary_value(
    frontier_count: usize,
    running: usize,
    trajectory_evaluation: &FrontierTrajectoryEvaluation,
) -> Value {
    json!({
        "frontier_count": frontier_count,
        "frontier_running_count": running,
        "trajectory_evaluation": trajectory_evaluation,
    })
}

pub(super) fn empty_frontier_summary_value(
    capsule_path: &Path,
    args: Args,
    generation: usize,
    capsule_status: &'static str,
    reason: Option<&'static str>,
    frontier: Value,
) -> Value {
    json!({
        "schema": "branch_tiny_capsule_summary",
        "seed": args.seed,
        "ascension": args.ascension,
        "capsule_status": capsule_status,
        "reason": reason,
        "blocker_kind": reason.unwrap_or(capsule_status),
        "generation": generation,
        "capsule_path": capsule_path.display().to_string(),
        "frontier": frontier,
    })
}

fn next_recommendation(
    capsule_path: &Path,
    args: Args,
    status: &BranchStatus,
    combat_case: &Value,
) -> (Option<String>, Option<&'static str>) {
    if matches!(
        status,
        BranchStatus::CombatGap { .. }
            | BranchStatus::OperationBudgetExhausted { .. }
            | BranchStatus::BudgetGap { .. }
    ) {
        return (
            combat_case.as_str().map(combat_case_review_command),
            Some("combat_case_review"),
        );
    }
    if status.is_resumable() {
        return (
            args.wall_ms
                .map(|wall_ms| branch_tiny_continue_command(capsule_path, wall_ms)),
            Some("continue_capsule"),
        );
    }
    (None, None)
}

fn combat_case_review_command(case: &str) -> String {
    format!("cargo run --quiet --bin combat_case_review -- --case \"{case}\" --ladder --compact")
}

fn branch_tiny_continue_command(capsule_path: &Path, wall_ms: u64) -> String {
    format!(
        "cargo run --quiet --bin branch_tiny -- --continue-capsule \"{}\" --continue-slices 1 --wall-ms {}",
        capsule_path.display(),
        wall_ms
    )
}

pub(super) fn result_value(
    generation: usize,
    branch: &Branch,
    combat_case: Value,
    accepted_high_loss_combat_diagnostics: Value,
    trajectory_evaluation: &FrontierTrajectoryEvaluation,
) -> Value {
    let run = &branch.session.run_state;
    json!({
        "schema": "branch_tiny_run_result",
        "generation": generation,
        "branch_id": branch.id,
        "parent_id": branch.parent_id,
        "policy_lane": &branch.policy_lane,
        "trajectory_snapshot": trajectory_snapshot::trajectory_snapshot(branch),
        "trajectory_evaluation": trajectory_evaluation,
        "status": status_value(&branch.status),
        "state": {
            "act": run.act_num,
            "floor": run.floor_num,
            "hp": run.current_hp,
            "max_hp": run.max_hp,
            "gold": run.gold,
            "deck_size": run.master_deck.len(),
            "strategic_deficit": run_state_json::strategic_deficit_value(run),
        },
        "deck": run_state_json::deck_value(run),
        "relics": run_state_json::relics_value(run),
        "potions": run_state_json::potions_value(run),
        "path": path_value(branch),
        "auto": branch.auto_steps.iter()
            .filter(|step| step.kind != RunControlAutoAppliedKindV1::AutoCapture)
            .map(|step| json!({"kind": format!("{:?}", step.kind), "label": step.label}))
            .collect::<Vec<_>>(),
        "combat": active_combat_value(branch),
        "combat_case": combat_case,
        "accepted_high_loss_combat_diagnostics": accepted_high_loss_combat_diagnostics,
        "combat_portfolio": branch.combat_portfolio.as_ref().map(combat_portfolio_json::capsule_value),
        "combat_search_attempts": &branch.combat_search,
        "combat_search_history": &branch.combat_search_history,
        "primary_search": super::primary_search_outcome::primary_search_outcome_value(
            &branch.combat_search,
            branch.combat_portfolio.as_ref()
        ),
        "failed_search": branch.combat_search.last(),
    })
}

pub(super) fn terminal_results_value(entries: Vec<Value>) -> Value {
    json!({
        "schema": "branch_tiny_terminal_results",
        "terminals": entries,
    })
}

fn active_combat_value(branch: &Branch) -> Option<Value> {
    let active = branch.session.active_combat.as_ref()?;
    serde_json::to_value(combat_summary(&CombatPosition::new(
        active.engine_state.clone(),
        active.combat_state.clone(),
    )))
    .ok()
}

pub(super) fn path_value(branch: &Branch) -> Value {
    json!({
        "schema": "branch_tiny_run_path",
        "branch_id": branch.id,
        "policy_lane": &branch.policy_lane,
        "steps": branch.path.iter().enumerate().map(path_step_value).collect::<Vec<_>>(),
    })
}

fn path_step_value((index, step): (usize, &BranchPathStep)) -> Value {
    json!({
        "step": index,
        "policy_lane": step.policy_lane,
        "state_before": step.state_before.as_ref(),
        "decision_delta": step.decision_delta.as_ref(),
        "key": serde_json::to_value(&step.key).unwrap_or(Value::Null),
        "label": step.label,
        "annotation": serde_json::to_value(&step.annotation).unwrap_or(Value::Null),
        "candidate_pool": serde_json::to_value(&step.candidate_pool).unwrap_or(Value::Null),
        "shop_boss_preview_candidates": serde_json::to_value(&step.shop_boss_preview_candidates)
            .unwrap_or(Value::Null),
        "shop_boss_preview_bundles": serde_json::to_value(&step.shop_boss_preview_bundles)
            .unwrap_or(Value::Null),
    })
}

pub(super) fn status_value(status: &BranchStatus) -> Value {
    match status {
        BranchStatus::Running { boundary, owner } => {
            json!({"kind": "running", "boundary": boundary, "owner": format!("{owner:?}")})
        }
        BranchStatus::AwaitingAuto { boundary, reason } => {
            json!({"kind": "awaiting_auto", "boundary": boundary, "reason": reason})
        }
        BranchStatus::Terminal(result) => json!({"kind": "terminal", "result": result.as_str()}),
        BranchStatus::AutomationGap { boundary, site } => {
            json!({"kind": "automation_gap", "boundary": boundary, "site": format!("{site:?}")})
        }
        BranchStatus::CombatGap { boundary, reason } => {
            json!({"kind": "combat_gap", "boundary": boundary, "reason": reason})
        }
        BranchStatus::OperationBudgetExhausted { boundary, reason } => {
            json!({"kind": "operation_budget_exhausted", "boundary": boundary, "reason": reason})
        }
        BranchStatus::BudgetGap { boundary, reason } => {
            json!({"kind": "budget_gap", "boundary": boundary, "reason": reason})
        }
        BranchStatus::ApplyFailed(reason) => json!({"kind": "apply_failed", "reason": reason}),
        BranchStatus::AdvanceFailed(reason) => {
            json!({"kind": "advance_failed", "reason": reason})
        }
    }
}

pub(super) fn terminal_manifest_status(status: &BranchStatus) -> &'static str {
    match status {
        BranchStatus::Terminal(_) => "terminal",
        BranchStatus::Running { .. } | BranchStatus::AwaitingAuto { .. } => "running",
        BranchStatus::AutomationGap { .. }
        | BranchStatus::CombatGap { .. }
        | BranchStatus::OperationBudgetExhausted { .. }
        | BranchStatus::BudgetGap { .. }
        | BranchStatus::ApplyFailed(_)
        | BranchStatus::AdvanceFailed(_) => "gap",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;
    use sts_simulator::eval::run_control::{RunControlConfig, RunControlSession};

    fn sample_args() -> Args {
        Args {
            seed: 99,
            ascension: 3,
            objective: super::super::run_contract::RunObjective::FirstTerminal,
            generations: 8,
            max_branches: 2,
            auto_ops: 13,
            search_nodes: 100,
            search_ms: 200,
            rescue_search_nodes: 300,
            rescue_search_ms: 400,
            boss_search_nodes: 500,
            boss_search_ms: 600,
            wall_ms: Some(700),
            checkpoint_before_combat_portfolio: true,
            shop_boss_preview_bundle_limit: 0,
            shop_boss_preview_target_floor: None,
            wall_capped_search_budget: true,
            wall_capped_boss_budget: true,
        }
    }

    fn sample_branch() -> Branch {
        Branch {
            id: 1,
            parent_id: None,
            path: Vec::new(),
            session: RunControlSession::new(RunControlConfig::default()),
            status: BranchStatus::AwaitingAuto {
                boundary: "Combat".to_string(),
                reason: "diagnostic".to_string(),
            },
            policy_lane: super::super::branch_policy_lane::BranchPolicyLane::default(),
            combat_portfolio: None,
            auto_steps: Vec::new(),
            combat_search: Vec::new(),
            combat_search_history: Vec::new(),
            accepted_high_loss_diagnostics: Vec::new(),
        }
    }

    fn challenger_sample_branch(lane_id: u8) -> Branch {
        let mut branch = sample_branch();
        branch.id = lane_id as usize + 1;
        branch.policy_lane = super::super::branch_policy_lane::BranchPolicyLane::challenger(
            sts_simulator::ai::strategy::challenger_policy_state::ChallengerPolicyState::new(
                lane_id,
            ),
        );
        branch
    }

    fn evaluation(branches: Vec<Branch>) -> FrontierTrajectoryEvaluation {
        trajectory_snapshot::frontier_trajectory_evaluation(&VecDeque::from(branches))
    }

    #[test]
    fn frontier_summary_exposes_paired_trajectory_evaluation() {
        let frontier = VecDeque::from([sample_branch(), challenger_sample_branch(1)]);
        let trajectory_evaluation = trajectory_snapshot::frontier_trajectory_evaluation(&frontier);
        let value = frontier_trajectory_summary_value(2, 2, &trajectory_evaluation);

        assert_eq!(value["frontier_count"], 2);
        assert_eq!(
            value["trajectory_evaluation"]["snapshots"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            value["trajectory_evaluation"]["comparisons"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn result_exposes_accepted_high_loss_diagnostic_paths() {
        let diagnostics = json!([{
            "capture": "target/capture.json",
            "evidence": "target/evidence.json"
        }]);

        let branch = sample_branch();
        let trajectory_evaluation = evaluation(vec![branch.clone()]);
        let value = result_value(
            3,
            &branch,
            Value::Null,
            diagnostics.clone(),
            &trajectory_evaluation,
        );

        assert_eq!(value["accepted_high_loss_combat_diagnostics"], diagnostics);
        assert!(value["combat_case"].is_null());
    }

    #[test]
    fn result_exposes_persistent_policy_lane_identity() {
        let mut branch = sample_branch();
        branch.policy_lane = super::super::branch_policy_lane::BranchPolicyLane::challenger(
            sts_simulator::ai::strategy::challenger_policy_state::ChallengerPolicyState::new(2),
        );

        let trajectory_evaluation = evaluation(vec![branch.clone()]);
        let value = result_value(3, &branch, Value::Null, Value::Null, &trajectory_evaluation);

        assert_eq!(value["policy_lane"]["kind"], "challenger");
        assert_eq!(value["policy_lane"]["policy"]["lane_id"], 2);
    }

    #[test]
    fn manifest_writes_run_contract_and_legacy_args_projection() {
        let value = manifest_value(
            sample_args(),
            "running",
            None,
            10,
            20,
            &Some("abc123".to_string()),
            &super::super::run_identity::SourceIdentity {
                git_commit: Some("abc123".to_string()),
                git_dirty: Some(true),
            },
        );

        assert_eq!(value["run_contract"]["game"]["seed"], 99);
        assert_eq!(value["run_identity"]["run_contract"]["game"]["seed"], 99);
        assert_eq!(value["source_identity"]["git_commit"], "abc123");
        assert_eq!(value["source_identity"]["git_dirty"], true);
        assert_eq!(value["run_contract"]["game"]["ascension"], 3);
        assert_eq!(value["run_contract"]["slice"]["slice_ms"], 700);
        assert_eq!(value["run_contract"]["combat_search"]["boss_ms"], 600);
        assert_eq!(value["args"]["wall_ms"], 700);
        assert_eq!(value["args_schema"], "legacy_args_projection_v1");
        assert!(value["run_contract"]["wall_capped_search_budget"].is_null());
    }

    #[test]
    fn continue_recommendation_uses_cargo_run_not_stale_exe() {
        let command = branch_tiny_continue_command(Path::new("target/capsule"), 60_000);

        assert_eq!(
            command,
            "cargo run --quiet --bin branch_tiny -- --continue-capsule \"target/capsule\" --continue-slices 1 --wall-ms 60000"
        );
        assert!(!command.contains("target\\debug"));
        assert!(!command.contains("branch_tiny.exe"));
    }

    #[test]
    fn combat_review_recommendation_uses_cargo_run_not_stale_exe() {
        let command = combat_case_review_command("target/cases/case.json");

        assert_eq!(
            command,
            "cargo run --quiet --bin combat_case_review -- --case \"target/cases/case.json\" --ladder --compact"
        );
        assert!(!command.contains("target\\debug"));
        assert!(!command.contains("combat_case_review.exe"));
    }

    #[test]
    fn primary_search_outcome_projects_profile_and_telemetry() {
        let attempt = sts_simulator::eval::run_control::CombatSearchTraceSummary {
            source: "search_combat".to_string(),
            lane: Some("primary".to_string()),
            profile_id: None,
            profile_max_nodes: None,
            profile_wall_ms: None,
            profile_potion_policy: None,
            profile_max_potions_used: None,
            profile_internal_no_win_rescue_enabled: None,
            act: 1,
            floor: 14,
            turn: 1,
            combat_kind: "hallway".to_string(),
            enemies: vec!["Spike Slime L".to_string()],
            coverage_status: "DeadlineHit".to_string(),
            complete_trajectory_found: false,
            complete_win_found: false,
            best_complete: None,
            best_win: None,
            best_hp_loss: None,
            nodes_to_first_win: Some(17),
            deadline_hit: true,
            nodes_expanded: 42,
            terminal_wins: 0,
            total_us: 125_000,
            unattributed_us: 7,
            rollout_us: 11,
            expansion_us: 13,
            child_bookkeeping_us: 17,
            engine_step_us: 19,
            pre_expand_us: 23,
            frontier_pop_us: 29,
            turn_plan_seed_us: 31,
            shadow_audit_us: 37,
            root_turn_plan_diag_us: 41,
        };
        let report = super::super::combat_search_report::CombatSearchPortfolioReport {
            status: super::super::combat_search_report::CombatSearchPortfolioStatus::Failed(
                "no_complete_winning_candidate".to_string(),
            ),
            max_nodes: 1_000,
            wall_ms: 500,
            action_keys: vec!["combat/play:Strike:target0".to_string()],
            attempts: vec![super::super::combat_search_report::CombatSearchLaneReport {
                label: "primary",
                status: super::super::combat_search_report::CombatSearchPortfolioStatus::Failed(
                    "no_complete_winning_candidate".to_string(),
                ),
                max_nodes: 1_000,
                wall_ms: 500,
                potion_policy: "never",
                max_potions_used: Some(0),
                action_keys: vec!["combat/play:Strike:target0".to_string()],
            }],
        };

        let value = super::super::primary_search_outcome::primary_search_outcome_value(
            &[attempt],
            Some(&report),
        );

        assert_eq!(value["status"], "no_accepted_line");
        assert_eq!(value["profile"]["profile_id"], "primary");
        assert_eq!(value["profile"]["stakes"], "hallway");
        assert_eq!(value["profile"]["max_nodes"], 1_000);
        assert_eq!(value["profile"]["wall_ms"], 500);
        assert_eq!(value["profile"]["potion_policy"], "never");
        assert_eq!(value["profile"]["max_potions_used"], 0);
        assert_eq!(value["profile"]["internal_no_win_rescue_enabled"], false);
        assert!(value["accepted_line"].is_null());
        assert_eq!(value["telemetry"]["expanded_nodes"], 42);
        assert_eq!(value["telemetry"]["terminal_wins"], 0);
        assert_eq!(value["telemetry"]["deadline_hit"], true);
        assert_eq!(value["telemetry"]["first_win_node"], 17);
        assert_eq!(value["telemetry"]["elapsed_ms"], 125);
        assert_eq!(value["telemetry"]["rollout_us"], 11);
        assert_eq!(value["telemetry"]["expansion_us"], 13);
        assert_eq!(value["telemetry"]["transition_us"], 19);
        assert_eq!(
            value["telemetry"]["selected_first_action"],
            "combat/play:Strike:target0"
        );
    }

    #[test]
    fn primary_search_outcome_uses_trace_profile_when_portfolio_missing() {
        let attempt = sts_simulator::eval::run_control::CombatSearchTraceSummary {
            source: "search_combat".to_string(),
            lane: Some("primary".to_string()),
            profile_id: Some("primary".to_string()),
            profile_max_nodes: Some(10_000),
            profile_wall_ms: Some(100),
            profile_potion_policy: Some("never".to_string()),
            profile_max_potions_used: Some(0),
            profile_internal_no_win_rescue_enabled: Some(false),
            act: 1,
            floor: 14,
            turn: 1,
            combat_kind: "hallway".to_string(),
            enemies: vec!["Spike Slime L".to_string()],
            coverage_status: "DeadlineHit".to_string(),
            complete_trajectory_found: false,
            complete_win_found: false,
            best_complete: None,
            best_win: None,
            best_hp_loss: None,
            nodes_to_first_win: None,
            deadline_hit: true,
            nodes_expanded: 55,
            terminal_wins: 0,
            total_us: 121_000,
            unattributed_us: 0,
            rollout_us: 38_563,
            expansion_us: 27_720,
            child_bookkeeping_us: 0,
            engine_step_us: 6_477,
            pre_expand_us: 0,
            frontier_pop_us: 0,
            turn_plan_seed_us: 0,
            shadow_audit_us: 0,
            root_turn_plan_diag_us: 0,
        };

        let value =
            super::super::primary_search_outcome::primary_search_outcome_value(&[attempt], None);

        assert_eq!(value["status"], "no_accepted_line");
        assert_eq!(value["profile"]["profile_id"], "primary");
        assert_eq!(value["profile"]["stakes"], "hallway");
        assert_eq!(value["profile"]["max_nodes"], 10_000);
        assert_eq!(value["profile"]["wall_ms"], 100);
        assert_eq!(value["profile"]["potion_policy"], "never");
        assert_eq!(value["profile"]["max_potions_used"], 0);
        assert_eq!(value["profile"]["internal_no_win_rescue_enabled"], false);
        assert_eq!(value["telemetry"]["expanded_nodes"], 55);
        assert_eq!(value["telemetry"]["deadline_hit"], true);
    }

    #[test]
    fn primary_search_outcome_projects_attempt_economics() {
        let attempt = sts_simulator::eval::run_control::CombatSearchTraceSummary {
            source: "search_combat".to_string(),
            lane: Some("primary".to_string()),
            profile_id: Some("primary".to_string()),
            profile_max_nodes: Some(10_000),
            profile_wall_ms: Some(100),
            profile_potion_policy: Some("never".to_string()),
            profile_max_potions_used: Some(0),
            profile_internal_no_win_rescue_enabled: Some(false),
            act: 1,
            floor: 14,
            turn: 1,
            combat_kind: "hallway".to_string(),
            enemies: vec!["Spike Slime L".to_string()],
            coverage_status: "DeadlineHit".to_string(),
            complete_trajectory_found: false,
            complete_win_found: false,
            best_complete: None,
            best_win: None,
            best_hp_loss: None,
            nodes_to_first_win: None,
            deadline_hit: true,
            nodes_expanded: 4,
            terminal_wins: 0,
            total_us: 1_000,
            unattributed_us: 100,
            rollout_us: 500,
            expansion_us: 200,
            child_bookkeeping_us: 0,
            engine_step_us: 100,
            pre_expand_us: 0,
            frontier_pop_us: 0,
            turn_plan_seed_us: 0,
            shadow_audit_us: 50,
            root_turn_plan_diag_us: 50,
        };

        let value =
            super::super::primary_search_outcome::primary_search_outcome_value(&[attempt], None);

        assert_eq!(value["telemetry"]["us_per_node"], 250);
        assert_eq!(value["telemetry"]["rollout_pct"], 50);
        assert_eq!(value["telemetry"]["expansion_pct"], 20);
        assert_eq!(value["telemetry"]["transition_pct"], 10);
        assert_eq!(value["telemetry"]["diagnostic_pct"], 10);
        assert_eq!(value["telemetry"]["unattributed_pct"], 10);
    }
}
