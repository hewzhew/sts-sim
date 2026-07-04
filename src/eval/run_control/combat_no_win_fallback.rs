use crate::ai::combat_search_v2::{
    filter_combat_search_legal_actions, plan_combat_turn_segment_v1, CombatSearchV2ActionTrace,
    CombatSearchV2Config, CombatSearchV2Report,
};
use crate::content::potions::PotionId;
use crate::sim::combat::{
    combat_terminal, CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal,
    EngineCombatStepper,
};
use crate::sim::combat_legal_actions::engine_local_moves;

use super::combat_line_executor::{
    apply_combat_turn_segment, apply_selected_combat_candidate_line,
    apply_smoke_bomb_survival_fallback, millis_to_micros_u64, CombatCandidateLinePerformance,
};
use super::commands::{RunControlCombatSegmentMode, RunControlSearchCombatOptions};
use super::session::{RunControlCommandOutcome, RunControlSession};
use super::trace_annotation::CombatAutomationTrajectorySource;

pub(super) fn try_apply_no_win_fallback(
    session: &mut RunControlSession,
    start: &CombatPosition,
    config: &CombatSearchV2Config,
    options: &RunControlSearchCombatOptions,
    search_report: &CombatSearchV2Report,
    saved_evidence: Option<&std::path::Path>,
    hp_loss_limit: Option<u32>,
) -> Result<Option<RunControlCommandOutcome>, String> {
    if let Some(outcome) = try_apply_complete_line_solver_after_no_win(
        session,
        start,
        config,
        search_report,
        saved_evidence,
        hp_loss_limit,
    )? {
        return Ok(Some(outcome));
    }
    if let Some(outcome) = try_apply_turn_segment_after_rejection(
        session,
        start,
        config,
        options,
        search_report,
        saved_evidence,
        "no_complete_winning_candidate",
    )? {
        return Ok(Some(outcome));
    }
    try_apply_smoke_bomb_survival_fallback_after_rejection(
        session,
        saved_evidence,
        "no_complete_winning_candidate",
    )
}

fn try_apply_complete_line_solver_after_no_win(
    session: &mut RunControlSession,
    start: &CombatPosition,
    config: &CombatSearchV2Config,
    search_report: &CombatSearchV2Report,
    saved_evidence: Option<&std::path::Path>,
    hp_loss_limit: Option<u32>,
) -> Result<Option<RunControlCommandOutcome>, String> {
    let Some(solution) = super::combat_complete_line_solver::try_solve_complete_line(start, config)
    else {
        return Ok(None);
    };
    if hp_loss_limit.is_some_and(|limit| solution.line.hp_loss > limit as i32) {
        return Ok(None);
    }
    let summary = format!(
        "complete_line_solver actions={}/{} delta={} hp_loss={}/{} saved={} budget=base:{}/{}ms repair:{}x{}/{}ms stops={}/{} nodes={} generated={} base_nodes={}/{} repair_nodes={}/{} repair={}/{}/{} elapsed_ms={}",
        solution.final_action_count,
        solution.base_action_count,
        solution.repair_action_count_delta,
        solution.final_hp_loss,
        solution.base_hp_loss,
        solution.repair_hp_loss_saved,
        solution.base_node_budget,
        solution.base_ms_budget,
        solution.repair_cut_budget,
        solution.repair_node_budget_per_cut,
        solution.repair_ms_budget_per_cut,
        solution.base_stop_reason,
        solution.last_repair_stop_reason.unwrap_or("none"),
        solution.nodes_expanded,
        solution.nodes_generated,
        solution.base_nodes_expanded,
        solution.base_nodes_generated,
        solution.repair_nodes_expanded,
        solution.repair_nodes_generated,
        solution.repair_attempts,
        solution.repair_wins,
        solution.repair_improvements,
        solution.elapsed_ms
    );
    apply_selected_combat_candidate_line(
        session,
        start,
        config,
        search_report,
        saved_evidence,
        solution.line,
        CombatAutomationTrajectorySource::CompleteLineSolver,
        summary,
        Some(CombatCandidateLinePerformance {
            nodes_expanded: solution.nodes_expanded as u64,
            nodes_generated: solution.nodes_generated as u64,
            total_us: millis_to_micros_u64(solution.elapsed_ms),
        }),
    )
    .map(Some)
}

pub(super) fn try_apply_turn_segment_after_rejection(
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
    apply_combat_turn_segment(
        session,
        start,
        search_report,
        &segment_report,
        saved_evidence,
        rejection_result,
    )
    .map(Some)
}

pub(super) fn try_apply_smoke_bomb_survival_fallback_after_rejection(
    session: &mut RunControlSession,
    saved_evidence: Option<&std::path::Path>,
    rejection_result: &'static str,
) -> Result<Option<RunControlCommandOutcome>, String> {
    let Some(active) = session.active_combat.as_ref() else {
        return Ok(None);
    };
    let smoke_input = engine_local_moves(&active.engine_state, &active.combat_state)
        .into_iter()
        .find(|input| match input {
            crate::state::core::ClientInput::UsePotion { potion_index, .. } => active
                .combat_state
                .entities
                .potions
                .get(*potion_index)
                .and_then(|potion| potion.as_ref())
                .is_some_and(|potion| potion.id == PotionId::SmokeBomb),
            _ => false,
        });
    let Some(smoke_input) = smoke_input else {
        return Ok(None);
    };
    apply_smoke_bomb_survival_fallback(session, smoke_input, saved_evidence, rejection_result)
        .map(Some)
}

pub(super) fn segment_mode_allows_turn_segment(
    mode: Option<RunControlCombatSegmentMode>,
    start: &CombatPosition,
) -> bool {
    match mode {
        Some(RunControlCombatSegmentMode::TurnBoundary) => true,
        Some(RunControlCombatSegmentMode::NonBossTurnBoundary) => !start.combat.meta.is_boss_fight,
        None => false,
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
                super::view_model::client_input_hint(&action.input)
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
