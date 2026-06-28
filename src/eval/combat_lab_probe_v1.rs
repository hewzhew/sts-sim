use serde::{Deserialize, Serialize};

use crate::eval::run_control::{
    build_decision_surface, CombatAutomationTrajectoryRecordV1, RunControlCommand,
    RunControlHpLossLimit, RunControlSearchCombatOptions, RunControlSession,
};
use crate::sim::combat_start::build_natural_combat_start;
use crate::state::core::{ActiveCombat, CombatStartRequest};
use crate::state::map::node::RoomType;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatLabProbeDiagnosisV1 {
    pub outcome_class: String,
    pub search_reason: String,
    pub confidence: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub signals: Vec<String>,
}

impl CombatLabProbeDiagnosisV1 {
    pub fn is_empty(&self) -> bool {
        self.outcome_class.is_empty()
            && self.search_reason.is_empty()
            && self.confidence.is_empty()
            && self.signals.is_empty()
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatLabProbePacketV1 {
    pub kind: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boss: Option<String>,
    pub boundary: String,
    pub result: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hp_loss: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_hp: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_hp: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actions: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub search_digest: Vec<String>,
    #[serde(default, skip_serializing_if = "CombatLabProbeDiagnosisV1::is_empty")]
    pub diagnosis: CombatLabProbeDiagnosisV1,
}

pub fn current_act_boss_preview_probe_v1(
    session: &RunControlSession,
    search_options: &RunControlSearchCombatOptions,
    source: impl Into<String>,
) -> CombatLabProbePacketV1 {
    let source = source.into();
    let boundary = build_decision_surface(session).view.header.title;
    let Some(boss) = session
        .run_state
        .boss_key
        .or_else(|| session.run_state.boss_list.first().copied())
    else {
        return CombatLabProbePacketV1 {
            kind: "current_act_boss_preview".to_string(),
            source,
            boss: None,
            boundary,
            result: "unavailable_no_current_act_boss".to_string(),
            diagnosis: diagnose_combat_lab_probe_v1("unavailable_no_current_act_boss", &[]),
            ..CombatLabProbePacketV1::default()
        };
    };

    let initial_hp = session.run_state.current_hp;
    let mut preview = session.clone();
    let request = CombatStartRequest::room(boss, RoomType::MonsterRoomBoss);
    let (engine_state, combat_state) =
        match build_natural_combat_start(&mut preview.run_state, boss, RoomType::MonsterRoomBoss) {
            Ok(start) => start,
            Err(err) => {
                let result = "unavailable_combat_start_failed";
                let search_digest = vec![format!("combat_start_failed={err}")];
                return CombatLabProbePacketV1 {
                    kind: "current_act_boss_preview".to_string(),
                    source,
                    boss: Some(format!("{boss:?}")),
                    boundary,
                    result: result.to_string(),
                    diagnosis: diagnose_combat_lab_probe_v1(result, &search_digest),
                    search_digest,
                    ..CombatLabProbePacketV1::default()
                };
            }
        };
    preview.engine_state = engine_state.clone();
    preview.active_combat = Some(ActiveCombat::new(
        engine_state,
        combat_state,
        request.context,
    ));
    let previous_trajectory_signature = preview
        .last_combat_automation_trajectory()
        .map(trajectory_signature_v1);

    let mut options = search_options.clone();
    if options.max_hp_loss.is_none() {
        options.max_hp_loss = Some(RunControlHpLossLimit::Unlimited);
    }
    match preview.apply_command(RunControlCommand::SearchCombat(options)) {
        Ok(outcome) => {
            let hp_loss = initial_hp.saturating_sub(preview.run_state.current_hp);
            let record = preview
                .last_combat_automation_trajectory()
                .filter(|record| {
                    Some(trajectory_signature_v1(record)) != previous_trajectory_signature
                });
            if let Some(record) = record {
                let result = boss_preview_result_label_v1(
                    preview.active_combat.is_none(),
                    record.source.label(),
                );
                let search_digest = search_message_digest_v1(&outcome.message);
                CombatLabProbePacketV1 {
                    kind: "current_act_boss_preview".to_string(),
                    source,
                    boss: Some(format!("{boss:?}")),
                    boundary,
                    result: result.to_string(),
                    hp_loss: Some(hp_loss),
                    final_hp: Some(preview.run_state.current_hp),
                    max_hp: Some(preview.run_state.max_hp),
                    actions: Some(record.action_count),
                    diagnosis: diagnose_combat_lab_probe_v1(result, &search_digest),
                    search_digest,
                }
            } else {
                let result = "unresolved_no_trajectory";
                let search_digest = search_message_digest_v1(&outcome.message);
                CombatLabProbePacketV1 {
                    kind: "current_act_boss_preview".to_string(),
                    source,
                    boss: Some(format!("{boss:?}")),
                    boundary,
                    result: result.to_string(),
                    hp_loss: Some(hp_loss),
                    final_hp: Some(preview.run_state.current_hp),
                    max_hp: Some(preview.run_state.max_hp),
                    diagnosis: diagnose_combat_lab_probe_v1(result, &search_digest),
                    search_digest,
                    ..CombatLabProbePacketV1::default()
                }
            }
        }
        Err(err) => {
            let result = "error";
            let search_digest = vec![format!("error={err}")];
            CombatLabProbePacketV1 {
                kind: "current_act_boss_preview".to_string(),
                source,
                boss: Some(format!("{boss:?}")),
                boundary,
                result: result.to_string(),
                diagnosis: diagnose_combat_lab_probe_v1(result, &search_digest),
                search_digest,
                ..CombatLabProbePacketV1::default()
            }
        }
    }
}

pub fn boss_preview_result_label_v1(
    combat_finished: bool,
    trajectory_source: &str,
) -> &'static str {
    if combat_finished && trajectory_source == "search_combat" {
        "complete_win_applied"
    } else if trajectory_source.contains("turn_segment") {
        "turn_segment_applied"
    } else {
        "partial_search_applied"
    }
}

pub fn diagnose_combat_lab_probe_v1(
    result: &str,
    search_digest: &[String],
) -> CombatLabProbeDiagnosisV1 {
    let digest = search_digest.join("\n").to_ascii_lowercase();
    let result_lower = result.to_ascii_lowercase();
    let outcome_class = if result_lower.starts_with("complete_win") {
        "complete_win"
    } else if result_lower.starts_with("turn_segment") || result_lower.starts_with("partial") {
        "partial_progress"
    } else if result_lower.starts_with("unresolved") {
        "unresolved"
    } else if result_lower.starts_with("unavailable") {
        "unavailable"
    } else if result_lower == "error" {
        "error"
    } else {
        "unknown"
    };

    let search_reason = if result == "unavailable_no_current_act_boss" {
        "missing_boss"
    } else if result == "unavailable_combat_start_failed" {
        "combat_start_failed"
    } else if result == "error" {
        "command_error"
    } else if result == "complete_win_applied" {
        "complete_win"
    } else if result == "turn_segment_applied" {
        "turn_segment"
    } else if digest.contains("complete_winning_candidate_exceeds_hp_loss_limit")
        || digest.contains("max_hp_loss")
        || digest.contains("hp-loss limit")
        || digest.contains("hp loss limit")
    {
        "hp_loss_limit"
    } else if digest.contains("wall-clock deadline hit")
        || digest.contains("deadlinehit")
        || digest.contains("deadline hit")
    {
        "wall_clock_deadline_hit"
    } else if digest.contains("timebudgetlimited") || digest.contains("time budget") {
        "time_budget_limited"
    } else if digest.contains("terminal_wins=0") || digest.contains("terminal_wins = 0") {
        "no_terminal_win"
    } else if result == "unresolved_no_trajectory" {
        "no_new_trajectory"
    } else if result == "partial_search_applied" {
        "partial_search"
    } else {
        "unknown"
    };

    let confidence = if result_lower.starts_with("unavailable") && search_digest.is_empty() {
        "no_search"
    } else if result_lower == "error" || result == "unavailable_combat_start_failed" {
        "engine_error"
    } else if !search_digest.is_empty() {
        "search_digest"
    } else {
        "result_label"
    };

    let mut signals = Vec::new();
    if digest.contains("deadline")
        || digest.contains("timebudgetlimited")
        || digest.contains("time budget")
        || digest.contains("max budget")
    {
        push_unique_signal_v1(&mut signals, "budget_limited");
    }
    if digest.contains("terminal_wins=0") || digest.contains("terminal_wins = 0") {
        push_unique_signal_v1(&mut signals, "no_terminal_wins");
    } else if digest.contains("terminal_wins=") {
        push_unique_signal_v1(&mut signals, "terminal_wins_found");
    }
    if digest.contains("complete_trajectory_found=false") {
        push_unique_signal_v1(&mut signals, "no_complete_trajectory");
    }
    if search_reason == "hp_loss_limit" {
        push_unique_signal_v1(&mut signals, "hp_loss_gate");
    }
    if result == "unresolved_no_trajectory" {
        push_unique_signal_v1(&mut signals, "no_new_trajectory");
    }

    CombatLabProbeDiagnosisV1 {
        outcome_class: outcome_class.to_string(),
        search_reason: search_reason.to_string(),
        confidence: confidence.to_string(),
        signals,
    }
}

fn push_unique_signal_v1(signals: &mut Vec<String>, signal: &str) {
    if !signals.iter().any(|existing| existing == signal) {
        signals.push(signal.to_string());
    }
}

fn trajectory_signature_v1(
    record: &CombatAutomationTrajectoryRecordV1,
) -> (String, usize, String, String) {
    (
        record.source.to_string(),
        record.action_count,
        record
            .actions
            .first()
            .map(|action| action.action_key.clone())
            .unwrap_or_default(),
        record
            .actions
            .last()
            .map(|action| action.action_key.clone())
            .unwrap_or_default(),
    )
}

fn search_message_digest_v1(message: &str) -> Vec<String> {
    let interesting_prefixes = [
        "  result=",
        "  detail=",
        "  best_complete_candidate",
        "  coverage_status=",
        "  terminal_wins=",
        "  nodes_expanded=",
        "  nodes_generated=",
        "  reason=",
    ];
    let mut lines = Vec::new();
    for line in message.lines() {
        let trimmed = line.trim_start();
        if interesting_prefixes
            .iter()
            .any(|prefix| line.starts_with(prefix) || trimmed.starts_with(prefix.trim_start()))
        {
            lines.push(trimmed.to_string());
        }
        if lines.len() >= 8 {
            break;
        }
    }
    if lines.is_empty() {
        if let Some(first) = message.lines().find(|line| !line.trim().is_empty()) {
            lines.push(first.trim().chars().take(180).collect());
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::run_control::{
        RunControlConfig, RunControlSearchCombatOptions, RunControlSession,
    };

    #[test]
    fn current_act_boss_preview_reports_missing_boss() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.boss_key = None;
        session.run_state.boss_list.clear();

        let packet = current_act_boss_preview_probe_v1(
            &session,
            &RunControlSearchCombatOptions::default(),
            "test_source",
        );

        assert_eq!(packet.kind, "current_act_boss_preview");
        assert_eq!(packet.source, "test_source");
        assert_eq!(packet.result, "unavailable_no_current_act_boss");
    }

    #[test]
    fn boss_preview_result_distinguishes_segments_from_complete_wins() {
        assert_eq!(
            boss_preview_result_label_v1(true, "search_combat"),
            "complete_win_applied"
        );
        assert_eq!(
            boss_preview_result_label_v1(false, "search_combat_turn_segment"),
            "turn_segment_applied"
        );
    }

    #[test]
    fn probe_diagnosis_classifies_deadline_unresolved_search() {
        let diagnosis = diagnose_combat_lab_probe_v1(
            "unresolved_no_trajectory",
            &[
                "result=Unresolved".to_string(),
                "terminal_wins=0".to_string(),
                "reason=wall-clock deadline hit; unresolved frontier remains".to_string(),
            ],
        );

        assert_eq!(diagnosis.outcome_class, "unresolved");
        assert_eq!(diagnosis.search_reason, "wall_clock_deadline_hit");
        assert_eq!(diagnosis.confidence, "search_digest");
        assert!(diagnosis.signals.contains(&"no_terminal_wins".to_string()));
        assert!(diagnosis.signals.contains(&"budget_limited".to_string()));
    }

    #[test]
    fn probe_diagnosis_does_not_confuse_time_budget_with_hp_loss_gate() {
        let diagnosis = diagnose_combat_lab_probe_v1(
            "unresolved_no_trajectory",
            &[
                "best_complete_candidate terminal=Loss final_hp=0 hp_loss=42 turns=2".to_string(),
                "coverage_status=TimeBudgetLimited".to_string(),
                "terminal_wins=0".to_string(),
            ],
        );

        assert_eq!(diagnosis.search_reason, "time_budget_limited");
        assert!(!diagnosis.signals.contains(&"hp_loss_gate".to_string()));
    }
}
