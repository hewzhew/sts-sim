use std::{path::Path, time::Instant};

use serde::Serialize;
use serde_json::{json, Value};
use sts_oracle_runtime::eval::run_control::{
    OracleAnalysisAdvanceRequestV1, OracleAnalysisAdvanceStatusV1, OracleAnalysisNodeViewV1,
    OracleRunBoundaryV1,
};
use sts_oracle_runtime::runtime::branch::{
    load_oracle_analysis_workspace_v1, save_oracle_analysis_workspace_v1,
};

use super::workspace_view;

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum OracleAnalysisDriveStopV1 {
    StepLimit,
    WallLimit,
    NoOwnerChoice,
    CombatUnresolved,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn drive(
    workspace: &Path,
    max_steps: u16,
    max_quanta: usize,
    quantum_nodes: usize,
    quantum_ms: u64,
    wall_ms: u64,
) -> Result<Value, String> {
    if max_steps == 0 || max_quanta == 0 || quantum_nodes == 0 || quantum_ms == 0 || wall_ms == 0 {
        return Err("oracle drive requires positive step, quantum, and wall budgets".to_string());
    }
    let mut analysis = load_oracle_analysis_workspace_v1(workspace)?;
    let started = Instant::now();
    let mut events = Vec::new();
    let mut stop = OracleAnalysisDriveStopV1::StepLimit;

    for step_index in 0..max_steps {
        let elapsed_ms = elapsed_millis(started);
        if elapsed_ms >= wall_ms {
            stop = OracleAnalysisDriveStopV1::WallLimit;
            break;
        }
        let current = analysis.view()?;
        let current_owner_order = workspace_view::current_owner_order(&analysis, current.node_id)?;
        if let Some(candidate_id) = current_owner_order.first() {
            let matches = current
                .choices
                .iter()
                .filter(|choice| &choice.candidate_id == candidate_id)
                .collect::<Vec<_>>();
            let [choice] = matches.as_slice() else {
                return Err(format!(
                    "oracle node {} has {} materialized choices for current-owner candidate '{}'; expected exactly one",
                    current.node_id,
                    matches.len(),
                    candidate_id,
                ));
            };
            let source_node_id = current.node_id;
            let candidate_id = choice.candidate_id.clone();
            let materialized_owner_rank = choice.owner_rank;
            let choice_ref = choice.choice_ref.clone();
            let result = analysis.try_choice(&choice_ref)?;
            save_oracle_analysis_workspace_v1(workspace, &analysis)?;
            events.push(json!({
                "kind": "owner_decision",
                "step_index": step_index,
                "source_node_id": source_node_id,
                "candidate_id": candidate_id,
                "materialized_owner_rank": materialized_owner_rank,
                "result": compact_drive_boundary(&result),
            }));
            continue;
        }

        if current.boundary != OracleRunBoundaryV1::Combat {
            stop = OracleAnalysisDriveStopV1::NoOwnerChoice;
            break;
        }
        let remaining_wall_ms = wall_ms.saturating_sub(elapsed_millis(started));
        if remaining_wall_ms == 0 {
            stop = OracleAnalysisDriveStopV1::WallLimit;
            break;
        }
        let (report, result) = analysis.advance(OracleAnalysisAdvanceRequestV1 {
            max_quanta,
            quantum_nodes,
            quantum_ms: Some(quantum_ms),
            wall_ms: Some(remaining_wall_ms),
            improve_incumbent: false,
        })?;
        save_oracle_analysis_workspace_v1(workspace, &analysis)?;
        let boundary_reached = matches!(
            report.status,
            OracleAnalysisAdvanceStatusV1::BoundaryReached { .. }
        );
        let combat = report.combat.as_ref().map(|combat| {
            json!({
                "root_exact_state_hash": combat.root_exact_state_hash,
                "stage_trace": combat.stage_trace,
                "generation_work": combat.generation_work,
                "exact_states": combat.exact_states,
                "max_player_turn": combat.max_player_turn,
                "incumbent_final_hp": combat.incumbent_final_hp,
                "incumbent_hp_loss": combat.incumbent_hp_loss,
                "incumbent_action_count": combat.incumbent_action_count,
                "incumbent_potions_used": combat.incumbent_potions_used,
                "incumbent_potion_slots": combat.incumbent_potion_slots,
                "incumbent_satisfies_satisfaction": combat.incumbent_satisfies_satisfaction,
                "incumbent_ends_quality_refinement": combat.incumbent_ends_quality_refinement,
            })
        });
        events.push(json!({
            "kind": "combat_advance",
            "step_index": step_index,
            "source_node_id": report.source_node_id,
            "status": report.status,
            "quanta_served": report.quanta_served,
            "elapsed_ms": report.elapsed_ms,
            "combat": combat,
            "result": compact_drive_boundary(&result),
        }));
        if !boundary_reached {
            stop = OracleAnalysisDriveStopV1::CombatUnresolved;
            break;
        }
    }

    let final_view = analysis.view()?;
    let current_owner_order = workspace_view::current_owner_order(&analysis, final_view.node_id)?;
    Ok(json!({
        "schema_name": "OracleAnalysisDriveV1",
        "schema_version": 1,
        "workspace": workspace,
        "requested_max_steps": max_steps,
        "completed_steps": events.len(),
        "elapsed_ms": elapsed_millis(started),
        "stop": stop,
        "events": events,
        "status": workspace_view::compact_node(&final_view, 8, &current_owner_order),
    }))
}

fn compact_drive_boundary(view: &OracleAnalysisNodeViewV1) -> Value {
    json!({
        "node": view.node_id,
        "boundary": view.boundary,
        "act": view.act,
        "floor": view.floor,
        "hp": view.current_hp,
        "max_hp": view.max_hp,
        "gold": view.gold,
    })
}

fn elapsed_millis(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}
