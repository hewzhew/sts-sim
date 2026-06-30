use sts_simulator::ai::strategy::decision_pipeline::{CleanupTarget, DecisionCandidateKind};
use sts_simulator::content::cards::{
    get_card_definition, is_starter_basic, is_starter_defend, is_starter_strike, CardId, CardType,
};
use sts_simulator::eval::run_control::DecisionCandidateKey;

pub(super) fn card_reward_kind(
    key: &Option<DecisionCandidateKey>,
) -> Option<DecisionCandidateKind> {
    match key {
        Some(DecisionCandidateKey::CardRewardPick { card, upgrades, .. }) => {
            Some(DecisionCandidateKind::CardRewardPick {
                card: *card,
                upgrades: *upgrades,
            })
        }
        Some(DecisionCandidateKey::CardRewardSkip { .. }) => {
            Some(DecisionCandidateKind::CardRewardSkip)
        }
        _ => None,
    }
}

pub(super) fn shop_tiny_kind(key: &Option<DecisionCandidateKey>) -> DecisionCandidateKind {
    match key {
        Some(DecisionCandidateKey::ShopBuyCard {
            card,
            upgrades,
            price,
            ..
        }) => DecisionCandidateKind::ShopBuyCard {
            card: *card,
            upgrades: *upgrades,
            price: *price,
        },
        Some(DecisionCandidateKey::ShopBuyRelic { relic, price, .. }) => {
            DecisionCandidateKind::ShopBuyRelic {
                relic: *relic,
                price: *price,
            }
        }
        Some(DecisionCandidateKey::ShopBuyPotion { potion, price, .. }) => {
            DecisionCandidateKind::ShopBuyPotion {
                potion: *potion,
                price: *price,
            }
        }
        Some(DecisionCandidateKey::ShopPurgeCard { card, .. }) => {
            DecisionCandidateKind::ShopPurge {
                target: classify_shop_purge_target(*card),
            }
        }
        Some(DecisionCandidateKey::ShopOpenRewards) => DecisionCandidateKind::ShopOpenRewards,
        Some(DecisionCandidateKey::ShopLeave) => DecisionCandidateKind::ShopLeave,
        _ => DecisionCandidateKind::Unsupported,
    }
}

pub(super) fn boss_relic_kind(key: &Option<DecisionCandidateKey>) -> DecisionCandidateKind {
    match key {
        Some(DecisionCandidateKey::BossRelicPick { relic, .. }) => {
            DecisionCandidateKind::BossRelicPick { relic: *relic }
        }
        Some(DecisionCandidateKey::BossRelicSkip) => DecisionCandidateKind::BossRelicSkip,
        _ => DecisionCandidateKind::Unsupported,
    }
}

pub(super) fn is_card_reward_key(key: &Option<DecisionCandidateKey>) -> bool {
    matches!(
        key,
        Some(
            DecisionCandidateKey::CardRewardOpen { .. }
                | DecisionCandidateKey::CardRewardPick { .. }
                | DecisionCandidateKey::CardRewardSingingBowl { .. }
                | DecisionCandidateKey::CardRewardSkip { .. }
        )
    )
}

pub(super) fn is_boss_relic_key(key: &Option<DecisionCandidateKey>) -> bool {
    matches!(
        key,
        Some(DecisionCandidateKey::BossRelicPick { .. } | DecisionCandidateKey::BossRelicSkip)
    )
}

fn classify_shop_purge_target(card: CardId) -> CleanupTarget {
    let definition = get_card_definition(card);
    match definition.card_type {
        CardType::Curse => CleanupTarget::Curse,
        CardType::Status => CleanupTarget::Status,
        _ if is_starter_strike(card) => CleanupTarget::StarterStrike,
        _ if is_starter_defend(card) => CleanupTarget::StarterDefend,
        _ if is_starter_basic(card) => CleanupTarget::OtherStarter,
        _ => CleanupTarget::Other,
    }
}
