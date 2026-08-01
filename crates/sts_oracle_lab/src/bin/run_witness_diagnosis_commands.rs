use std::path::Path;

use serde_json::{json, Value};
use sts_oracle_runtime::eval::run_control::{
    exact_diagnose_run_progress_journal_v1, exact_replay_run_progress_journal_prefix_v1,
    RunControlSessionCheckpointV1, RunProgressJournalV1, RunWitnessCombatTimelineEntryV1,
};
use sts_oracle_runtime::runtime::branch::{
    current_oracle_candidate_order_v1, load_oracle_analysis_workspace_v1,
    save_oracle_run_continuation_v1, OracleRunContinuationV1,
};

pub(super) fn diagnose(
    workspace: &Path,
    node: usize,
    max_pivots: usize,
    details: bool,
    first_divergence_continuation_output: Option<&Path>,
) -> Result<Value, String> {
    let analysis = load_oracle_analysis_workspace_v1(workspace)?;
    let continuation = analysis.continuation(node)?;
    let expected_final = continuation.session.clone().into_session()?;
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
            "policy": {
                "decisions_with_owner_preferences": report.policy.decisions_with_owner_preferences,
                "decisions_without_owner_preferences": report.policy.decisions_without_owner_preferences,
                "rank_zero_agreements": report.policy.rank_zero_agreements,
                "nonzero_rank_choices": report.policy.nonzero_rank_choices,
                "choices_absent_from_owner_preferences": report.policy.choices_absent_from_owner_preferences,
                "discrepancy_sum": report.policy.discrepancy_sum,
                "max_owner_rank": report.policy.max_owner_rank,
                "first_divergence": report.policy.first_divergence,
                "combat_sources": report.policy.combat_sources,
            },
            "combat_count": report.combat_timeline.len(),
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
        "schema_name": "ExactOracleRunWitnessDiagnosisV1",
        "schema_version": 1,
        "workspace": workspace,
        "node_id": node,
        "max_pivots": max_pivots.max(1),
        "report": report,
        "first_divergence_continuation": divergence_continuation,
    }))
}

fn compact_combat_pivot(entry: &RunWitnessCombatTimelineEntryV1) -> Value {
    json!({
        "journal_entry": entry.journal_entry,
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
