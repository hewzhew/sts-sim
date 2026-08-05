use std::path::Path;

use serde_json::{json, Value};
use sts_oracle_runtime::eval::combat_case::load_combat_case;
use sts_oracle_runtime::eval::combat_case_context::combat_case_replay_identity_v1;
use sts_oracle_runtime::eval::run_control::{
    exact_census_run_progress_journal_combat_roots_v1, exact_diagnose_run_progress_journal_v1,
    exact_replay_run_progress_journal_prefix_v1, RunControlSessionCheckpointV1,
    RunProgressJournalV1, RunWitnessCombatRootIdentityV1, RunWitnessCombatTimelineEntryV1,
};
use sts_oracle_runtime::runtime::branch::{
    current_oracle_candidate_order_v1, load_oracle_analysis_workspace_v1,
    save_oracle_run_continuation_v1, OracleRunContinuationV1,
};

pub(super) fn diagnose(
    workspace: &Path,
    node: Option<usize>,
    case: Option<&Path>,
    max_pivots: usize,
    details: bool,
    first_divergence_continuation_output: Option<&Path>,
) -> Result<Value, String> {
    let analysis = load_oracle_analysis_workspace_v1(workspace)?;
    let node = node.unwrap_or_else(|| analysis.session.cursor_node_id());
    let continuation = analysis.continuation(node)?;
    let expected_final = continuation.session.clone().into_session()?;
    let case_origin = case
        .map(|case| {
            let census = exact_census_run_progress_journal_combat_roots_v1(
                continuation.seed,
                continuation.ascension,
                &continuation.journal,
                &expected_final,
            );
            match_combat_case_origin(
                case,
                continuation.seed,
                continuation.ascension,
                &census.combat_roots,
                census.replay_error.as_deref(),
            )
        })
        .transpose()?;
    let report = exact_diagnose_run_progress_journal_v1(
        analysis.seed,
        analysis.ascension,
        &continuation.journal,
        &expected_final,
        current_oracle_candidate_order_v1,
        max_pivots,
    )?;

    let divergence_continuation = first_divergence_continuation_output
        .map(|output| {
            let divergence = report.policy.first_divergence.as_ref().ok_or_else(|| {
                "cannot export a first-divergence continuation because the current owner agrees with every audited decision".to_string()
            })?;
            let historical = exact_replay_run_progress_journal_prefix_v1(
                continuation.seed,
                continuation.ascension,
                &continuation.journal,
                &expected_final,
                divergence.journal_entry,
            )?;
            let prefix = RunProgressJournalV1::from_committed_steps(
                continuation
                    .journal
                    .entries()
                    .iter()
                    .take(divergence.journal_entry)
                    .cloned()
                    .collect(),
            )?;
            let mut checkpoint = RunControlSessionCheckpointV1::from_session(&historical);
            checkpoint.clear_combat_diagnostics_for_external_checkpoint();
            let output_continuation = OracleRunContinuationV1 {
                schema_name: continuation.schema_name.clone(),
                schema_version: continuation.schema_version,
                seed: continuation.seed,
                ascension: continuation.ascension,
                journal: prefix,
                session: checkpoint,
                explorer_frontier: None,
            };
            save_oracle_run_continuation_v1(output, &output_continuation)?;
            Ok::<_, String>(json!({
                "output": output,
                "journal_entry": divergence.journal_entry,
                "act": divergence.act,
                "floor": divergence.floor,
                "historical_choice": divergence.chosen_label,
                "current_owner_first_choice": divergence.owner_first_label,
                "journal_entries": output_continuation.journal.len(),
            }))
        })
        .transpose()?;

    let report = if details {
        serde_json::to_value(&report)
            .map_err(|error| format!("failed to encode run witness diagnosis: {error}"))?
    } else {
        json!({
            "replay": report.replay,
            "line_identity": report.line_identity,
            "policy": {
                "decisions_with_owner_preferences": report.policy.decisions_with_owner_preferences,
                "decisions_without_owner_preferences": report.policy.decisions_without_owner_preferences,
                "rank_zero_agreements": report.policy.rank_zero_agreements,
                "nonzero_rank_choices": report.policy.nonzero_rank_choices,
                "choices_absent_from_owner_preferences": report.policy.choices_absent_from_owner_preferences,
                "discrepancy_sum": report.policy.discrepancy_sum,
                "max_owner_rank": report.policy.max_owner_rank,
                "same_potion_kind_discard_choices": report.policy.same_potion_kind_discard_choices,
                "first_divergence": report.policy.first_divergence,
                "first_unclassified_divergence": report.policy.first_unclassified_divergence,
                "combat_sources": report.policy.combat_sources,
            },
            "combat_count": report.combat_timeline.len(),
            "current_combat_root": report.current_combat_root,
            "highest_peak_hp_loss_combats": report.highest_peak_hp_loss_combats
                .iter()
                .map(compact_combat_pivot)
                .collect::<Vec<_>>(),
            "lowest_post_combat_hp_combats": report.lowest_post_combat_hp_combats
                .iter()
                .map(compact_combat_pivot)
                .collect::<Vec<_>>(),
            "recovery_pivots": report.recovery_pivots,
            "current_hp_epoch": {
                "last_full_hp_reset": report.current_hp_epoch.last_full_hp_reset,
                "start": report.current_hp_epoch.start,
                "current": report.current_hp_epoch.current,
                "net_hp_change": report.current_hp_epoch.net_hp_change,
                "combat_timeline": report.current_hp_epoch.combat_timeline
                    .iter()
                    .map(compact_combat_pivot)
                    .collect::<Vec<_>>(),
            },
        })
    };

    Ok(json!({
        "schema_name": "ExactOracleRunWitnessDiagnosisV2",
        "schema_version": 2,
        "workspace": workspace,
        "node_id": node,
        "node_identity_scope": "workspace_local_only",
        "case_origin": case_origin,
        "max_pivots": max_pivots.max(1),
        "report": report,
        "first_divergence_continuation": divergence_continuation,
    }))
}

fn compact_combat_pivot(entry: &RunWitnessCombatTimelineEntryV1) -> Value {
    json!({
        "journal_entry": entry.journal_entry,
        "root_identity": entry.root_identity,
        "act": entry.act,
        "floor": entry.floor,
        "encounter": entry.encounter,
        "resolution_kind": entry.resolution_kind,
        "source": entry.source,
        "action_count": entry.action_count,
        "hp_before": entry.hp_before,
        "minimum_combat_hp": entry.minimum_combat_hp,
        "hp_after": entry.hp_after,
        "peak_hp_loss": entry.peak_hp_loss,
        "net_hp_change": entry.net_hp_change,
        "potions_before": entry.potions_before,
        "potions_after": entry.potions_after,
        "preceding_strategic_decisions": entry.preceding_strategic_decisions.iter()
            .map(|decision| json!({
                "journal_entry": decision.journal_entry,
                "act": decision.act,
                "floor": decision.floor,
                "boundary": decision.boundary,
                "chosen_label": decision.chosen_label,
            }))
            .collect::<Vec<_>>(),
    })
}

fn match_combat_case_origin(
    case_path: &Path,
    seed: u64,
    ascension: u8,
    roots: &[RunWitnessCombatRootIdentityV1],
    replay_error: Option<&str>,
) -> Result<Value, String> {
    let case = load_combat_case(case_path)?;
    if case.source.seed != seed || case.source.ascension != ascension {
        return Err(format!(
            "combat case origin mismatch: selected run is seed {seed} ascension {ascension}, but case is seed {} ascension {}",
            case.source.seed, case.source.ascension
        ));
    }
    let case_identity = combat_case_replay_identity_v1(&case)?;
    let case_run_fingerprint = case_identity.run_session_fingerprint.as_deref().ok_or_else(|| {
        format!(
            "combat case {} has no exact production run-session fingerprint; state-only or derived cases cannot be matched to a run witness",
            case_path.display()
        )
    })?;
    let matches = roots
        .iter()
        .filter(|entry| {
            entry.root_exact_state_hash == case_identity.root_exact_state_hash
                && entry.run_session_fingerprint == case_run_fingerprint
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [entry] => {
            if let Some(replay_error) = replay_error {
                return Err(format!(
                    "combat case origin matched, but the selected run witness failed exact replay after that root: {replay_error}"
                ));
            }
            Ok(json!({
                "status": "matched",
                "case": case_path,
                "case_identity": case_identity,
                "matched_journal_entry": entry.journal_entry,
                "matched_root_identity": entry,
            }))
        }
        [] => {
            let candidates = roots
                .iter()
                .filter(|root| {
                    root.resources.act == case.run.act && root.resources.floor == case.run.floor
                })
                .take(8)
                .map(|root| {
                    json!({
                        "origin": root.origin,
                        "journal_entry": root.journal_entry,
                        "boundary": root.boundary,
                        "root_exact_state_hash": root.root_exact_state_hash,
                        "run_session_fingerprint": root.run_session_fingerprint,
                    })
                })
                .collect::<Vec<_>>();
            let candidates = serde_json::to_string(&candidates)
                .map_err(|error| format!("failed to encode origin candidates: {error}"))?;
            let replay_status = replay_error.map_or_else(
                || "selected run witness replay completed".to_string(),
                |error| format!("selected run witness replay stopped early: {error}"),
            );
            Err(format!(
                "combat case origin mismatch: {} did not match any validated combat root; {replay_status}; same-floor candidates={candidates}",
                case_path.display()
            ))
        }
        entries => Err(format!(
            "combat case origin is ambiguous: {} matches journal entries {:?}",
            case_path.display(),
            entries
                .iter()
                .map(|entry| entry.journal_entry)
                .collect::<Vec<_>>()
        )),
    }
}
