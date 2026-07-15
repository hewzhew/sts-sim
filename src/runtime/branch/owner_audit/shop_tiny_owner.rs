use sts_simulator::ai::shop_policy_v1::{
    build_shop_decision_context_v1, compile_shop_decision_v1, CompiledShopDecisionV1,
    ShopCompileModeV1, ShopFutureShopV1, ShopMawBankStateV1, ShopPlanStepV1, ShopPolicyConfigV1,
    ShopThreatWindowV1, ShopVisitFactsV1,
};
use sts_simulator::ai::strategy::decision_pipeline::{
    DecisionCandidateKind, DecisionPipelineContext,
};
use sts_simulator::ai::strategy::deck_plan::DeckPlanSnapshot;
use sts_simulator::ai::strategy::reward_admission::{
    assess_reward_admission_from_master_deck, RewardAdmission,
};
use sts_simulator::eval::run_control::{DecisionCandidateKey, DecisionSurface, RunControlSession};
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::state::core::EngineState;

use super::candidate_ir_adapter::shop_tiny_kind;
use super::expansion_policy::shop_tiny_choice_expansion;
use super::owner_candidate_eval::candidate_annotation;
use super::owner_commands::executable_choices;
use super::owner_model::{ChoiceAnnotation, OwnerChoice, OwnerChoiceExpansion};
use super::shop_investment::shop_investment_for_surface;
use super::shop_route_evidence::{future_elite_distance, future_shop_distance};

pub(super) fn shop_tiny_owner_choices(
    session: &RunControlSession,
    surface: &DecisionSurface,
) -> Vec<OwnerChoice> {
    let base_context = shop_tiny_context(session);
    let deck = &session.run_state.master_deck;
    let shop_investment = shop_investment_for_surface(session, surface, deck, base_context);
    let context = shop_investment
        .map(|shop| base_context.with_shop_investment(shop))
        .unwrap_or(base_context);
    let selected_step = compiled_shop_rollout_step(session);
    let choices = executable_choices(surface)
        .into_iter()
        .map(|mut choice| {
            choice.annotation = shop_tiny_candidate_for_choice(context, deck, &choice);
            choice
        })
        .enumerate()
        .collect::<Vec<_>>();
    order_choices_by_compiled_step(choices, selected_step.as_ref())
}

fn order_choices_by_compiled_step(
    mut choices: Vec<(usize, OwnerChoice)>,
    selected_step: Option<&ShopPlanStepV1>,
) -> Vec<OwnerChoice> {
    let selected_choice_index = selected_step.and_then(|step| {
        choices
            .iter()
            .find(|(_, choice)| shop_plan_step_matches_choice(step, choice))
            .map(|(index, _)| *index)
    });
    let mut auto_purge_targets = Vec::new();
    for (index, choice) in choices.iter_mut() {
        if selected_choice_index.is_some() {
            choice.expansion =
                shop_tiny_choice_expansion(&choice.annotation, &mut auto_purge_targets);
        } else {
            choice.expansion = OwnerChoiceExpansion::InspectOnly(
                "compiled shop plan has no executable head on the current surface",
            );
        }
        if selected_choice_index == Some(*index) {
            choice.expansion = OwnerChoiceExpansion::AutoAllowed;
        }
    }
    choices.sort_by_key(|(index, _)| (selected_choice_index != Some(*index), *index));
    choices.into_iter().map(|(_, choice)| choice).collect()
}

fn shop_tiny_context(session: &RunControlSession) -> DecisionPipelineContext {
    let deck_plan = DeckPlanSnapshot::from_run_state(&session.run_state);
    DecisionPipelineContext::shop(deck_plan, session.run_state.gold)
}

fn shop_tiny_candidate_for_choice(
    context: DecisionPipelineContext,
    deck: &[CombatCard],
    choice: &OwnerChoice,
) -> ChoiceAnnotation {
    let kind = shop_tiny_kind(&choice.key);
    candidate_annotation(context, kind, shop_card_admission(deck, kind))
}

fn shop_card_admission(
    deck: &[CombatCard],
    kind: DecisionCandidateKind,
) -> Option<RewardAdmission> {
    if let DecisionCandidateKind::ShopBuyCard { card, upgrades, .. } = kind {
        Some(assess_reward_admission_from_master_deck(
            deck, card, upgrades,
        ))
    } else {
        None
    }
}

fn compiled_shop_rollout_step(session: &RunControlSession) -> Option<ShopPlanStepV1> {
    let EngineState::Shop(shop) = &session.engine_state else {
        return None;
    };
    let base_context = build_shop_decision_context_v1(&session.run_state, shop);
    let visit = shop_visit_facts(session, base_context.need.floors_to_boss);
    let context = base_context.with_visit_facts(visit);
    let compiled = compile_shop_decision_v1(
        &context,
        &ShopPolicyConfigV1::default(),
        ShopCompileModeV1::ExecutePlanHead { max_plans: 8 },
    );
    compiled_rollout_plan(&compiled)?.steps.first().cloned()
}

fn compiled_rollout_plan(
    compiled: &CompiledShopDecisionV1,
) -> Option<&sts_simulator::ai::shop_policy_v1::ShopPlanV1> {
    let projection = compiled.rollout_head.as_ref()?;
    compiled
        .candidate_plans
        .iter()
        .find(|candidate| candidate.plan.plan_id == projection.plan_id)
        .map(|candidate| &candidate.plan)
}

fn shop_visit_facts(session: &RunControlSession, floors_to_boss: i32) -> ShopVisitFactsV1 {
    let visit_context = session.shop_visit_context();
    let maw_bank_live_now = session.run_state.relics.iter().any(|relic| {
        relic.id == sts_simulator::content::relics::RelicId::MawBank && !relic.used_up
    });
    let maw_bank = match visit_context {
        Some(context) if context.maw_bank_live_at_entry && context.spent_gold_in_visit => {
            ShopMawBankStateV1::BrokenThisVisit
        }
        Some(context) if context.maw_bank_live_at_entry => ShopMawBankStateV1::LiveUnspent,
        _ if maw_bank_live_now => ShopMawBankStateV1::LiveUnspent,
        _ => ShopMawBankStateV1::Absent,
    };
    let future_shop = if session.run_state.map.graph.is_empty() {
        ShopFutureShopV1::Unknown
    } else {
        future_shop_distance(session)
            .map(ShopFutureShopV1::VisibleIn)
            .unwrap_or(ShopFutureShopV1::NotVisible)
    };
    let elite_distance = future_elite_distance(session);
    let next_threat = match elite_distance {
        Some(distance) if i32::from(distance) < floors_to_boss => {
            ShopThreatWindowV1::EliteIn(distance)
        }
        _ if floors_to_boss >= 0 => ShopThreatWindowV1::BossIn(floors_to_boss),
        Some(distance) => ShopThreatWindowV1::EliteIn(distance),
        None if session.run_state.map.graph.is_empty() => ShopThreatWindowV1::Unknown,
        None => ShopThreatWindowV1::NoVisibleHardFight,
    };
    ShopVisitFactsV1 {
        entry_gold: visit_context
            .map(|context| context.entry_gold)
            .unwrap_or(session.run_state.gold),
        spent_gold_in_visit: visit_context.is_some_and(|context| context.spent_gold_in_visit),
        maw_bank,
        future_shop,
        next_threat,
    }
}

fn shop_plan_step_matches_choice(step: &ShopPlanStepV1, choice: &OwnerChoice) -> bool {
    match (step, choice.key.as_ref()) {
        (
            ShopPlanStepV1::BuyCard { index, card, cost },
            Some(DecisionCandidateKey::ShopBuyCard {
                shop_slot,
                card: choice_card,
                price,
                ..
            }),
        ) => index == shop_slot && card == choice_card && cost == price,
        (
            ShopPlanStepV1::BuyRelic { index, relic, cost },
            Some(DecisionCandidateKey::ShopBuyRelic {
                shop_slot,
                relic: choice_relic,
                price,
            }),
        ) => index == shop_slot && relic == choice_relic && cost == price,
        (
            ShopPlanStepV1::BuyPotion {
                index,
                potion,
                cost,
            },
            Some(DecisionCandidateKey::ShopBuyPotion {
                shop_slot,
                potion: choice_potion,
                price,
            }),
        ) => index == shop_slot && potion == choice_potion && cost == price,
        (
            ShopPlanStepV1::RemoveCard {
                deck_index, card, ..
            },
            Some(DecisionCandidateKey::ShopPurgeCard {
                deck_index: choice_index,
                card: choice_card,
                ..
            }),
        ) => deck_index == choice_index && card == choice_card,
        (ShopPlanStepV1::LeaveShop, Some(DecisionCandidateKey::ShopLeave)) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::content::cards::CardId;
    use sts_simulator::content::potions::PotionId;
    use sts_simulator::content::relics::RelicId;
    use sts_simulator::eval::run_control::{
        build_decision_surface, DecisionCandidateKey, RunControlConfig,
    };
    use sts_simulator::runtime::combat::CombatCard;
    use sts_simulator::state::core::EngineState;
    use sts_simulator::state::shop::{ShopCard, ShopPotion, ShopRelic, ShopState};

    fn shop_session() -> RunControlSession {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.gold = 224;
        session.run_state.master_deck = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Strike, 2),
            CombatCard::new(CardId::Defend, 3),
            CombatCard::new(CardId::Defend, 4),
            CombatCard::new(CardId::Bash, 5),
            CombatCard::new(CardId::Immolate, 6),
            CombatCard::new(CardId::IronWave, 7),
            CombatCard::new(CardId::Cleave, 8),
            CombatCard::new(CardId::ShrugItOff, 9),
            CombatCard::new(CardId::PommelStrike, 10),
            CombatCard::new(CardId::Bloodletting, 11),
        ];
        session
    }

    #[test]
    fn shop_tiny_prefers_low_hp_waffle_before_cleanup() {
        let mut session = shop_session();
        session.run_state.current_hp = 41;
        session.run_state.max_hp = 85;
        session.run_state.gold = 335;
        let mut shop = ShopState::new();
        shop.purge_cost = 75;
        shop.relics.push(ShopRelic {
            relic_id: RelicId::Waffle,
            price: 155,
            can_buy: true,
            blocked_reason: None,
        });
        shop.potions.push(ShopPotion {
            potion_id: PotionId::FearPotion,
            price: 50,
            can_buy: true,
            blocked_reason: None,
        });
        session.engine_state = EngineState::Shop(shop);

        let surface = build_decision_surface(&session);
        let choices = shop_tiny_owner_choices(&session, &surface);

        assert!(
            matches!(
                choices.first().and_then(|choice| choice.key.as_ref()),
                Some(DecisionCandidateKey::ShopBuyRelic {
                    relic: RelicId::Waffle,
                    price: 155,
                    ..
                })
            ),
            "low HP Waffle should be treated as survival repair before cleanup/potions; got {:?}",
            choices.first().map(|choice| choice.label.as_str())
        );
    }

    #[test]
    fn near_checkpoint_combat_patch_executes_before_optional_cleanup() {
        let mut session = shop_session();
        session.run_state.act_num = 1;
        session.run_state.floor_num = 13;
        session.run_state.gold = 162;
        session.run_state.relics.clear();
        session.run_state.potions = vec![None, None, None];
        let mut shop = ShopState::new();
        shop.cards.push(ShopCard {
            card_id: CardId::FiendFire,
            upgrades: 0,
            price: 152,
            can_buy: true,
            blocked_reason: None,
        });
        shop.cards.push(ShopCard {
            card_id: CardId::Bludgeon,
            upgrades: 0,
            price: 162,
            can_buy: true,
            blocked_reason: None,
        });
        shop.potions.push(ShopPotion {
            potion_id: PotionId::GamblersBrew,
            price: 73,
            can_buy: true,
            blocked_reason: None,
        });
        session.engine_state = EngineState::Shop(shop);

        let surface = build_decision_surface(&session);
        let choices = shop_tiny_owner_choices(&session, &surface);

        assert!(
            matches!(
                choices.first().and_then(|choice| choice.key.as_ref()),
                Some(DecisionCandidateKey::ShopBuyPotion {
                    potion: PotionId::GamblersBrew,
                    price: 73,
                    ..
                })
            ),
            "an admitted near-threat combat patch should execute before optional deck cleanup; got {:?}",
            choices.first().map(|choice| choice.label.as_str())
        );
    }

    #[test]
    fn unmatched_compiled_plan_head_cannot_fall_back_to_legacy_auto_choice() {
        let mut session = shop_session();
        let mut shop = ShopState::new();
        shop.potions.push(ShopPotion {
            potion_id: PotionId::FirePotion,
            price: 50,
            can_buy: true,
            blocked_reason: None,
        });
        session.engine_state = EngineState::Shop(shop);
        let surface = build_decision_surface(&session);
        let choices = executable_choices(&surface)
            .into_iter()
            .enumerate()
            .collect::<Vec<_>>();
        let stale_head = ShopPlanStepV1::BuyRelic {
            index: 0,
            relic: RelicId::Waffle,
            cost: 155,
        };

        let ordered = order_choices_by_compiled_step(choices, Some(&stale_head));

        assert!(ordered
            .iter()
            .all(|choice| matches!(choice.expansion, OwnerChoiceExpansion::InspectOnly(_))));
    }
}
