use crate::ai::noncombat_strategy_v1::{
    build_run_strategy_snapshot_from_run_state_v2, StrategyPackageIdV2, StrategyPlanSupportV1,
};
use crate::content::cards::{get_card_definition, CardTag, CardType};
use crate::state::core::{ClientInput, EngineState};
use crate::state::shop::ShopState;

use super::session::{RunControlCommandOutcome, RunControlSession};

pub(super) fn apply_shop_policy_purge(
    session: &mut RunControlSession,
) -> Result<Option<(RunControlCommandOutcome, String)>, String> {
    let Some((deck_idx, reason)) = shop_purge_target(session) else {
        return Ok(None);
    };
    let card_name = get_card_definition(session.run_state.master_deck[deck_idx].id).name;
    let summary = format!("shop policy: purge {card_name} [{reason}]");
    let outcome = session.apply_input(ClientInput::PurgeCard(deck_idx))?;
    Ok(Some((outcome, summary)))
}

fn shop_purge_target(session: &RunControlSession) -> Option<(usize, &'static str)> {
    let EngineState::Shop(shop) = &session.engine_state else {
        return None;
    };
    if !shop.purge_available || session.run_state.gold < shop.purge_cost {
        return None;
    }

    if let Some(idx) = session
        .run_state
        .master_deck
        .iter()
        .position(|card| purge_eligible(session, card) && is_curse(card.id))
    {
        return Some((idx, "curse cleanup"));
    }

    if !starter_cleanup_allowed(session, shop) {
        return None;
    }
    session
        .run_state
        .master_deck
        .iter()
        .position(|card| purge_eligible(session, card) && is_starter_strike(card.id))
        .map(|idx| (idx, "CorePlanProtection Strong"))
}

fn purge_eligible(session: &RunControlSession, card: &crate::runtime::combat::CombatCard) -> bool {
    crate::state::core::master_deck_card_is_purgeable(card)
        && !crate::state::core::master_deck_card_is_bottled(card, &session.run_state.relics)
}

fn is_curse(card_id: crate::content::cards::CardId) -> bool {
    get_card_definition(card_id).card_type == CardType::Curse
}

fn is_starter_strike(card_id: crate::content::cards::CardId) -> bool {
    get_card_definition(card_id)
        .tags
        .contains(&CardTag::StarterStrike)
}

fn starter_cleanup_allowed(session: &RunControlSession, shop: &ShopState) -> bool {
    if affordable_purchase_exists(shop, session.run_state.gold) {
        return false;
    }

    let snapshot = build_run_strategy_snapshot_from_run_state_v2(&session.run_state);
    let core_plan = snapshot.support(StrategyPackageIdV2::CorePlanProtection);
    if core_plan != StrategyPlanSupportV1::Strong {
        return false;
    }

    let patch_window = snapshot.support(StrategyPackageIdV2::CombatPatchWindow);
    !matches!(
        patch_window,
        StrategyPlanSupportV1::Strong | StrategyPlanSupportV1::Plausible
    )
}

fn affordable_purchase_exists(shop: &ShopState, gold: i32) -> bool {
    shop.cards
        .iter()
        .any(|card| card.can_buy && card.price <= gold)
        || shop
            .relics
            .iter()
            .any(|relic| relic.can_buy && relic.price <= gold)
        || shop
            .potions
            .iter()
            .any(|potion| potion.can_buy && potion.price <= gold)
}
