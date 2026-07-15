use sts_simulator::ai::strategy::decision_pipeline::{
    CandidateOrderKey, DecisionCandidateKind, DecisionPipelineContext,
};
use sts_simulator::ai::strategy::deck_plan::DeckPlanSnapshot;
use sts_simulator::ai::strategy::reward_admission::{
    assess_reward_admission_from_master_deck, RewardAdmission,
};
use sts_simulator::ai::strategy::shop_boss_preview::shop_boss_preview_bundles;
use sts_simulator::eval::run_control::{DecisionCandidateKey, DecisionSurface, RunControlSession};
use sts_simulator::runtime::combat::CombatCard;

use super::candidate_ir_adapter::shop_tiny_kind;
use super::expansion_policy::shop_tiny_choice_expansion;
use super::owner_candidate_eval::candidate_annotation;
use super::owner_commands::executable_choices;
use super::owner_model::{ChoiceAnnotation, OwnerChoice, OwnerChoiceExpansion};
use super::shop_investment::shop_investment_for_surface;

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
    let mut choices = executable_choices(surface)
        .into_iter()
        .map(|mut choice| {
            choice.annotation = shop_tiny_candidate_for_choice(context, deck, &choice);
            choice
        })
        .enumerate()
        .collect::<Vec<_>>();
    let preferred_bundle_next = preferred_shop_boss_bundle_next_kind(session, &choices);
    let mut auto_purge_targets = Vec::new();
    for (_, choice) in choices.iter_mut() {
        choice.expansion = shop_tiny_choice_expansion(&choice.annotation, &mut auto_purge_targets);
        if preferred_bundle_next.is_some_and(|preferred| shop_tiny_kind(&choice.key) == preferred) {
            choice.expansion = OwnerChoiceExpansion::AutoAllowed;
        }
    }
    choices.sort_by_key(|(index, choice)| {
        (
            bundle_execution_rank(&preferred_bundle_next, &choice.key),
            shop_tiny_choice_rank(choice),
            *index,
        )
    });
    choices.into_iter().map(|(_, choice)| choice).collect()
}

fn shop_tiny_context(session: &RunControlSession) -> DecisionPipelineContext {
    let deck_plan = DeckPlanSnapshot::from_run_state(&session.run_state);
    DecisionPipelineContext::shop(deck_plan, session.run_state.gold)
}

fn hard_checkpoint_imminent(session: &RunControlSession) -> bool {
    session.run_state.act_num == 1 && session.run_state.floor_num >= 13
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

fn shop_tiny_choice_rank(choice: &OwnerChoice) -> (u8, CandidateOrderKey) {
    match &choice.annotation {
        ChoiceAnnotation::Candidate(decision) => decision.evaluation.auto_order_key(false),
        _ => (u8::MAX, CandidateOrderKey::fallback()),
    }
}

fn preferred_shop_boss_bundle_next_kind(
    session: &RunControlSession,
    choices: &[(usize, OwnerChoice)],
) -> Option<DecisionCandidateKind> {
    if !hard_checkpoint_imminent(session) {
        return None;
    }
    let executable_kinds = choices
        .iter()
        .map(|(_, choice)| shop_tiny_kind(&choice.key))
        .collect::<Vec<_>>();
    let bundle = shop_boss_preview_bundles(executable_kinds, session.run_state.gold, 2)
        .into_iter()
        .find(|bundle| !bundle.items.is_empty())?;
    bundle.items.into_iter().find(|kind| {
        choices
            .iter()
            .any(|(_, choice)| shop_tiny_kind(&choice.key) == *kind)
    })
}

fn bundle_execution_rank(
    preferred_bundle_next: &Option<DecisionCandidateKind>,
    key: &Option<DecisionCandidateKey>,
) -> u8 {
    match preferred_bundle_next {
        Some(kind) if shop_tiny_kind(key) == *kind => 0,
        Some(_) => 1,
        None => 0,
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
    fn hard_checkpoint_bundle_next_item_preempts_random_potion_spend() {
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
                Some(DecisionCandidateKey::ShopBuyCard {
                    card: CardId::FiendFire,
                    price: 152,
                    ..
                })
            ),
            "hard checkpoint bundle execution should buy Fiend Fire before spending on random/deferred potions; got {:?}",
            choices.first().map(|choice| choice.label.as_str())
        );
    }
}
