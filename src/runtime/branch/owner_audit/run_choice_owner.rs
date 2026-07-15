use sts_simulator::ai::deck_mutation_compiler_v1::{
    compile_deck_mutation_decision_v1, DeckMutationCompilerRequestV1,
};
use sts_simulator::content::cards::get_card_definition;
use sts_simulator::eval::run_control::{RunControlSession, RunDecisionAction};
use sts_simulator::state::core::{
    ClientInput, EngineState, RunPendingChoiceReason, RunPendingChoiceState,
};
use sts_simulator::state::selection::{SelectionResolution, SelectionScope};

use super::owner_model::{ChoiceAnnotation, OwnerChoice, OwnerChoiceExpansion, OwnerDecision};

pub(super) fn can_handle(reason: RunPendingChoiceReason) -> bool {
    matches!(
        reason,
        RunPendingChoiceReason::Purge
            | RunPendingChoiceReason::PurgeNonBottled
            | RunPendingChoiceReason::Upgrade
            | RunPendingChoiceReason::Transform
            | RunPendingChoiceReason::TransformNonBottled
            | RunPendingChoiceReason::TransformUpgraded
            | RunPendingChoiceReason::Duplicate
            | RunPendingChoiceReason::BottleFlame
            | RunPendingChoiceReason::BottleLightning
            | RunPendingChoiceReason::BottleTornado
    )
}

pub(super) fn run_choice_owner_decision(session: &RunControlSession) -> OwnerDecision {
    let EngineState::RunPendingChoice(choice) = &session.engine_state else {
        return OwnerDecision::Gap("RunChoice owner requires RunPendingChoice state".to_string());
    };
    if !can_handle(choice.reason) {
        return OwnerDecision::Gap(format!(
            "RunChoice owner has no policy for {:?}",
            choice.reason
        ));
    }
    deck_mutation_owner_decision(session, choice)
}

fn deck_mutation_owner_decision(
    session: &RunControlSession,
    choice: &RunPendingChoiceState,
) -> OwnerDecision {
    if choice.min_choices != choice.max_choices {
        return OwnerDecision::Gap(format!(
            "{:?} requires fixed-count committed selection, got {}-{}",
            choice.reason, choice.min_choices, choice.max_choices
        ));
    }

    let decision = compile_deck_mutation_decision_v1(
        &session.run_state,
        choice,
        DeckMutationCompilerRequestV1::committed_forced_execute_one(),
    );
    let Some(selected_plan) = decision.selected_plan else {
        return OwnerDecision::Gap(format!(
            "{:?} has no legal committed deck mutation target",
            choice.reason
        ));
    };
    if selected_plan.step.deck_indices.len() != choice.max_choices {
        return OwnerDecision::Gap(format!(
            "{:?} selected {} target(s), expected {}",
            choice.reason,
            selected_plan.step.deck_indices.len(),
            choice.max_choices
        ));
    }

    let uuids = selected_plan
        .step
        .deck_indices
        .iter()
        .filter_map(|idx| {
            session
                .run_state
                .master_deck
                .get(*idx)
                .map(|card| card.uuid)
        })
        .collect::<Vec<_>>();
    if uuids.len() != choice.max_choices {
        return OwnerDecision::Gap(format!(
            "{:?} selected target index outside master deck",
            choice.reason
        ));
    }

    let labels = selected_plan
        .step
        .cards
        .iter()
        .map(|card| {
            let name = get_card_definition(card.card).name;
            if card.upgrades == 0 {
                name.to_string()
            } else {
                format!("{name}+{}", card.upgrades)
            }
        })
        .collect::<Vec<_>>();
    let risk_note = if selected_plan.risks.is_empty() {
        String::new()
    } else {
        format!(" risks={}", selected_plan.risks.join("; "))
    };

    OwnerDecision::Candidates(vec![OwnerChoice {
        key: None,
        action: RunDecisionAction::Input(ClientInput::SubmitSelection(
            SelectionResolution::card_uuids(SelectionScope::Deck, uuids),
        )),
        label: format!(
            "{:?} {} role={:?} confidence={:.2}{}",
            choice.reason,
            labels.join(", "),
            selected_plan.role,
            selected_plan.confidence,
            risk_note
        ),
        annotation: ChoiceAnnotation::None,
        expansion: OwnerChoiceExpansion::AutoAllowed,
    }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::content::cards::CardId;
    use sts_simulator::eval::run_control::{RunControlConfig, RunDecisionAction};
    use sts_simulator::state::events::EventId;
    use sts_simulator::state::selection::{DomainEventSource, SelectionScope};

    #[test]
    fn event_origin_upgrade_produces_one_typed_run_choice_target() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.add_card_to_deck(CardId::Bash);
        session.engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
            min_choices: 1,
            max_choices: 1,
            reason: RunPendingChoiceReason::Upgrade,
            source: DomainEventSource::Event(EventId::UpgradeShrine),
            return_state: Box::new(EngineState::EventRoom),
        });

        let OwnerDecision::Candidates(choices) = run_choice_owner_decision(&session) else {
            panic!("event-origin upgrade must be owned by RunChoice");
        };
        let [choice] = choices.as_slice() else {
            panic!("RunChoice must produce one committed candidate");
        };
        let RunDecisionAction::Input(ClientInput::SubmitSelection(resolution)) = &choice.action
        else {
            panic!("RunChoice candidate must submit a typed selection");
        };
        assert_eq!(resolution.scope, SelectionScope::Deck);
        assert_eq!(resolution.selected_card_uuids().len(), 1);
    }
}
