use crate::ai::shop_policy_v1::{
    build_shop_decision_context_v1, plan_shop_decision_v1, ShopPolicyActionV1, ShopPolicyConfigV1,
};
use crate::content::cards::{get_card_definition, CardId};
use crate::eval::run_control::RunControlSession;
use crate::state::core::EngineState;

pub(crate) struct ShopBranchOption {
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) card: Option<CardId>,
    pub(crate) effect_kind: String,
    pub(crate) effect_label: String,
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
            }])
        }
        ShopPolicyActionV1::Stop { .. } if !context.affordable_purchase_exists => {
            Some(vec![ShopBranchOption {
                label: "Leave shop".to_string(),
                command: "leave".to_string(),
                card: None,
                effect_kind: "shop_leave".to_string(),
                effect_label: "Leave shop | no affordable purchase".to_string(),
            }])
        }
        ShopPolicyActionV1::Stop { .. } => None,
    }
}
