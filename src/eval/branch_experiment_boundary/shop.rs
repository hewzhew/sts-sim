use crate::ai::shop_policy_v1::{
    build_shop_decision_context_v1, plan_shop_decision_v1, shop_card_conversion_priority_v1,
    shop_conversion_pressure_v1, shop_potion_conversion_priority_for_v1,
    shop_relic_conversion_priority_v1, ShopPolicyActionV1, ShopPolicyConfigV1,
};
use crate::content::cards::{get_card_definition, CardId};
use crate::content::potions::get_potion_definition;
use crate::content::relics::RelicId;
use crate::eval::run_control::RunControlSession;
use crate::state::core::EngineState;
use crate::state::shop::ShopState;

const MAX_SHOP_PURCHASE_OPTIONS_PER_BRANCH: usize = 4;
const SHOP_COMMAND_SEQUENCE_SEPARATOR: &str = " && ";

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
        ShopPolicyActionV1::Purchase { .. } => low_fanout_purchase_branch_options(shop, session),
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
            options.push(ScoredShopBranchOption {
                score: shop_card_conversion_priority_v1(card.card_id, &session.run_state),
                cost: card.price,
                can_start_combo: true,
                can_follow_combo: true,
                option: ShopBranchOption {
                    label: format!("Buy {card_name}"),
                    command: format!("buy card {idx}"),
                    card: Some(card.card_id),
                    effect_kind: "shop_buy_card".to_string(),
                    effect_label: format!("Buy {card_name} | {} gold", card.price),
                    representative_count: 1,
                    suppressed_count: 0,
                },
            });
        }
    }
    for (idx, relic) in shop.relics.iter().enumerate() {
        if relic.can_buy && relic.price <= session.run_state.gold {
            options.push(ScoredShopBranchOption {
                score: shop_relic_conversion_priority_v1(relic.relic_id),
                cost: relic.price,
                can_start_combo: shop_relic_purchase_keeps_shop_open(relic.relic_id),
                can_follow_combo: false,
                option: ShopBranchOption {
                    label: format!("Buy {:?}", relic.relic_id),
                    command: format!("buy relic {idx}"),
                    card: None,
                    effect_kind: "shop_buy_relic".to_string(),
                    effect_label: format!("Buy {:?} | {} gold", relic.relic_id, relic.price),
                    representative_count: 1,
                    suppressed_count: 0,
                },
            });
        }
    }
    for (idx, potion) in shop.potions.iter().enumerate() {
        if potion.can_buy && potion.price <= session.run_state.gold {
            let potion_name = get_potion_definition(potion.potion_id).name;
            options.push(ScoredShopBranchOption {
                score: shop_potion_conversion_priority_for_v1(potion.potion_id, &session.run_state),
                cost: potion.price,
                can_start_combo: true,
                can_follow_combo: true,
                option: ShopBranchOption {
                    label: format!("Buy {potion_name}"),
                    command: format!("buy potion {idx}"),
                    card: None,
                    effect_kind: "shop_buy_potion".to_string(),
                    effect_label: format!("Buy {potion_name} potion | {} gold", potion.price),
                    representative_count: 1,
                    suppressed_count: 0,
                },
            });
        }
    }

    if options.is_empty() {
        return None;
    }
    let mut options = select_shop_purchase_portfolio(
        options,
        session.run_state.gold,
        shop_combo_pressure(&session.run_state, shop),
    );
    if !shop_conversion_pressure_v1(&session.run_state, shop) {
        options.push(ShopBranchOption {
            label: "Leave shop".to_string(),
            command: "leave".to_string(),
            card: None,
            effect_kind: "shop_leave".to_string(),
            effect_label: "Leave shop | decline selected shop purchase portfolio".to_string(),
            representative_count: 1,
            suppressed_count: 0,
        });
    }
    Some(options)
}

#[derive(Clone, Debug)]
struct ScoredShopBranchOption {
    score: i32,
    cost: i32,
    can_start_combo: bool,
    can_follow_combo: bool,
    option: ShopBranchOption,
}

fn select_shop_purchase_portfolio(
    mut options: Vec<ScoredShopBranchOption>,
    gold: i32,
    conversion_pressure: bool,
) -> Vec<ShopBranchOption> {
    options.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.option.command.cmp(&right.option.command))
    });

    let combo = conversion_pressure
        .then(|| best_shop_combo_option(&options, gold))
        .flatten();
    if options.len() <= MAX_SHOP_PURCHASE_OPTIONS_PER_BRANCH && combo.is_none() {
        return options.into_iter().map(|entry| entry.option).collect();
    }

    let mut selected = Vec::<ShopBranchOption>::new();
    let mut represented_commands = std::collections::BTreeSet::<String>::new();
    if let Some(combo) = combo {
        selected.push(combo.option);
    }

    for effect_kind in ["shop_buy_relic", "shop_buy_card", "shop_buy_potion"] {
        if selected.len() >= MAX_SHOP_PURCHASE_OPTIONS_PER_BRANCH {
            break;
        }
        if let Some(entry) = options.iter().find(|entry| {
            entry.option.effect_kind == effect_kind
                && !represented_commands.contains(&entry.option.command)
        }) {
            represented_commands.insert(entry.option.command.clone());
            selected.push(entry.option.clone());
        }
    }
    for entry in &options {
        if selected.len() >= MAX_SHOP_PURCHASE_OPTIONS_PER_BRANCH {
            break;
        }
        if represented_commands.insert(entry.option.command.clone()) {
            selected.push(entry.option.clone());
        }
    }

    let suppressed_count = options.len().saturating_sub(represented_commands.len());
    if suppressed_count > 0 {
        if let Some(option) = selected.first_mut() {
            option.suppressed_count = suppressed_count;
            option.effect_label = format!(
                "{} | shop portfolio cap suppressed {suppressed_count} affordable purchase(s)",
                option.effect_label
            );
        }
    }
    selected
}

fn shop_combo_pressure(run_state: &crate::state::run::RunState, shop: &ShopState) -> bool {
    let affordable_count = affordable_purchase_count(shop, run_state.gold);
    affordable_count >= 3
        && (affordable_count > MAX_SHOP_PURCHASE_OPTIONS_PER_BRANCH
            || (run_state.gold >= 300 && shop_conversion_pressure_v1(run_state, shop)))
}

fn affordable_purchase_count(shop: &ShopState, gold: i32) -> usize {
    shop.cards
        .iter()
        .filter(|card| card.can_buy && card.price <= gold)
        .count()
        + shop
            .relics
            .iter()
            .filter(|relic| relic.can_buy && relic.price <= gold)
            .count()
        + shop
            .potions
            .iter()
            .filter(|potion| potion.can_buy && potion.price <= gold)
            .count()
}

fn best_shop_combo_option(
    options: &[ScoredShopBranchOption],
    gold: i32,
) -> Option<ScoredShopBranchOption> {
    let mut best = None::<ScoredShopBranchOption>;
    for first in options.iter().filter(|entry| entry.can_start_combo) {
        for second in options.iter().filter(|entry| {
            entry.can_follow_combo
                && entry.option.command != first.option.command
                && entry.option.effect_kind != first.option.effect_kind
        }) {
            if first.cost.saturating_add(second.cost) > gold {
                continue;
            }
            let candidate = shop_combo_option(first, second);
            if best
                .as_ref()
                .is_none_or(|current| candidate.score > current.score)
            {
                best = Some(candidate);
            }
        }
    }
    best
}

fn shop_combo_option(
    first: &ScoredShopBranchOption,
    second: &ScoredShopBranchOption,
) -> ScoredShopBranchOption {
    let score = first.score.saturating_add(second.score).saturating_add(100);
    let cost = first.cost.saturating_add(second.cost);
    ScoredShopBranchOption {
        score,
        cost,
        can_start_combo: false,
        can_follow_combo: false,
        option: ShopBranchOption {
            label: format!("{} + {}", first.option.label, second.option.label),
            command: format!(
                "{}{SHOP_COMMAND_SEQUENCE_SEPARATOR}{}",
                first.option.command, second.option.command
            ),
            card: first.option.card.or(second.option.card),
            effect_kind: "shop_buy_combo".to_string(),
            effect_label: format!(
                "{} then {} | total {cost} gold | shop portfolio combo",
                first.option.label, second.option.label
            ),
            representative_count: 2,
            suppressed_count: 0,
        },
    }
}

fn shop_relic_purchase_keeps_shop_open(relic: RelicId) -> bool {
    !matches!(
        relic,
        RelicId::Orrery
            | RelicId::DollysMirror
            | RelicId::BottledFlame
            | RelicId::BottledLightning
            | RelicId::BottledTornado
            | RelicId::Cauldron
    )
}
