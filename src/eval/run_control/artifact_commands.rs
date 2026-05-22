use std::path::{Path, PathBuf};

use crate::eval::combat_capture::{
    capture_combat_position_from_run_v1, save_combat_capture_v1, CombatCaptureV1,
};

use super::decision_case::{
    default_run_decision_case_path, save_run_decision_case_v1, RunDecisionCaseV1,
};
use super::outcome::{save_combat_baseline_outcome_v1, CombatBaselineOutcomeV1};
use super::registry::{add_case_to_benchmark_registry, BenchmarkCasePaths};
use super::session::{RunControlCommandOutcome, RunControlSession};

pub(super) fn apply_save_decision_case(
    session: &RunControlSession,
    path: Option<PathBuf>,
) -> Result<RunControlCommandOutcome, String> {
    let path = path.unwrap_or_else(|| default_run_decision_case_path(session));
    let decision_case = RunDecisionCaseV1::from_session(session);
    save_run_decision_case_v1(&path, &decision_case)?;
    Ok(RunControlCommandOutcome::message(format!(
        "saved RunDecisionCaseV1 to {} [label_role={} trainable_as_action_label={} policy_quality_claim={}]",
        path.display(),
        decision_case.label_role,
        decision_case.trainable_as_action_label,
        decision_case.policy_quality_claim
    )))
}

pub(super) fn apply_capture(
    session: &RunControlSession,
    path: PathBuf,
    label: Option<String>,
) -> Result<RunControlCommandOutcome, String> {
    let capture = session.save_current_combat_capture(&path, label)?;
    Ok(RunControlCommandOutcome::message(format!(
        "saved CombatCaptureV1 to {} [{} hp={}, turn={}, enemies={}]",
        path.display(),
        capture.summary.engine_state,
        capture.summary.player_hp,
        capture.summary.turn_count,
        capture.summary.monsters.len()
    )))
}

pub(super) fn apply_capture_case(
    session: &RunControlSession,
    root: PathBuf,
    case_id: String,
    label: Option<String>,
) -> Result<RunControlCommandOutcome, String> {
    let paths = BenchmarkCasePaths::for_case(&root, &case_id);
    let capture = session.save_current_combat_capture(
        &paths.capture_path,
        label.or_else(|| Some(case_id.clone())),
    )?;
    let paths = add_case_to_benchmark_registry(&root, &case_id)?;
    Ok(RunControlCommandOutcome::message(format!(
        "saved CombatCaptureV1 case {case_id} to {} and registered {} [{} hp={}, turn={}, enemies={} trust={:?}]",
        paths.capture_path.display(),
        paths.benchmark_manifest.display(),
        capture.summary.engine_state,
        capture.summary.player_hp,
        capture.summary.turn_count,
        capture.summary.monsters.len(),
        capture.trust_level
    )))
}

pub(super) fn apply_save_baseline(
    session: &RunControlSession,
    path: PathBuf,
    case_id: Option<String>,
) -> Result<RunControlCommandOutcome, String> {
    let baseline = session.save_last_combat_baseline(
        &path,
        case_id.unwrap_or_else(|| inferred_case_id_from_path(&path)),
    )?;
    Ok(render_saved_baseline(
        format!("saved CombatBaselineOutcomeV1 to {}", path.display()),
        &baseline,
    ))
}

pub(super) fn apply_save_baseline_case(
    session: &RunControlSession,
    root: PathBuf,
    case_id: String,
) -> Result<RunControlCommandOutcome, String> {
    let paths = BenchmarkCasePaths::for_case(&root, &case_id);
    let baseline = session.save_last_combat_baseline(&paths.baseline_path, case_id.clone())?;
    let registry_note = if paths.capture_path.exists() {
        let paths = add_case_to_benchmark_registry(&root, &case_id)?;
        format!(" and registered {}", paths.benchmark_manifest.display())
    } else {
        " [benchmark not registered: matching capture is missing]".to_string()
    };
    Ok(render_saved_baseline(
        format!(
            "saved CombatBaselineOutcomeV1 to {}{}",
            paths.baseline_path.display(),
            registry_note
        ),
        &baseline,
    ))
}

pub(super) fn apply_register_benchmark_case(
    root: PathBuf,
    case_id: String,
) -> Result<RunControlCommandOutcome, String> {
    let paths = add_case_to_benchmark_registry(&root, &case_id)?;
    let baseline_status = if paths.baseline_path.exists() {
        paths.baseline_path.display().to_string()
    } else {
        "none".to_string()
    };
    Ok(RunControlCommandOutcome::message(format!(
        "registered benchmark case {case_id} in {} [capture={}, baseline={}]",
        paths.benchmark_manifest.display(),
        paths.capture_path.display(),
        baseline_status
    )))
}

impl RunControlSession {
    pub fn save_current_combat_capture(
        &self,
        path: &Path,
        label: Option<String>,
    ) -> Result<CombatCaptureV1, String> {
        let position = self.current_active_combat_position()?;
        let capture = capture_combat_position_from_run_v1(label, &position, &self.run_state)?;
        save_combat_capture_v1(path, &capture)?;
        Ok(capture)
    }

    pub fn last_combat_baseline(&self) -> Option<&CombatBaselineOutcomeV1> {
        self.combat_outcomes.last()
    }

    pub fn save_last_combat_baseline(
        &self,
        path: &Path,
        case_id: String,
    ) -> Result<CombatBaselineOutcomeV1, String> {
        let mut baseline = self
            .combat_outcomes
            .last()
            .cloned()
            .ok_or_else(|| "no completed combat baseline is available".to_string())?;
        baseline.case_id = case_id;
        save_combat_baseline_outcome_v1(path, &baseline)?;
        Ok(baseline)
    }
}

fn render_saved_baseline(
    prefix: String,
    baseline: &CombatBaselineOutcomeV1,
) -> RunControlCommandOutcome {
    RunControlCommandOutcome::message(format!(
        "{prefix} [case={} terminal={:?} hp_loss={} final_hp={} turns={} potions_used={} cards_played={}]",
        baseline.case_id,
        baseline.terminal,
        baseline.hp_loss,
        baseline.final_hp,
        baseline.turns,
        baseline.potions_used,
        baseline.cards_played
    ))
}

fn inferred_case_id_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.trim().is_empty())
        .unwrap_or("last_combat")
        .trim_end_matches(".baseline")
        .to_string()
}
