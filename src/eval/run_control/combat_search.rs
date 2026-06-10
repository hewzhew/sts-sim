use crate::ai::combat_search_v2::{
    filter_combat_search_legal_actions, plan_combat_turn_segment_v1, run_combat_search_v2,
    CombatSearchV2ActionTrace, CombatSearchV2Config, CombatSearchV2Report,
    CombatSearchV2TurnSegmentReport, SearchTerminalLabel,
};
use crate::sim::combat::{
    combat_terminal, CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal,
    EngineCombatStepper,
};
use crate::state::core::{EngineState, RunResult};

use super::commands::{
    RunControlCombatSegmentMode, RunControlHpLossLimit, RunControlSearchCombatOptions,
    RunControlSearchEvidenceTarget,
};
use super::registry::BenchmarkCasePaths;
use super::search_evidence::{save_combat_search_evidence_v1, CombatSearchEvidenceContextV1};
use super::session::{RunControlCommandOutcome, RunControlSession};
use super::trace_annotation::{CombatAutomationActionV1, RunControlTraceAnnotationV1};
use super::transition_report::{
    action_result_from_transition, render_action_result, ActionResult, ActionResultChange,
    CardSnapshot, RunApplyStatus, RunVisibleSnapshot, TransitionAction,
};
use super::view_model::client_input_hint;

pub(super) fn apply_search_combat(
    session: &mut RunControlSession,
    options: RunControlSearchCombatOptions,
) -> Result<RunControlCommandOutcome, String> {
    let options = high_stakes_search_options(session, options);
    let start = session.current_active_combat_position()?;
    let config = search_config(session, options.clone());
    let report = run_combat_search_v2(&start.engine, &start.combat, config.clone());
    let saved_evidence =
        save_search_evidence_if_requested(session, options.evidence.as_ref(), &report)?;
    if search_report_has_invalid_card_identity(&report) {
        let mut outcome = RunControlCommandOutcome::message(format!(
            "{}{}\n\n{}",
            render_search_rejection(&report, "invalid_card_identity", None),
            render_saved_evidence_note(saved_evidence.as_deref()),
            super::render::render_run_control_state(session)
        ));
        outcome.search_evidence_path = saved_evidence;
        return Ok(outcome);
    }
    let Some(trajectory) = report
        .best_complete_trajectory
        .as_ref()
        .filter(|trajectory| trajectory.terminal == SearchTerminalLabel::Win)
    else {
        if let Some(outcome) = try_apply_turn_segment_after_rejection(
            session,
            &start,
            &config,
            &options,
            &report,
            saved_evidence.as_deref(),
            "no_complete_winning_candidate",
        )? {
            return Ok(outcome);
        }
        let mut outcome = RunControlCommandOutcome::message(format!(
            "{}{}\n\n{}",
            render_search_rejection(&report, "no_complete_winning_candidate", None),
            render_saved_evidence_note(saved_evidence.as_deref()),
            super::render::render_run_control_state(session)
        ));
        outcome.search_evidence_path = saved_evidence;
        return Ok(outcome);
    };
    if let Some(max_hp_loss) = effective_hp_loss_limit(session, &options) {
        if trajectory.hp_loss > max_hp_loss as i32 {
            if let Some(outcome) = try_apply_turn_segment_after_rejection(
                session,
                &start,
                &config,
                &options,
                &report,
                saved_evidence.as_deref(),
                "complete_winning_candidate_exceeds_hp_loss_limit",
            )? {
                return Ok(outcome);
            }
            let mut outcome = RunControlCommandOutcome::message(format!(
                "{}{}\n\n{}",
                render_search_rejection(
                    &report,
                    "complete_winning_candidate_exceeds_hp_loss_limit",
                    Some(format!(
                        "candidate_hp_loss={} max_hp_loss={max_hp_loss}",
                        trajectory.hp_loss
                    )),
                ),
                render_saved_evidence_note(saved_evidence.as_deref()),
                super::render::render_run_control_state(session)
            ));
            outcome.search_evidence_path = saved_evidence;
            return Ok(outcome);
        }
    }

    verify_trajectory_replays_to_win(&start, &trajectory.actions, &config)?;

    let before_snapshot = RunVisibleSnapshot::capture(session);
    let applied = trajectory.actions.clone();
    let mut automation_actions = Vec::new();
    session.mark_current_combat_search_resolved();
    for action in &applied {
        let outcome = session.apply_input(action.input.clone())?;
        automation_actions.push(CombatAutomationActionV1 {
            step_index: action.step_index,
            action_key: action.action_key.clone(),
            input: action.input.clone(),
            drawn_cards: drawn_cards_from_action_result(outcome.action_result.as_ref()),
        });
    }
    let after_snapshot = RunVisibleSnapshot::capture(session);
    let status = current_run_apply_status(session);
    let mut transition_label = format!("search-combat applied {} actions", applied.len());
    if let Some(path) = saved_evidence.as_ref() {
        transition_label.push_str(&format!(" saved_search={}", path.display()));
    }
    let action_result = action_result_from_transition(
        TransitionAction {
            label: transition_label,
        },
        &before_snapshot,
        &after_snapshot,
        status,
    );
    let message = format!(
        "{}{}\n{}\n{}",
        render_search_application(&report, &applied),
        render_saved_evidence_note(saved_evidence.as_deref()),
        render_action_result(&action_result),
        super::render::render_run_control_state(session)
    );
    let mut outcome =
        RunControlCommandOutcome::action(message, action_result).with_trace_annotations(vec![
            combat_automation_trace_annotation("search_combat", automation_actions),
        ]);
    outcome.search_evidence_path = saved_evidence;
    Ok(outcome)
}

fn try_apply_turn_segment_after_rejection(
    session: &mut RunControlSession,
    start: &CombatPosition,
    config: &CombatSearchV2Config,
    options: &RunControlSearchCombatOptions,
    search_report: &CombatSearchV2Report,
    saved_evidence: Option<&std::path::Path>,
    rejection_result: &'static str,
) -> Result<Option<RunControlCommandOutcome>, String> {
    if !segment_mode_allows_turn_segment(options.segment_mode, start) {
        return Ok(None);
    }

    let segment_report = plan_combat_turn_segment_v1(&start.engine, &start.combat, config);
    let Some(trajectory) = segment_report.selected.as_ref() else {
        return Ok(None);
    };
    verify_segment_trajectory_replays(start, &trajectory.actions, config)?;

    let before_snapshot = RunVisibleSnapshot::capture(session);
    let applied = trajectory.actions.clone();
    let mut automation_actions = Vec::new();
    session.mark_current_combat_search_resolved();
    for action in &applied {
        let outcome = session.apply_input(action.input.clone())?;
        automation_actions.push(CombatAutomationActionV1 {
            step_index: action.step_index,
            action_key: action.action_key.clone(),
            input: action.input.clone(),
            drawn_cards: drawn_cards_from_action_result(outcome.action_result.as_ref()),
        });
    }
    let after_snapshot = RunVisibleSnapshot::capture(session);
    let status = current_run_apply_status(session);
    let mut transition_label = format!(
        "search-combat segment applied {} actions (partial turn; not terminal claim)",
        applied.len()
    );
    if let Some(path) = saved_evidence.as_ref() {
        transition_label.push_str(&format!(" saved_search={}", path.display()));
    }
    let action_result = action_result_from_transition(
        TransitionAction {
            label: transition_label,
        },
        &before_snapshot,
        &after_snapshot,
        status,
    );
    let message = format!(
        "{}{}\n{}\n{}",
        render_segment_application(search_report, &segment_report, rejection_result),
        render_saved_evidence_note(saved_evidence),
        render_action_result(&action_result),
        super::render::render_run_control_state(session)
    );
    let mut outcome =
        RunControlCommandOutcome::action(message, action_result).with_trace_annotations(vec![
            combat_automation_trace_annotation("search_combat_turn_segment", automation_actions),
        ]);
    outcome.search_evidence_path = saved_evidence.map(|path| path.to_path_buf());
    Ok(Some(outcome))
}

fn segment_mode_allows_turn_segment(
    mode: Option<RunControlCombatSegmentMode>,
    start: &CombatPosition,
) -> bool {
    match mode {
        Some(RunControlCombatSegmentMode::TurnBoundary) => true,
        Some(RunControlCombatSegmentMode::NonBossTurnBoundary) => !start.combat.meta.is_boss_fight,
        None => false,
    }
}

fn combat_automation_trace_annotation(
    source: impl Into<String>,
    actions: Vec<CombatAutomationActionV1>,
) -> RunControlTraceAnnotationV1 {
    RunControlTraceAnnotationV1::CombatAutomationTrajectory {
        source: source.into(),
        action_count: actions.len(),
        actions,
        label_role: "simulator_generated_not_teacher_label".to_string(),
    }
}

fn drawn_cards_from_action_result(action_result: Option<&ActionResult>) -> Vec<CardSnapshot> {
    action_result
        .into_iter()
        .flat_map(|result| result.changes.iter())
        .filter_map(|change| match change {
            ActionResultChange::CombatCardDrawn { card } => Some(card.clone()),
            _ => None,
        })
        .collect()
}

fn save_search_evidence_if_requested(
    session: &RunControlSession,
    target: Option<&RunControlSearchEvidenceTarget>,
    report: &CombatSearchV2Report,
) -> Result<Option<std::path::PathBuf>, String> {
    let Some(target) = target else {
        return Ok(None);
    };
    let (path, capture_case_id, capture_root, capture_path) = match target {
        RunControlSearchEvidenceTarget::Path(path) => {
            (next_available_evidence_path(path), None, None, None)
        }
        RunControlSearchEvidenceTarget::LastCaptureCase => {
            let case = session.active_capture_case().ok_or_else(|| {
                "search evidence save=case requires the current combat to have a matching cap <case_id>"
                    .to_string()
            })?;
            let paths = BenchmarkCasePaths::for_case(&case.root, &case.case_id);
            let base_path = case.root.join("search_evidence").join(format!(
                "{}.step{}.search.json",
                case.case_id, session.decision_step
            ));
            (
                next_available_evidence_path(&base_path),
                Some(case.case_id.clone()),
                Some(case.root.display().to_string()),
                Some(paths.capture_path.display().to_string()),
            )
        }
    };
    save_combat_search_evidence_v1(
        &path,
        CombatSearchEvidenceContextV1 {
            source_kind: "run_control_search_combat",
            decision_step: session.decision_step,
            capture_case_id,
            capture_root,
            capture_path,
        },
        report,
    )?;
    Ok(Some(path))
}

fn effective_hp_loss_limit(
    session: &RunControlSession,
    options: &RunControlSearchCombatOptions,
) -> Option<u32> {
    match options.max_hp_loss {
        Some(RunControlHpLossLimit::Limit(limit)) => Some(limit),
        Some(RunControlHpLossLimit::Unlimited) => None,
        None => session.search_max_hp_loss,
    }
}

pub(in crate::eval::run_control) fn high_stakes_search_options(
    session: &RunControlSession,
    mut options: RunControlSearchCombatOptions,
) -> RunControlSearchCombatOptions {
    let plan = super::combat_auto_policy::combat_auto_search_plan(session, &options);
    if options.potion_policy.is_none() && session.search_potion_policy.is_none() {
        options.potion_policy = plan.primary_potion_policy;
    }
    if options.max_potions_used.is_none() && session.search_max_potions_used.is_none() {
        options.max_potions_used = plan.primary_max_potions_used;
    }
    options
}

fn search_report_has_invalid_card_identity(report: &CombatSearchV2Report) -> bool {
    report
        .diagnostics
        .card_identity
        .states_with_uuid_card_id_conflict
        > 0
}

fn next_available_evidence_path(path: &std::path::Path) -> std::path::PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new(""));
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("search_evidence");
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("json");
    for idx in 2..10_000 {
        let candidate = parent.join(format!("{stem}.{idx}.{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{stem}.overflow.{ext}"))
}

fn render_saved_evidence_note(path: Option<&std::path::Path>) -> String {
    path.map(|path| format!("\nSearch evidence saved: {}", path.display()))
        .unwrap_or_default()
}

fn search_config(
    session: &RunControlSession,
    options: RunControlSearchCombatOptions,
) -> CombatSearchV2Config {
    let defaults = CombatSearchV2Config::default();
    let stop_on_win_hp_loss_at_most = effective_hp_loss_limit(session, &options);
    CombatSearchV2Config {
        max_nodes: options
            .max_nodes
            .or(session.search_max_nodes)
            .unwrap_or(defaults.max_nodes),
        max_actions_per_line: options
            .max_actions_per_line
            .unwrap_or(defaults.max_actions_per_line),
        max_engine_steps_per_action: options
            .max_engine_steps_per_action
            .unwrap_or(defaults.max_engine_steps_per_action),
        wall_time: options
            .wall_ms
            .or(session.search_wall_ms)
            .map(std::time::Duration::from_millis),
        stop_on_win_hp_loss_at_most,
        input_label: Some(format!(
            "run_play_driver:search_combat:step{}",
            session.decision_step
        )),
        potion_policy: options
            .potion_policy
            .or(session.search_potion_policy)
            .unwrap_or(defaults.potion_policy),
        max_potions_used: options
            .max_potions_used
            .or(session.search_max_potions_used)
            .or(defaults.max_potions_used),
        rollout_policy: options.rollout_policy.unwrap_or(defaults.rollout_policy),
        rollout_max_evaluations: options
            .rollout_max_evaluations
            .unwrap_or(defaults.rollout_max_evaluations),
        rollout_max_actions: options
            .rollout_max_actions
            .unwrap_or(defaults.rollout_max_actions),
        rollout_beam_width: options
            .rollout_beam_width
            .unwrap_or(defaults.rollout_beam_width),
        turn_plan_policy: options
            .turn_plan_policy
            .unwrap_or(defaults.turn_plan_policy),
        frontier_policy: options.frontier_policy.unwrap_or(defaults.frontier_policy),
    }
}

fn verify_trajectory_replays_to_win(
    start: &CombatPosition,
    actions: &[CombatSearchV2ActionTrace],
    config: &CombatSearchV2Config,
) -> Result<(), String> {
    let stepper = EngineCombatStepper;
    let mut position = start.clone();
    for action in actions {
        let choices = filter_combat_search_legal_actions(
            stepper.legal_action_choices(&position),
            config.potion_policy,
            &position.combat,
        );
        let Some(choice) = choices
            .iter()
            .find(|choice| choice.input == action.input && choice.action_key == action.action_key)
        else {
            return Err(format!(
                "search-combat dry-run drift at step {}: expected {} ({})",
                action.step_index,
                action.action_key,
                client_input_hint(&action.input)
            ));
        };
        let step = stepper.apply_to_stable(
            &position,
            choice.input.clone(),
            CombatStepLimits {
                max_engine_steps: config.max_engine_steps_per_action,
                deadline: None,
            },
        );
        if step.truncated {
            return Err(format!(
                "search-combat dry-run truncated at step {} after {} engine steps",
                action.step_index, step.engine_steps
            ));
        }
        position = step.position;
    }
    match combat_terminal(&position.engine, &position.combat) {
        CombatTerminal::Win => Ok(()),
        other => Err(format!(
            "search-combat dry-run did not finish as win; terminal={other:?}"
        )),
    }
}

fn verify_segment_trajectory_replays(
    start: &CombatPosition,
    actions: &[CombatSearchV2ActionTrace],
    config: &CombatSearchV2Config,
) -> Result<(), String> {
    if actions.is_empty() {
        return Err("search-combat segment dry-run refused empty action list".to_string());
    }
    let stepper = EngineCombatStepper;
    let mut position = start.clone();
    for action in actions {
        let choices = filter_combat_search_legal_actions(
            stepper.legal_action_choices(&position),
            config.potion_policy,
            &position.combat,
        );
        let Some(choice) = choices
            .iter()
            .find(|choice| choice.input == action.input && choice.action_key == action.action_key)
        else {
            return Err(format!(
                "search-combat segment dry-run drift at step {}: expected {} ({})",
                action.step_index,
                action.action_key,
                client_input_hint(&action.input)
            ));
        };
        let step = stepper.apply_to_stable(
            &position,
            choice.input.clone(),
            CombatStepLimits {
                max_engine_steps: config.max_engine_steps_per_action,
                deadline: None,
            },
        );
        if step.truncated {
            return Err(format!(
                "search-combat segment dry-run truncated at step {} after {} engine steps",
                action.step_index, step.engine_steps
            ));
        }
        position = step.position;
    }
    match combat_terminal(&position.engine, &position.combat) {
        CombatTerminal::Loss => Err("search-combat segment dry-run ended in loss".to_string()),
        CombatTerminal::Win | CombatTerminal::Unresolved => Ok(()),
    }
}

fn render_search_rejection(
    report: &CombatSearchV2Report,
    result: &'static str,
    detail: Option<String>,
) -> String {
    let mut lines = vec![
        "Search combat did not modify state.".to_string(),
        format!("  result={result}"),
    ];
    if let Some(detail) = detail {
        lines.push(format!("  detail={detail}"));
    }
    if let Some(candidate) = report.best_complete_trajectory.as_ref() {
        lines.push(format!(
            "  best_complete_candidate terminal={:?} final_hp={} hp_loss={} turns={} cards_played={} potions_used={} actions={}",
            candidate.terminal,
            candidate.final_hp,
            candidate.hp_loss,
            candidate.turns,
            candidate.cards_played,
            candidate.potions_used,
            candidate.actions.len()
        ));
    } else {
        lines.push("  best_complete_candidate=none".to_string());
    }
    lines.extend([
        format!("  coverage_status={:?}", report.outcome.coverage_status),
        render_search_policy_summary(report),
        render_search_diagnostics_summary(report),
        render_policy_evidence_summary(report),
        format!(
            "  complete_trajectory_found={}",
            report.outcome.complete_trajectory_found
        ),
        format!("  terminal_wins={}", report.stats.terminal_wins),
        format!("  nodes_expanded={}", report.stats.nodes_expanded),
        format!("  nodes_generated={}", report.stats.nodes_generated),
        format!(
            "  rollouts={} rollout_wins={} rollout_skips={}",
            report.rollout.evaluations, report.rollout.terminal_wins, report.rollout.budget_skips
        ),
        format!("  reliability={}", report.evidence_reliability.reliability),
        format!("  coverage_reason={}", report.outcome.coverage_reason),
    ]);
    lines.join("\n")
}

fn render_search_application(
    report: &CombatSearchV2Report,
    actions: &[CombatSearchV2ActionTrace],
) -> String {
    let trajectory = report
        .best_complete_trajectory
        .as_ref()
        .expect("caller only renders after selecting a complete trajectory");
    let mut lines = vec![
        "Search combat applied complete winning candidate.".to_string(),
        format!(
            "  coverage_status={:?} reliability={}",
            report.outcome.coverage_status, report.evidence_reliability.reliability
        ),
        render_search_policy_summary(report),
        render_search_diagnostics_summary(report),
        render_policy_evidence_summary(report),
        format!("  coverage_reason={}", report.outcome.coverage_reason),
        format!("  terminal={:?}", trajectory.terminal),
        format!(
            "  final_hp={} hp_loss={} turns={} cards_played={} potions_used={}",
            trajectory.final_hp,
            trajectory.hp_loss,
            trajectory.turns,
            trajectory.cards_played,
            trajectory.potions_used
        ),
        format!(
            "  nodes_expanded={} nodes_generated={} nodes_to_first_win={:?}",
            report.stats.nodes_expanded,
            report.stats.nodes_generated,
            report.stats.nodes_to_first_win
        ),
        format!(
            "  rollout_policy={} rollouts={} rollout_wins={} rollout_skips={}",
            report.rollout.policy,
            report.rollout.evaluations,
            report.rollout.terminal_wins,
            report.rollout.budget_skips
        ),
        format!(
            "  action_count={} potion_policy={}",
            actions.len(),
            report.search_policy.potion_policy
        ),
    ];
    for action in actions.iter().take(12) {
        lines.push(format!(
            "    {} | {} | {}",
            action.step_index,
            client_input_hint(&action.input),
            action.action_key
        ));
    }
    if actions.len() > 12 {
        lines.push(format!("    ... {} more actions", actions.len() - 12));
    }
    lines.join("\n")
}

fn render_segment_application(
    search_report: &CombatSearchV2Report,
    segment_report: &CombatSearchV2TurnSegmentReport,
    rejection_result: &'static str,
) -> String {
    let trajectory = segment_report
        .selected
        .as_ref()
        .expect("caller only renders after selecting a segment");
    let mut lines = vec![
        "Search combat applied partial turn segment.".to_string(),
        format!("  behavior_label={}", segment_report.behavior_label),
        format!("  source={}", segment_report.source),
        format!("  original_search_result={rejection_result}"),
        format!(
            "  segment_bucket={} stop_reason={} candidate_count={} nodes_expanded={} nodes_generated={}",
            segment_report.selected_bucket.unwrap_or("unknown"),
            segment_report.selected_stop_reason.unwrap_or("unknown"),
            segment_report.candidate_count,
            segment_report.nodes_expanded,
            segment_report.nodes_generated
        ),
        format!(
            "  segment_terminal={:?} final_hp={} hp_loss={} turns={} cards_played={} potions_used={}",
            trajectory.terminal,
            trajectory.final_hp,
            trajectory.hp_loss,
            trajectory.turns,
            trajectory.cards_played,
            trajectory.potions_used
        ),
        format!(
            "  search_coverage={:?} reliability={}",
            search_report.outcome.coverage_status, search_report.evidence_reliability.reliability
        ),
        render_search_policy_summary(search_report),
        render_policy_evidence_summary(search_report),
        "  terminal_claim=none; this is an exact applied prefix, not a complete-win proof"
            .to_string(),
        format!("  action_count={}", trajectory.actions.len()),
    ];
    for action in trajectory.actions.iter().take(12) {
        lines.push(format!(
            "    {} | {} | {}",
            action.step_index,
            client_input_hint(&action.input),
            action.action_key
        ));
    }
    if trajectory.actions.len() > 12 {
        lines.push(format!(
            "    ... {} more actions",
            trajectory.actions.len() - 12
        ));
    }
    lines.join("\n")
}

fn render_search_policy_summary(report: &CombatSearchV2Report) -> String {
    format!(
        "  frontier_policy={} turn_plan_policy={} rollout_policy={}",
        report.search_policy.frontier_policy,
        report.search_policy.turn_plan_policy,
        report.rollout.policy
    )
}

fn render_search_diagnostics_summary(report: &CombatSearchV2Report) -> String {
    format!(
        "  search_diagnostics=frontier_remaining={} unresolved_leaf={} max_actions_cut={} engine_step_cut={} potion_budget_cut={} turn_plan_observed={} turn_plan_seeded={} pending_states={} pending_high_fanout={} rollout_budget_skips={}",
        report.frontier.remaining_states,
        report.frontier.unresolved_leaf_count,
        report.frontier.max_actions_cut_count,
        report.frontier.engine_step_limit_count,
        report.frontier.potion_budget_cut_count,
        report.diagnostics.turn_plan.root_states_observed,
        report.diagnostics.turn_plan.frontier_seeded_nodes,
        report.diagnostics.pending_choice.pending_choice_states,
        report.diagnostics.pending_choice.high_fanout_states,
        report.rollout.budget_skips,
    )
}

fn render_policy_evidence_summary(report: &CombatSearchV2Report) -> String {
    format!("  {}", report.policy_evidence.machine_summary())
}

fn current_run_apply_status(session: &RunControlSession) -> RunApplyStatus {
    match session.engine_state {
        EngineState::GameOver(RunResult::Victory) => RunApplyStatus::Victory,
        EngineState::GameOver(RunResult::Defeat) => RunApplyStatus::Defeat,
        _ => RunApplyStatus::Running,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::{
        combat_automation_trace_annotation, effective_hp_loss_limit, high_stakes_search_options,
        next_available_evidence_path, search_config, segment_mode_allows_turn_segment,
    };
    use crate::ai::combat_search_v2::CombatSearchV2PotionPolicy;
    use crate::eval::run_control::trace_annotation::{
        CombatAutomationActionV1, RunControlTraceAnnotationV1,
    };
    use crate::eval::run_control::{
        RunControlConfig, RunControlHpLossLimit, RunControlSearchCombatOptions, RunControlSession,
    };
    use crate::state::core::{
        ActiveCombat, ClientInput, CombatContext, EngineState, RoomCombatContext,
    };
    use crate::state::map::node::RoomType;

    fn session_with_active_combat(
        mut combat: crate::runtime::combat::CombatState,
    ) -> RunControlSession {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            {
                combat.entities.monsters = vec![crate::test_support::test_monster(
                    crate::content::monsters::EnemyId::JawWorm,
                )];
                combat
            },
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        session
    }

    fn session_with_combat_flags(is_boss_fight: bool, is_elite_fight: bool) -> RunControlSession {
        let mut combat = crate::test_support::blank_test_combat();
        combat.meta.is_boss_fight = is_boss_fight;
        combat.meta.is_elite_fight = is_elite_fight;
        session_with_active_combat(combat)
    }

    fn options_with_hp_loss(max_hp_loss: RunControlHpLossLimit) -> RunControlSearchCombatOptions {
        RunControlSearchCombatOptions {
            max_hp_loss: Some(max_hp_loss),
            ..RunControlSearchCombatOptions::default()
        }
    }

    fn options_with_potion_budget(
        potion_policy: CombatSearchV2PotionPolicy,
        max_potions_used: u32,
    ) -> RunControlSearchCombatOptions {
        RunControlSearchCombatOptions {
            potion_policy: Some(potion_policy),
            max_potions_used: Some(max_potions_used),
            ..RunControlSearchCombatOptions::default()
        }
    }

    fn assert_potion_budget(
        options: RunControlSearchCombatOptions,
        expected_policy: Option<CombatSearchV2PotionPolicy>,
        expected_max_used: Option<u32>,
    ) {
        assert_eq!(options.potion_policy, expected_policy);
        assert_eq!(options.max_potions_used, expected_max_used);
    }

    #[test]
    fn search_evidence_path_does_not_overwrite_existing_file() {
        let root = std::env::temp_dir().join(format!(
            "sts_search_evidence_path_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp dir should be created");
        let base = root.join("case.step1.search.json");
        fs::write(&base, "{}").expect("base file should be written");

        let next = next_available_evidence_path(&base);

        assert_ne!(next, base);
        assert_eq!(
            next.file_name().and_then(|name| name.to_str()),
            Some("case.step1.search.2.json")
        );
        assert!(!next.exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn combat_automation_trace_annotation_preserves_action_inputs() {
        let annotation = combat_automation_trace_annotation(
            "unit_test",
            vec![CombatAutomationActionV1 {
                step_index: 7,
                action_key: "combat/end_turn".to_string(),
                input: ClientInput::EndTurn,
                drawn_cards: Vec::new(),
            }],
        );

        let RunControlTraceAnnotationV1::CombatAutomationTrajectory {
            source,
            action_count,
            actions,
            label_role,
        } = annotation
        else {
            panic!("expected combat automation trajectory annotation")
        };
        assert_eq!(source, "unit_test");
        assert_eq!(action_count, 1);
        assert_eq!(actions[0].step_index, 7);
        assert_eq!(actions[0].action_key, "combat/end_turn");
        assert_eq!(actions[0].input, ClientInput::EndTurn);
        assert_eq!(label_role, "simulator_generated_not_teacher_label");
    }

    #[test]
    fn hp_loss_limit_uses_session_default_and_command_override() {
        let session = RunControlSession::new(RunControlConfig {
            search_max_hp_loss: Some(12),
            ..RunControlConfig::default()
        });

        assert_eq!(
            effective_hp_loss_limit(&session, &RunControlSearchCombatOptions::default()),
            Some(12)
        );
        assert_eq!(
            search_config(&session, RunControlSearchCombatOptions::default())
                .stop_on_win_hp_loss_at_most,
            Some(12)
        );
        assert_eq!(
            effective_hp_loss_limit(
                &session,
                &options_with_hp_loss(RunControlHpLossLimit::Limit(4))
            ),
            Some(4)
        );
        assert_eq!(
            search_config(
                &session,
                options_with_hp_loss(RunControlHpLossLimit::Limit(4))
            )
            .stop_on_win_hp_loss_at_most,
            Some(4)
        );
        assert_eq!(
            effective_hp_loss_limit(
                &session,
                &options_with_hp_loss(RunControlHpLossLimit::Unlimited)
            ),
            None
        );
        assert_eq!(
            search_config(
                &session,
                options_with_hp_loss(RunControlHpLossLimit::Unlimited)
            )
            .stop_on_win_hp_loss_at_most,
            None
        );
    }

    #[test]
    fn search_config_uses_session_budget_defaults_and_command_override() {
        let session = RunControlSession::new(RunControlConfig {
            search_max_nodes: Some(1234),
            search_wall_ms: Some(5678),
            ..RunControlConfig::default()
        });

        let config = search_config(&session, RunControlSearchCombatOptions::default());
        assert_eq!(config.max_nodes, 1234);
        assert_eq!(config.wall_time, Some(Duration::from_millis(5678)));

        let config = search_config(
            &session,
            RunControlSearchCombatOptions {
                max_nodes: Some(90),
                wall_ms: Some(12),
                ..RunControlSearchCombatOptions::default()
            },
        );
        assert_eq!(config.max_nodes, 90);
        assert_eq!(config.wall_time, Some(Duration::from_millis(12)));
    }

    #[test]
    fn search_config_uses_session_potion_defaults_and_command_override() {
        let session = RunControlSession::new(RunControlConfig {
            search_potion_policy: Some(CombatSearchV2PotionPolicy::SemanticBudgeted),
            search_max_potions_used: Some(2),
            ..RunControlConfig::default()
        });

        let config = search_config(&session, RunControlSearchCombatOptions::default());
        assert_eq!(
            config.potion_policy,
            CombatSearchV2PotionPolicy::SemanticBudgeted
        );
        assert_eq!(config.max_potions_used, Some(2));

        let config = search_config(
            &session,
            RunControlSearchCombatOptions {
                potion_policy: Some(CombatSearchV2PotionPolicy::Never),
                max_potions_used: Some(0),
                ..RunControlSearchCombatOptions::default()
            },
        );
        assert_eq!(config.potion_policy, CombatSearchV2PotionPolicy::Never);
        assert_eq!(config.max_potions_used, Some(0));
    }

    #[test]
    fn high_stakes_search_options_enables_semantic_potions_for_boss_manual_search() {
        let session = session_with_combat_flags(true, false);

        let options =
            high_stakes_search_options(&session, RunControlSearchCombatOptions::default());

        assert_potion_budget(
            options,
            Some(CombatSearchV2PotionPolicy::SemanticBudgeted),
            Some(2),
        );
    }

    #[test]
    fn high_stakes_search_options_enables_single_semantic_potion_for_elite_manual_search() {
        let session = session_with_combat_flags(false, true);

        let options =
            high_stakes_search_options(&session, RunControlSearchCombatOptions::default());

        assert_potion_budget(
            options,
            Some(CombatSearchV2PotionPolicy::SemanticBudgeted),
            Some(1),
        );
    }

    #[test]
    fn non_boss_segment_mode_allows_hallway_partial_turns_but_blocks_boss_partial_turns() {
        let hallway = session_with_combat_flags(false, false);
        let hallway_start = hallway
            .current_active_combat_position()
            .expect("hallway combat position");
        assert!(segment_mode_allows_turn_segment(
            Some(crate::eval::run_control::RunControlCombatSegmentMode::NonBossTurnBoundary),
            &hallway_start
        ));

        let boss = session_with_combat_flags(true, false);
        let boss_start = boss
            .current_active_combat_position()
            .expect("boss combat position");
        assert!(!segment_mode_allows_turn_segment(
            Some(crate::eval::run_control::RunControlCombatSegmentMode::NonBossTurnBoundary),
            &boss_start
        ));
        assert!(segment_mode_allows_turn_segment(
            Some(crate::eval::run_control::RunControlCombatSegmentMode::TurnBoundary),
            &boss_start
        ));
    }

    #[test]
    fn high_stakes_search_options_keeps_ordinary_manual_search_no_potion_default() {
        let session = session_with_combat_flags(false, false);

        let options =
            high_stakes_search_options(&session, RunControlSearchCombatOptions::default());

        assert_potion_budget(options, None, None);
    }

    #[test]
    fn high_stakes_search_options_respects_user_potion_override() {
        let session = session_with_combat_flags(true, false);

        let options = high_stakes_search_options(
            &session,
            options_with_potion_budget(CombatSearchV2PotionPolicy::Never, 0),
        );

        assert_potion_budget(options, Some(CombatSearchV2PotionPolicy::Never), Some(0));
    }
}
