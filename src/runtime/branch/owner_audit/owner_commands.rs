use sts_simulator::eval::run_control::{DecisionSurface, RunDecisionAction};
use sts_simulator::state::core::ClientInput;

use super::owner_model::{
    ChoiceAnnotation, OwnerChoice, OwnerChoiceExpansion, OwnerDecision, OwnerRoutine,
};

pub(super) fn executable_choices(surface: &DecisionSurface) -> Vec<OwnerChoice> {
    executable_choices_with_cancel(surface, false)
}

pub(super) fn executable_choices_including_cancel(surface: &DecisionSurface) -> Vec<OwnerChoice> {
    executable_choices_with_cancel(surface, true)
}

pub(super) fn visible_input_decision(
    surface: &DecisionSurface,
    input: ClientInput,
) -> OwnerDecision {
    if surface
        .visible_executable_inputs
        .iter()
        .any(|visible| visible == &input)
    {
        OwnerDecision::Routine(OwnerRoutine::Action(RunDecisionAction::Input(input)))
    } else {
        OwnerDecision::Gap(format!("routine input {input:?} is not visible"))
    }
}

fn executable_choices_with_cancel(
    surface: &DecisionSurface,
    include_cancel: bool,
) -> Vec<OwnerChoice> {
    surface
        .view
        .candidates
        .iter()
        .filter_map(|candidate| {
            let action = candidate.action.executable_action()?;
            if !include_owner_choice_action(&action, include_cancel) {
                return None;
            }
            Some(OwnerChoice {
                key: candidate.key.clone(),
                action,
                label: candidate.label.clone(),
                annotation: ChoiceAnnotation::None,
                expansion: OwnerChoiceExpansion::AutoAllowed,
            })
        })
        .collect()
}

fn include_owner_choice_action(action: &RunDecisionAction, include_cancel: bool) -> bool {
    include_cancel || !is_navigation_only_action(action)
}

fn is_navigation_only_action(action: &RunDecisionAction) -> bool {
    matches!(action, RunDecisionAction::Input(ClientInput::Cancel))
}
