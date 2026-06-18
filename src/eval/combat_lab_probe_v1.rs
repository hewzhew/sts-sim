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
                return CombatLabProbePacketV1 {
                    kind: "current_act_boss_preview".to_string(),
                    source,
                    boss: Some(format!("{boss:?}")),
                    boundary,
                    result: "unavailable_combat_start_failed".to_string(),
                    search_digest: vec![format!("combat_start_failed={err}")],
                    ..CombatLabProbePacketV1::default()
                }
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
                CombatLabProbePacketV1 {
                    kind: "current_act_boss_preview".to_string(),
                    source,
                    boss: Some(format!("{boss:?}")),
                    boundary,
                    result: boss_preview_result_label_v1(
                        preview.active_combat.is_none(),
                        &record.source,
                    )
                    .to_string(),
                    hp_loss: Some(hp_loss),
                    final_hp: Some(preview.run_state.current_hp),
                    max_hp: Some(preview.run_state.max_hp),
                    actions: Some(record.action_count),
                    search_digest: search_message_digest_v1(&outcome.message),
                }
            } else {
                CombatLabProbePacketV1 {
                    kind: "current_act_boss_preview".to_string(),
                    source,
                    boss: Some(format!("{boss:?}")),
                    boundary,
                    result: "unresolved_no_trajectory".to_string(),
                    hp_loss: Some(hp_loss),
                    final_hp: Some(preview.run_state.current_hp),
                    max_hp: Some(preview.run_state.max_hp),
                    search_digest: search_message_digest_v1(&outcome.message),
                    ..CombatLabProbePacketV1::default()
                }
            }
        }
        Err(err) => CombatLabProbePacketV1 {
            kind: "current_act_boss_preview".to_string(),
            source,
            boss: Some(format!("{boss:?}")),
            boundary,
            result: "error".to_string(),
            search_digest: vec![format!("error={err}")],
            ..CombatLabProbePacketV1::default()
        },
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

fn trajectory_signature_v1(
    record: &CombatAutomationTrajectoryRecordV1,
) -> (String, usize, String, String) {
    (
        record.source.clone(),
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
}
