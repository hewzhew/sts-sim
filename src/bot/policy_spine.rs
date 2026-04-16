use crate::bot::agent::Agent;
use crate::bot::event_policy::{EventChoiceDecision, EventDecisionContext as EventPolicyContext};
use crate::bot::reward_heuristics::RewardScreenEvaluation;
use crate::bot::search::{self, SearchDiagnostics};
use crate::content::potions::PotionId;
use crate::rewards::state::{RewardCard, RewardState};
use crate::runtime::combat::CombatState;
use crate::shop::ShopState;
use crate::state::core::{ClientInput, EngineState};
use crate::state::run::RunState;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionDomain {
    Combat,
    RewardCard,
    RewardClaim,
    Shop,
    Event,
    LegacyInput,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecisionMetadata {
    pub domain: DecisionDomain,
    pub source: &'static str,
    pub rationale_key: Option<&'static str>,
    pub confidence: Option<f32>,
    pub fallback_used: bool,
}

impl DecisionMetadata {
    pub const fn new(
        domain: DecisionDomain,
        source: &'static str,
        rationale_key: Option<&'static str>,
        confidence: Option<f32>,
        fallback_used: bool,
    ) -> Self {
        Self {
            domain,
            source,
            rationale_key,
            confidence,
            fallback_used,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockedPotionOffer {
    pub potion_id: PotionId,
}

#[derive(Clone, Copy, Debug)]
pub struct CombatDecisionContext<'a> {
    pub engine: &'a EngineState,
    pub combat: &'a CombatState,
    pub verbose: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct RewardCardDecisionContext<'a> {
    pub reward_cards: &'a [RewardCard],
    pub can_skip: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct RewardClaimDecisionContext<'a> {
    pub reward: &'a RewardState,
    pub blocked_potion_offers: &'a [BlockedPotionOffer],
}

#[derive(Clone, Copy, Debug)]
pub struct ShopDecisionContext<'a> {
    pub shop: &'a ShopState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RewardCardDecisionAction {
    Pick(usize),
    Skip,
}

#[derive(Clone, Debug)]
pub struct RewardCardDecision {
    pub meta: DecisionMetadata,
    pub action: RewardCardDecisionAction,
    pub evaluation: RewardScreenEvaluation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RewardClaimDecisionAction {
    Claim(usize),
    DiscardPotion(usize),
    Proceed,
}

#[derive(Clone, Debug)]
pub struct RewardClaimDecision {
    pub meta: DecisionMetadata,
    pub action: RewardClaimDecisionAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShopDecisionAction {
    BuyCard(usize),
    BuyRelic(usize),
    BuyPotion(usize),
    PurgeCard(usize),
    DiscardPotion(usize),
    Leave,
}

#[derive(Clone, Debug)]
pub struct ShopDecision {
    pub meta: DecisionMetadata,
    pub action: ShopDecisionAction,
}

#[derive(Clone, Debug)]
pub struct CombatDecision {
    pub meta: DecisionMetadata,
    pub chosen_input: ClientInput,
    pub diagnostics: SearchDiagnostics,
}

#[derive(Clone, Debug)]
pub struct EventDecision {
    pub meta: DecisionMetadata,
    pub decision: EventChoiceDecision,
}

#[derive(Clone, Debug)]
pub enum BotPolicyDecision {
    Combat(CombatDecision),
    RewardCard(RewardCardDecision),
    RewardClaim(RewardClaimDecision),
    Shop(ShopDecision),
    Event(EventDecision),
    LegacyInput {
        meta: DecisionMetadata,
        input: ClientInput,
    },
}

impl BotPolicyDecision {
    pub fn meta(&self) -> &DecisionMetadata {
        match self {
            Self::Combat(decision) => &decision.meta,
            Self::RewardCard(decision) => &decision.meta,
            Self::RewardClaim(decision) => &decision.meta,
            Self::Shop(decision) => &decision.meta,
            Self::Event(decision) => &decision.meta,
            Self::LegacyInput { meta, .. } => meta,
        }
    }
}

impl Agent {
    pub fn decide_policy(
        &mut self,
        engine: &EngineState,
        run_state: &RunState,
        combat: Option<&CombatState>,
        verbose: bool,
    ) -> BotPolicyDecision {
        match engine {
            EngineState::PendingChoice(crate::state::core::PendingChoice::CardRewardSelect {
                cards,
                can_skip,
                ..
            }) => {
                let reward_cards: Vec<_> = cards
                    .iter()
                    .copied()
                    .map(|card_id| RewardCard::new(card_id, 0))
                    .collect();
                BotPolicyDecision::RewardCard(self.decide_reward_card_policy(
                    run_state,
                    RewardCardDecisionContext {
                        reward_cards: &reward_cards,
                        can_skip: *can_skip,
                    },
                ))
            }
            EngineState::CombatPlayerTurn
            | EngineState::PendingChoice(_)
            | EngineState::EventCombat(_) => {
                if let Some(combat) = combat {
                    BotPolicyDecision::Combat(self.decide_combat_policy(
                        CombatDecisionContext {
                            engine,
                            combat,
                            verbose,
                        },
                    ))
                } else {
                    BotPolicyDecision::LegacyInput {
                        meta: DecisionMetadata::new(
                            DecisionDomain::LegacyInput,
                            "missing_combat_state",
                            Some("missing_combat_state"),
                            None,
                            false,
                        ),
                        input: ClientInput::Proceed,
                    }
                }
            }
            EngineState::RewardScreen(reward) => {
                if let Some(cards) = &reward.pending_card_choice {
                    BotPolicyDecision::RewardCard(
                        self.decide_reward_card_policy(
                            run_state,
                            RewardCardDecisionContext {
                                reward_cards: cards,
                                can_skip: reward.skippable,
                            },
                        ),
                    )
                } else {
                    BotPolicyDecision::RewardClaim(
                        self.decide_reward_claim_policy(
                            run_state,
                            RewardClaimDecisionContext {
                                reward,
                                blocked_potion_offers: &[],
                            },
                        ),
                    )
                }
            }
            EngineState::Shop(shop) => BotPolicyDecision::Shop(self.decide_shop_policy(
                run_state,
                ShopDecisionContext { shop },
            )),
            EngineState::EventRoom => {
                if let Some(event_state) = &run_state.event_state {
                    let choices = crate::engine::event_handler::get_event_choices(run_state);
                    let context =
                        crate::bot::event_policy::local_event_context(run_state, event_state, &choices);
                    BotPolicyDecision::Event(self.decide_event_policy(run_state, &context))
                } else {
                    BotPolicyDecision::LegacyInput {
                        meta: DecisionMetadata::new(
                            DecisionDomain::LegacyInput,
                            "missing_event_state",
                            Some("missing_event_state"),
                            None,
                            false,
                        ),
                        input: ClientInput::EventChoice(0),
                    }
                }
            }
            EngineState::MapNavigation => BotPolicyDecision::LegacyInput {
                meta: DecisionMetadata::new(
                    DecisionDomain::LegacyInput,
                    "map_policy",
                    Some("map_navigation"),
                    None,
                    false,
                ),
                input: self.decide_map(run_state),
            },
            EngineState::BossRelicSelect(state) => BotPolicyDecision::LegacyInput {
                meta: DecisionMetadata::new(
                    DecisionDomain::LegacyInput,
                    "boss_relic_policy",
                    Some("boss_relic_choice"),
                    None,
                    false,
                ),
                input: self.decide_boss_relic_policy(run_state, state),
            },
            EngineState::Campfire => BotPolicyDecision::LegacyInput {
                meta: DecisionMetadata::new(
                    DecisionDomain::LegacyInput,
                    "campfire_policy",
                    Some("campfire_choice"),
                    None,
                    false,
                ),
                input: self.decide_campfire(run_state),
            },
            EngineState::RunPendingChoice(choice_state) => BotPolicyDecision::LegacyInput {
                meta: DecisionMetadata::new(
                    DecisionDomain::LegacyInput,
                    "pending_choice_policy",
                    Some("run_pending_choice"),
                    None,
                    false,
                ),
                input: self.decide_run_pending_choice(run_state, choice_state),
            },
            EngineState::GameOver(_) => BotPolicyDecision::LegacyInput {
                meta: DecisionMetadata::new(
                    DecisionDomain::LegacyInput,
                    "game_over_policy",
                    Some("proceed_game_over"),
                    None,
                    false,
                ),
                input: ClientInput::Proceed,
            },
            _ => BotPolicyDecision::LegacyInput {
                meta: DecisionMetadata::new(
                    DecisionDomain::LegacyInput,
                    "default_proceed",
                    Some("default_proceed"),
                    None,
                    false,
                ),
                input: ClientInput::Proceed,
            },
        }
    }

    pub fn decide_combat_policy(&mut self, ctx: CombatDecisionContext<'_>) -> CombatDecision {
        let diagnostics = search::diagnose_root_search_with_depth(
            ctx.engine,
            ctx.combat,
            &self.db,
            self.coverage_mode(),
            self.active_curiosity_target(),
            self.bot_depth(),
            4000,
        );
        let chosen_input = diagnostics.chosen_move.clone();
        CombatDecision {
            meta: DecisionMetadata::new(
                DecisionDomain::Combat,
                "combat_search",
                Some("search_root_policy"),
                combat_search_confidence(&diagnostics),
                false,
            ),
            chosen_input,
            diagnostics,
        }
    }

    pub fn decide_reward_card_policy(
        &self,
        run_state: &RunState,
        ctx: RewardCardDecisionContext<'_>,
    ) -> RewardCardDecision {
        let offered_cards: Vec<_> = ctx.reward_cards.iter().map(|reward_card| reward_card.id).collect();
        let evaluation = crate::bot::evaluate_reward_screen_for_run_detailed(&offered_cards, run_state);

        if let Some(idx) = self
            .active_curiosity_target()
            .and_then(|_| self.curiosity_reward_pick(&offered_cards, run_state))
        {
            return RewardCardDecision {
                meta: DecisionMetadata::new(
                    DecisionDomain::RewardCard,
                    "curiosity_reward_override",
                    Some("curiosity_reward_override"),
                    None,
                    true,
                ),
                action: RewardCardDecisionAction::Pick(idx),
                evaluation,
            };
        }

        if let Some(idx) = evaluation.recommended_choice {
            return RewardCardDecision {
                meta: DecisionMetadata::new(
                    DecisionDomain::RewardCard,
                    "reward_policy",
                    Some("reward_recommended_pick"),
                    Some(evaluation.best_combined_score),
                    false,
                ),
                action: RewardCardDecisionAction::Pick(idx),
                evaluation,
            };
        }

        if let Some(idx) = conservative_reward_pick_after_skip(&evaluation) {
            return RewardCardDecision {
                meta: DecisionMetadata::new(
                    DecisionDomain::RewardCard,
                    "reward_policy",
                    Some("conservative_skip_fallback"),
                    Some(evaluation.best_combined_score),
                    true,
                ),
                action: RewardCardDecisionAction::Pick(idx),
                evaluation,
            };
        }

        let action = if ctx.can_skip {
            RewardCardDecisionAction::Skip
        } else {
            RewardCardDecisionAction::Pick(0)
        };
        let rationale_key = if ctx.can_skip {
            Some("reward_intentional_skip")
        } else {
            Some("reward_forced_pick")
        };
        RewardCardDecision {
            meta: DecisionMetadata::new(
                DecisionDomain::RewardCard,
                "reward_policy",
                rationale_key,
                Some(evaluation.best_combined_score),
                false,
            ),
            action,
            evaluation,
        }
    }

    pub fn decide_reward_claim_policy(
        &self,
        run_state: &RunState,
        ctx: RewardClaimDecisionContext<'_>,
    ) -> RewardClaimDecision {
        if let Some(idx) = self
            .active_curiosity_target()
            .and_then(|_| self.curiosity_reward_claim(&ctx.reward.items))
        {
            return RewardClaimDecision {
                meta: DecisionMetadata::new(
                    DecisionDomain::RewardClaim,
                    "curiosity_reward_claim_override",
                    Some("curiosity_reward_claim_override"),
                    None,
                    true,
                ),
                action: RewardClaimDecisionAction::Claim(idx),
            };
        }

        if let Some(offered_potion) = ctx
            .blocked_potion_offers
            .iter()
            .max_by_key(|offer| self.reward_potion_score(run_state, offer.potion_id))
        {
            let offered_score = self.reward_potion_score(run_state, offered_potion.potion_id);
            if let Some(discard_idx) = self.best_potion_discard_for_score(
                run_state,
                offered_score,
                |agent, rs, potion_id| agent.reward_potion_score(rs, potion_id),
            ) {
                return RewardClaimDecision {
                    meta: DecisionMetadata::new(
                        DecisionDomain::RewardClaim,
                        "reward_claim_policy",
                        Some("replace_blocked_reward_potion"),
                        Some(offered_score as f32),
                        false,
                    ),
                    action: RewardClaimDecisionAction::DiscardPotion(discard_idx),
                };
            }
        }

        for (idx, item) in ctx.reward.items.iter().enumerate() {
            match item {
                crate::rewards::state::RewardItem::Potion { .. } => {
                    if run_state.potions.iter().any(|slot| slot.is_none()) {
                        return RewardClaimDecision {
                            meta: DecisionMetadata::new(
                                DecisionDomain::RewardClaim,
                                "reward_claim_policy",
                                Some("claim_potion_empty_slot"),
                                None,
                                false,
                            ),
                            action: RewardClaimDecisionAction::Claim(idx),
                        };
                    }
                }
                _ => {
                    return RewardClaimDecision {
                        meta: DecisionMetadata::new(
                            DecisionDomain::RewardClaim,
                            "reward_claim_policy",
                            Some("claim_non_potion_reward"),
                            None,
                            false,
                        ),
                        action: RewardClaimDecisionAction::Claim(idx),
                    };
                }
            }
        }

        RewardClaimDecision {
            meta: DecisionMetadata::new(
                DecisionDomain::RewardClaim,
                "reward_claim_policy",
                Some("reward_proceed"),
                None,
                false,
            ),
            action: RewardClaimDecisionAction::Proceed,
        }
    }

    pub fn decide_shop_policy(
        &self,
        run_state: &RunState,
        ctx: ShopDecisionContext<'_>,
    ) -> ShopDecision {
        if let Some(cmd) = self.curiosity_shop_pick(run_state, ctx.shop) {
            return ShopDecision {
                meta: DecisionMetadata::new(
                    DecisionDomain::Shop,
                    "curiosity_shop_override",
                    Some("curiosity_shop_override"),
                    None,
                    true,
                ),
                action: shop_action_from_input(cmd).unwrap_or(ShopDecisionAction::Leave),
            };
        }

        let input = self.decide_shop_input(run_state, ctx.shop);
        let mut action = shop_action_from_input(input).unwrap_or(ShopDecisionAction::Leave);
        let mut meta = DecisionMetadata::new(
            DecisionDomain::Shop,
            "shop_policy",
            Some("shop_standard_decision"),
            None,
            false,
        );

        if matches!(action, ShopDecisionAction::Leave) {
            if let Some((discard_idx, offered_score)) =
                self.best_blocked_shop_potion_replacement(run_state, ctx.shop)
            {
                action = ShopDecisionAction::DiscardPotion(discard_idx);
                meta = DecisionMetadata::new(
                    DecisionDomain::Shop,
                    "shop_policy",
                    Some("replace_blocked_shop_potion"),
                    Some(offered_score as f32),
                    false,
                );
            }
        }

        ShopDecision { meta, action }
    }

    pub fn decide_event_policy(
        &self,
        run_state: &RunState,
        context: &EventPolicyContext,
    ) -> EventDecision {
        let decision = crate::bot::choose_event_option(run_state, context).unwrap_or_else(|| {
            let fallback_index = context
                .options
                .iter()
                .position(|option| !option.disabled)
                .unwrap_or(0);
            EventChoiceDecision {
                option_index: fallback_index,
                command_index: fallback_index,
                family: crate::bot::EventPolicyFamily::CompatibilityFallback,
                rationale_key: Some("compatibility_fallback_adapter"),
                score: None,
                safety_override_applied: false,
                rationale: Some("compatibility_fallback_adapter"),
            }
        });
        EventDecision {
            meta: DecisionMetadata::new(
                DecisionDomain::Event,
                "event_policy",
                decision.rationale_key,
                decision.score.map(|score| score as f32),
                matches!(
                    decision.family,
                    crate::bot::EventPolicyFamily::CompatibilityFallback
                ),
            ),
            decision,
        }
    }

    fn best_blocked_shop_potion_replacement(
        &self,
        run_state: &RunState,
        shop: &ShopState,
    ) -> Option<(usize, i32)> {
        let (offered_score, _) = shop
            .potions
            .iter()
            .filter(|potion| {
                run_state.gold >= potion.price
                    && potion.blocked_reason.as_deref() == Some("potion_slots_full")
            })
            .filter_map(|potion| {
                let score = self.shop_potion_score(run_state, potion.potion_id);
                let purchase_score =
                    self.shop_potion_purchase_score(run_state, shop, potion.potion_id, potion.price);
                (purchase_score >= 72).then_some((score, purchase_score))
            })
            .max_by_key(|(score, purchase_score)| (*purchase_score, *score))?;

        self.best_potion_discard_for_score(run_state, offered_score, |agent, rs, potion_id| {
            agent.shop_potion_score(rs, potion_id)
        })
        .map(|discard_idx| (discard_idx, offered_score))
    }
}

fn combat_search_confidence(diagnostics: &SearchDiagnostics) -> Option<f32> {
    if diagnostics.top_moves.len() < 2 {
        return diagnostics.top_moves.first().map(|move_stat| move_stat.avg_score);
    }
    Some(diagnostics.top_moves[0].avg_score - diagnostics.top_moves[1].avg_score)
}

fn conservative_reward_pick_after_skip(evaluation: &RewardScreenEvaluation) -> Option<usize> {
    if evaluation.recommended_choice.is_some() {
        return evaluation.recommended_choice;
    }
    let best_idx = evaluation
        .offered_cards
        .iter()
        .enumerate()
        .max_by(|(_, lhs), (_, rhs)| lhs.combined_score.total_cmp(&rhs.combined_score))
        .map(|(idx, _)| idx)?;
    if evaluation.best_local_score >= 35 && evaluation.best_combined_score >= 35.0 {
        Some(best_idx)
    } else {
        None
    }
}

fn shop_action_from_input(input: ClientInput) -> Option<ShopDecisionAction> {
    match input {
        ClientInput::BuyCard(idx) => Some(ShopDecisionAction::BuyCard(idx)),
        ClientInput::BuyRelic(idx) => Some(ShopDecisionAction::BuyRelic(idx)),
        ClientInput::BuyPotion(idx) => Some(ShopDecisionAction::BuyPotion(idx)),
        ClientInput::PurgeCard(idx) => Some(ShopDecisionAction::PurgeCard(idx)),
        ClientInput::DiscardPotion(idx) => Some(ShopDecisionAction::DiscardPotion(idx)),
        ClientInput::Proceed | ClientInput::Cancel => Some(ShopDecisionAction::Leave),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{BotPolicyDecision, DecisionDomain};
    use crate::content::cards::CardId;
    use crate::rewards::state::{RewardCard, RewardItem, RewardState};
    use crate::state::core::EngineState;

    #[test]
    fn reward_screen_routes_to_card_or_claim_domain_based_on_pending_choice() {
        let mut agent = crate::bot::Agent::new();
        let run_state = crate::state::run::RunState::new(1, 0, false, "Ironclad");

        let mut card_reward = RewardState::new();
        card_reward.pending_card_choice = Some(vec![RewardCard::new(CardId::Strike, 0)]);
        let card_decision =
            agent.decide_policy(&EngineState::RewardScreen(card_reward), &run_state, None, false);
        match card_decision {
            BotPolicyDecision::RewardCard(decision) => {
                assert_eq!(decision.meta.domain, DecisionDomain::RewardCard);
            }
            other => panic!("expected RewardCard decision, got {other:?}"),
        }

        let mut claim_reward = RewardState::new();
        claim_reward.items.push(RewardItem::Gold { amount: 25 });
        let claim_decision =
            agent.decide_policy(&EngineState::RewardScreen(claim_reward), &run_state, None, false);
        match claim_decision {
            BotPolicyDecision::RewardClaim(decision) => {
                assert_eq!(decision.meta.domain, DecisionDomain::RewardClaim);
            }
            other => panic!("expected RewardClaim decision, got {other:?}"),
        }
    }

    #[test]
    fn shop_and_event_room_route_to_typed_domains() {
        let mut agent = crate::bot::Agent::new();

        let run_state = crate::state::run::RunState::new(1, 0, false, "Ironclad");
        let shop_decision =
            agent.decide_policy(&EngineState::Shop(crate::shop::ShopState::new()), &run_state, None, false);
        match shop_decision {
            BotPolicyDecision::Shop(decision) => {
                assert_eq!(decision.meta.domain, DecisionDomain::Shop);
            }
            other => panic!("expected Shop decision, got {other:?}"),
        }

        let mut event_run_state = crate::state::run::RunState::new(1, 0, false, "Ironclad");
        event_run_state.event_state =
            Some(crate::state::events::EventState::new(crate::state::events::EventId::Neow));
        let event_decision =
            agent.decide_policy(&EngineState::EventRoom, &event_run_state, None, false);
        match event_decision {
            BotPolicyDecision::Event(decision) => {
                assert_eq!(decision.meta.domain, DecisionDomain::Event);
            }
            other => panic!("expected Event decision, got {other:?}"),
        }
    }
}
