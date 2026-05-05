use crate::runtime::combat::CombatState;
use crate::state::core::{ClientInput, EngineState};
use crate::state::run::RunState;
use crate::state::selection::{SelectionResolution, SelectionScope, SelectionTargetRef};

/// An autonomous agent that can decide the next `ClientInput` based on the game state.
pub struct Agent {
    bot_depth: u32,
}

impl Agent {
    pub fn new() -> Self {
        Self { bot_depth: 6 }
    }

    pub fn set_bot_depth(&mut self, depth: u32) {
        self.bot_depth = depth;
    }

    pub fn set_coverage_mode(&mut self, _mode: crate::bot::CoverageMode) {}

    pub fn set_curiosity_target(&mut self, _target: Option<crate::bot::CuriosityTarget>) {}

    pub(crate) const fn bot_depth(&self) -> u32 {
        self.bot_depth
    }

    pub fn decide(
        &mut self,
        es: &EngineState,
        rs: &RunState,
        cs: &Option<CombatState>,
        verbose: bool,
    ) -> ClientInput {
        match self.decide_policy(es, rs, cs.as_ref(), verbose) {
            crate::bot::BotPolicyDecision::Combat(decision) => decision.chosen_input,
            crate::bot::BotPolicyDecision::RewardCard(decision) => match decision.action {
                crate::bot::RewardCardAction::Pick(idx) => match es {
                    EngineState::PendingChoice(
                        crate::state::core::PendingChoice::CardRewardSelect { .. },
                    ) => ClientInput::SubmitDiscoverChoice(idx),
                    _ => ClientInput::SelectCard(idx),
                },
                crate::bot::RewardCardAction::Skip => match es {
                    EngineState::PendingChoice(
                        crate::state::core::PendingChoice::CardRewardSelect { .. },
                    ) => ClientInput::Cancel,
                    _ => ClientInput::Proceed,
                },
            },
            crate::bot::BotPolicyDecision::RewardClaim(decision) => match decision.action {
                crate::bot::RewardClaimAction::Claim(idx) => ClientInput::ClaimReward(idx),
                crate::bot::RewardClaimAction::DiscardPotion(idx) => {
                    ClientInput::DiscardPotion(idx)
                }
                crate::bot::RewardClaimAction::Proceed => ClientInput::Proceed,
            },
            crate::bot::BotPolicyDecision::Shop(decision) => match decision.action {
                crate::bot::ShopAction::BuyCard(idx) => ClientInput::BuyCard(idx),
                crate::bot::ShopAction::BuyRelic(idx) => ClientInput::BuyRelic(idx),
                crate::bot::ShopAction::BuyPotion(idx) => ClientInput::BuyPotion(idx),
                crate::bot::ShopAction::PurgeCard(idx) => ClientInput::PurgeCard(idx),
                crate::bot::ShopAction::DiscardPotion(idx) => ClientInput::DiscardPotion(idx),
                crate::bot::ShopAction::Leave => ClientInput::Proceed,
            },
            crate::bot::BotPolicyDecision::Event(decision) => {
                ClientInput::EventChoice(decision.decision.option_index)
            }
            crate::bot::BotPolicyDecision::DeckImprovement(decision) => {
                selection_input_from_deck_ops(rs, &decision.assessment)
                    .unwrap_or(ClientInput::Cancel)
            }
            crate::bot::BotPolicyDecision::Map(decision) => {
                ClientInput::SelectMapNode(decision.chosen_x as usize)
            }
            crate::bot::BotPolicyDecision::BossRelic(decision) => {
                ClientInput::SubmitRelicChoice(decision.chosen_index)
            }
            crate::bot::BotPolicyDecision::Campfire(decision) => {
                ClientInput::CampfireOption(decision.choice)
            }
            crate::bot::BotPolicyDecision::LegacyInput { input, .. } => input,
        }
    }
}

fn selection_input_from_deck_ops(
    run_state: &RunState,
    assessment: &crate::bot::DeckOpsAssessment,
) -> Option<ClientInput> {
    use crate::bot::DeckOperationKind;

    match assessment.operation {
        DeckOperationKind::Add(_) | DeckOperationKind::VampiresExchange => {
            Some(ClientInput::Proceed)
        }
        DeckOperationKind::Remove => {
            let indices = crate::bot::deck_ops::best_purge_indices(run_state, 1);
            Some(selection_from_indices(run_state, indices))
        }
        DeckOperationKind::Upgrade => crate::bot::deck_ops::best_upgrade_index(run_state)
            .map(|idx| selection_from_indices(run_state, vec![idx])),
        DeckOperationKind::Duplicate => crate::bot::deck_ops::best_duplicate_index(run_state)
            .map(|idx| selection_from_indices(run_state, vec![idx])),
        DeckOperationKind::Transform {
            count,
            upgraded_context,
        } => {
            let indices =
                crate::bot::deck_ops::best_transform_indices(run_state, count, upgraded_context);
            Some(selection_from_indices(run_state, indices))
        }
    }
}

fn selection_from_indices(run_state: &RunState, indices: Vec<usize>) -> ClientInput {
    ClientInput::SubmitSelection(SelectionResolution {
        scope: SelectionScope::Deck,
        selected: indices
            .into_iter()
            .filter_map(|idx| run_state.master_deck.get(idx))
            .map(|card| SelectionTargetRef::CardUuid(card.uuid))
            .collect(),
    })
}
