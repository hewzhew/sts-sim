use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, to_value, Value};
use sts_simulator::eval::combat_case::{
    living_enemy_names, save_combat_case, CombatCase, CombatCaseBranchEvidence, CombatCaseGap,
    CombatCasePathStep, CombatCaseRngSummary, CombatCaseRunSummary, CombatCaseSource,
};
use sts_simulator::runtime::combat::CombatState;
use sts_simulator::sim::combat::CombatPosition;
use sts_simulator::state::core::EngineState;

use super::branch_path::BranchPathStep;
use super::{trajectory_snapshot, Args, Branch, BranchStatus};

pub(super) fn save_combat_gap_case(
    dir: &Path,
    args: Args,
    generation: usize,
    branch: &Branch,
) -> Result<Option<PathBuf>, String> {
    let Some(position) = current_stable_combat_position(branch) else {
        return Ok(None);
    };
    let (boundary, reason) = match &branch.status {
        BranchStatus::CombatGap { boundary, reason }
        | BranchStatus::OperationBudgetExhausted { boundary, reason }
        | BranchStatus::BudgetGap { boundary, reason } => (boundary.as_str(), reason.as_str()),
        _ => return Ok(None),
    };
    fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    let path = dir.join(case_filename(
        args.seed,
        generation,
        branch,
        &position.combat,
    ));
    let mut case = CombatCase::new(
        CombatCaseSource {
            seed: args.seed,
            ascension: args.ascension,
            generation,
            branch_id: branch.id,
            parent_id: branch.parent_id,
        },
        CombatCaseGap {
            boundary: boundary.to_string(),
            reason: reason.to_string(),
            search_nodes: args.search_nodes,
            search_ms: args.search_ms,
            rescue_search_nodes: args.rescue_search_nodes,
            rescue_search_ms: args.rescue_search_ms,
        },
        CombatCaseRunSummary {
            act: branch.session.run_state.act_num,
            floor: branch.session.run_state.floor_num,
            hp: branch.session.run_state.current_hp,
            max_hp: branch.session.run_state.max_hp,
            gold: branch.session.run_state.gold,
            deck_size: branch.session.run_state.master_deck.len(),
            relic_count: branch.session.run_state.relics.len(),
            potion_slots: branch.session.run_state.potions.len(),
        },
        branch.combat_search.clone(),
        branch.combat_search.last().cloned(),
        branch.path.iter().map(path_step).collect(),
        CombatCaseRngSummary::from_pool(&branch.session.run_state.rng_pool),
        position,
    );
    case.branch_evidence = Some(branch_evidence(branch));
    save_combat_case(&path, &case)?;
    Ok(Some(path))
}

fn current_stable_combat_position(branch: &Branch) -> Option<CombatPosition> {
    let active = branch.session.active_combat.as_ref()?;
    if !matches!(
        active.engine_state,
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
    ) {
        return None;
    }
    Some(CombatPosition::new(
        active.engine_state.clone(),
        active.combat_state.clone(),
    ))
}

fn path_step(step: &BranchPathStep) -> CombatCasePathStep {
    CombatCasePathStep {
        key: to_value(&step.key).unwrap_or(Value::Null),
        label: step.label.clone(),
        state_before: step
            .state_before
            .as_ref()
            .and_then(|state| to_value(state).ok()),
        decision_evidence: Some(json!({
            "policy_lane": step.policy_lane,
            "policy_selection": &step.policy_selection,
            "annotation": &step.annotation,
            "decision_delta": &step.decision_delta,
            "candidate_pool": &step.candidate_pool,
            "shop_boss_preview_candidates": &step.shop_boss_preview_candidates,
        })),
    }
}

fn branch_evidence(branch: &Branch) -> CombatCaseBranchEvidence {
    CombatCaseBranchEvidence {
        schema: "branch_policy_combat_evidence_v0".to_string(),
        policy_lane: to_value(&branch.policy_lane).unwrap_or(Value::Null),
        trajectory_snapshot: trajectory_snapshot::trajectory_snapshot(branch),
    }
}

fn case_filename(seed: u64, generation: usize, branch: &Branch, combat: &CombatState) -> String {
    let enemies = living_enemy_names(combat)
        .into_iter()
        .map(|name| slug(&name))
        .collect::<Vec<_>>()
        .join("_");
    format!(
        "seed{}_g{:02}_b{:04}_a{}f{}{}.json",
        seed,
        generation,
        branch.id,
        branch.session.run_state.act_num,
        branch.session.run_state.floor_num,
        if enemies.is_empty() {
            String::new()
        } else {
            format!("_{enemies}")
        }
    )
}

fn slug(raw: &str) -> String {
    let mut out = String::new();
    let mut last_sep = false;
    for ch in raw.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_sep = false;
        } else if !last_sep {
            out.push('_');
            last_sep = true;
        }
    }
    out.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use sts_simulator::ai::strategy::candidate_pressure_response::StrategyCommitmentKind;
    use sts_simulator::ai::strategy::challenger_policy_state::ChallengerPolicyState;
    use sts_simulator::ai::strategy::decision_pipeline::CandidateLane;
    use sts_simulator::ai::strategy::pressure_assessment::PressureAxis;
    use sts_simulator::eval::run_control::RunDecisionAction;
    use sts_simulator::eval::run_control::{RunControlConfig, RunControlSession};

    use super::*;
    use crate::runtime::branch::owner_audit::branch_model::BranchStatus;
    use crate::runtime::branch::owner_audit::branch_path::{
        BranchPathCandidateSnapshot, BranchPathPolicySelectionSnapshot,
        BranchPathShopBossPreviewSnapshot, BranchPathStep, ChoiceAnnotationSnapshot,
    };
    use crate::runtime::branch::owner_audit::branch_policy_lane::BranchPolicyLane;
    use crate::runtime::branch::owner_audit::decision_delta::DecisionDeltaSnapshot;
    use crate::runtime::branch::owner_audit::owner_model::{
        ChoiceAnnotation, OwnerChoice, OwnerChoiceExpansion,
    };
    use crate::runtime::branch::owner_audit::policy_expansion_plan::{
        PolicyExpansionClass, PolicyExpansionEvidence,
    };

    fn branch_path_step_with_all_evidence() -> BranchPathStep {
        BranchPathStep {
            policy_lane: "challenger-1".to_string(),
            policy_selection: Some(BranchPathPolicySelectionSnapshot::from_evidence(
                &PolicyExpansionEvidence {
                    class: PolicyExpansionClass::CommitmentRepair,
                    matched_pressure_axes: vec![PressureAxis::GrowthHorizon],
                    matched_commitments: vec![StrategyCommitmentKind::ExhaustEngine],
                    original_lane: CandidateLane::Reject,
                    original_inspect_only: Some("candidate score rejected".to_string()),
                    overrode_reject: true,
                    checkpoint_ref: "branch-0/step-0".to_string(),
                },
            )),
            candidate_id: None,
            key: None,
            action_debug: "Noop".to_string(),
            label: "candidate".to_string(),
            annotation: ChoiceAnnotationSnapshot::none(),
            state_before: None,
            decision_delta: Some(DecisionDeltaSnapshot {
                deck_size_before: 10,
                deck_size_after: 11,
                gold_before: 100,
                gold_after: 100,
                changes: Vec::new(),
                improved_fields: vec!["block_or_mitigation".to_string()],
                worsened_fields: Vec::new(),
                saturated_fields: Vec::new(),
                adds_card_without_gap_improvement: false,
                burden_worsened: false,
            }),
            candidate_pool: BranchPathCandidateSnapshot::from_choices(
                &[OwnerChoice {
                    candidate_id: "combat-gap".to_string(),
                    key: None,
                    action: RunDecisionAction::Input(
                        sts_simulator::state::core::ClientInput::Proceed,
                    ),
                    label: "candidate".to_string(),
                    annotation: ChoiceAnnotation::None,
                    expansion: OwnerChoiceExpansion::AutoAllowed,
                }],
                0,
            ),
            shop_boss_preview_candidates: vec![BranchPathShopBossPreviewSnapshot {
                rank: 1,
                label: "preview".to_string(),
                candidate: json!({"kind": "shop_leave"}),
                class: "safe".to_string(),
                reason: "fixture".to_string(),
            }],
        }
    }

    fn challenger_branch() -> Branch {
        Branch {
            id: 2,
            parent_id: Some(1),
            path: Vec::new(),
            session: RunControlSession::new(RunControlConfig::default()),
            status: BranchStatus::CombatGap {
                boundary: "boss".to_string(),
                reason: "no win".to_string(),
            },
            policy_lane: BranchPolicyLane::challenger(ChallengerPolicyState::new(1)),
            combat_portfolio: None,
            recent_progress_journal: Default::default(),
            recent_planner_capture: Default::default(),
            combat_search: Vec::new(),
            combat_search_history: Vec::new(),
            comparison_search_start: None,
            accepted_high_loss_diagnostics: Vec::new(),
        }
    }

    #[test]
    fn path_projection_keeps_complete_recorded_decision_evidence() {
        let projected = path_step(&branch_path_step_with_all_evidence());
        let evidence = projected.decision_evidence.unwrap();

        assert_eq!(evidence["policy_lane"], "challenger-1");
        assert_eq!(evidence["candidate_pool"].as_array().unwrap().len(), 1);
        assert_eq!(
            evidence["shop_boss_preview_candidates"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(!evidence["annotation"].is_null());
        assert!(!evidence["decision_delta"].is_null());
        assert_eq!(evidence["policy_selection"]["class"], "commitment_repair");
        assert_eq!(evidence["policy_selection"]["overrode_reject"], true);
    }

    #[test]
    fn challenger_branch_evidence_keeps_policy_and_trajectory_identity() {
        let evidence = branch_evidence(&challenger_branch());

        assert_eq!(evidence.policy_lane["kind"], "challenger");
        assert_eq!(evidence.policy_lane["policy"]["lane_id"], 1);
        assert_eq!(evidence.trajectory_snapshot.lane, "challenger-1");
    }
}
