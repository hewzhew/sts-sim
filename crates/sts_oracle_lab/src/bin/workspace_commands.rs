use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{json, Value};
use sts_oracle_runtime::eval::run_control::OracleAnalysisAdvanceRequestV1;
use sts_oracle_runtime::runtime::branch::{
    current_oracle_candidate_order_v1, load_oracle_analysis_workspace_v1,
    oracle_live_combat_diagnostic_v1, save_oracle_analysis_workspace_v1, OracleAnalysisWorkspaceV1,
};
use sts_oracle_runtime::state::core::ClientInput;

use super::workspace_view;

pub(super) fn view(workspace: &Path, node: Option<usize>) -> Result<Value, String> {
    let analysis = load_oracle_analysis_workspace_v1(workspace)?;
    encode(workspace_view::selected(&analysis, node)?)
}

pub(super) fn status(workspace: &Path, node: Option<usize>, limit: usize) -> Result<Value, String> {
    let analysis = load_oracle_analysis_workspace_v1(workspace)?;
    let view = workspace_view::selected(&analysis, node)?;
    let current_owner_order = current_owner_order_at(&analysis, view.node_id)?;
    Ok(workspace_view::compact_node(
        &view,
        limit,
        &current_owner_order,
    ))
}

pub(super) fn choose(
    workspace: &Path,
    owner_rank: u64,
    expected_node: Option<usize>,
) -> Result<Value, String> {
    let mut analysis = load_oracle_analysis_workspace_v1(workspace)?;
    if let Some(expected) = expected_node {
        let actual = analysis.session.cursor_node_id();
        if expected != actual {
            return Err(format!(
                "oracle choose expected cursor node {expected}, but current cursor is {actual}"
            ));
        }
    }
    let current = analysis.view()?;
    let current_owner_order = current_owner_order_at(&analysis, current.node_id)?;
    let owner_rank = usize::try_from(owner_rank)
        .map_err(|_| "oracle choose owner rank exceeds platform usize".to_string())?;
    let candidate_id = current_owner_order.get(owner_rank).ok_or_else(|| {
        format!(
            "oracle node {} current owner has no candidate at rank {owner_rank}",
            current.node_id
        )
    })?;
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
    let view = analysis.try_choice(&choice.choice_ref.clone())?;
    save_oracle_analysis_workspace_v1(workspace, &analysis)?;
    let current_owner_order = current_owner_order_at(&analysis, view.node_id)?;
    Ok(workspace_view::compact_node(&view, 8, &current_owner_order))
}

pub(super) fn owner(workspace: &Path, steps: u8) -> Result<Value, String> {
    let mut analysis = load_oracle_analysis_workspace_v1(workspace)?;
    let mut applied = Vec::new();
    let mut stopped = "step_limit";
    for _ in 0..steps {
        let current = analysis.view()?;
        let current_owner_order = current_owner_order_at(&analysis, current.node_id)?;
        let Some(candidate_id) = current_owner_order.first() else {
            stopped = "no_owner_choice";
            break;
        };
        let choices = current
            .choices
            .iter()
            .filter(|choice| &choice.candidate_id == candidate_id)
            .collect::<Vec<_>>();
        let [choice] = choices.as_slice() else {
            return Err(format!(
                "oracle node {} has {} materialized choices for current-owner candidate '{}'; expected exactly one",
                current.node_id,
                choices.len(),
                candidate_id,
            ));
        };
        let candidate_id = choice.candidate_id.clone();
        let label = choice.label.clone();
        let choice_ref = choice.choice_ref.clone();
        applied.push(json!({
            "node": current.node_id,
            "candidate_id": candidate_id,
            "label": label,
            "materialized_owner_rank": choice.owner_rank,
        }));
        analysis.try_choice(&choice_ref)?;
    }
    if !applied.is_empty() {
        save_oracle_analysis_workspace_v1(workspace, &analysis)?;
    }
    let final_view = analysis.view()?;
    let current_owner_order = current_owner_order_at(&analysis, final_view.node_id)?;
    Ok(json!({
        "requested_steps": steps,
        "applied_count": applied.len(),
        "applied": applied,
        "stopped": stopped,
        "status": workspace_view::compact_node(&final_view, 8, &current_owner_order),
    }))
}

fn current_owner_order_at(
    analysis: &OracleAnalysisWorkspaceV1,
    node: usize,
) -> Result<Vec<String>, String> {
    let session = analysis.continuation(node)?.session.into_session()?;
    Ok(current_oracle_candidate_order_v1(&session))
}

pub(super) fn timeline(
    workspace: &Path,
    node: Option<usize>,
    tail: usize,
) -> Result<Value, String> {
    let analysis = load_oracle_analysis_workspace_v1(workspace)?;
    let node = node.unwrap_or_else(|| analysis.session.cursor_node_id());
    if tail == 0 || tail > 500 {
        return Err("timeline tail must be in 1..=500".to_string());
    }
    workspace_view::compact_timeline(&analysis, workspace, node, tail)
}

pub(super) fn export_combat_case(
    workspace: &Path,
    node: Option<usize>,
    output: &Path,
) -> Result<Value, String> {
    let analysis = load_oracle_analysis_workspace_v1(workspace)?;
    let node = node.unwrap_or_else(|| analysis.session.cursor_node_id());
    let case = workspace_view::combat_case(&analysis, node)?;
    sts_oracle_runtime::eval::combat_case::save_combat_case(output, &case)?;
    Ok(json!({
        "node": node,
        "output": output,
        "combat": case.combat,
    }))
}

pub(super) fn combat(
    workspace: &Path,
    node: Option<usize>,
    max_engine_steps_per_transition: usize,
) -> Result<Value, String> {
    let analysis = load_oracle_analysis_workspace_v1(workspace)?;
    let node = node.unwrap_or_else(|| analysis.session.cursor_node_id());
    oracle_live_combat_diagnostic_v1(&analysis, node, max_engine_steps_per_transition)
}

pub(super) fn tree(workspace: &Path) -> Result<Value, String> {
    let analysis = load_oracle_analysis_workspace_v1(workspace)?;
    encode(analysis.session.tree())
}

pub(super) fn try_choice(workspace: &Path, choice_ref: &str) -> Result<Value, String> {
    mutate(workspace, |analysis| analysis.try_choice(choice_ref))
}

pub(super) fn focus(workspace: &Path, node: usize) -> Result<Value, String> {
    mutate(workspace, |analysis| {
        analysis.session.focus_node(node)?;
        analysis.view()
    })
}

pub(super) fn follow(workspace: &Path, edge: u64) -> Result<Value, String> {
    mutate(workspace, |analysis| {
        analysis.session.follow_edge(edge)?;
        analysis.view()
    })
}

pub(super) fn back(workspace: &Path) -> Result<Value, String> {
    mutate(workspace, |analysis| {
        analysis.session.back()?;
        analysis.view()
    })
}

pub(super) fn promote(workspace: &Path) -> Result<Value, String> {
    mutate(workspace, |analysis| {
        analysis.session.promote_cursor();
        analysis.view()
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn advance(
    workspace: &Path,
    max_quanta: usize,
    quantum_nodes: usize,
    quantum_ms: u64,
    wall_ms: Option<u64>,
    improve_incumbent: bool,
    detailed: bool,
) -> Result<Value, String> {
    let mut analysis = load_oracle_analysis_workspace_v1(workspace)?;
    let (report, view) = analysis.advance(OracleAnalysisAdvanceRequestV1 {
        max_quanta,
        quantum_nodes,
        quantum_ms: Some(quantum_ms),
        wall_ms,
        improve_incumbent,
    })?;
    save_oracle_analysis_workspace_v1(workspace, &analysis)?;
    if detailed {
        return Ok(json!({ "report": report, "view": view }));
    }
    let combat = report.combat.as_ref().map(|combat| {
        json!({
            "root_exact_state_hash": combat.root_exact_state_hash,
            "stage_trace": combat.stage_trace,
            "generation_work": combat.generation_work,
            "current_search_generation_work": combat.current_search_generation_work,
            "exact_states": combat.exact_states,
            "completed_turn_options": combat.completed_turn_options,
            "retained_state_work": combat.retained_state_work,
            "max_player_turn": combat.max_player_turn,
            "incumbent_discovery_source": combat.incumbent_discovery_source,
            "incumbent_final_hp": combat.incumbent_final_hp,
            "incumbent_hp_loss": combat.incumbent_hp_loss,
            "incumbent_action_count": combat.incumbent_action_count,
            "incumbent_satisfies_satisfaction": combat.incumbent_satisfies_satisfaction,
            "incumbent_ends_quality_refinement": combat.incumbent_ends_quality_refinement,
            "last_status": combat.last_status,
        })
    });
    Ok(json!({
        "schema_name": "OracleAnalysisAdvanceSummaryV1",
        "schema_version": 1,
        "source_node_id": report.source_node_id,
        "status": report.status,
        "quanta_served": report.quanta_served,
        "elapsed_ms": report.elapsed_ms,
        "combat": combat,
        "result": {
            "node": view.node_id,
            "boundary": view.boundary,
            "act": view.act,
            "floor": view.floor,
            "hp": view.current_hp,
            "max_hp": view.max_hp,
            "gold": view.gold,
            "choice_count": view.choices.len(),
            "child_count": view.children.len(),
        },
    }))
}

pub(super) fn accept_combat(workspace: &Path) -> Result<Value, String> {
    mutate(
        workspace,
        OracleAnalysisWorkspaceV1::accept_combat_incumbent,
    )
}

pub(super) fn accept_combat_actions(
    workspace: &Path,
    action_paths: &[PathBuf],
) -> Result<Value, String> {
    let action_lists = action_paths
        .iter()
        .map(|path| {
            serde_json::from_slice::<Vec<ClientInput>>(
                &std::fs::read(path).map_err(|error| error.to_string())?,
            )
            .map_err(|error| {
                format!(
                    "invalid combat witness action list '{}': {error}",
                    path.display()
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let actions = action_lists.into_iter().flatten().collect::<Vec<_>>();
    mutate(workspace, |analysis| {
        analysis.accept_combat_actions(&actions)
    })
}

pub(super) fn restart_combat(workspace: &Path) -> Result<Value, String> {
    mutate(workspace, |analysis| {
        analysis.session.restart_cursor_combat_search()?;
        analysis.view()
    })
}

pub(super) fn history(
    workspace: &Path,
    node: Option<usize>,
    journal: bool,
) -> Result<Value, String> {
    let analysis = load_oracle_analysis_workspace_v1(workspace)?;
    let node = node.unwrap_or_else(|| analysis.session.cursor_node_id());
    if journal {
        encode(analysis.session.journal_entries(node)?)
    } else {
        encode(analysis.session.replay(node)?)
    }
}

fn mutate<T: Serialize>(
    workspace: &Path,
    operation: impl FnOnce(&mut OracleAnalysisWorkspaceV1) -> Result<T, String>,
) -> Result<Value, String> {
    let mut analysis = load_oracle_analysis_workspace_v1(workspace)?;
    let output = operation(&mut analysis)?;
    save_oracle_analysis_workspace_v1(workspace, &analysis)?;
    encode(output)
}

fn encode(value: impl Serialize) -> Result<Value, String> {
    serde_json::to_value(value)
        .map_err(|error| format!("failed to encode workspace result: {error}"))
}
