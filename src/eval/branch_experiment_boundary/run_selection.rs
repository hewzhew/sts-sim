use crate::ai::deck_mutation_compiler_v1::{
    compile_deck_mutation_decision_v1, DeckMutationCompilerModeV1, DeckMutationPlanCandidateV1,
};
use crate::content::cards::CardId;
use crate::eval::branch_experiment::{
    BranchExperimentChoiceCardV1, BranchExperimentChoiceDecisionSignalV1,
};
use crate::eval::run_control::RunControlSession;
use crate::state::core::EngineState;

pub(super) const MAX_RUN_SELECTION_OPTIONS_PER_BRANCH: usize = 12;

#[derive(Clone, Debug)]
pub(super) struct RunSelectionBranchOption {
    pub(super) label: String,
    pub(super) command: String,
    pub(super) card: Option<CardId>,
    pub(super) upgrades: Option<u8>,
    pub(super) selected_cards: Vec<BranchExperimentChoiceCardV1>,
    pub(super) effect_kind: String,
    pub(super) effect_key: String,
    pub(super) effect_label: String,
    pub(super) representative_count: usize,
    pub(super) suppressed_count: usize,
    pub(super) decision_signal: Option<BranchExperimentChoiceDecisionSignalV1>,
}

pub(super) fn run_selection_branch_options(
    session: &RunControlSession,
) -> Option<Vec<RunSelectionBranchOption>> {
    let EngineState::RunPendingChoice(choice) = &session.engine_state else {
        return None;
    };
    let decision = compile_deck_mutation_decision_v1(
        &session.run_state,
        choice,
        DeckMutationCompilerModeV1::BranchTopK {
            max_active: MAX_RUN_SELECTION_OPTIONS_PER_BRANCH,
        },
    );
    let options = decision
        .branch_active_plans
        .into_iter()
        .map(run_selection_branch_option_from_compiled)
        .collect::<Vec<_>>();
    (!options.is_empty()).then_some(options)
}

fn run_selection_branch_option_from_compiled(
    plan: DeckMutationPlanCandidateV1,
) -> RunSelectionBranchOption {
    let decision_signal = Some(super::deck_mutation_decision_signal_v1(&plan));
    RunSelectionBranchOption {
        label: plan.step.effect_label.clone(),
        command: plan.step.command,
        card: (plan.step.cards.len() == 1).then(|| plan.step.cards[0].card),
        upgrades: (plan.step.cards.len() == 1).then(|| plan.step.cards[0].upgrades),
        selected_cards: plan
            .step
            .cards
            .iter()
            .map(|card| BranchExperimentChoiceCardV1 {
                card: card.card,
                upgrades: card.upgrades,
            })
            .collect(),
        effect_kind: plan.step.effect_kind,
        effect_key: plan.step.effect_key,
        effect_label: plan.step.effect_label,
        representative_count: plan.representative_count,
        suppressed_count: plan.suppressed_count,
        decision_signal,
    }
}
