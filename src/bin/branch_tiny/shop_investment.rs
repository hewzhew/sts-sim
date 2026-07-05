use sts_simulator::ai::strategy::decision_pipeline::{
    evaluate_decision_candidate, CandidateLane, DecisionCandidateKind, DecisionPipelineContext,
    MembershipCardInvestmentEvidence, ShopInvestmentEvidence,
};
use sts_simulator::ai::strategy::reward_admission::assess_reward_admission_from_master_deck;
use sts_simulator::eval::run_control::{DecisionCandidateKey, DecisionSurface, RunControlSession};
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::state::map::RoomType;
use sts_simulator::state::shop::membership_card_discounted_price;

pub(super) fn shop_investment_for_surface(
    session: &RunControlSession,
    surface: &DecisionSurface,
    deck: &[CombatCard],
    base_context: DecisionPipelineContext,
) -> Option<ShopInvestmentEvidence> {
    let membership_price = surface
        .view
        .candidates
        .iter()
        .find_map(|candidate| match candidate.key.as_ref() {
            Some(DecisionCandidateKey::ShopBuyRelic {
                relic: sts_simulator::content::relics::RelicId::MembershipCard,
                price,
                ..
            }) => Some(*price),
            _ => None,
        })?;
    let gold_after_membership = session.run_state.gold - membership_price;
    let after_membership_context =
        DecisionPipelineContext::shop(base_context.deck_plan, gold_after_membership);
    let mut eligible = surface
        .view
        .candidates
        .iter()
        .filter_map(|candidate| {
            shop_payoff_opportunity_for_key(
                candidate.key.as_ref()?,
                deck,
                after_membership_context,
                gold_after_membership,
            )
        })
        .collect::<Vec<_>>();
    eligible.sort_by_key(|item| {
        (
            u8::from(item.lane_after_membership_card != CandidateLane::Mainline),
            -(item.price_now - item.price_after_membership_card),
        )
    });

    let mut remaining_gold = gold_after_membership;
    let mut estimated_savings = 0;
    let mut has_mainline_payoff = false;
    let mut has_any_payoff = false;
    for item in eligible {
        if item.price_after_membership_card > remaining_gold {
            continue;
        }
        remaining_gold -= item.price_after_membership_card;
        estimated_savings += item.price_now - item.price_after_membership_card;
        has_any_payoff = true;
        has_mainline_payoff |= item.lane_after_membership_card == CandidateLane::Mainline;
    }

    let membership_card = if has_mainline_payoff && estimated_savings >= membership_price {
        MembershipCardInvestmentEvidence::SameShopAmortized
    } else if has_any_payoff {
        MembershipCardInvestmentEvidence::SameShopUnamortized
    } else if has_visible_future_shop(session) {
        MembershipCardInvestmentEvidence::FutureShop
    } else {
        MembershipCardInvestmentEvidence::NoPayoff
    };
    Some(ShopInvestmentEvidence { membership_card })
}

fn has_visible_future_shop(session: &RunControlSession) -> bool {
    let map = &session.run_state.map;
    if map.graph.is_empty() {
        return false;
    }
    let mut frontier = Vec::new();
    if map.current_y == -1 {
        if let Some(row) = map.graph.first() {
            frontier.extend(row.iter().map(|node| (node.x, node.y)));
        }
    } else if let Some(current) = map.get_current_node() {
        frontier.extend(current.edges.iter().map(|edge| (edge.dst_x, edge.dst_y)));
    }

    while let Some((x, y)) = frontier.pop() {
        let Some(node) = map
            .graph
            .get(y.max(0) as usize)
            .and_then(|row| row.get(x.max(0) as usize))
        else {
            continue;
        };
        if node.class == Some(RoomType::ShopRoom) {
            return true;
        }
        frontier.extend(node.edges.iter().map(|edge| (edge.dst_x, edge.dst_y)));
    }
    false
}

#[derive(Clone, Copy)]
struct ShopPayoffOpportunity {
    price_now: i32,
    price_after_membership_card: i32,
    lane_after_membership_card: CandidateLane,
}

fn shop_payoff_opportunity_for_key(
    key: &DecisionCandidateKey,
    deck: &[CombatCard],
    context: DecisionPipelineContext,
    gold_after_membership: i32,
) -> Option<ShopPayoffOpportunity> {
    let (price_now, discounted_kind, admission) = match key {
        DecisionCandidateKey::ShopBuyCard {
            card,
            upgrades,
            price,
            ..
        } => {
            let price_after_membership_card = membership_card_discounted_price(*price);
            (
                *price,
                DecisionCandidateKind::ShopBuyCard {
                    card: *card,
                    upgrades: *upgrades,
                    price: price_after_membership_card,
                },
                Some(assess_reward_admission_from_master_deck(
                    deck, *card, *upgrades,
                )),
            )
        }
        DecisionCandidateKey::ShopBuyRelic { relic, price, .. } => {
            let price_after_membership_card = membership_card_discounted_price(*price);
            (
                *price,
                DecisionCandidateKind::ShopBuyRelic {
                    relic: *relic,
                    price: price_after_membership_card,
                },
                None,
            )
        }
        DecisionCandidateKey::ShopBuyPotion { potion, price, .. } => {
            let price_after_membership_card = membership_card_discounted_price(*price);
            (
                *price,
                DecisionCandidateKind::ShopBuyPotion {
                    potion: *potion,
                    price: price_after_membership_card,
                },
                None,
            )
        }
        _ => return None,
    };
    let price_after_membership_card = match discounted_kind {
        DecisionCandidateKind::ShopBuyCard { price, .. }
        | DecisionCandidateKind::ShopBuyRelic { price, .. }
        | DecisionCandidateKind::ShopBuyPotion { price, .. } => price,
        _ => return None,
    };
    if matches!(
        discounted_kind,
        DecisionCandidateKind::ShopBuyRelic {
            relic: sts_simulator::content::relics::RelicId::MembershipCard,
            ..
        }
    ) || price_after_membership_card > gold_after_membership
    {
        return None;
    }
    let evaluation = evaluate_decision_candidate(context, discounted_kind, admission.as_ref());
    if !matches!(
        evaluation.lane,
        CandidateLane::Mainline | CandidateLane::Probe
    ) {
        return None;
    }
    Some(ShopPayoffOpportunity {
        price_now,
        price_after_membership_card,
        lane_after_membership_card: evaluation.lane,
    })
}
