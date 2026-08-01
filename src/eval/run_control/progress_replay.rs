use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::content::potions::PotionId;

use super::combat_line_executor::drawn_cards_from_action_result;
use super::combat_line_trace::{
    combat_automation_opportunity_state_v1, combat_automation_step_state_v1,
};
use super::oracle_run_explorer::run_session_fingerprint_v2;
use super::{
    DecisionCandidateKey, RunCombatResolutionBoundaryV1, RunCombatResolutionV1, RunControlConfig,
    RunControlSession, RunDecisionBoundaryV1, RunDecisionTransactionV1, RunProgressJournalV1,
    RunProgressStepV1,
};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ExactRunProgressReplayReportV1 {
    pub seed: u64,
    pub ascension: u8,
    pub journal_entries: usize,
    pub decisions: usize,
    pub forced_transitions: usize,
    pub combat_resolutions: usize,
    pub combat_actions: usize,
    pub final_fingerprint: String,
    pub act: u8,
    pub floor: i32,
    pub current_hp: i32,
    pub max_hp: i32,
    pub engine_state: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WitnessPolicyDecisionAuditV1 {
    pub journal_entry: usize,
    pub decision_ordinal: usize,
    pub act: u8,
    pub floor: i32,
    pub boundary: String,
    pub location: String,
    pub chosen_candidate_id: String,
    pub chosen_label: String,
    pub owner_rank: Option<usize>,
    pub owner_candidate_count: usize,
    pub owner_first_candidate_id: Option<String>,
    pub owner_first_label: Option<String>,
    pub resources: RunWitnessResourceSnapshotV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct RunWitnessPotionSnapshotV1 {
    pub id: PotionId,
    pub uuid: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RunWitnessResourceSnapshotV1 {
    pub act: u8,
    pub floor: i32,
    pub current_hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub deck_size: usize,
    pub potions: Vec<Option<RunWitnessPotionSnapshotV1>>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RunWitnessStrategicDecisionV1 {
    pub journal_entry: usize,
    pub act: u8,
    pub floor: i32,
    pub boundary: String,
    pub chosen_label: String,
    pub key: DecisionCandidateKey,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RunWitnessCombatTimelineEntryV1 {
    pub journal_entry: usize,
    pub act: u8,
    pub floor: i32,
    pub encounter: String,
    pub resolution_kind: String,
    pub source: String,
    pub action_count: usize,
    pub hp_before: i32,
    pub minimum_combat_hp: i32,
    pub hp_after: i32,
    pub peak_hp_loss: i32,
    pub net_hp_change: i32,
    pub potions_before: Vec<Option<RunWitnessPotionSnapshotV1>>,
    pub potions_after: Vec<Option<RunWitnessPotionSnapshotV1>>,
    pub preceding_strategic_decisions: Vec<RunWitnessStrategicDecisionV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RunWitnessRecoveryPivotV1 {
    pub journal_entry: usize,
    pub act: u8,
    pub floor: i32,
    pub boundary: String,
    pub chosen_label: Option<String>,
    pub hp_before: i32,
    pub hp_after: i32,
    pub hp_recovered: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExactRunWitnessDiagnosisReportV1 {
    pub replay: ExactRunProgressReplayReportV1,
    pub policy: ExactRunWitnessPolicyAuditReportV1,
    pub combat_timeline: Vec<RunWitnessCombatTimelineEntryV1>,
    pub highest_peak_hp_loss_combats: Vec<RunWitnessCombatTimelineEntryV1>,
    pub lowest_post_combat_hp_combats: Vec<RunWitnessCombatTimelineEntryV1>,
    pub recovery_pivots: Vec<RunWitnessRecoveryPivotV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExactRunWitnessPolicyAuditReportV1 {
    pub replay: ExactRunProgressReplayReportV1,
    pub decisions_with_owner_preferences: usize,
    pub decisions_without_owner_preferences: usize,
    pub rank_zero_agreements: usize,
    pub nonzero_rank_choices: usize,
    pub choices_absent_from_owner_preferences: usize,
    pub discrepancy_sum: usize,
    pub max_owner_rank: Option<usize>,
    pub first_divergence: Option<WitnessPolicyDecisionAuditV1>,
    pub divergences: Vec<WitnessPolicyDecisionAuditV1>,
    pub combat_sources: BTreeMap<String, usize>,
}

/// Re-executes a committed run journal from the canonical initial state and
/// verifies every recorded decision/combat boundary plus the final normalized
/// session fingerprint. This is deliberately independent of owner policy and
/// search: a saved witness is accepted only when its exact recorded actions
/// still produce the saved terminal state.
pub fn exact_replay_run_progress_journal_v1(
    seed: u64,
    ascension: u8,
    journal: &RunProgressJournalV1,
    expected_final: &RunControlSession,
) -> Result<ExactRunProgressReplayReportV1, String> {
    exact_replay_run_progress_journal_observed_v1(
        seed,
        ascension,
        journal,
        expected_final,
        |_, _, _| Ok(()),
        |_, _, _| Ok(()),
    )
}

/// Replays a committed journal up to, but not including, one entry and
/// returns the exact session at that historical boundary.
///
/// This is the supported extraction path for historical combat cases. It
/// validates every preceding decision and combat instead of asking tooling to
/// edit continuation JSON or reconstruct persistent journal nodes.
pub fn exact_replay_run_progress_journal_prefix_v1(
    seed: u64,
    ascension: u8,
    journal: &RunProgressJournalV1,
    expected_final: &RunControlSession,
    stop_before_entry: usize,
) -> Result<RunControlSession, String> {
    if stop_before_entry > journal.len() {
        return Err(format!(
            "journal prefix entry {stop_before_entry} exceeds journal length {}",
            journal.len()
        ));
    }
    let mut session = canonical_replay_session(seed, ascension, expected_final);
    let mut counters = ExactReplayCountersV1::default();
    let mut before_decision =
        |_: usize, _: &RunControlSession, _: &RunDecisionTransactionV1| Ok(());
    for (entry_index, entry) in journal.entries().iter().take(stop_before_entry).enumerate() {
        apply_exact_progress_entry_v1(
            entry_index,
            entry,
            &mut session,
            &mut counters,
            &mut before_decision,
        )?;
    }
    Ok(session)
}

/// Replays an exact witness and compares every committed non-combat choice
/// against the ordering produced by the current owner implementation.
///
/// The owner remains read-only: its ordering neither mutates the replay nor
/// decides whether the historical witness is valid. An absent rank means that
/// the current owner did not include the committed action in its preference
/// list; it is reported separately instead of being converted into an
/// arbitrary discrepancy score.
pub fn exact_audit_run_progress_journal_policy_v1<F>(
    seed: u64,
    ascension: u8,
    journal: &RunProgressJournalV1,
    expected_final: &RunControlSession,
    decision_order: F,
) -> Result<ExactRunWitnessPolicyAuditReportV1, String>
where
    F: FnMut(&RunControlSession) -> Vec<String>,
{
    exact_audit_run_progress_journal_policy_observed_v1(
        seed,
        ascension,
        journal,
        expected_final,
        decision_order,
        |_, _, _| Ok(()),
    )
}

/// Replays one exact run witness once and returns the compact typed pivots
/// needed to choose a bounded counterfactual. This is diagnostic only: it
/// neither changes owner policy nor assigns causal credit to a divergence.
pub fn exact_diagnose_run_progress_journal_v1<F>(
    seed: u64,
    ascension: u8,
    journal: &RunProgressJournalV1,
    expected_final: &RunControlSession,
    decision_order: F,
    max_pivots: usize,
) -> Result<ExactRunWitnessDiagnosisReportV1, String>
where
    F: FnMut(&RunControlSession) -> Vec<String>,
{
    let initial = canonical_replay_session(seed, ascension, expected_final);
    let mut before_resources = run_witness_resource_snapshot_v1(&initial);
    let mut strategic_decisions_since_combat = Vec::new();
    let mut combat_timeline = Vec::new();
    let mut recovery_pivots = Vec::new();

    let policy = exact_audit_run_progress_journal_policy_observed_v1(
        seed,
        ascension,
        journal,
        expected_final,
        decision_order,
        |entry_index, entry, session| {
            let after_resources = run_witness_resource_snapshot_v1(session);
            if after_resources.current_hp > before_resources.current_hp {
                let (boundary, chosen_label) = progress_entry_label_v1(entry);
                recovery_pivots.push(RunWitnessRecoveryPivotV1 {
                    journal_entry: entry_index,
                    act: before_resources.act,
                    floor: before_resources.floor,
                    boundary,
                    chosen_label,
                    hp_before: before_resources.current_hp,
                    hp_after: after_resources.current_hp,
                    hp_recovered: after_resources.current_hp - before_resources.current_hp,
                });
            }

            match entry {
                RunProgressStepV1::Decision(record) => {
                    if let Some(decision) =
                        strategic_decision_v1(entry_index, &before_resources, record)
                    {
                        strategic_decisions_since_combat.push(decision);
                    }
                }
                RunProgressStepV1::CombatResolution(record) => {
                    let minimum_combat_hp = record
                        .trajectory
                        .actions
                        .iter()
                        .filter_map(|action| {
                            action.combat_after.as_ref().map(|state| state.player_hp)
                        })
                        .fold(
                            before_resources.current_hp.min(after_resources.current_hp),
                            i32::min,
                        );
                    combat_timeline.push(RunWitnessCombatTimelineEntryV1 {
                        journal_entry: entry_index,
                        act: before_resources.act,
                        floor: before_resources.floor,
                        encounter: combat_encounter_label_v1(record),
                        resolution_kind: format!("{:?}", record.kind),
                        source: record.trajectory.source.label().to_string(),
                        action_count: record.trajectory.action_count,
                        hp_before: before_resources.current_hp,
                        minimum_combat_hp,
                        hp_after: after_resources.current_hp,
                        peak_hp_loss: (before_resources.current_hp - minimum_combat_hp).max(0),
                        net_hp_change: after_resources.current_hp - before_resources.current_hp,
                        potions_before: before_resources.potions.clone(),
                        potions_after: after_resources.potions.clone(),
                        preceding_strategic_decisions: std::mem::take(
                            &mut strategic_decisions_since_combat,
                        ),
                    });
                }
                RunProgressStepV1::ForcedTransition(_) | RunProgressStepV1::Stop(_) => {}
            }
            before_resources = after_resources;
            Ok(())
        },
    )?;

    let pivot_limit = max_pivots.max(1);
    let mut highest_peak_hp_loss_combats = combat_timeline.clone();
    highest_peak_hp_loss_combats.sort_by(|left, right| {
        right
            .peak_hp_loss
            .cmp(&left.peak_hp_loss)
            .then_with(|| left.journal_entry.cmp(&right.journal_entry))
    });
    highest_peak_hp_loss_combats.truncate(pivot_limit);

    let mut lowest_post_combat_hp_combats = combat_timeline.clone();
    lowest_post_combat_hp_combats.sort_by(|left, right| {
        left.hp_after
            .cmp(&right.hp_after)
            .then_with(|| left.journal_entry.cmp(&right.journal_entry))
    });
    lowest_post_combat_hp_combats.truncate(pivot_limit);

    recovery_pivots.sort_by(|left, right| {
        right
            .hp_recovered
            .cmp(&left.hp_recovered)
            .then_with(|| left.journal_entry.cmp(&right.journal_entry))
    });
    recovery_pivots.truncate(pivot_limit);

    Ok(ExactRunWitnessDiagnosisReportV1 {
        replay: policy.replay.clone(),
        policy,
        combat_timeline,
        highest_peak_hp_loss_combats,
        lowest_post_combat_hp_combats,
        recovery_pivots,
    })
}

fn exact_audit_run_progress_journal_policy_observed_v1<F, G>(
    seed: u64,
    ascension: u8,
    journal: &RunProgressJournalV1,
    expected_final: &RunControlSession,
    mut decision_order: F,
    after_entry: G,
) -> Result<ExactRunWitnessPolicyAuditReportV1, String>
where
    F: FnMut(&RunControlSession) -> Vec<String>,
    G: FnMut(usize, &RunProgressStepV1, &RunControlSession) -> Result<(), String>,
{
    let mut decision_audits = Vec::new();
    let replay = exact_replay_run_progress_journal_observed_v1(
        seed,
        ascension,
        journal,
        expected_final,
        |entry_index, session, record| {
            let owner_order = decision_order(session);
            let selected_id = record.selection.candidate_id.clone();
            let owner_rank = owner_order
                .iter()
                .position(|candidate_id| candidate_id == &selected_id);
            let chosen_label = record
                .before
                .candidates
                .iter()
                .find(|candidate| candidate.candidate_id == selected_id)
                .map(|candidate| candidate.label.clone())
                .unwrap_or_else(|| selected_id.clone());
            let owner_first_candidate_id = owner_order.first().cloned();
            let owner_first_label = owner_first_candidate_id.as_ref().and_then(|candidate_id| {
                record
                    .before
                    .candidates
                    .iter()
                    .find(|candidate| &candidate.candidate_id == candidate_id)
                    .map(|candidate| candidate.label.clone())
            });
            decision_audits.push(WitnessPolicyDecisionAuditV1 {
                journal_entry: entry_index,
                decision_ordinal: decision_audits.len(),
                act: session.run_state.act_num,
                floor: session.run_state.floor_num,
                boundary: record.before.title.clone(),
                location: record.before.location.clone(),
                chosen_candidate_id: selected_id,
                chosen_label,
                owner_rank,
                owner_candidate_count: owner_order.len(),
                owner_first_candidate_id,
                owner_first_label,
                resources: run_witness_resource_snapshot_v1(session),
            });
            Ok(())
        },
        after_entry,
    )?;

    let decisions_with_owner_preferences = decision_audits
        .iter()
        .filter(|audit| audit.owner_candidate_count > 0)
        .count();
    let decisions_without_owner_preferences = decision_audits
        .len()
        .saturating_sub(decisions_with_owner_preferences);
    let rank_zero_agreements = decision_audits
        .iter()
        .filter(|audit| audit.owner_rank == Some(0))
        .count();
    let nonzero_rank_choices = decision_audits
        .iter()
        .filter(|audit| audit.owner_rank.is_some_and(|rank| rank > 0))
        .count();
    let choices_absent_from_owner_preferences = decision_audits
        .iter()
        .filter(|audit| audit.owner_candidate_count > 0 && audit.owner_rank.is_none())
        .count();
    let discrepancy_sum = decision_audits
        .iter()
        .filter_map(|audit| audit.owner_rank)
        .sum();
    let max_owner_rank = decision_audits
        .iter()
        .filter_map(|audit| audit.owner_rank)
        .max();
    let divergences = decision_audits
        .into_iter()
        .filter(|audit| {
            audit.owner_candidate_count > 0
                && (audit.owner_rank.is_none() || audit.owner_rank.is_some_and(|rank| rank > 0))
        })
        .collect::<Vec<_>>();
    let first_divergence = divergences.first().cloned();
    let mut combat_sources = BTreeMap::new();
    for entry in journal.entries() {
        if let RunProgressStepV1::CombatResolution(record) = entry {
            *combat_sources
                .entry(record.trajectory.source.label().to_string())
                .or_insert(0) += 1;
        }
    }

    Ok(ExactRunWitnessPolicyAuditReportV1 {
        replay,
        decisions_with_owner_preferences,
        decisions_without_owner_preferences,
        rank_zero_agreements,
        nonzero_rank_choices,
        choices_absent_from_owner_preferences,
        discrepancy_sum,
        max_owner_rank,
        first_divergence,
        divergences,
        combat_sources,
    })
}

fn run_witness_resource_snapshot_v1(session: &RunControlSession) -> RunWitnessResourceSnapshotV1 {
    RunWitnessResourceSnapshotV1 {
        act: session.run_state.act_num,
        floor: session.run_state.floor_num,
        current_hp: session.run_state.current_hp,
        max_hp: session.run_state.max_hp,
        gold: session.run_state.gold,
        deck_size: session.run_state.master_deck.len(),
        potions: session
            .run_state
            .potions
            .iter()
            .map(|slot| {
                slot.as_ref().map(|potion| RunWitnessPotionSnapshotV1 {
                    id: potion.id,
                    uuid: potion.uuid,
                })
            })
            .collect(),
    }
}

fn strategic_decision_v1(
    journal_entry: usize,
    resources: &RunWitnessResourceSnapshotV1,
    record: &RunDecisionTransactionV1,
) -> Option<RunWitnessStrategicDecisionV1> {
    let selected = record
        .before
        .candidates
        .iter()
        .find(|candidate| candidate.candidate_id == record.selection.candidate_id)?;
    let key = selected.key.clone()?;
    if !is_strategic_decision_key_v1(&key) {
        return None;
    }
    Some(RunWitnessStrategicDecisionV1 {
        journal_entry,
        act: resources.act,
        floor: resources.floor,
        boundary: record.before.title.clone(),
        chosen_label: selected.label.clone(),
        key,
    })
}

fn is_strategic_decision_key_v1(key: &DecisionCandidateKey) -> bool {
    matches!(
        key,
        DecisionCandidateKey::RouteSelect { .. }
            | DecisionCandidateKey::EventOption { .. }
            | DecisionCandidateKey::CardRewardPick { .. }
            | DecisionCandidateKey::CardRewardSingingBowl { .. }
            | DecisionCandidateKey::CardRewardSkip { .. }
            | DecisionCandidateKey::BossRelicPick { .. }
            | DecisionCandidateKey::BossRelicSkip
            | DecisionCandidateKey::CampfireRest
            | DecisionCandidateKey::CampfireSmith { .. }
            | DecisionCandidateKey::CampfireDig
            | DecisionCandidateKey::CampfireLift
            | DecisionCandidateKey::CampfireToke { .. }
            | DecisionCandidateKey::CampfireRecall
            | DecisionCandidateKey::ShopPurgeCard { .. }
            | DecisionCandidateKey::ShopBuyCard { .. }
            | DecisionCandidateKey::ShopBuyRelic { .. }
            | DecisionCandidateKey::ShopBuyPotion { .. }
    )
}

fn progress_entry_label_v1(entry: &RunProgressStepV1) -> (String, Option<String>) {
    match entry {
        RunProgressStepV1::Decision(record) => {
            let chosen_label = record
                .before
                .candidates
                .iter()
                .find(|candidate| candidate.candidate_id == record.selection.candidate_id)
                .map(|candidate| candidate.label.clone());
            (record.before.title.clone(), chosen_label)
        }
        RunProgressStepV1::ForcedTransition(record) => (
            record.before.title.clone(),
            Some(format!("{:?}", record.kind)),
        ),
        RunProgressStepV1::CombatResolution(record) => (
            record.before.title.clone(),
            Some(record.trajectory.source.label().to_string()),
        ),
        RunProgressStepV1::Stop(record) => ("Stop".to_string(), Some(record.reason.clone())),
    }
}

fn combat_encounter_label_v1(record: &RunCombatResolutionV1) -> String {
    let labels = record
        .trajectory
        .actions
        .iter()
        .find_map(|action| action.combat_after.as_ref())
        .map(|state| {
            state
                .monsters
                .iter()
                .map(|monster| monster.label.as_str())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if labels.is_empty() {
        record.before.title.clone()
    } else {
        labels.join(" + ")
    }
}

/// Replaces one committed combat trajectory while preserving the surrounding
/// strategic history. The replacement is accepted only when its exact combat
/// boundaries match and the entire resulting journal still replays to the
/// original final session fingerprint.
pub fn splice_exact_combat_resolution_v1(
    seed: u64,
    ascension: u8,
    journal: &RunProgressJournalV1,
    expected_final: &RunControlSession,
    journal_entry: usize,
    replacement: &RunCombatResolutionV1,
) -> Result<(RunProgressJournalV1, ExactRunProgressReplayReportV1), String> {
    let original = journal
        .entries()
        .get(journal_entry)
        .and_then(RunProgressStepV1::as_combat_resolution)
        .ok_or_else(|| {
            format!("journal entry {journal_entry} is not a committed combat resolution")
        })?;
    if !combat_boundaries_match(&replacement.before, &original.before) {
        return Err(format!(
            "replacement combat before-boundary does not match journal entry {journal_entry}"
        ));
    }
    if !combat_boundaries_match(&replacement.after, &original.after) {
        return Err(format!(
            "replacement combat after-boundary does not match journal entry {journal_entry}"
        ));
    }

    let mut entries = journal.entries().to_vec();
    entries[journal_entry] = RunProgressStepV1::CombatResolution(replacement.clone());
    let journal = RunProgressJournalV1::from_committed_steps(entries)?;
    let replay = exact_replay_run_progress_journal_v1(seed, ascension, &journal, expected_final)?;
    Ok((journal, replay))
}

fn exact_replay_run_progress_journal_observed_v1<F, G>(
    seed: u64,
    ascension: u8,
    journal: &RunProgressJournalV1,
    expected_final: &RunControlSession,
    mut before_decision: F,
    mut after_entry: G,
) -> Result<ExactRunProgressReplayReportV1, String>
where
    F: FnMut(usize, &RunControlSession, &RunDecisionTransactionV1) -> Result<(), String>,
    G: FnMut(usize, &RunProgressStepV1, &RunControlSession) -> Result<(), String>,
{
    let mut session = canonical_replay_session(seed, ascension, expected_final);
    let mut counters = ExactReplayCountersV1::default();

    for (entry_index, entry) in journal.entries().iter().enumerate() {
        apply_exact_progress_entry_v1(
            entry_index,
            entry,
            &mut session,
            &mut counters,
            &mut before_decision,
        )?;
        after_entry(entry_index, entry, &session)?;
    }

    let final_fingerprint = run_session_fingerprint_v2(&session);
    let expected_fingerprint = run_session_fingerprint_v2(expected_final);
    if final_fingerprint != expected_fingerprint {
        return Err(format!(
            "journal replay final fingerprint mismatch: expected {expected_fingerprint}, got {final_fingerprint}"
        ));
    }

    Ok(ExactRunProgressReplayReportV1 {
        seed,
        ascension,
        journal_entries: journal.len(),
        decisions: counters.decisions,
        forced_transitions: counters.forced_transitions,
        combat_resolutions: counters.combat_resolutions,
        combat_actions: counters.combat_actions,
        final_fingerprint,
        act: session.run_state.act_num,
        floor: session.run_state.floor_num,
        current_hp: session.run_state.current_hp,
        max_hp: session.run_state.max_hp,
        engine_state: format!("{:?}", session.engine_state),
    })
}

#[derive(Default)]
struct ExactReplayCountersV1 {
    decisions: usize,
    forced_transitions: usize,
    combat_resolutions: usize,
    combat_actions: usize,
}

fn canonical_replay_session(
    seed: u64,
    ascension: u8,
    expected_final: &RunControlSession,
) -> RunControlSession {
    RunControlSession::new(RunControlConfig {
        seed,
        ascension_level: ascension,
        final_act: false,
        player_class: expected_final.run_state.player_class,
        reward_automation: expected_final.reward_automation.clone(),
        ..RunControlConfig::default()
    })
}

fn apply_exact_progress_entry_v1<F>(
    entry_index: usize,
    entry: &RunProgressStepV1,
    session: &mut RunControlSession,
    counters: &mut ExactReplayCountersV1,
    before_decision: &mut F,
) -> Result<(), String>
where
    F: FnMut(usize, &RunControlSession, &RunDecisionTransactionV1) -> Result<(), String>,
{
    match entry {
        RunProgressStepV1::Decision(record) => {
            let actual_before = RunDecisionBoundaryV1::capture(session);
            if !decision_boundaries_match(&actual_before, &record.before) {
                return Err(format!(
                    "journal entry {entry_index} decision before-boundary mismatch: expected {:?}, got {:?}",
                    record.before,
                    actual_before,
                ));
            }
            before_decision(entry_index, session, record)?;
            session
                .apply_decision_action(record.action.clone())
                .map_err(|error| {
                    format!("journal entry {entry_index} decision replay failed: {error}")
                })?;
            let actual_after = RunDecisionBoundaryV1::capture(session);
            if !decision_boundaries_match(&actual_after, &record.after) {
                return Err(format!(
                    "journal entry {entry_index} decision after-boundary mismatch: expected {:?}, got {:?}",
                    record.after,
                    actual_after,
                ));
            }
            counters.decisions = counters.decisions.saturating_add(1);
        }
        RunProgressStepV1::ForcedTransition(record) => {
            let actual_before = RunDecisionBoundaryV1::capture(session);
            if !decision_boundaries_match(&actual_before, &record.before) {
                return Err(format!(
                    "journal entry {entry_index} forced-transition before-boundary mismatch"
                ));
            }
            session
                .apply_forced_transition(record.kind)
                .map_err(|error| {
                    format!("journal entry {entry_index} forced-transition replay failed: {error}")
                })?;
            let actual_after = RunDecisionBoundaryV1::capture(session);
            if !decision_boundaries_match(&actual_after, &record.after) {
                return Err(format!(
                    "journal entry {entry_index} forced-transition after-boundary mismatch"
                ));
            }
            counters.forced_transitions = counters.forced_transitions.saturating_add(1);
        }
        RunProgressStepV1::CombatResolution(record) => {
            let actual_before = RunCombatResolutionBoundaryV1::capture(session);
            if !combat_boundaries_match(&actual_before, &record.before) {
                return Err(format!(
                    "journal entry {entry_index} combat before-boundary mismatch: expected '{} @ {}', got '{} @ {}'",
                    record.before.title,
                    record.before.location,
                    actual_before.title,
                    actual_before.location,
                ));
            }
            session.mark_current_combat_search_resolved();
            for (action_index, action) in record.trajectory.actions.iter().enumerate() {
                let opportunity = combat_automation_opportunity_state_v1(session);
                if opportunity != action.opportunity_before {
                    return Err(format!(
                        "journal entry {entry_index} combat action {action_index} opportunity mismatch"
                    ));
                }
                let outcome = session
                    .apply_combat_resolution_input(action.input.clone())
                    .map_err(|error| {
                        format!(
                            "journal entry {entry_index} combat action {action_index} replay failed: {error}"
                        )
                    })?;
                let drawn_cards = drawn_cards_from_action_result(outcome.action_result.as_ref());
                if drawn_cards != action.drawn_cards {
                    return Err(format!(
                        "journal entry {entry_index} combat action {action_index} drawn-card mismatch"
                    ));
                }
                let combat_after = combat_automation_step_state_v1(session);
                if combat_after != action.combat_after {
                    return Err(format!(
                        "journal entry {entry_index} combat action {action_index} successor mismatch"
                    ));
                }
                counters.combat_actions = counters.combat_actions.saturating_add(1);
            }
            let actual_after = RunCombatResolutionBoundaryV1::capture(session);
            if !combat_boundaries_match(&actual_after, &record.after) {
                return Err(format!(
                    "journal entry {entry_index} combat after-boundary mismatch: expected {:?}, got {:?}",
                    record.after,
                    actual_after,
                ));
            }
            counters.combat_resolutions = counters.combat_resolutions.saturating_add(1);
        }
        RunProgressStepV1::Stop(_) => {
            return Err(format!(
                "journal entry {entry_index} contains a non-committed Stop record"
            ));
        }
    }
    Ok(())
}

fn combat_boundaries_match(
    actual: &RunCombatResolutionBoundaryV1,
    expected: &RunCombatResolutionBoundaryV1,
) -> bool {
    // combat_sequence is diagnostic bookkeeping and is deliberately removed
    // by the canonical run-session fingerprint. Historical search/import
    // paths can observe the same exact combat with a different counter while
    // preserving every game-semantic state transition.
    actual.decision_step == expected.decision_step
        && actual.title == expected.title
        && actual.location == expected.location
        && actual.active_combat == expected.active_combat
}

fn decision_boundaries_match(
    actual: &RunDecisionBoundaryV1,
    expected: &RunDecisionBoundaryV1,
) -> bool {
    // The visible candidate surface may grow when a newer build exposes an
    // additional legal action (for example explicit potion discard). The
    // journal owns the action that was actually committed; replay validates
    // that action against the current exact state. Requiring every unrelated
    // visible candidate to remain byte-identical would reject valid witnesses
    // for a presentation/schema change rather than a game-state divergence.
    actual.decision_step == expected.decision_step
        && actual.title == expected.title
        && actual.location == expected.location
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_witness_policy_audit_is_exact_and_search_free() {
        let seed = 20260713006;
        let config = RunControlConfig {
            seed,
            ascension_level: 0,
            ..RunControlConfig::default()
        };
        let expected_final = RunControlSession::new(config);
        let report = exact_audit_run_progress_journal_policy_v1(
            seed,
            0,
            &RunProgressJournalV1::default(),
            &expected_final,
            |_| panic!("an empty journal must not ask the owner for an ordering"),
        )
        .unwrap();

        assert_eq!(report.replay.journal_entries, 0);
        assert_eq!(report.replay.combat_resolutions, 0);
        assert_eq!(report.decisions_with_owner_preferences, 0);
        assert!(report.divergences.is_empty());
        assert!(report.combat_sources.is_empty());
    }

    #[test]
    fn empty_witness_diagnosis_is_compact_exact_and_search_free() {
        let seed = 20260713006;
        let expected_final = RunControlSession::new(RunControlConfig {
            seed,
            ascension_level: 0,
            ..RunControlConfig::default()
        });
        let report = exact_diagnose_run_progress_journal_v1(
            seed,
            0,
            &RunProgressJournalV1::default(),
            &expected_final,
            |_| panic!("an empty diagnosis must not ask the owner for an ordering"),
            5,
        )
        .unwrap();

        assert_eq!(report.replay.journal_entries, 0);
        assert!(report.policy.first_divergence.is_none());
        assert!(report.combat_timeline.is_empty());
        assert!(report.highest_peak_hp_loss_combats.is_empty());
        assert!(report.lowest_post_combat_hp_combats.is_empty());
        assert!(report.recovery_pivots.is_empty());
    }
}
