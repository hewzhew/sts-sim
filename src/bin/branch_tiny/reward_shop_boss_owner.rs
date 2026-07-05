use sts_simulator::ai::strategy::decision_pipeline::{
    evaluate_decision_candidate, CandidateLane, CandidateOrderKey, DecisionCandidateKind,
    DecisionPipelineContext, MembershipCardInvestmentEvidence, ShopInvestmentEvidence,
};
use sts_simulator::ai::strategy::deck_plan::DeckPlanSnapshot;
use sts_simulator::ai::strategy::reward_admission::{
    assess_reward_admission_from_master_deck, reward_admission_order_key_v1, skip_reward_admission,
    RewardAdmission, RewardAdmissionOrderKeyV1,
};
use sts_simulator::eval::run_control::{DecisionCandidateKey, DecisionSurface, RunControlSession};
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::state::map::RoomType;
use sts_simulator::state::shop::membership_card_discounted_price;

use super::candidate_ir_adapter::{card_reward_kind, is_card_reward_key, shop_tiny_kind};
use super::expansion_policy::{expansion_from_evaluation, shop_tiny_choice_expansion};
use super::owner_model::{
    ChoiceAnnotation, OwnerCandidateDecision, OwnerChoice, OwnerChoiceExpansion,
};
use super::owners::executable_choices;

pub(super) fn card_reward_owner_choices(
    session: &RunControlSession,
    surface: &DecisionSurface,
) -> Vec<OwnerChoice> {
    let deck_plan = DeckPlanSnapshot::from_run_state(&session.run_state);
    let context = DecisionPipelineContext::reward(deck_plan);
    let mut choices = executable_choices(surface)
        .into_iter()
        .filter(|choice| is_card_reward_key(&choice.key))
        .map(|mut choice| {
            choice.annotation = reward_annotation_for_choice(session, &choice, context);
            choice.expansion = card_reward_choice_expansion(&choice);
            choice
        })
        .enumerate()
        .collect::<Vec<_>>();
    let has_mainline_take = choices
        .iter()
        .any(|(_, choice)| is_mainline_card_reward_take(choice));
    choices.sort_by_key(|(index, choice)| {
        (card_reward_choice_rank(choice, has_mainline_take), *index)
    });
    choices.into_iter().map(|(_, choice)| choice).collect()
}

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
    let mut auto_purge_targets = Vec::new();
    for (_, choice) in choices.iter_mut() {
        choice.expansion = shop_tiny_choice_expansion(&choice.annotation, &mut auto_purge_targets);
    }
    choices.sort_by_key(|(index, choice)| (shop_tiny_choice_rank(choice), *index));
    choices.into_iter().map(|(_, choice)| choice).collect()
}

fn shop_tiny_context(session: &RunControlSession) -> DecisionPipelineContext {
    DecisionPipelineContext::shop(
        DeckPlanSnapshot::from_run_state(&session.run_state),
        session.run_state.gold,
    )
}

fn shop_investment_for_surface(
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

fn reward_annotation_for_choice(
    session: &RunControlSession,
    choice: &OwnerChoice,
    context: DecisionPipelineContext,
) -> ChoiceAnnotation {
    match card_reward_kind(&choice.key) {
        Some(DecisionCandidateKind::CardRewardPick { card, upgrades }) => {
            let deck = &session.run_state.master_deck;
            candidate_annotation(
                context,
                DecisionCandidateKind::CardRewardPick { card, upgrades },
                Some(assess_reward_admission_from_master_deck(
                    deck, card, upgrades,
                )),
            )
        }
        Some(DecisionCandidateKind::CardRewardSkip) => candidate_annotation(
            context,
            DecisionCandidateKind::CardRewardSkip,
            Some(skip_reward_admission()),
        ),
        _ => ChoiceAnnotation::None,
    }
}

fn candidate_annotation(
    context: DecisionPipelineContext,
    kind: DecisionCandidateKind,
    admission: Option<RewardAdmission>,
) -> ChoiceAnnotation {
    let evaluation = evaluate_decision_candidate(context, kind, admission.as_ref());
    ChoiceAnnotation::Candidate(OwnerCandidateDecision {
        admission,
        evaluation,
    })
}

fn card_reward_choice_expansion(choice: &OwnerChoice) -> OwnerChoiceExpansion {
    expansion_from_evaluation(choice.annotation.evaluation())
}

fn card_reward_choice_rank(
    choice: &OwnerChoice,
    has_mainline_take: bool,
) -> (u8, CandidateOrderKey, RewardAdmissionOrderKeyV1) {
    match &choice.key {
        Some(DecisionCandidateKey::CardRewardOpen { .. }) => (
            0,
            CandidateOrderKey::fallback(),
            RewardAdmissionOrderKeyV1::empty_or_deferred(),
        ),
        Some(DecisionCandidateKey::CardRewardPick { .. }) => (
            1,
            choice
                .annotation
                .evaluation()
                .map(|evaluation| evaluation.order_key(has_mainline_take))
                .unwrap_or_else(CandidateOrderKey::fallback),
            choice
                .annotation
                .admission()
                .map(reward_admission_order_key_v1)
                .unwrap_or_else(RewardAdmissionOrderKeyV1::empty_or_deferred),
        ),
        Some(DecisionCandidateKey::CardRewardSingingBowl { .. }) => (
            1,
            CandidateOrderKey::optional_skip(has_mainline_take),
            RewardAdmissionOrderKeyV1::unscored_optional_reward(),
        ),
        Some(DecisionCandidateKey::CardRewardSkip { .. }) => (
            1,
            choice
                .annotation
                .evaluation()
                .map(|evaluation| evaluation.order_key(has_mainline_take))
                .unwrap_or_else(|| CandidateOrderKey::optional_skip(has_mainline_take)),
            choice
                .annotation
                .admission()
                .map(reward_admission_order_key_v1)
                .unwrap_or_else(RewardAdmissionOrderKeyV1::static_skip_boundary),
        ),
        _ => (
            2,
            CandidateOrderKey::fallback(),
            RewardAdmissionOrderKeyV1::empty_or_deferred(),
        ),
    }
}

fn is_mainline_card_reward_take(choice: &OwnerChoice) -> bool {
    matches!(
        choice.key,
        Some(DecisionCandidateKey::CardRewardPick { .. })
    ) && choice
        .annotation
        .evaluation()
        .is_some_and(|evaluation| evaluation.is_mainline())
}

fn shop_tiny_choice_rank(choice: &OwnerChoice) -> (u8, CandidateOrderKey) {
    match &choice.annotation {
        ChoiceAnnotation::Candidate(decision) => decision.evaluation.auto_order_key(false),
        _ => (u8::MAX, CandidateOrderKey::fallback()),
    }
}
