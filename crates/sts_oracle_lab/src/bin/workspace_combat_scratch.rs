use std::path::Path;

use serde_json::{json, Value};
use sts_oracle_runtime::eval::run_control::OracleAnalysisCombatScratchSearchRequestV1;
use sts_oracle_runtime::runtime::branch::{
    load_oracle_analysis_workspace_v1, OracleAnalysisWorkspaceV1,
};

use super::workspace_commands::{encode, mutate};

pub(super) fn start(
    workspace: &Path,
    node: Option<usize>,
    max_engine_steps_per_transition: usize,
    selection_offset: usize,
    selection_limit: usize,
) -> Result<Value, String> {
    mutate(workspace, |analysis| {
        analysis.session.start_combat_scratch(
            node,
            max_engine_steps_per_transition,
            selection_offset,
            selection_limit,
        )
    })
}

pub(super) fn status(
    workspace: &Path,
    selection_offset: usize,
    selection_limit: usize,
) -> Result<Value, String> {
    let analysis = load_oracle_analysis_workspace_v1(workspace)?;
    encode(
        analysis
            .session
            .combat_scratch_view(selection_offset, selection_limit)?,
    )
}

pub(super) fn play(
    workspace: &Path,
    action_ref: &str,
    selection_offset: usize,
    selection_limit: usize,
) -> Result<Value, String> {
    mutate(workspace, |analysis| {
        analysis
            .session
            .play_combat_scratch_action(action_ref, selection_offset, selection_limit)
    })
}

pub(super) fn back(
    workspace: &Path,
    selection_offset: usize,
    selection_limit: usize,
) -> Result<Value, String> {
    mutate(workspace, |analysis| {
        analysis
            .session
            .back_combat_scratch(selection_offset, selection_limit)
    })
}

pub(super) fn focus(
    workspace: &Path,
    scratch_node: u64,
    selection_offset: usize,
    selection_limit: usize,
) -> Result<Value, String> {
    mutate(workspace, |analysis| {
        analysis
            .session
            .focus_combat_scratch_node(scratch_node, selection_offset, selection_limit)
    })
}

pub(super) fn tree(workspace: &Path) -> Result<Value, String> {
    let analysis = load_oracle_analysis_workspace_v1(workspace)?;
    encode(analysis.session.combat_scratch_tree()?)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn search(
    workspace: &Path,
    max_quanta: usize,
    quantum_nodes: usize,
    quantum_ms: u64,
    wall_ms: u64,
    selection_offset: usize,
    selection_limit: usize,
) -> Result<Value, String> {
    mutate(workspace, |analysis| {
        let (report, view) = analysis.session.search_combat_scratch(
            OracleAnalysisCombatScratchSearchRequestV1 {
                max_quanta,
                quantum_nodes,
                quantum_ms,
                wall_ms,
            },
            selection_offset,
            selection_limit,
        )?;
        Ok(json!({"report": report, "view": view}))
    })
}

pub(super) fn commit(workspace: &Path) -> Result<Value, String> {
    mutate(workspace, OracleAnalysisWorkspaceV1::commit_combat_scratch)
}

pub(super) fn clear(workspace: &Path) -> Result<Value, String> {
    mutate(workspace, |analysis| {
        Ok(json!({
            "schema_name": "OracleAnalysisCombatScratchClearReceiptV1",
            "schema_version": 1,
            "cleared": analysis.session.clear_combat_scratch(),
            "run_cursor_node_id": analysis.session.cursor_node_id(),
        }))
    })
}
