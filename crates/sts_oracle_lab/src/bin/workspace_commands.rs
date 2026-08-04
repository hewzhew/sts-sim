use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;
use serde_json::{json, Value};
use sts_oracle_runtime::eval::combat_lab_v1::atomic_write_json;
use sts_oracle_runtime::eval::run_control::{
    exact_replay_run_progress_journal_v1, run_progress_journal_fingerprint_v1,
    ExactRunProgressReplayReportV1, OracleAnalysisAdvanceRequestV1,
};
use sts_oracle_runtime::runtime::branch::{
    load_oracle_analysis_workspace_v1, oracle_live_combat_diagnostic_v1,
    save_oracle_analysis_workspace_v1, OracleAnalysisWorkspaceArtifactV1,
    OracleAnalysisWorkspaceV1,
};
use sts_oracle_runtime::state::core::ClientInput;

use super::workspace_view;

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceCompactionReceiptV1<'a> {
    schema_name: &'static str,
    schema_version: u32,
    source_workspace: &'a Path,
    source_node_id: usize,
    source_node_count: usize,
    source_bytes: u64,
    output_workspace: &'a Path,
    output_node_id: usize,
    output_node_count: usize,
    output_bytes: u64,
    bytes_removed: u64,
    reduction_basis_points: u64,
    journal_entries: usize,
    journal_fingerprint: String,
    final_state_fingerprint: String,
    source_workspace_modified: bool,
    exact_roundtrip_verified: bool,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceRepackReceiptV1<'a> {
    schema_name: &'static str,
    schema_version: u32,
    source_workspace: &'a Path,
    source_bytes: u64,
    output_workspace: &'a Path,
    output_bytes: u64,
    bytes_removed: u64,
    reduction_basis_points: u64,
    node_count: usize,
    source_workspace_modified: bool,
    complete_tree_preserved: bool,
    artifact_write_verified: bool,
    workspace_restore_verified: bool,
}

struct VerifiedWorkspaceNodeV1 {
    node_id: usize,
    journal_entries: usize,
    journal_fingerprint: String,
    replay: ExactRunProgressReplayReportV1,
}

fn verify_workspace_node(
    analysis: &OracleAnalysisWorkspaceV1,
    node_id: usize,
) -> Result<VerifiedWorkspaceNodeV1, String> {
    let continuation = analysis.continuation(node_id)?;
    let expected = continuation.session.clone().into_session()?;
    let replay = exact_replay_run_progress_journal_v1(
        analysis.seed,
        analysis.ascension,
        &continuation.journal,
        &expected,
    )?;
    Ok(VerifiedWorkspaceNodeV1 {
        node_id,
        journal_entries: continuation.journal.len(),
        journal_fingerprint: run_progress_journal_fingerprint_v1(&continuation.journal),
        replay,
    })
}

fn require_same_exact_node(
    source: &VerifiedWorkspaceNodeV1,
    candidate: &VerifiedWorkspaceNodeV1,
    context: &str,
) -> Result<(), String> {
    if source.replay == candidate.replay
        && source.journal_fingerprint == candidate.journal_fingerprint
    {
        Ok(())
    } else {
        Err(format!("{context} failed exact-state verification"))
    }
}

pub(super) fn repack_workspace(workspace: &Path, output: &Path) -> Result<Value, String> {
    if output.exists() {
        return Err(format!(
            "oracle repack workspace output already exists: '{}'",
            output.display()
        ));
    }
    let source_bytes = fs::metadata(workspace)
        .map_err(|error| format!("failed to inspect '{}': {error}", workspace.display()))?
        .len();
    let analysis = load_oracle_analysis_workspace_v1(workspace)?;
    let node_count = analysis.session.tree().nodes.len();
    let expected_artifact = analysis.artifact()?;
    let expected_bytes = serde_json::to_vec(&expected_artifact)
        .map_err(|error| format!("failed to encode pooled workspace checkpoint: {error}"))?;

    atomic_write_json(output, &expected_artifact)?;
    let written_bytes = fs::read(output)
        .map_err(|error| format!("failed to read '{}': {error}", output.display()))?;
    if expected_bytes != written_bytes {
        return Err("written oracle repack workspace failed artifact write verification".into());
    }
    let written_artifact =
        serde_json::from_slice::<OracleAnalysisWorkspaceArtifactV1>(&written_bytes)
            .map_err(|error| format!("failed to parse written pooled workspace: {error}"))?;
    let restored = OracleAnalysisWorkspaceV1::restore(written_artifact)?;
    let source_tree = serde_json::to_vec(&analysis.session.tree())
        .map_err(|error| format!("failed to encode source workspace tree: {error}"))?;
    let restored_tree = serde_json::to_vec(&restored.session.tree())
        .map_err(|error| format!("failed to encode restored workspace tree: {error}"))?;
    if source_tree != restored_tree {
        return Err("written oracle repack workspace changed the variation tree".into());
    }

    let output_bytes = fs::metadata(output)
        .map_err(|error| format!("failed to inspect '{}': {error}", output.display()))?
        .len();
    let bytes_removed = source_bytes.saturating_sub(output_bytes);
    let reduction_basis_points = if source_bytes == 0 {
        0
    } else {
        bytes_removed.saturating_mul(10_000) / source_bytes
    };
    encode(WorkspaceRepackReceiptV1 {
        schema_name: "OracleAnalysisWorkspaceRepackReceipt",
        schema_version: 1,
        source_workspace: workspace,
        source_bytes,
        output_workspace: output,
        output_bytes,
        bytes_removed,
        reduction_basis_points,
        node_count,
        source_workspace_modified: false,
        complete_tree_preserved: true,
        artifact_write_verified: true,
        workspace_restore_verified: true,
    })
}

pub(super) fn compact_workspace(
    workspace: &Path,
    node: Option<usize>,
    output: &Path,
) -> Result<Value, String> {
    if output.exists() {
        return Err(format!(
            "oracle compact workspace output already exists: '{}'",
            output.display()
        ));
    }
    let source_bytes = fs::metadata(workspace)
        .map_err(|error| format!("failed to inspect '{}': {error}", workspace.display()))?
        .len();
    let analysis = load_oracle_analysis_workspace_v1(workspace)?;
    let source_node_id = node.unwrap_or_else(|| analysis.session.cursor_node_id());
    let source_node_count = analysis.session.tree().nodes.len();
    let source = verify_workspace_node(&analysis, source_node_id)?;

    let compact = analysis.compact_from_node(source_node_id)?;
    let output_node_id = compact.session.cursor_node_id();
    let output_node_count = compact.session.tree().nodes.len();
    let in_memory = verify_workspace_node(&compact, output_node_id)?;
    require_same_exact_node(&source, &in_memory, "oracle compact workspace")?;

    save_oracle_analysis_workspace_v1(output, &compact)?;
    let restored = load_oracle_analysis_workspace_v1(output)?;
    let restored_node_id = restored.session.cursor_node_id();
    let restored = verify_workspace_node(&restored, restored_node_id)?;
    require_same_exact_node(&source, &restored, "written oracle compact workspace")?;

    let output_bytes = fs::metadata(output)
        .map_err(|error| format!("failed to inspect '{}': {error}", output.display()))?
        .len();
    let bytes_removed = source_bytes.saturating_sub(output_bytes);
    let reduction_basis_points = if source_bytes == 0 {
        0
    } else {
        bytes_removed.saturating_mul(10_000) / source_bytes
    };
    encode(WorkspaceCompactionReceiptV1 {
        schema_name: "OracleAnalysisWorkspaceCompactionReceipt",
        schema_version: 1,
        source_workspace: workspace,
        source_node_id,
        source_node_count,
        source_bytes,
        output_workspace: output,
        output_node_id: restored.node_id,
        output_node_count,
        output_bytes,
        bytes_removed,
        reduction_basis_points,
        journal_entries: restored.journal_entries,
        journal_fingerprint: restored.journal_fingerprint,
        final_state_fingerprint: restored.replay.final_fingerprint,
        source_workspace_modified: false,
        exact_roundtrip_verified: true,
    })
}

pub(super) fn view(workspace: &Path, node: Option<usize>) -> Result<Value, String> {
    let analysis = load_oracle_analysis_workspace_v1(workspace)?;
    encode(workspace_view::selected(&analysis, node)?)
}

pub(super) fn status(workspace: &Path, node: Option<usize>, limit: usize) -> Result<Value, String> {
    let analysis = load_oracle_analysis_workspace_v1(workspace)?;
    let view = workspace_view::selected(&analysis, node)?;
    let current_owner_order = workspace_view::current_owner_order(&analysis, view.node_id)?;
    Ok(workspace_view::compact_node(
        &view,
        limit,
        &current_owner_order,
    ))
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

pub(super) fn mutate<T: Serialize>(
    workspace: &Path,
    operation: impl FnOnce(&mut OracleAnalysisWorkspaceV1) -> Result<T, String>,
) -> Result<Value, String> {
    let mut analysis = load_oracle_analysis_workspace_v1(workspace)?;
    let output = operation(&mut analysis)?;
    save_oracle_analysis_workspace_v1(workspace, &analysis)?;
    encode(output)
}

pub(super) fn encode(value: impl Serialize) -> Result<Value, String> {
    serde_json::to_value(value)
        .map_err(|error| format!("failed to encode workspace result: {error}"))
}
