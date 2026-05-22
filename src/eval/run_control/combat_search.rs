use crate::ai::combat_search_v2::{
    run_combat_search_v2, CombatSearchV2ActionTrace, CombatSearchV2Config,
    CombatSearchV2PotionPolicy, CombatSearchV2Report, SearchProofStatus, SearchTerminalLabel,
};
use crate::sim::combat::{
    combat_terminal, CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal,
    EngineCombatStepper,
};
use crate::sim::combat_action::CombatActionChoice;
use crate::state::core::{ClientInput, EngineState, RunResult};

use super::commands::RunControlSearchCombatOptions;
use super::session::{RunControlCommandOutcome, RunControlSession};
use super::transition_report::{
    action_result_from_transition, render_action_result, RunApplyStatus, RunVisibleSnapshot,
    TransitionAction,
};
use super::view_model::client_input_hint;

pub(super) fn apply_search_combat(
    session: &mut RunControlSession,
    options: RunControlSearchCombatOptions,
) -> Result<RunControlCommandOutcome, String> {
    let start = session.current_active_combat_position()?;
    let config = search_config(options, session.decision_step);
    let report = run_combat_search_v2(&start.engine, &start.combat, config.clone());
    let Some(trajectory) = report
        .best_complete_trajectory
        .as_ref()
        .filter(|trajectory| trajectory.terminal == SearchTerminalLabel::Win)
    else {
        return Ok(RunControlCommandOutcome::message(format!(
            "{}\n\n{}",
            render_search_rejection(&report),
            super::render::render_run_control_state(session)
        )));
    };

    verify_trajectory_replays_to_win(&start, &trajectory.actions, &config)?;

    let before_snapshot = RunVisibleSnapshot::capture(session);
    let applied = trajectory.actions.clone();
    for action in &applied {
        session.apply_input(action.input.clone())?;
    }
    let after_snapshot = RunVisibleSnapshot::capture(session);
    let status = current_run_apply_status(session);
    let action_result = action_result_from_transition(
        TransitionAction {
            label: format!(
                "search-combat applied {} actions [proof_status={:?}]",
                applied.len(),
                report.outcome.proof_status
            ),
        },
        &before_snapshot,
        &after_snapshot,
        status,
    );
    let message = format!(
        "{}\n{}\n{}",
        render_search_application(&report, &applied),
        render_action_result(&action_result),
        super::render::render_run_control_state(session)
    );
    Ok(RunControlCommandOutcome::action(message, action_result))
}

fn search_config(
    options: RunControlSearchCombatOptions,
    decision_step: u64,
) -> CombatSearchV2Config {
    let defaults = CombatSearchV2Config::default();
    CombatSearchV2Config {
        max_nodes: options.max_nodes.unwrap_or(defaults.max_nodes),
        max_actions_per_line: options
            .max_actions_per_line
            .unwrap_or(defaults.max_actions_per_line),
        max_engine_steps_per_action: options
            .max_engine_steps_per_action
            .unwrap_or(defaults.max_engine_steps_per_action),
        wall_time: options.wall_ms.map(std::time::Duration::from_millis),
        input_label: Some(format!("run_play_driver:search_combat:step{decision_step}")),
        potion_policy: options.potion_policy.unwrap_or(defaults.potion_policy),
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
        let choices = filtered_legal_choices(
            stepper.legal_action_choices(&position),
            config.potion_policy,
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

fn filtered_legal_choices(
    choices: Vec<CombatActionChoice>,
    potion_policy: CombatSearchV2PotionPolicy,
) -> Vec<CombatActionChoice> {
    match potion_policy {
        CombatSearchV2PotionPolicy::All => choices,
        CombatSearchV2PotionPolicy::Never => choices
            .into_iter()
            .filter(|choice| !is_potion_input(&choice.input))
            .collect(),
    }
}

fn is_potion_input(input: &ClientInput) -> bool {
    matches!(
        input,
        ClientInput::UsePotion { .. } | ClientInput::DiscardPotion(_)
    )
}

fn render_search_rejection(report: &CombatSearchV2Report) -> String {
    format!(
        "Search combat did not modify state.\n  terminal_report={:?}\n  proof_status={:?}\n  complete_trajectory_found={}\n  terminal_wins={}\n  nodes_expanded={}\n  nodes_generated={}\n  reason={}",
        report.outcome.terminal,
        report.outcome.proof_status,
        report.outcome.complete_trajectory_found,
        report.stats.terminal_wins,
        report.stats.nodes_expanded,
        report.stats.nodes_generated,
        report.outcome.reason
    )
}

fn render_search_application(
    report: &CombatSearchV2Report,
    actions: &[CombatSearchV2ActionTrace],
) -> String {
    let trajectory = report
        .best_complete_trajectory
        .as_ref()
        .expect("caller only renders after selecting a complete trajectory");
    let optimality = if report.outcome.proof_status == SearchProofStatus::Exhaustive {
        "exact_under_supplied_state"
    } else {
        "not_claimed_budgeted_complete_win"
    };
    let mut lines = vec![
        "Search combat applied complete winning trajectory.".to_string(),
        format!(
            "  proof_status={:?} optimality={optimality}",
            report.outcome.proof_status
        ),
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

fn current_run_apply_status(session: &RunControlSession) -> RunApplyStatus {
    match session.engine_state {
        EngineState::GameOver(RunResult::Victory) => RunApplyStatus::Victory,
        EngineState::GameOver(RunResult::Defeat) => RunApplyStatus::Defeat,
        _ => RunApplyStatus::Running,
    }
}
