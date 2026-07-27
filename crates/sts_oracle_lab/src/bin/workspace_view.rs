use serde_json::{json, Value};
use sts_oracle_runtime::eval::combat_case::CombatCase;
use sts_oracle_runtime::eval::run_control::{OracleAnalysisNodeViewV1, RunProgressStepV1};
use sts_oracle_runtime::runtime::branch::OracleAnalysisWorkspaceV1;

pub(super) fn selected(
    analysis: &OracleAnalysisWorkspaceV1,
    node: Option<usize>,
) -> Result<OracleAnalysisNodeViewV1, String> {
    if let Some(node) = node {
        analysis.session.view_node(node)
    } else {
        analysis.view()
    }
}

pub(super) fn compact_node(view: &OracleAnalysisNodeViewV1, limit: usize) -> Value {
    let choices = view
        .choices
        .iter()
        .take(limit)
        .map(|choice| {
            json!({
                "choice_ref": choice.choice_ref,
                "kind": choice.kind,
                "candidate_id": choice.candidate_id,
                "label": choice.label,
                "owner_rank": choice.owner_rank,
                "path_discrepancy": choice.path_discrepancy,
            })
        })
        .collect::<Vec<_>>();
    let children = view
        .children
        .iter()
        .take(limit)
        .map(|child| {
            json!({
                "edge_id": child.edge_id,
                "child_node_id": child.child_node_id,
                "kind": child.kind,
                "label": child.label,
                "is_on_mainline": child.is_on_mainline,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "node": view.node_id,
        "parent": view.canonical_parent_node_id,
        "act": view.act,
        "floor": view.floor,
        "hp": view.current_hp,
        "max_hp": view.max_hp,
        "gold": view.gold,
        "boundary": view.boundary,
        "event": view.event,
        "choice_count": view.choices.len(),
        "choices_shown": choices.len(),
        "choices_truncated": view.choices.len() > choices.len(),
        "choices": choices,
        "child_count": view.children.len(),
        "children_shown": children.len(),
        "children_truncated": view.children.len() > children.len(),
        "children": children,
        "encounter": view.encounter,
        "combat": view.combat,
    })
}

pub(super) fn compact_timeline(
    analysis: &OracleAnalysisWorkspaceV1,
    node: usize,
    tail: usize,
) -> Result<Value, String> {
    let entries = analysis.session.journal_entries(node)?;
    let start = entries.len().saturating_sub(tail);
    let compact = entries[start..]
        .iter()
        .enumerate()
        .map(|(offset, entry)| match entry {
            RunProgressStepV1::Decision(record) => json!({
                "journal_index": start + offset,
                "kind": "decision",
                "location": record.before.location,
                "title": record.before.title,
                "chosen": record.result.chosen_label,
                "candidates": record.before.candidates.iter().map(|candidate| &candidate.label).collect::<Vec<_>>(),
            }),
            RunProgressStepV1::ForcedTransition(record) => json!({
                "journal_index": start + offset,
                "kind": "forced_transition",
                "location": record.before.location,
                "title": record.before.title,
            }),
            RunProgressStepV1::CombatResolution(record) => json!({
                "journal_index": start + offset,
                "kind": "combat_resolution",
                "location": record.before.location,
                "title": record.before.title,
                "resolution": record.kind,
                "actions": record.trajectory.action_count,
                "changes": record.result.changes,
            }),
            RunProgressStepV1::Stop(record) => json!({
                "journal_index": start + offset,
                "kind": "stop",
                "stop_kind": record.kind,
                "reason": record.reason,
            }),
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "node": node,
        "total_entries": entries.len(),
        "returned_entries": compact.len(),
        "entries": compact,
    }))
}

pub(super) fn combat_case(
    analysis: &OracleAnalysisWorkspaceV1,
    node: usize,
) -> Result<CombatCase, String> {
    let view = analysis.session.view_node(node)?;
    let (search_nodes, search_ms) = if view.encounter.as_ref().is_some_and(|it| it.is_boss) {
        (analysis.budget.boss_nodes, analysis.budget.boss_ms)
    } else if view.encounter.as_ref().is_some_and(|it| it.is_elite) {
        (analysis.budget.elite_nodes, analysis.budget.elite_ms)
    } else {
        (analysis.budget.hallway_nodes, analysis.budget.hallway_ms)
    };
    analysis.session.combat_case(
        node,
        analysis.seed,
        analysis.ascension,
        search_nodes,
        search_ms,
    )
}
