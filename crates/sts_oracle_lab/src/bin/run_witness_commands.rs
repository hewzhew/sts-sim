use std::path::Path;

use serde_json::{json, Value};
use sts_oracle_runtime::ai::combat_state_key::combat_exact_state_hash_v2;
use sts_oracle_runtime::eval::combat_case::{
    save_combat_case, CombatCase, CombatCaseGap, CombatCasePathStep, CombatCaseRngSummary,
    CombatCaseRunSummary, CombatCaseSource,
};
use sts_oracle_runtime::eval::combat_case_context::capture_combat_case_production_context_v1;
use sts_oracle_runtime::eval::run_control::{
    exact_audit_run_progress_journal_policy_v1, exact_replay_run_progress_journal_prefix_v1,
    exact_replay_run_progress_journal_v1, splice_exact_combat_resolution_v1,
    RunControlSessionCheckpointV1, RunProgressJournalV1, RunProgressStepV1,
};
use sts_oracle_runtime::runtime::branch::{
    current_oracle_candidate_order_v1, load_oracle_analysis_workspace_v1,
    recover_oracle_analysis_combat_case_v1, save_oracle_run_continuation_v1,
    OracleRunContinuationV1,
};
use sts_oracle_runtime::sim::combat::{combat_terminal, CombatPosition};

use super::combat_evidence_manifest::{
    combat_evidence_manifest_path_for_actions, write_combat_evidence_manifest,
    CombatEvidenceManifestEntryV1, CombatEvidenceProducerV1,
};
use super::combat_replay_tools::{replay_combat_inputs, save_combat_inputs};

const EVIDENCE_REPLAY_MAX_ENGINE_STEPS_PER_TRANSITION: usize = 250;

pub(super) fn export_continuation(
    workspace: &Path,
    node: Option<usize>,
    output: &Path,
) -> Result<Value, String> {
    let analysis = load_oracle_analysis_workspace_v1(workspace)?;
    let node = node.unwrap_or_else(|| analysis.session.cursor_node_id());
    let continuation = analysis.continuation(node)?;
    let journal_entries = continuation.journal.entries().len();
    save_oracle_run_continuation_v1(output, &continuation)?;
    Ok(json!({
        "schema_name": "OracleAnalysisContinuationExportV1",
        "workspace": workspace,
        "node_id": node,
        "output": output,
        "journal_entries": journal_entries,
    }))
}

pub(super) fn recover_combat_case(
    workspace: &Path,
    branch: usize,
    output: &Path,
) -> Result<Value, String> {
    let case = recover_oracle_analysis_combat_case_v1(workspace, branch)?;
    save_combat_case(output, &case)?;
    Ok(json!({
        "schema_name": "OracleRecoveredCombatCaseV1",
        "workspace": workspace,
        "branch_id": branch,
        "output": output,
        "source": case.source,
        "run": case.run,
        "combat": case.combat,
        "path_steps": case.path.len(),
    }))
}

pub(super) fn verify(workspace: &Path, node: usize) -> Result<Value, String> {
    let analysis = load_oracle_analysis_workspace_v1(workspace)?;
    let continuation = analysis.continuation(node)?;
    let expected_final = continuation.session.into_session()?;
    let report = exact_replay_run_progress_journal_v1(
        analysis.seed,
        analysis.ascension,
        &continuation.journal,
        &expected_final,
    )?;
    Ok(json!({
        "schema_name": "ExactOracleRunWitnessReplayV1",
        "schema_version": 1,
        "workspace": workspace,
        "node_id": node,
        "report": report,
    }))
}

pub(super) fn audit_policy(workspace: &Path, node: usize, details: bool) -> Result<Value, String> {
    let analysis = load_oracle_analysis_workspace_v1(workspace)?;
    let continuation = analysis.continuation(node)?;
    let expected_final = continuation.session.into_session()?;
    let report = exact_audit_run_progress_journal_policy_v1(
        analysis.seed,
        analysis.ascension,
        &continuation.journal,
        &expected_final,
        current_oracle_candidate_order_v1,
    )?;
    let report = if details {
        serde_json::to_value(report)
            .map_err(|error| format!("failed to encode witness policy audit: {error}"))?
    } else {
        json!({
            "replay": report.replay,
            "decisions_with_owner_preferences": report.decisions_with_owner_preferences,
            "decisions_without_owner_preferences": report.decisions_without_owner_preferences,
            "rank_zero_agreements": report.rank_zero_agreements,
            "nonzero_rank_choices": report.nonzero_rank_choices,
            "choices_absent_from_owner_preferences": report.choices_absent_from_owner_preferences,
            "discrepancy_sum": report.discrepancy_sum,
            "max_owner_rank": report.max_owner_rank,
            "first_divergence": report.first_divergence,
            "combat_sources": report.combat_sources,
        })
    };
    Ok(json!({
        "schema_name": "ExactOracleRunWitnessPolicyAuditV1",
        "schema_version": 1,
        "workspace": workspace,
        "node_id": node,
        "report": report,
    }))
}

pub(super) fn splice_combat(
    workspace: &Path,
    node: usize,
    journal_entry: usize,
    replacement_workspace: &Path,
    replacement_node: usize,
    output: &Path,
) -> Result<Value, String> {
    let analysis = load_oracle_analysis_workspace_v1(workspace)?;
    let continuation = analysis.continuation(node)?;
    let replacement_analysis = load_oracle_analysis_workspace_v1(replacement_workspace)?;
    let replacement_continuation = replacement_analysis.continuation(replacement_node)?;
    if continuation.seed != replacement_continuation.seed
        || continuation.ascension != replacement_continuation.ascension
    {
        return Err("combat splice requires matching seed and ascension".to_string());
    }
    let replacement = replacement_continuation
        .journal
        .entries()
        .iter()
        .rev()
        .find_map(RunProgressStepV1::as_combat_resolution)
        .ok_or_else(|| "replacement witness contains no committed combat resolution".to_string())?;
    let original_source = continuation
        .journal
        .entries()
        .get(journal_entry)
        .and_then(RunProgressStepV1::as_combat_resolution)
        .map(|record| record.trajectory.source.label())
        .ok_or_else(|| format!("journal entry {journal_entry} is not a combat resolution"))?;
    let expected_final = continuation.session.clone().into_session()?;
    let (journal, replay) = splice_exact_combat_resolution_v1(
        continuation.seed,
        continuation.ascension,
        &continuation.journal,
        &expected_final,
        journal_entry,
        replacement,
    )?;
    let replacement_source = replacement.trajectory.source.label();
    let output_continuation = OracleRunContinuationV1 {
        schema_name: continuation.schema_name,
        schema_version: continuation.schema_version,
        seed: continuation.seed,
        ascension: continuation.ascension,
        journal,
        session: continuation.session,
        explorer_frontier: None,
    };
    save_oracle_run_continuation_v1(output, &output_continuation)?;
    Ok(json!({
        "schema_name": "ExactOracleCombatWitnessSpliceV1",
        "schema_version": 1,
        "workspace": workspace,
        "node_id": node,
        "journal_entry": journal_entry,
        "original_source": original_source,
        "replacement_workspace": replacement_workspace,
        "replacement_node_id": replacement_node,
        "replacement_source": replacement_source,
        "output": output,
        "replay": replay,
    }))
}

pub(super) fn export_historical_combat(
    workspace: &Path,
    node: usize,
    journal_entry: usize,
    case_output: &Path,
    actions_output: &Path,
    continuation_output: Option<&Path>,
) -> Result<Value, String> {
    let analysis = load_oracle_analysis_workspace_v1(workspace)?;
    let continuation = analysis.continuation(node)?;
    let resolution = continuation
        .journal
        .entries()
        .get(journal_entry)
        .and_then(RunProgressStepV1::as_combat_resolution)
        .cloned()
        .ok_or_else(|| format!("journal entry {journal_entry} is not a combat resolution"))?;
    let expected_final = continuation.session.clone().into_session()?;
    let historical = exact_replay_run_progress_journal_prefix_v1(
        continuation.seed,
        continuation.ascension,
        &continuation.journal,
        &expected_final,
        journal_entry,
    )?;
    let active = historical.active_combat.as_ref().ok_or_else(|| {
        format!("journal entry {journal_entry} does not begin at an active combat")
    })?;
    let position = CombatPosition::new(active.engine_state.clone(), active.combat_state.clone());
    let path = continuation
        .journal
        .entries()
        .iter()
        .take(journal_entry)
        .filter_map(RunProgressStepV1::as_decision)
        .map(|record| CombatCasePathStep {
            key: Value::Null,
            label: record.result.chosen_label.clone(),
            state_before: Some(json!({
                "title": record.before.title,
                "location": record.before.location,
            })),
            decision_evidence: Some(json!({
                "candidate_id": record.selection.candidate_id,
                "source": record.selection.source,
                "candidates": record.before.candidates.iter()
                    .map(|candidate| &candidate.label)
                    .collect::<Vec<_>>(),
            })),
        })
        .collect::<Vec<_>>();
    let mut case = CombatCase::new(
        CombatCaseSource {
            seed: continuation.seed,
            ascension: continuation.ascension,
            generation: path.len(),
            branch_id: node,
            parent_id: None,
        },
        CombatCaseGap {
            boundary: format!(
                "Act {} Floor {} historical combat",
                historical.run_state.act_num, historical.run_state.floor_num
            ),
            reason: "verified_run_witness_extraction".to_string(),
            search_nodes: 0,
            search_ms: 0,
            rescue_search_nodes: 0,
            rescue_search_ms: 0,
        },
        CombatCaseRunSummary {
            act: historical.run_state.act_num,
            floor: historical.run_state.floor_num,
            hp: historical.run_state.current_hp,
            max_hp: historical.run_state.max_hp,
            gold: historical.run_state.gold,
            deck_size: historical.run_state.master_deck.len(),
            relic_count: historical.run_state.relics.len(),
            potion_slots: historical.run_state.potions.len(),
        },
        Vec::new(),
        None,
        path,
        CombatCaseRngSummary::from_pool(&historical.run_state.rng_pool),
        position,
    );
    case.production_context = Some(capture_combat_case_production_context_v1(
        &case,
        &historical,
    )?);
    let actions = resolution
        .trajectory
        .actions
        .iter()
        .map(|action| action.input.clone())
        .collect::<Vec<_>>();
    let final_position = replay_combat_inputs(
        case.position.clone(),
        &actions,
        EVIDENCE_REPLAY_MAX_ENGINE_STEPS_PER_TRANSITION,
    )?;
    let root_exact_state_hash =
        combat_exact_state_hash_v2(&case.position.engine, &case.position.combat);
    let final_terminal = combat_terminal(&final_position.engine, &final_position.combat);
    let final_player_hp = final_position.combat.entities.player.current_hp;
    save_combat_case(case_output, &case)?;
    save_combat_inputs(actions_output, actions.iter().cloned())?;
    let manifest_output = combat_evidence_manifest_path_for_actions(actions_output);
    if let Some(output) = continuation_output {
        let prefix_journal = RunProgressJournalV1::from_committed_steps(
            continuation.journal.entries()[..journal_entry].to_vec(),
        )?;
        let prefix = OracleRunContinuationV1 {
            schema_name: continuation.schema_name,
            schema_version: continuation.schema_version,
            seed: continuation.seed,
            ascension: continuation.ascension,
            journal: prefix_journal,
            session: RunControlSessionCheckpointV1::from_session(&historical),
            explorer_frontier: None,
        };
        save_oracle_run_continuation_v1(output, &prefix)?;
    }
    write_combat_evidence_manifest(
        &manifest_output,
        CombatEvidenceProducerV1::HistoricalCombatWitnessExport,
        root_exact_state_hash,
        case_output.to_path_buf(),
        vec![CombatEvidenceManifestEntryV1::from_actions(
            format!("journal_entry_{journal_entry}"),
            vec![actions_output.to_path_buf()],
            &actions,
            final_terminal,
            Some(final_player_hp),
        )?],
    )?;
    Ok(json!({
        "schema_name": "HistoricalCombatWitnessExportV2",
        "schema_version": 2,
        "workspace": workspace,
        "node_id": node,
        "journal_entry": journal_entry,
        "source": resolution.trajectory.source.label(),
        "case_output": case_output,
        "actions_output": actions_output,
        "manifest_output": manifest_output,
        "continuation_output": continuation_output,
        "action_count": resolution.trajectory.actions.len(),
        "combat": case.combat,
    }))
}
