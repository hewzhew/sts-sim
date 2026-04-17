use crate::bot::agent::Agent;
use crate::bot::combat::{self, CombatDiagnostics};
use crate::bot::deck_ops::{self, DeckOperationKind, DeckOpsAssessment};
use crate::bot::{
    boss_relic, campfire, event, map, reward, shop, BossRelicDecisionDiagnostics,
    CampfireDecisionDiagnostics, EventDecision as EventDomainDecision, MapDecisionDiagnostics,
    RewardClaimDiagnostics, RewardDecisionDiagnostics, ShopDecisionDiagnostics,
};
use crate::rewards::state::{RewardCard, RewardState};
use crate::runtime::combat::CombatState;
use crate::shop::ShopState;
use crate::state::core::{CampfireChoice, ClientInput, EngineState};
use crate::state::run::RunState;
use serde::{Deserialize, Serialize};

pub use crate::bot::reward::BlockedPotionOffer;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionDomain {
    Combat,
    RewardCard,
    RewardClaim,
    Shop,
    Event,
    DeckImprovement,
    Map,
    BossRelic,
    Campfire,
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

#[derive(Clone, Copy, Debug)]
pub struct DeckImprovementDecisionContext<'a> {
    pub run_state: &'a RunState,
    pub operation: DeckOperationKind,
}

#[derive(Clone, Debug)]
pub struct RewardCardDecision {
    pub meta: DecisionMetadata,
    pub action: reward::RewardCardAction,
    pub diagnostics: RewardDecisionDiagnostics,
}

#[derive(Clone, Debug)]
pub struct RewardClaimDecision {
    pub meta: DecisionMetadata,
    pub action: reward::RewardClaimAction,
    pub diagnostics: RewardClaimDiagnostics,
}

#[derive(Clone, Debug)]
pub struct ShopDecision {
    pub meta: DecisionMetadata,
    pub action: shop::ShopAction,
    pub diagnostics: ShopDecisionDiagnostics,
}

#[derive(Clone, Debug)]
pub struct CombatDecision {
    pub meta: DecisionMetadata,
    pub chosen_input: ClientInput,
    pub diagnostics: CombatDiagnostics,
}

#[derive(Clone, Debug)]
pub struct EventDecisionPolicy {
    pub meta: DecisionMetadata,
    pub decision: EventDomainDecision,
}

#[derive(Clone, Debug)]
pub struct DeckImprovementDecision {
    pub meta: DecisionMetadata,
    pub assessment: DeckOpsAssessment,
}

#[derive(Clone, Debug)]
pub struct MapDecision {
    pub meta: DecisionMetadata,
    pub chosen_x: i32,
    pub diagnostics: MapDecisionDiagnostics,
}

#[derive(Clone, Debug)]
pub struct BossRelicDecision {
    pub meta: DecisionMetadata,
    pub chosen_index: usize,
    pub diagnostics: BossRelicDecisionDiagnostics,
}

#[derive(Clone, Debug)]
pub struct CampfireDecision {
    pub meta: DecisionMetadata,
    pub choice: CampfireChoice,
    pub diagnostics: CampfireDecisionDiagnostics,
}

#[derive(Clone, Debug)]
pub enum BotPolicyDecision {
    Combat(CombatDecision),
    RewardCard(RewardCardDecision),
    RewardClaim(RewardClaimDecision),
    Shop(ShopDecision),
    Event(EventDecisionPolicy),
    DeckImprovement(DeckImprovementDecision),
    Map(MapDecision),
    BossRelic(BossRelicDecision),
    Campfire(CampfireDecision),
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
            Self::DeckImprovement(decision) => &decision.meta,
            Self::Map(decision) => &decision.meta,
            Self::BossRelic(decision) => &decision.meta,
            Self::Campfire(decision) => &decision.meta,
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
                let reward_cards = cards
                    .iter()
                    .copied()
                    .map(|card_id| RewardCard::new(card_id, 0))
                    .collect::<Vec<_>>();
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
                    BotPolicyDecision::Combat(self.decide_combat_policy(CombatDecisionContext {
                        engine,
                        combat,
                        verbose,
                    }))
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
                    BotPolicyDecision::RewardCard(self.decide_reward_card_policy(
                        run_state,
                        RewardCardDecisionContext {
                            reward_cards: cards,
                            can_skip: reward.skippable,
                        },
                    ))
                } else {
                    BotPolicyDecision::RewardClaim(self.decide_reward_claim_policy(
                        run_state,
                        RewardClaimDecisionContext {
                            reward,
                            blocked_potion_offers: &[],
                        },
                    ))
                }
            }
            EngineState::Shop(shop_state) => BotPolicyDecision::Shop(
                self.decide_shop_policy(run_state, ShopDecisionContext { shop: shop_state }),
            ),
            EngineState::EventRoom => {
                if let Some(event_state) = &run_state.event_state {
                    BotPolicyDecision::Event(self.decide_event_policy(run_state, event_state))
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
            EngineState::MapNavigation => BotPolicyDecision::Map(self.decide_map_policy(run_state)),
            EngineState::BossRelicSelect(state) => {
                BotPolicyDecision::BossRelic(self.decide_boss_relic_policy(run_state, state))
            }
            EngineState::Campfire => {
                BotPolicyDecision::Campfire(self.decide_campfire_policy(run_state))
            }
            EngineState::RunPendingChoice(choice_state) => BotPolicyDecision::DeckImprovement(
                self.assess_deck_improvement_policy(DeckImprovementDecisionContext {
                    run_state,
                    operation: deck_operation_for_pending_choice(choice_state.reason.clone()),
                }),
            ),
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
        let diagnostics =
            combat::diagnose_root_search_with_depth(ctx.engine, ctx.combat, self.bot_depth(), 4000);
        let chosen_input = diagnostics.chosen_move.clone();
        CombatDecision {
            meta: DecisionMetadata::new(
                DecisionDomain::Combat,
                "combat_baseline",
                Some("projected_turn_close"),
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
        let (action, diagnostics) = reward::decide_cards(run_state, ctx.reward_cards, ctx.can_skip);
        let rationale_key = match action {
            reward::RewardCardAction::Pick(_) => diagnostics.recommended_rationale_key,
            reward::RewardCardAction::Skip => Some(diagnostics.skip_rationale_key),
        };
        RewardCardDecision {
            meta: DecisionMetadata::new(
                DecisionDomain::RewardCard,
                "reward_baseline",
                rationale_key,
                Some((diagnostics.best_score - diagnostics.skip_score).abs() as f32),
                false,
            ),
            action,
            diagnostics,
        }
    }

    pub fn decide_reward_claim_policy(
        &self,
        run_state: &RunState,
        ctx: RewardClaimDecisionContext<'_>,
    ) -> RewardClaimDecision {
        let (action, diagnostics) =
            reward::decide_claim(run_state, ctx.reward, ctx.blocked_potion_offers);
        RewardClaimDecision {
            meta: DecisionMetadata::new(
                DecisionDomain::RewardClaim,
                "reward_claim_baseline",
                Some(diagnostics.rationale_key),
                None,
                false,
            ),
            action,
            diagnostics,
        }
    }

    pub fn decide_shop_policy(
        &self,
        run_state: &RunState,
        ctx: ShopDecisionContext<'_>,
    ) -> ShopDecision {
        let (action, diagnostics) = shop::decide(run_state, ctx.shop);
        let rationale_key = diagnostics
            .top_options
            .first()
            .map(|option| option.rationale_key)
            .or(Some("leave_shop"));
        ShopDecision {
            meta: DecisionMetadata::new(
                DecisionDomain::Shop,
                "shop_baseline",
                rationale_key,
                diagnostics
                    .top_options
                    .first()
                    .map(|option| option.normalized_score as f32),
                false,
            ),
            action,
            diagnostics,
        }
    }

    pub fn decide_event_policy(
        &self,
        run_state: &RunState,
        event_state: &crate::state::events::EventState,
    ) -> EventDecisionPolicy {
        let decision = event::decide_local(run_state, event_state).unwrap_or(EventDomainDecision {
            option_index: 0,
            command_index: 0,
            summary: "fallback option=0".to_string(),
            detail: "fallback option=0".to_string(),
            diagnostics: event::EventDecisionDiagnostics {
                chosen_index: 0,
                fallback_used: true,
                protocol_status: "missing_event_state",
                options: Vec::new(),
                audit: serde_json::json!({"planner":"event_baseline","mode":"fallback"}),
            },
            deck_ops: None,
        });
        EventDecisionPolicy {
            meta: DecisionMetadata::new(
                DecisionDomain::Event,
                "event_baseline",
                Some(
                    decision
                        .diagnostics
                        .options
                        .iter()
                        .find(|option| option.option_index == decision.option_index)
                        .map(|option| option.rationale_key)
                        .unwrap_or("event_baseline"),
                ),
                decision
                    .diagnostics
                    .options
                    .iter()
                    .find(|option| option.option_index == decision.option_index)
                    .map(|option| option.score as f32),
                decision.diagnostics.fallback_used,
            ),
            decision,
        }
    }

    pub fn assess_deck_improvement_policy(
        &self,
        ctx: DeckImprovementDecisionContext<'_>,
    ) -> DeckImprovementDecision {
        let assessment = deck_ops::assess(ctx.run_state, ctx.operation);
        DeckImprovementDecision {
            meta: DecisionMetadata::new(
                DecisionDomain::DeckImprovement,
                "deck_ops_baseline",
                Some(assessment.rationale_key),
                Some(assessment.total_score as f32),
                false,
            ),
            assessment,
        }
    }

    pub fn decide_map_policy(&mut self, run_state: &RunState) -> MapDecision {
        let (chosen_x, diagnostics) = map::decide(run_state).unwrap_or((
            0,
            MapDecisionDiagnostics {
                chosen_x: Some(0),
                chosen_y: None,
                top_options: Vec::new(),
            },
        ));
        MapDecision {
            meta: DecisionMetadata::new(
                DecisionDomain::Map,
                "map_baseline",
                diagnostics
                    .top_options
                    .first()
                    .map(|option| option.rationale_key),
                diagnostics
                    .top_options
                    .first()
                    .map(|option| option.total_score as f32),
                false,
            ),
            chosen_x,
            diagnostics,
        }
    }

    pub fn decide_boss_relic_policy(
        &self,
        run_state: &RunState,
        state: &crate::rewards::state::BossRelicChoiceState,
    ) -> BossRelicDecision {
        let (chosen_index, diagnostics) = boss_relic::decide(run_state, &state.relics);
        BossRelicDecision {
            meta: DecisionMetadata::new(
                DecisionDomain::BossRelic,
                "boss_relic_baseline",
                diagnostics
                    .top_candidates
                    .first()
                    .map(|candidate| candidate.primary_reason),
                diagnostics
                    .top_candidates
                    .first()
                    .map(|candidate| candidate.confidence as f32),
                false,
            ),
            chosen_index,
            diagnostics,
        }
    }

    pub fn decide_campfire_policy(&self, run_state: &RunState) -> CampfireDecision {
        let (choice, diagnostics) = campfire::decide(run_state);
        CampfireDecision {
            meta: DecisionMetadata::new(
                DecisionDomain::Campfire,
                "campfire_baseline",
                diagnostics
                    .top_options
                    .first()
                    .map(|option| option.rationale_key),
                diagnostics
                    .top_options
                    .first()
                    .map(|option| option.score as f32),
                false,
            ),
            choice,
            diagnostics,
        }
    }
}

fn combat_search_confidence(diagnostics: &CombatDiagnostics) -> Option<f32> {
    if diagnostics.top_moves.len() < 2 {
        return diagnostics
            .top_moves
            .first()
            .map(|move_stat| move_stat.avg_score);
    }
    Some(diagnostics.top_moves[0].avg_score - diagnostics.top_moves[1].avg_score)
}

fn deck_operation_for_pending_choice(
    reason: crate::state::core::RunPendingChoiceReason,
) -> DeckOperationKind {
    match reason {
        crate::state::core::RunPendingChoiceReason::Purge => DeckOperationKind::Remove,
        crate::state::core::RunPendingChoiceReason::Upgrade => DeckOperationKind::Upgrade,
        crate::state::core::RunPendingChoiceReason::Transform => DeckOperationKind::Transform {
            count: 1,
            upgraded_context: false,
        },
        crate::state::core::RunPendingChoiceReason::TransformUpgraded => {
            DeckOperationKind::Transform {
                count: 1,
                upgraded_context: true,
            }
        }
        crate::state::core::RunPendingChoiceReason::Duplicate => DeckOperationKind::Duplicate,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BotPolicyDecision, DecisionDomain, DeckImprovementDecisionContext, DeckOperationKind,
    };
    use crate::content::cards::CardId;
    use crate::rewards::state::{RewardCard, RewardItem, RewardState};
    use crate::state::core::EngineState;

    #[test]
    fn reward_screen_routes_to_card_or_claim_domain_based_on_pending_choice() {
        let mut agent = crate::bot::Agent::new();
        let run_state = crate::state::run::RunState::new(1, 0, false, "Ironclad");

        let mut card_reward = RewardState::new();
        card_reward.pending_card_choice = Some(vec![RewardCard::new(CardId::Strike, 0)]);
        let card_decision = agent.decide_policy(
            &EngineState::RewardScreen(card_reward),
            &run_state,
            None,
            false,
        );
        match card_decision {
            BotPolicyDecision::RewardCard(decision) => {
                assert_eq!(decision.meta.domain, DecisionDomain::RewardCard);
            }
            other => panic!("expected RewardCard decision, got {other:?}"),
        }

        let mut claim_reward = RewardState::new();
        claim_reward.items.push(RewardItem::Gold { amount: 25 });
        let claim_decision = agent.decide_policy(
            &EngineState::RewardScreen(claim_reward),
            &run_state,
            None,
            false,
        );
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
        let shop_decision = agent.decide_policy(
            &EngineState::Shop(crate::shop::ShopState::new()),
            &run_state,
            None,
            false,
        );
        match shop_decision {
            BotPolicyDecision::Shop(decision) => {
                assert_eq!(decision.meta.domain, DecisionDomain::Shop);
            }
            other => panic!("expected Shop decision, got {other:?}"),
        }

        let mut event_run_state = crate::state::run::RunState::new(1, 0, false, "Ironclad");
        event_run_state.event_state = Some(crate::state::events::EventState::new(
            crate::state::events::EventId::Neow,
        ));
        let event_decision =
            agent.decide_policy(&EngineState::EventRoom, &event_run_state, None, false);
        match event_decision {
            BotPolicyDecision::Event(decision) => {
                assert_eq!(decision.meta.domain, DecisionDomain::Event);
            }
            other => panic!("expected Event decision, got {other:?}"),
        }
    }

    #[test]
    fn deck_improvement_policy_exposes_typed_domain() {
        let agent = crate::bot::Agent::new();
        let mut run_state = crate::state::run::RunState::new(1, 0, false, "Ironclad");
        run_state
            .master_deck
            .push(crate::runtime::combat::CombatCard::new(
                CardId::Parasite,
                73_001,
            ));
        let decision = agent.assess_deck_improvement_policy(DeckImprovementDecisionContext {
            run_state: &run_state,
            operation: DeckOperationKind::Remove,
        });
        assert_eq!(decision.meta.domain, DecisionDomain::DeckImprovement);
        assert_eq!(
            decision
                .assessment
                .best_candidate
                .and_then(|candidate| candidate.target_uuid),
            Some(73_001)
        );
    }
}
