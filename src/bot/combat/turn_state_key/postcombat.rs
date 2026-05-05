use crate::rewards::state::{BossRelicChoiceState, RewardCard, RewardItem, RewardState};
use crate::runtime::combat::{CombatMeta, CombatState, MetaChange, PlayerEntity};
use crate::shop::{ShopCard, ShopPotion, ShopRelic, ShopState};
use crate::state::core::{EventCombatState, PostCombatReturn, RunPendingChoiceState, RunResult};
use crate::state::EngineState;

use super::types::{
    StableBossRelicKey, StableEventCombatKey, StableMetaChangeKey, StableMetaKey,
    StablePostCombatReturnKey, StablePostcombatPlayerKey, StablePostcombatRuntimeKey,
    StableRewardCardKey, StableRewardItemKey, StableRewardKey, StableRunPendingChoiceKey,
    StableRunPendingReturnKey, StableShopKey, StableShopRowKey,
};

pub(super) fn stable_postcombat_player_key(player: &PlayerEntity) -> StablePostcombatPlayerKey {
    StablePostcombatPlayerKey {
        current_hp: player.current_hp,
        max_hp: player.max_hp,
        gold: player.gold,
        relics: format!("{:?}", player.relics),
        energy_master: player.energy_master,
    }
}

pub(super) fn stable_meta_key(meta: &CombatMeta) -> StableMetaKey {
    StableMetaKey {
        player_class: meta.player_class.to_string(),
        ascension_level: meta.ascension_level,
        is_boss_fight: meta.is_boss_fight,
        is_elite_fight: meta.is_elite_fight,
        meta_changes: meta
            .meta_changes
            .iter()
            .map(stable_meta_change_key)
            .collect(),
    }
}

pub(super) fn stable_postcombat_runtime_key(combat: &CombatState) -> StablePostcombatRuntimeKey {
    let mut pending_rewards = combat
        .runtime
        .pending_rewards
        .iter()
        .map(stable_reward_item_key)
        .collect::<Vec<_>>();
    pending_rewards.sort();

    StablePostcombatRuntimeKey {
        pending_rewards,
        combat_mugged: combat.runtime.combat_mugged,
        combat_smoked: combat.runtime.combat_smoked,
    }
}

pub(super) fn stable_reward_key(state: &RewardState) -> StableRewardKey {
    let mut items = state
        .items
        .iter()
        .map(stable_reward_item_key)
        .collect::<Vec<_>>();
    items.sort();

    let mut pending_card_choice = state
        .pending_card_choice
        .as_ref()
        .map(|cards| cards.iter().map(stable_reward_card_key).collect::<Vec<_>>())
        .unwrap_or_default();
    pending_card_choice.sort();

    StableRewardKey {
        screen_context: format!("{:?}", state.screen_context),
        skippable: state.skippable,
        items,
        pending_card_choice,
    }
}

pub(super) fn stable_shop_key(state: &ShopState) -> StableShopKey {
    let mut cards = state
        .cards
        .iter()
        .map(stable_shop_card_key)
        .collect::<Vec<_>>();
    let mut relics = state
        .relics
        .iter()
        .map(stable_shop_relic_key)
        .collect::<Vec<_>>();
    let mut potions = state
        .potions
        .iter()
        .map(stable_shop_potion_key)
        .collect::<Vec<_>>();
    cards.sort();
    relics.sort();
    potions.sort();

    StableShopKey {
        purge_cost: state.purge_cost,
        purge_available: state.purge_available,
        cards,
        relics,
        potions,
    }
}

pub(super) fn stable_run_pending_choice_key(
    state: &RunPendingChoiceState,
) -> StableRunPendingChoiceKey {
    StableRunPendingChoiceKey {
        min_choices: state.min_choices,
        max_choices: state.max_choices,
        reason: format!("{:?}", state.reason),
        return_state: stable_run_pending_return_key(&state.return_state),
    }
}

pub(super) fn stable_run_pending_return_key(state: &EngineState) -> StableRunPendingReturnKey {
    match state {
        EngineState::RewardScreen(reward) => {
            StableRunPendingReturnKey::Reward(stable_reward_key(reward))
        }
        EngineState::Campfire => StableRunPendingReturnKey::Campfire,
        EngineState::Shop(shop) => StableRunPendingReturnKey::Shop(stable_shop_key(shop)),
        EngineState::MapNavigation => StableRunPendingReturnKey::MapNavigation,
        EngineState::EventRoom => StableRunPendingReturnKey::EventRoom,
        EngineState::BossRelicSelect(state) => {
            StableRunPendingReturnKey::BossRelic(stable_boss_relic_key(state))
        }
        EngineState::RunPendingChoice(state) => StableRunPendingReturnKey::RunPendingChoice(
            Box::new(stable_run_pending_choice_key(state)),
        ),
        EngineState::GameOver(result) => {
            StableRunPendingReturnKey::GameOver(stable_run_result_signature(result))
        }
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::PendingChoice(_)
        | EngineState::EventCombat(_) => StableRunPendingReturnKey::Combat,
    }
}

pub(super) fn stable_event_combat_key(state: &EventCombatState) -> StableEventCombatKey {
    StableEventCombatKey {
        encounter_key: state.encounter_key.to_string(),
        reward_allowed: state.reward_allowed,
        no_cards_in_rewards: state.no_cards_in_rewards,
        post_combat_return: stable_post_combat_return_key(&state.post_combat_return),
        rewards: stable_reward_key(&state.rewards),
    }
}

pub(super) fn stable_boss_relic_key(state: &BossRelicChoiceState) -> StableBossRelicKey {
    let mut relics = state
        .relics
        .iter()
        .map(|relic| format!("{relic:?}"))
        .collect::<Vec<_>>();
    relics.sort();
    StableBossRelicKey { relics }
}

pub(super) fn stable_run_result_signature(result: &RunResult) -> &'static str {
    match result {
        RunResult::Victory => "victory",
        RunResult::Defeat => "defeat",
    }
}

fn stable_reward_item_key(item: &RewardItem) -> StableRewardItemKey {
    match item {
        RewardItem::Gold { amount } => StableRewardItemKey::Gold(*amount),
        RewardItem::StolenGold { amount } => StableRewardItemKey::StolenGold(*amount),
        RewardItem::Card { cards } => {
            let mut cards = cards.iter().map(stable_reward_card_key).collect::<Vec<_>>();
            cards.sort();
            StableRewardItemKey::Card(cards)
        }
        RewardItem::Relic { relic_id } => StableRewardItemKey::Relic(format!("{relic_id:?}")),
        RewardItem::Potion { potion_id } => StableRewardItemKey::Potion(format!("{potion_id:?}")),
        RewardItem::EmeraldKey => StableRewardItemKey::EmeraldKey,
        RewardItem::SapphireKey => StableRewardItemKey::SapphireKey,
    }
}

fn stable_reward_card_key(card: &RewardCard) -> StableRewardCardKey {
    StableRewardCardKey {
        id: format!("{:?}", card.id),
        upgrades: card.upgrades,
    }
}

fn stable_shop_card_key(card: &ShopCard) -> StableShopRowKey {
    StableShopRowKey {
        id: format!("{:?}", card.card_id),
        price: card.price,
        can_buy: card.can_buy,
        blocked_reason: card.blocked_reason.clone(),
    }
}

fn stable_shop_relic_key(relic: &ShopRelic) -> StableShopRowKey {
    StableShopRowKey {
        id: format!("{:?}", relic.relic_id),
        price: relic.price,
        can_buy: relic.can_buy,
        blocked_reason: relic.blocked_reason.clone(),
    }
}

fn stable_shop_potion_key(potion: &ShopPotion) -> StableShopRowKey {
    StableShopRowKey {
        id: format!("{:?}", potion.potion_id),
        price: potion.price,
        can_buy: potion.can_buy,
        blocked_reason: potion.blocked_reason.clone(),
    }
}

fn stable_post_combat_return_key(value: &PostCombatReturn) -> StablePostCombatReturnKey {
    match value {
        PostCombatReturn::EventRoom => StablePostCombatReturnKey::EventRoom,
        PostCombatReturn::MapNavigation => StablePostCombatReturnKey::MapNavigation,
    }
}

fn stable_meta_change_key(change: &MetaChange) -> StableMetaChangeKey {
    match change {
        MetaChange::AddCardToMasterDeck(card) => {
            StableMetaChangeKey::AddCardToMasterDeck(format!("{card:?}"))
        }
    }
}
