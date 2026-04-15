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

    /// Sets the search depth for combat decision tree.
    pub fn set_bot_depth(&mut self, depth: u32) {
        self.bot_depth = depth;
    }

    pub fn set_coverage_mode(&mut self, mode: crate::bot::coverage::CoverageMode) {
        self.coverage_mode = mode;
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
        match es {
            EngineState::PendingChoice(crate::state::core::PendingChoice::CardRewardSelect {
                cards,
                can_skip,
                ..
            }) => match crate::bot::reward_heuristics::evaluate_reward_screen(cards) {
                Some(idx) => ClientInput::SubmitDiscoverChoice(
                    self.active_curiosity_target()
                        .and_then(|_| self.curiosity_reward_pick(cards, rs))
                        .unwrap_or(idx),
                ),
                None if *can_skip => ClientInput::Cancel,
                None => ClientInput::SubmitDiscoverChoice(0),
            },
            EngineState::CombatPlayerTurn
            | EngineState::PendingChoice(_)
            | EngineState::EventCombat(_) => {
                if let Some(combat) = cs {
                    let chosen = crate::bot::search::find_best_move(
                        es,
                        combat,
                        self.bot_depth,
                        verbose,
                        &self.db,
                        self.coverage_mode,
                        self.active_curiosity_target(),
                    );

                    // Live coverage tracking: mark executing moves as tested
                    match &chosen {
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

                    self.record_signature_for_choice(es, combat, &chosen);

                    chosen
                } else {
                    ClientInput::Proceed
                }
            }
            EngineState::MapNavigation => self.decide_map(rs),
            EngineState::RewardScreen(reward) => {
                use crate::rewards::state::RewardItem;

                // 1. Handle pending card choice
                if let Some(cards) = &reward.pending_card_choice {
                    let offered_cards: Vec<_> =
                        cards.iter().map(|reward_card| reward_card.id).collect();

                    if let Some(idx) = self
                        .active_curiosity_target()
                        .and_then(|_| self.curiosity_reward_pick(&offered_cards, rs))
                        .or_else(|| {
                            crate::bot::reward_heuristics::evaluate_reward_screen_for_run(
                                &offered_cards,
                                rs,
                            )
                        })
                    {
                        ClientInput::SelectCard(idx)
                    } else {
                        ClientInput::Proceed
                    }
                } else if !reward.items.is_empty() {
                    if let Some(idx) = self
                        .active_curiosity_target()
                        .and_then(|_| self.curiosity_reward_claim(&reward.items))
                    {
                        return ClientInput::ClaimReward(idx);
                    }

                    // 2. Handle claiming items
                    let mut claimed = false;
                    let mut claim_idx = 0;

                    for (i, item) in reward.items.iter().enumerate() {
                        match item {
                            RewardItem::Potion { .. } => {
                                // Claim potion only if we have an empty slot
                                if rs.potions.iter().any(|p| p.is_none()) {
                                    claim_idx = i;
                                    claimed = true;
                                    break;
                                }
                            }
                            // Always claim gold/relics/cards/etc.
                            _ => {
                                claim_idx = i;
                                claimed = true;
                                break;
                            }
                        }
                    }

                    if claimed {
                        ClientInput::ClaimReward(claim_idx)
                    } else {
                        // Leftover items (e.g. potions when full), proceed
                        ClientInput::Proceed
                    }
                } else {
                    ClientInput::Proceed
                }
            }
            EngineState::BossRelicSelect(bs) => {
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
            EngineState::Campfire => self.decide_campfire(rs),
            EngineState::EventRoom => self.decide_event(rs),
            EngineState::Shop(shop) => self.decide_shop(rs, shop),
            EngineState::RunPendingChoice(choice_state) => {
                use crate::state::core::RunPendingChoiceReason;
                match choice_state.reason {
                    RunPendingChoiceReason::Purge => {
                        if rs.master_deck.is_empty() {
                            ClientInput::Cancel
                        } else {
                            let indices = self.best_purge_indices(
                                rs,
                                choice_state.max_choices.min(rs.master_deck.len()),
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
                    RunPendingChoiceReason::Transform
                    | RunPendingChoiceReason::TransformUpgraded => {
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
            EngineState::GameOver(_) => ClientInput::Proceed,
            _ => ClientInput::Proceed,
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
