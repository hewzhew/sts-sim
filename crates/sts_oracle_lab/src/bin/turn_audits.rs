use std::path::PathBuf;

use clap::Args;
use serde_json::json;
use sts_combat_planner::CombatPolicyChoice;
use sts_oracle_runtime::eval::combat_case::{load_combat_case, save_combat_case};
use sts_oracle_runtime::eval::run_control::existing_combat_knowledge_policy_v1;
use sts_oracle_runtime::sim::combat::{CombatStepLimits, CombatStepper, EngineCombatStepper};
use sts_oracle_runtime::sim::combat_action::combat_action_key;
use sts_oracle_runtime::state::core::ClientInput;

use super::combat_policy_controls::load_action_imitation_policy;
use super::combat_replay_tools::save_combat_inputs;
use super::combat_trace_view::{combat_action_label, combat_turn_snapshot};
use super::print_json;

#[derive(Debug, Args)]
pub(super) struct TurnActionAuditArgs {
    #[arg(long)]
    case: PathBuf,
    #[arg(long)]
    action_imitation_artifact: Option<PathBuf>,
    /// Optional exact action list used only to reach the audited prefix.
    #[arg(long)]
    actions: Option<PathBuf>,
    /// Number of actions from --actions to replay before auditing.
    #[arg(long, default_value_t = 0, requires = "actions")]
    through: usize,
    #[arg(long, default_value_t = 250)]
    max_engine_steps_per_transition: usize,
}

pub(super) fn run_action(args: TurnActionAuditArgs) -> Result<(), String> {
    let TurnActionAuditArgs {
        case,
        action_imitation_artifact,
        actions,
        through,
        max_engine_steps_per_transition,
    } = args;
    let case = load_combat_case(&case)?;
    let mut position = case.position;
    if let Some(actions) = actions {
        let actions = serde_json::from_slice::<Vec<ClientInput>>(
            &std::fs::read(actions).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("invalid prefix action list: {error}"))?;
        if through > actions.len() {
            return Err(format!(
                "--through {through} exceeds the {} available prefix actions",
                actions.len()
            ));
        }
        for (index, input) in actions.into_iter().take(through).enumerate() {
            if EngineCombatStepper
                .choice_for_legal_input(&position, &input)
                .is_none()
            {
                return Err(format!("prefix action {index} is not legal"));
            }
            let result = EngineCombatStepper.apply_to_stable(
                &position,
                input,
                CombatStepLimits {
                    max_engine_steps: max_engine_steps_per_transition,
                    deadline: None,
                },
            );
            if result.truncated || result.timed_out {
                return Err(format!(
                    "prefix action {index} did not reach a stable state"
                ));
            }
            position = result.position;
        }
    }

    let policy = action_imitation_artifact
        .as_deref()
        .map(|path| load_action_imitation_policy(path, existing_combat_knowledge_policy_v1()))
        .transpose()?
        .unwrap_or_else(existing_combat_knowledge_policy_v1);
    let surface = EngineCombatStepper.legal_action_surface(&position);
    let choices = surface
        .atomic_actions
        .iter()
        .map(CombatPolicyChoice::Atomic)
        .chain(
            surface
                .selection_families
                .iter()
                .map(CombatPolicyChoice::StructuredSelection),
        )
        .collect::<Vec<_>>();
    let raw_weights = policy.weights(&position, &choices);
    let raw_weights = (raw_weights.len() == choices.len())
        .then_some(raw_weights)
        .unwrap_or_else(|| vec![1.0; choices.len()]);
    let safe_weights = raw_weights
        .iter()
        .map(|weight| {
            if weight.is_finite() && *weight > 0.0 {
                *weight
            } else {
                1.0
            }
        })
        .collect::<Vec<_>>();
    let total = safe_weights.iter().sum::<f64>();
    let uniform = 1.0 / safe_weights.len().max(1) as f64;
    let probabilities = safe_weights
        .iter()
        .map(|weight| 0.95 * (*weight / total) + 0.05 * uniform)
        .collect::<Vec<_>>();
    let atomic_priority_diagnostics =
                sts_oracle_runtime::ai::combat_search_v2::oracle_action_policy::
                    oracle_atomic_action_policy_priority_diagnostics_v1(
                        &position,
                        &surface.atomic_actions,
                    );
    let atomic = surface
                .atomic_actions
                .iter()
                .enumerate()
                .map(|(index, input)| {
                    let result = EngineCombatStepper.apply_to_stable(
                        &position,
                        input.clone(),
                        CombatStepLimits {
                            max_engine_steps: max_engine_steps_per_transition,
                            deadline: None,
                        },
                    );
                    let raw_weight = safe_weights[index];
                    let rank = 1 + safe_weights
                        .iter()
                        .filter(|candidate| **candidate > raw_weight)
                        .count();
                    let successor_guides = (!result.truncated && !result.timed_out)
                        .then(|| {
                            policy
                                .turn_generation_guides(&result.position)
                                .into_iter()
                                .map(|guide| json!({
                                    "lane": guide.lane.value(),
                                    "components": guide.rank.components(),
                                }))
                                .collect::<Vec<_>>()
                        });
                    json!({
                        "canonical_index": index,
                        "label": combat_action_label(&position, input),
                        "key": combat_action_key(&position.combat, input),
                        "raw_weight": raw_weight,
                        "probability": probabilities[index],
                        "ordinal_rank": rank,
                        "priority": atomic_priority_diagnostics[index],
                        "transition": {
                            "truncated": result.truncated,
                            "timed_out": result.timed_out,
                            "engine_steps": result.engine_steps,
                            "terminal": format!("{:?}", result.terminal),
                            "exact_successor_hash": sts_oracle_runtime::ai::combat_state_key::combat_exact_state_hash_v2(
                                &result.position.engine,
                                &result.position.combat,
                            ),
                            "snapshot": combat_turn_snapshot(&result.position),
                            "generation_guides": successor_guides,
                        },
                    })
                })
                .collect::<Vec<_>>();
    let family_offset = surface.atomic_actions.len();
    let structured_families = surface
        .selection_families
        .iter()
        .enumerate()
        .map(|(index, family)| {
            let weight_index = family_offset + index;
            let raw_weight = safe_weights[weight_index];
            let rank = 1 + safe_weights
                .iter()
                .filter(|candidate| **candidate > raw_weight)
                .count();
            json!({
                "family_index": index,
                "reason": format!("{:?}", family.reason),
                "declared_min": family.declared_min,
                "effective_max": family.effective_max,
                "eligible_domain_count": family.eligible_domain_count,
                "raw_weight": raw_weight,
                "probability": probabilities[weight_index],
                "ordinal_rank": rank,
            })
        })
        .collect::<Vec<_>>();
    print_json(&json!({
        "schema_name": "OracleTurnActionAuditV1",
        "schema_version": 2,
        "through": through,
        "position_hash": sts_oracle_runtime::ai::combat_state_key::combat_exact_state_hash_v2(
            &position.engine,
            &position.combat,
        ),
        "position": combat_turn_snapshot(&position),
        "current_generation_guides": policy
            .turn_generation_guides(&position)
            .into_iter()
            .map(|guide| json!({
                "lane": guide.lane.value(),
                "components": guide.rank.components(),
            }))
            .collect::<Vec<_>>(),
        "atomic_actions": atomic,
        "structured_families": structured_families,
    }))
}

#[derive(Debug, Args)]
pub(super) struct TurnPlanAuditArgs {
    #[arg(long)]
    case: PathBuf,
    #[arg(long, default_value_t = 256)]
    max_inner_nodes: usize,
    #[arg(long, default_value_t = 24)]
    max_end_states: usize,
    #[arg(long, default_value_t = 24)]
    per_bucket_limit: usize,
    #[arg(long, default_value_t = 250)]
    max_engine_steps_per_transition: usize,
    /// Number of selected non-loss turn plans shown by the default compact
    /// report.
    #[arg(long, default_value_t = 8)]
    limit: usize,
    /// Include every selected plan and the complete preselection audit.
    #[arg(long)]
    full: bool,
    /// Export this zero-based rank among the displayed non-loss plans.
    #[arg(long)]
    export_rank: Option<usize>,
    /// Save the selected plan's exact next-turn state as a combat case.
    #[arg(long, requires = "export_rank")]
    export_case: Option<PathBuf>,
    /// Save the selected plan's exact ClientInput list.
    #[arg(long, requires = "export_rank")]
    export_actions: Option<PathBuf>,
}

pub(super) fn run_plan(args: TurnPlanAuditArgs) -> Result<(), String> {
    let TurnPlanAuditArgs {
        case,
        max_inner_nodes,
        max_end_states,
        per_bucket_limit,
        max_engine_steps_per_transition,
        limit,
        full,
        export_rank,
        export_case,
        export_actions,
    } = args;
    let case = load_combat_case(&case)?;
    let mut config = sts_oracle_runtime::ai::combat_search_v2::CombatSearchV2Config::default();
    config.max_engine_steps_per_action = max_engine_steps_per_transition.max(1);
    config.turn_plan_probe_max_inner_nodes = Some(max_inner_nodes.max(1));
    config.turn_plan_probe_max_end_states = Some(max_end_states.max(1));
    config.turn_plan_probe_per_bucket_limit = Some(per_bucket_limit.max(1));
    config.input_label = Some("oracle_lab_turn_plan_audit".to_string());
    let audit = sts_oracle_runtime::ai::combat_search_v2::
                enumerate_combat_search_v2_turn_plan_probe_candidates_across_pending_choices(
                    &case.position.engine,
                    &case.position.combat,
                    &config,
                );
    let exported_plan = if let Some(rank) = export_rank {
        let candidate = audit
            .candidates
            .iter()
            .filter(|candidate| candidate.report.bucket != "terminal_loss")
            .nth(rank)
            .ok_or_else(|| format!("non-loss turn-plan rank {rank} is unavailable"))?;
        if let Some(path) = export_case.as_ref() {
            let mut exported = case.clone();
            exported.position = candidate.position.clone();
            exported.combat =
                sts_oracle_runtime::eval::combat_case::combat_summary(&exported.position);
            exported.run.hp = exported.position.combat.entities.player.current_hp;
            exported.run.max_hp = exported.position.combat.entities.player.max_hp;
            exported.gap.boundary =
                format!("{} + audited turn plan rank {rank}", exported.gap.boundary);
            exported.gap.reason = "oracle_lab_turn_plan_audit_successor".to_string();
            exported.combat_search_attempts.clear();
            exported.failed_search = None;
            save_combat_case(path, &exported)?;
        }
        if let Some(path) = export_actions.as_ref() {
            save_combat_inputs(
                path,
                candidate
                    .report
                    .actions
                    .iter()
                    .map(|action| action.input.clone()),
            )?;
        }
        Some(json!({
            "rank": rank,
            "plan_index": candidate.report.plan_index,
            "case": export_case,
            "actions": export_actions,
        }))
    } else {
        None
    };
    let selected = audit
        .report
        .candidates
        .iter()
        .map(|candidate| {
            json!({
                "plan_index": candidate.plan_index,
                "bucket": candidate.bucket,
                "stop_reason": candidate.stop_reason,
                "action_count": candidate.action_count,
                "actions": candidate.actions.iter().map(|action| {
                    json!({
                        "key": action.action_key,
                        "debug": action.action_debug,
                    })
                }).collect::<Vec<_>>(),
                "end_exact_state_hash": candidate.steps.last().map(|step| {
                    step.state_after_exact_state_hash.as_str()
                }),
                "final_hp": candidate.eval_final_hp,
                "risk_margin": candidate.eval_risk_margin,
                "enemy_progress": candidate.eval_enemy_progress,
            })
        })
        .collect::<Vec<_>>();
    let preselection = audit
        .report
        .selection_audit
        .candidates
        .iter()
        .map(|candidate| {
            json!({
                "preselection_rank": candidate.preselection_rank,
                "selected_plan_index": candidate.selected_plan_index,
                "outcome": candidate.outcome,
                "drop_reason": candidate.drop_reason,
                "bucket": candidate.bucket,
                "action_keys": candidate.action_keys,
            })
        })
        .collect::<Vec<_>>();
    if !full {
        let compact_selected = audit
            .report
            .candidates
            .iter()
            .filter(|candidate| candidate.bucket != "terminal_loss")
            .take(limit)
            .map(|candidate| {
                json!({
                    "plan_index": candidate.plan_index,
                    "bucket": candidate.bucket,
                    "stop_reason": candidate.stop_reason,
                    "action_count": candidate.action_count,
                    "actions": candidate.actions.iter().map(|action| {
                        action.action_key.as_str()
                    }).collect::<Vec<_>>(),
                    "end_exact_state_hash": candidate.steps.last().map(|step| {
                        step.state_after_exact_state_hash.as_str()
                    }),
                    "final_hp": candidate.eval_final_hp,
                    "risk_margin": candidate.eval_risk_margin,
                    "enemy_progress": candidate.eval_enemy_progress,
                })
            })
            .collect::<Vec<_>>();
        return print_json(&json!({
            "schema_name": "OracleTurnPlanAuditCompactV1",
            "schema_version": 1,
            "behavioral_scope": "read_only_no_search_seeding",
            "config": audit.report.config,
            "enumeration": audit.report.enumeration,
            "exported_plan": exported_plan,
            "selected_non_loss": compact_selected,
        }));
    }
    print_json(&json!({
        "schema_name": "OracleTurnPlanAuditV1",
        "schema_version": 1,
        "behavioral_scope": "read_only_no_search_seeding",
        "config": audit.report.config,
        "enumeration": audit.report.enumeration,
        "exported_plan": exported_plan,
        "preselection": preselection,
        "selected": selected,
    }))
}
