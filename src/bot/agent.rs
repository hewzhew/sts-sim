use crate::runtime::combat::CombatState;
use crate::state::core::{ClientInput, EngineState};
use crate::state::run::RunState;
use crate::state::selection::{SelectionResolution, SelectionScope, SelectionTargetRef};

const CURIOSITY_RUNTIME_ENABLED: bool = false;

/// An autonomous agent that can decide the next `ClientInput` based on the game state.
pub struct Agent {
    bot_depth: u32,
    /// Pre-computed optimal map path for current act (x-coords for y=0..14, plus boss)
    pub(crate) map_path: Vec<i32>,
    pub db: crate::bot::coverage::CoverageDb,
    coverage_mode: crate::bot::coverage::CoverageMode,
    pub(crate) curiosity_target: Option<crate::bot::coverage::CuriosityTarget>,
}

impl Agent {
    pub fn new() -> Self {
        Self {
            bot_depth: 6,
            map_path: Vec::new(),
            db: crate::bot::coverage::CoverageDb::load_or_default(),
            coverage_mode: crate::bot::coverage::CoverageMode::PreferNovel,
            curiosity_target: None,
        }
    }

    pub(crate) fn new_policy_model() -> Self {
        Self {
            bot_depth: 6,
            map_path: Vec::new(),
            db: crate::bot::coverage::CoverageDb::default(),
            coverage_mode: crate::bot::coverage::CoverageMode::PreferNovel,
            curiosity_target: None,
        }
    }

    /// Sets the search depth for combat decision tree.
    pub fn set_bot_depth(&mut self, depth: u32) {
        self.bot_depth = depth;
    }

    pub(crate) const fn bot_depth(&self) -> u32 {
        self.bot_depth
    }

    pub fn set_coverage_mode(&mut self, mode: crate::bot::coverage::CoverageMode) {
        self.coverage_mode = mode;
    }

    pub(crate) const fn coverage_mode(&self) -> crate::bot::coverage::CoverageMode {
        self.coverage_mode
    }

    pub fn set_curiosity_target(&mut self, target: Option<crate::bot::coverage::CuriosityTarget>) {
        self.curiosity_target = target;
    }

    pub(crate) const fn curiosity_runtime_enabled() -> bool {
        CURIOSITY_RUNTIME_ENABLED
    }

    pub(crate) fn active_curiosity_target(&self) -> Option<&crate::bot::coverage::CuriosityTarget> {
        if Self::curiosity_runtime_enabled() {
            self.curiosity_target.as_ref()
        } else {
            None
        }
    }

    /// Primary entry point for the bot to decide the next move.
    pub fn decide(
        &mut self,
        es: &EngineState,
        rs: &RunState,
        cs: &Option<CombatState>,
        verbose: bool,
    ) -> ClientInput {
        match self.decide_policy(es, rs, cs.as_ref(), verbose) {
            crate::bot::BotPolicyDecision::Combat(decision) => {
                self.record_live_coverage(decision.chosen_input.clone(), decision.diagnostics.chosen_move.clone(), cs.as_ref());
                if let Some(combat) = cs.as_ref() {
                    self.record_signature_for_choice(es, combat, &decision.chosen_input);
                }
                decision.chosen_input
            }
            crate::bot::BotPolicyDecision::RewardCard(decision) => match decision.action {
                crate::bot::RewardCardDecisionAction::Pick(idx) => match es {
                    EngineState::PendingChoice(crate::state::core::PendingChoice::CardRewardSelect { .. }) => {
                        ClientInput::SubmitDiscoverChoice(idx)
                    }
                    _ => ClientInput::SelectCard(idx),
                },
                crate::bot::RewardCardDecisionAction::Skip => match es {
                    EngineState::PendingChoice(crate::state::core::PendingChoice::CardRewardSelect { .. }) => {
                        ClientInput::Cancel
                    }
                    _ => ClientInput::Proceed,
                },
            },
            crate::bot::BotPolicyDecision::RewardClaim(decision) => match decision.action {
                crate::bot::RewardClaimDecisionAction::Claim(idx) => ClientInput::ClaimReward(idx),
                crate::bot::RewardClaimDecisionAction::DiscardPotion(idx) => {
                    ClientInput::DiscardPotion(idx)
                }
                crate::bot::RewardClaimDecisionAction::Proceed => ClientInput::Proceed,
            },
            crate::bot::BotPolicyDecision::Shop(decision) => match decision.action {
                crate::bot::ShopDecisionAction::BuyCard(idx) => ClientInput::BuyCard(idx),
                crate::bot::ShopDecisionAction::BuyRelic(idx) => ClientInput::BuyRelic(idx),
                crate::bot::ShopDecisionAction::BuyPotion(idx) => ClientInput::BuyPotion(idx),
                crate::bot::ShopDecisionAction::PurgeCard(idx) => ClientInput::PurgeCard(idx),
                crate::bot::ShopDecisionAction::DiscardPotion(idx) => {
                    ClientInput::DiscardPotion(idx)
                }
                crate::bot::ShopDecisionAction::Leave => ClientInput::Proceed,
            },
            crate::bot::BotPolicyDecision::Event(decision) => {
                ClientInput::EventChoice(decision.decision.option_index)
            }
            crate::bot::BotPolicyDecision::LegacyInput { input, .. } => input,
        }
    }

    fn record_live_coverage(
        &mut self,
        chosen: ClientInput,
        executed: ClientInput,
        combat: Option<&CombatState>,
    ) {
        let _ = chosen;
        let Some(combat) = combat else {
            return;
        };
        match &executed {
            ClientInput::PlayCard { card_index, .. } => {
                if let Some(card) = combat.zones.hand.get(*card_index) {
                    let def = crate::content::cards::get_card_definition(card.id);
                    self.db.tested_cards.insert(def.name.to_string());
                    self.db.save();
                }
            }
            ClientInput::UsePotion { potion_index, .. } => {
                if let Some(Some(p)) = combat.entities.potions.get(*potion_index) {
                    let def = crate::content::potions::get_potion_definition(p.id);
                    self.db.tested_potions.insert(def.name.to_string());
                    self.db.save();
                }
            }
            _ => {}
        }
    }

    pub(crate) fn decide_boss_relic_policy(
        &self,
        rs: &RunState,
        bs: &crate::rewards::state::BossRelicChoiceState,
    ) -> ClientInput {
        if let Some(idx) = self
            .active_curiosity_target()
            .and_then(|_| self.curiosity_boss_relic_pick(&bs.relics))
        {
            return ClientInput::SubmitRelicChoice(idx);
        }

        let mut best_idx = 0;
        let mut best_score = i32::MIN;
        for (i, relic) in bs.relics.iter().enumerate() {
            let score = self.boss_relic_score(rs, *relic);
            if score > best_score {
                best_score = score;
                best_idx = i;
            }
        }

        ClientInput::SubmitRelicChoice(best_idx)
    }

    pub(crate) fn decide_run_pending_choice(
        &self,
        rs: &RunState,
        choice_state: &crate::state::core::RunPendingChoiceState,
    ) -> ClientInput {
        use crate::state::core::RunPendingChoiceReason;
        match choice_state.reason {
            RunPendingChoiceReason::Purge => {
                if rs.master_deck.is_empty() {
                    ClientInput::Cancel
                } else {
                    let indices =
                        self.best_purge_indices(rs, choice_state.max_choices.min(rs.master_deck.len()));
                    ClientInput::SubmitSelection(SelectionResolution {
                        scope: SelectionScope::Deck,
                        selected: indices
                            .into_iter()
                            .filter_map(|idx| rs.master_deck.get(idx))
                            .map(|card| SelectionTargetRef::CardUuid(card.uuid))
                            .collect(),
                    })
                }
            }
            RunPendingChoiceReason::Upgrade => {
                if let Some(best_idx) = self.best_upgrade_index(rs) {
                    ClientInput::SubmitSelection(SelectionResolution {
                        scope: SelectionScope::Deck,
                        selected: rs
                            .master_deck
                            .get(best_idx)
                            .map(|card| vec![SelectionTargetRef::CardUuid(card.uuid)])
                            .unwrap_or_default(),
                    })
                } else {
                    ClientInput::Cancel
                }
            }
            RunPendingChoiceReason::Transform | RunPendingChoiceReason::TransformUpgraded => {
                if rs.master_deck.is_empty() {
                    ClientInput::Cancel
                } else {
                    let indices = self.best_transform_indices(
                        rs,
                        choice_state.max_choices.min(rs.master_deck.len()),
                        matches!(
                            choice_state.reason,
                            RunPendingChoiceReason::TransformUpgraded
                        ),
                    );
                    ClientInput::SubmitSelection(SelectionResolution {
                        scope: SelectionScope::Deck,
                        selected: indices
                            .into_iter()
                            .filter_map(|idx| rs.master_deck.get(idx))
                            .map(|card| SelectionTargetRef::CardUuid(card.uuid))
                            .collect(),
                    })
                }
            }
            RunPendingChoiceReason::Duplicate => {
                if let Some(best_idx) = self.best_duplicate_index(rs) {
                    ClientInput::SubmitSelection(SelectionResolution {
                        scope: SelectionScope::Deck,
                        selected: rs
                            .master_deck
                            .get(best_idx)
                            .map(|card| vec![SelectionTargetRef::CardUuid(card.uuid)])
                            .unwrap_or_default(),
                    })
                } else {
                    ClientInput::Cancel
                }
            }
        }
    }

    fn record_signature_for_choice(
        &mut self,
        engine: &EngineState,
        combat: &CombatState,
        input: &ClientInput,
    ) {
        let before_engine = engine.clone();
        let before_state = combat.clone();
        let mut after_engine = engine.clone();
        let mut after_state = combat.clone();
        let alive = crate::diff::replay::tick_until_stable(
            &mut after_engine,
            &mut after_state,
            input.clone(),
        );
        if !alive && !matches!(after_engine, EngineState::GameOver(_)) {
            return;
        }
        let signature = crate::bot::coverage_signatures::signature_from_transition_with_archetypes(
            &before_engine,
            &before_state,
            input,
            &after_engine,
            &after_state,
            crate::bot::coverage::archetype_tags_for_combat(&before_state),
        );
        self.db.record_signature(&signature);
        self.db.save();
    }
}
