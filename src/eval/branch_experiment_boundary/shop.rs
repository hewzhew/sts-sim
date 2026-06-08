use crate::ai::shop_policy_v1::{
    build_shop_decision_context_v1, plan_shop_decision_v1, ShopPolicyActionV1, ShopPolicyConfigV1,
};
use crate::content::cards::{get_card_definition, CardId};
use crate::content::potions::get_potion_definition;
use crate::eval::run_control::RunControlSession;
use crate::state::core::EngineState;
use crate::state::shop::ShopState;

const MAX_SHOP_PURCHASE_OPTIONS_PER_BRANCH: usize = 4;

#[derive(Clone, Debug)]
pub(crate) struct ShopBranchOption {
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) card: Option<CardId>,
    pub(crate) effect_kind: String,
    pub(crate) effect_label: String,
    pub(crate) representative_count: usize,
    pub(crate) suppressed_count: usize,
}

pub(crate) fn shop_branch_options(session: &RunControlSession) -> Option<Vec<ShopBranchOption>> {
    let EngineState::Shop(shop) = &session.engine_state else {
        return None;
    };
    if shop.pending_reward_overlay.is_some() {
        return None;
    }

    let context = build_shop_decision_context_v1(&session.run_state, shop);
    let decision = plan_shop_decision_v1(&context, &ShopPolicyConfigV1::default());
    match decision.action {
        ShopPolicyActionV1::Purge {
            deck_index,
            card,
            confidence,
            reason,
        } => {
            let card_name = get_card_definition(card).name;
            Some(vec![ShopBranchOption {
                label: format!("Purge {card_name}"),
                command: format!("purge {deck_index}"),
                card: Some(card),
                effect_kind: "shop_purge".to_string(),
                effect_label: format!("Purge {card_name} | confidence={confidence:.2} | {reason}"),
                representative_count: 1,
                suppressed_count: 0,
            }])
        }
        ShopPolicyActionV1::Stop { .. } if !context.affordable_purchase_exists => {
            Some(vec![ShopBranchOption {
                label: "Leave shop".to_string(),
                command: "leave".to_string(),
                card: None,
                effect_kind: "shop_leave".to_string(),
                effect_label: "Leave shop | no affordable purchase".to_string(),
                representative_count: 1,
                suppressed_count: 0,
            }])
        }
        ShopPolicyActionV1::Stop { .. } => low_fanout_purchase_branch_options(shop, session),
    }
}

fn low_fanout_purchase_branch_options(
    shop: &ShopState,
    session: &RunControlSession,
) -> Option<Vec<ShopBranchOption>> {
    let mut options = Vec::new();
    for (idx, card) in shop.cards.iter().enumerate() {
        if card.can_buy && card.price <= session.run_state.gold {
            let card_name = get_card_definition(card.card_id).name;
            options.push(ShopBranchOption {
                label: format!("Buy {card_name}"),
                command: format!("buy card {idx}"),
                card: Some(card.card_id),
                effect_kind: "shop_buy_card".to_string(),
                effect_label: format!("Buy {card_name} | {} gold", card.price),
                representative_count: 1,
                suppressed_count: 0,
            });
        }
    }
    for (idx, relic) in shop.relics.iter().enumerate() {
        if relic.can_buy && relic.price <= session.run_state.gold {
            options.push(ShopBranchOption {
                label: format!("Buy {:?}", relic.relic_id),
                command: format!("buy relic {idx}"),
                card: None,
                effect_kind: "shop_buy_relic".to_string(),
                effect_label: format!("Buy {:?} | {} gold", relic.relic_id, relic.price),
                representative_count: 1,
                suppressed_count: 0,
            });
        }
    }
    for (idx, potion) in shop.potions.iter().enumerate() {
        if potion.can_buy && potion.price <= session.run_state.gold {
            let potion_name = get_potion_definition(potion.potion_id).name;
            options.push(ShopBranchOption {
                label: format!("Buy {potion_name}"),
                command: format!("buy potion {idx}"),
                card: None,
                effect_kind: "shop_buy_potion".to_string(),
                effect_label: format!("Buy {potion_name} potion | {} gold", potion.price),
                representative_count: 1,
                suppressed_count: 0,
            });
        }
    }

    if options.is_empty() {
        return None;
    }
    let mut options = select_shop_purchase_portfolio(options);
    options.push(ShopBranchOption {
        label: "Leave shop".to_string(),
        command: "leave".to_string(),
        card: None,
        effect_kind: "shop_leave".to_string(),
        effect_label: "Leave shop | decline selected shop purchase portfolio".to_string(),
        representative_count: 1,
        suppressed_count: 0,
    });
    Some(options)
}

fn select_shop_purchase_portfolio(options: Vec<ShopBranchOption>) -> Vec<ShopBranchOption> {
    if options.len() <= MAX_SHOP_PURCHASE_OPTIONS_PER_BRANCH {
        return options;
    }

    let mut selected_indices = Vec::<usize>::new();
    for effect_kind in ["shop_buy_card", "shop_buy_relic", "shop_buy_potion"] {
        if let Some(index) = options
            .iter()
            .position(|option| option.effect_kind == effect_kind)
        {
            selected_indices.push(index);
        }
    }
    for index in 0..options.len() {
        if selected_indices.len() >= MAX_SHOP_PURCHASE_OPTIONS_PER_BRANCH {
            break;
        }
        if !selected_indices.contains(&index) {
            selected_indices.push(index);
        }
    }
    selected_indices.sort_unstable();

    let suppressed_count = options.len().saturating_sub(selected_indices.len());
    selected_indices
        .into_iter()
        .enumerate()
        .map(|(selected_position, index)| {
            let mut option = options[index].clone();
            if selected_position == 0 && suppressed_count > 0 {
                option.suppressed_count = suppressed_count;
                option.effect_label = format!(
                    "{} | shop portfolio cap suppressed {suppressed_count} affordable purchase(s)",
                    option.effect_label
                );
            }
            option
        })
        .collect()
}
