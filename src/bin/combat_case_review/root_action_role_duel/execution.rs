use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2DecisionMicroscopeReport, CombatSearchV2Report,
};
use sts_simulator::eval::combat_case::CombatCase;
use sts_simulator::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal, EngineCombatStepper,
};

use super::super::focus::review_focus;
use super::super::key_card_lifecycle::key_card_lifecycle;
use super::super::options::ReviewOptions;
use super::super::search_runner::run_configured_search;
use super::super::search_types::SearchReview;
use super::config::duel_search_config;
use super::transition::{root_potions_used, root_transition};
use super::types::{DuelSelection, RootActionRoleDuel, RootActionRoleDuelCandidate};

pub(super) fn run_duel(
    options: &ReviewOptions,
    case: &CombatCase,
    microscope: &CombatSearchV2DecisionMicroscopeReport,
    selection: &DuelSelection,
) -> Option<RootActionRoleDuel> {
    let candidate = microscope.candidates.get(selection.candidate_index)?;
    let stepper = EngineCombatStepper;
    let step = stepper.apply_to_stable(
        &case.position,
        candidate.input.clone(),
        CombatStepLimits {
            max_engine_steps: 250,
            deadline: None,
        },
    );
    let root_transition = root_transition(&step.position, &step, candidate);
    let child_case = child_case(case, &step.position);
    let (child_search, child_report) = if step.alive
        && !step.truncated
        && !step.timed_out
        && matches!(step.terminal, CombatTerminal::Unresolved)
    {
        let (search, report) =
            run_child_search(options, &child_case, root_potions_used(&candidate.input));
        (Some(search), Some(report))
    } else {
        (None, None)
    };
    let child_best_complete_final_state = child_report
        .as_ref()
        .and_then(|report| report.best_complete_trajectory.as_ref())
        .map(|trajectory| trajectory.final_state.clone());
    let child_focus = child_search
        .as_ref()
        .map(|search| review_focus(std::slice::from_ref(search)));
    let key_card_lifecycle_after_root = child_focus
        .as_ref()
        .and_then(|focus| key_card_lifecycle(&child_case.position, focus.as_ref()));

    Some(RootActionRoleDuel {
        selection_reasons: selection.reasons.clone(),
        root_candidate: RootActionRoleDuelCandidate {
            ordered_index: candidate.ordered_index,
            action_key: candidate.action_key.clone(),
            action_role: candidate.action_role,
            selected_by_best_complete: candidate.selected_by_best_complete,
            input: candidate.input.clone(),
        },
        root_transition,
        child_search,
        child_best_complete_final_state,
        child_focus: child_focus.flatten(),
        key_card_lifecycle_after_root,
    })
}

fn run_child_search(
    options: &ReviewOptions,
    case: &CombatCase,
    root_potions_used: u32,
) -> (SearchReview, CombatSearchV2Report) {
    let mut config = duel_search_config(options, "root_action_role_duel_child");
    config.max_potions_used = Some(
        options
            .diagnostic_potion_max
            .saturating_sub(root_potions_used),
    );
    run_configured_search(
        "root_action_role_duel_child",
        case,
        config,
        options.action_preview_limit,
    )
}

fn child_case(case: &CombatCase, position: &CombatPosition) -> CombatCase {
    let mut child = case.clone();
    child.position = position.clone();
    child
}
