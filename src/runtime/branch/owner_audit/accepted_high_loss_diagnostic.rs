use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use sts_simulator::eval::combat_capture::{
    capture_combat_position_from_auto_run_v1, save_combat_capture_v1, CombatCaptureV1,
};
use sts_simulator::eval::run_control::{
    accepted_combat_line_evidence_v1, combat_automation_trajectories_v1,
    combat_search_trace_summaries, AcceptedCombatLineEvidenceV1,
    CombatAutomationTrajectoryRecordV1, CombatAutomationTrajectorySource, CombatSearchTraceSummary,
    RunControlSession, RunControlTraceAnnotationV1,
};
use sts_simulator::sim::combat::CombatPosition;
use sts_simulator::state::core::EngineState;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AcceptedCombatIdentityV1 {
    pub(super) act: u8,
    pub(super) floor: i32,
    pub(super) turn: u32,
    pub(super) enemies: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AcceptedHighLossDiagnosticDraft {
    pub(super) identity: AcceptedCombatIdentityV1,
    pub(super) lane: String,
    pub(super) capture: CombatCaptureV1,
    pub(super) evidence: AcceptedCombatLineEvidenceV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) search: Option<CombatSearchTraceSummary>,
    pub(super) trajectory: CombatAutomationTrajectoryRecordV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) hard_hp_loss_limit: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct WrittenAcceptedHighLossDiagnostic {
    pub(super) capture_path: PathBuf,
    pub(super) evidence_path: PathBuf,
    pub(super) original_hp_loss: i32,
    pub(super) selected_hp_loss: i32,
    pub(super) hp_saved_by_selection: i32,
}

impl WrittenAcceptedHighLossDiagnostic {
    pub(super) fn value(&self) -> Value {
        json!({
            "capture": self.capture_path.display().to_string(),
            "evidence": self.evidence_path.display().to_string(),
            "original_hp_loss": self.original_hp_loss,
            "selected_hp_loss": self.selected_hp_loss,
            "hp_saved_by_selection": self.hp_saved_by_selection,
        })
    }
}

pub(super) fn high_loss_trigger(max_hp: i32, original_hp_loss: i32, selected_hp_loss: i32) -> bool {
    let max_hp = i64::from(max_hp.max(1));
    i64::from(original_hp_loss.max(0)).saturating_mul(4) >= max_hp
        || i64::from(selected_hp_loss.max(0)).saturating_mul(4) >= max_hp
}

pub(super) fn accepted_high_loss_diagnostic(
    capture: CombatCaptureV1,
    lane: &str,
    annotations: &[RunControlTraceAnnotationV1],
    committed: bool,
    hard_hp_loss_limit: Option<u32>,
) -> Option<AcceptedHighLossDiagnosticDraft> {
    if !committed {
        return None;
    }
    let evidence = accepted_combat_line_evidence_v1(annotations)?.clone();
    if !high_loss_trigger(
        capture.summary.player_max_hp,
        evidence.original.hp_loss,
        evidence.selected.hp_loss,
    ) {
        return None;
    }
    let trajectory = combat_automation_trajectories_v1(annotations)
        .find(|trajectory| trajectory.source == CombatAutomationTrajectorySource::SearchCombat)
        .map(CombatAutomationTrajectoryRecordV1::from_ref)?;
    let run = capture.provenance.run_config.as_ref()?;
    let identity = AcceptedCombatIdentityV1 {
        act: run.act_num?,
        floor: run.floor_num?,
        turn: capture.summary.turn_count,
        enemies: capture
            .summary
            .monsters
            .iter()
            .filter(|monster| monster.alive)
            .map(|monster| monster.enemy_id.clone())
            .collect(),
    };
    Some(AcceptedHighLossDiagnosticDraft {
        identity,
        lane: lane.to_string(),
        capture,
        evidence,
        search: combat_search_trace_summaries(annotations).next(),
        trajectory,
        hard_hp_loss_limit,
    })
}

pub(super) fn capture_active_combat(
    session: &RunControlSession,
) -> Result<Option<CombatCaptureV1>, String> {
    let Some(active) = session.active_combat.as_ref() else {
        return Ok(None);
    };
    if !matches!(
        active.engine_state,
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
    ) {
        return Ok(None);
    }
    let position = CombatPosition::new(active.engine_state.clone(), active.combat_state.clone());
    capture_combat_position_from_auto_run_v1(
        Some("accepted high-loss candidate".to_string()),
        &position,
        &session.run_state,
    )
    .map(Some)
}

pub(super) fn extend_unique_diagnostics(
    target: &mut Vec<AcceptedHighLossDiagnosticDraft>,
    incoming: impl IntoIterator<Item = AcceptedHighLossDiagnosticDraft>,
) {
    for diagnostic in incoming {
        if target
            .iter()
            .any(|existing| existing.identity == diagnostic.identity)
        {
            continue;
        }
        target.push(diagnostic);
    }
}

pub(super) fn write_diagnostic_pair(
    dir: &Path,
    seed: u64,
    generation: usize,
    branch_id: usize,
    draft: &AcceptedHighLossDiagnosticDraft,
) -> Result<WrittenAcceptedHighLossDiagnostic, String> {
    fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    let enemy_slug = slug(&draft.identity.enemies.join("_"));
    let stem = format!(
        "seed{seed}_g{generation:02}_b{branch_id:04}_a{}f{}t{}_{}",
        draft.identity.act, draft.identity.floor, draft.identity.turn, enemy_slug
    );
    let capture_path = dir.join(format!("{stem}.capture.json"));
    let evidence_path = dir.join(format!("{stem}.evidence.json"));
    save_combat_capture_v1(&capture_path, &draft.capture)?;
    let payload = json!({
        "schema": "accepted_high_loss_combat_evidence_v1",
        "label_role": "diagnostic_not_teacher_label",
        "identity": &draft.identity,
        "lane": &draft.lane,
        "capture": capture_path.display().to_string(),
        "start_hp": draft.capture.summary.player_hp,
        "max_hp": draft.capture.summary.player_max_hp,
        "hard_hp_loss_limit": draft.hard_hp_loss_limit,
        "original_hp_loss": draft.evidence.original.hp_loss,
        "selected_hp_loss": draft.evidence.selected.hp_loss,
        "hp_saved_by_selection": draft.evidence.hp_saved_by_selection,
        "accepted_line": &draft.evidence,
        "search": &draft.search,
        "trajectory": &draft.trajectory,
    });
    let encoded = serde_json::to_string_pretty(&payload).map_err(|error| error.to_string())?;
    fs::write(&evidence_path, encoded).map_err(|error| error.to_string())?;
    Ok(WrittenAcceptedHighLossDiagnostic {
        capture_path,
        evidence_path,
        original_hp_loss: draft.evidence.original.hp_loss,
        selected_hp_loss: draft.evidence.selected.hp_loss,
        hp_saved_by_selection: draft.evidence.hp_saved_by_selection,
    })
}

fn slug(raw: &str) -> String {
    let mut out = String::new();
    let mut last_sep = false;
    for ch in raw.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_sep = false;
        } else if !last_sep {
            out.push('_');
            last_sep = true;
        }
    }
    let out = out.trim_matches('_');
    if out.is_empty() {
        "combat".to_string()
    } else {
        out.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::ai::combat_search_v2::SearchTerminalLabel;
    use sts_simulator::eval::combat_capture::capture_combat_position_from_auto_run_v1;
    use sts_simulator::eval::run_control::{
        AcceptedCombatLineEvidenceV1, CombatAutomationActionV1, CombatAutomationTrajectoryRecordV1,
        CombatAutomationTrajectorySource, CombatSearchTerminalLineSummary,
    };
    use sts_simulator::sim::combat::CombatPosition;
    use sts_simulator::state::core::{ClientInput, EngineState};

    fn terminal_win(final_hp: i32, hp_loss: i32) -> CombatSearchTerminalLineSummary {
        CombatSearchTerminalLineSummary {
            terminal: SearchTerminalLabel::Win,
            final_hp,
            hp_loss,
            turns: 3,
            cards_played: 6,
            potions_used: 0,
            potions_discarded: 0,
            action_count: 9,
        }
    }

    fn capture() -> sts_simulator::eval::combat_capture::CombatCaptureV1 {
        let mut run = sts_simulator::state::run::RunState::new(7, 0, false, "IRONCLAD");
        run.act_num = 2;
        run.floor_num = 21;
        let mut combat = sts_simulator::test_support::blank_test_combat();
        let mut monster = sts_simulator::test_support::test_monster(
            sts_simulator::content::monsters::EnemyId::SnakePlant,
        );
        monster.set_planned_move_id(1);
        monster.set_planned_visible_spec(Some(
            sts_simulator::runtime::monster_move::MonsterMoveSpec::Unknown,
        ));
        combat.entities.monsters = vec![monster];
        capture_combat_position_from_auto_run_v1(
            Some("accepted high loss".to_string()),
            &CombatPosition::new(EngineState::CombatPlayerTurn, combat),
            &run,
        )
        .expect("test combat should capture")
    }

    fn annotations() -> Vec<sts_simulator::eval::run_control::RunControlTraceAnnotationV1> {
        vec![
            AcceptedCombatLineEvidenceV1::new(terminal_win(20, 24), terminal_win(20, 24), None)
                .into_annotation(),
            CombatAutomationTrajectoryRecordV1::new(
                CombatAutomationTrajectorySource::SearchCombat,
                vec![CombatAutomationActionV1 {
                    step_index: 0,
                    action_key: "combat/end_turn".to_string(),
                    input: ClientInput::EndTurn,
                    drawn_cards: Vec::new(),
                    combat_after: None,
                }],
            )
            .into_annotation(),
        ]
    }

    #[test]
    fn high_loss_trigger_checks_original_and_selected_lines() {
        assert!(high_loss_trigger(74, 35, 15));
        assert!(high_loss_trigger(74, 10, 24));
        assert!(!high_loss_trigger(74, 15, 18));
    }

    #[test]
    fn rejected_lane_never_produces_accepted_high_loss_diagnostic() {
        assert!(accepted_high_loss_diagnostic(
            capture(),
            "primary",
            &annotations(),
            false,
            Some(26)
        )
        .is_none());
    }

    #[test]
    fn committed_high_loss_lane_retains_capture_and_trajectory() {
        let draft =
            accepted_high_loss_diagnostic(capture(), "primary", &annotations(), true, Some(26))
                .expect("committed high-loss win should be retained");

        assert_eq!(draft.identity.act, 2);
        assert_eq!(draft.identity.floor, 21);
        assert_eq!(draft.evidence.selected.hp_loss, 24);
        assert_eq!(draft.trajectory.action_count, 1);
        assert_eq!(draft.hard_hp_loss_limit, Some(26));
    }

    #[test]
    fn extend_unique_diagnostics_deduplicates_combat_identity() {
        let draft =
            accepted_high_loss_diagnostic(capture(), "primary", &annotations(), true, Some(26))
                .unwrap();
        let mut diagnostics = Vec::new();

        extend_unique_diagnostics(&mut diagnostics, vec![draft.clone(), draft]);

        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn write_diagnostic_pair_emits_replayable_capture_and_evidence() {
        let draft =
            accepted_high_loss_diagnostic(capture(), "primary", &annotations(), true, Some(26))
                .unwrap();
        let root = std::env::temp_dir().join("accepted_high_loss_diagnostic_pair");
        let _ = std::fs::remove_dir_all(&root);

        let written = write_diagnostic_pair(&root, 7, 22, 22, &draft).unwrap();

        let replay =
            sts_simulator::eval::combat_capture::load_combat_capture_v1(&written.capture_path)
                .unwrap();
        let evidence: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&written.evidence_path).unwrap())
                .unwrap();
        assert_eq!(replay.summary.player_max_hp, 80);
        assert_eq!(evidence["original_hp_loss"], 24);
        assert_eq!(evidence["selected_hp_loss"], 24);
        assert_eq!(evidence["label_role"], "diagnostic_not_teacher_label");

        let _ = std::fs::remove_dir_all(root);
    }
}
