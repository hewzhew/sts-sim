use crate::ai::strategy::decision_pipeline::{
    CandidateEvaluation, CandidateLane, DecisionCandidateIr, DecisionCandidateKind,
};
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShopGoldOpportunity {
    pub current_gold: i32,
    pub active_maw_bank: bool,
    pub future_rooms_before_next_shop: u8,
    pub survival_purchase_needed: bool,
    pub boss_answer_needed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopPurchaseBundleKind {
    LeaveWithGold,
    RemoveOnly,
    BuyOneCard,
    BuyOneRelic,
    BuyOnePotion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ShopPurchaseBundleVerdict {
    HardSurvivalBuy,
    HardBossAnswerBuy,
    EfficientBundleBuy,
    ContextBuy,
    PreserveGoldPreferred,
    Reject,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShopPurchaseBundleFacts {
    pub kind: ShopPurchaseBundleKind,
    pub total_cost: i32,
    pub gold_after: i32,
    pub breaks_maw_bank: bool,
    pub future_gold_lost_if_breaks_maw_bank: i32,
    pub preserves_remove_option: bool,
    pub preserves_next_shop_option: bool,
    pub solves_next_fight: bool,
    pub solves_boss_gap: bool,
    pub adds_deck_burden: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopPurchaseBundleDecision {
    pub candidate: DecisionCandidateIr,
    pub candidate_score: i32,
    pub facts: ShopPurchaseBundleFacts,
    pub verdict: ShopPurchaseBundleVerdict,
    pub reason: &'static str,
}

pub fn evaluate_shop_purchase_bundle(
    opportunity: ShopGoldOpportunity,
    candidate: &CandidateEvaluation,
) -> ShopPurchaseBundleDecision {
    let facts = bundle_facts(opportunity, candidate.candidate.kind);
    let (verdict, reason) = bundle_verdict(candidate, facts);
    ShopPurchaseBundleDecision {
        candidate: candidate.candidate,
        candidate_score: candidate.total_score(),
        facts,
        verdict,
        reason,
    }
}

pub fn shop_purchase_bundle_order_key(decision: &ShopPurchaseBundleDecision) -> (u8, i32) {
    (
        match decision.verdict {
            ShopPurchaseBundleVerdict::HardSurvivalBuy => 0,
            ShopPurchaseBundleVerdict::HardBossAnswerBuy => 1,
            ShopPurchaseBundleVerdict::EfficientBundleBuy => 2,
            ShopPurchaseBundleVerdict::ContextBuy => 3,
            ShopPurchaseBundleVerdict::PreserveGoldPreferred => 4,
            ShopPurchaseBundleVerdict::Reject => 5,
        },
        -decision_score(decision),
    )
}

fn bundle_facts(
    opportunity: ShopGoldOpportunity,
    kind: DecisionCandidateKind,
) -> ShopPurchaseBundleFacts {
    let (bundle_kind, total_cost, adds_deck_burden, solves_next_fight, solves_boss_gap) = match kind
    {
        DecisionCandidateKind::ShopLeave => (
            ShopPurchaseBundleKind::LeaveWithGold,
            0,
            false,
            false,
            false,
        ),
        DecisionCandidateKind::ShopPurge { .. } => {
            (ShopPurchaseBundleKind::RemoveOnly, 75, false, false, false)
        }
        DecisionCandidateKind::ShopBuyCard { price, .. } => (
            ShopPurchaseBundleKind::BuyOneCard,
            price,
            true,
            false,
            false,
        ),
        DecisionCandidateKind::ShopBuyRelic { relic, price } => (
            ShopPurchaseBundleKind::BuyOneRelic,
            price,
            false,
            false,
            opportunity.boss_answer_needed && is_boss_answer_relic(relic),
        ),
        DecisionCandidateKind::ShopBuyPotion { potion, price } => (
            ShopPurchaseBundleKind::BuyOnePotion,
            price,
            false,
            opportunity.survival_purchase_needed && is_hard_survival_potion(potion),
            false,
        ),
        _ => (
            ShopPurchaseBundleKind::LeaveWithGold,
            0,
            false,
            false,
            false,
        ),
    };
    let gold_after = opportunity.current_gold - total_cost;
    let breaks_maw_bank = opportunity.active_maw_bank && total_cost > 0;
    let future_gold_lost_if_breaks_maw_bank = if breaks_maw_bank {
        i32::from(opportunity.future_rooms_before_next_shop) * 12
    } else {
        0
    };
    ShopPurchaseBundleFacts {
        kind: bundle_kind,
        total_cost,
        gold_after,
        breaks_maw_bank,
        future_gold_lost_if_breaks_maw_bank,
        preserves_remove_option: gold_after >= 75,
        preserves_next_shop_option: gold_after >= 120,
        solves_next_fight,
        solves_boss_gap,
        adds_deck_burden,
    }
}

fn bundle_verdict(
    candidate: &CandidateEvaluation,
    facts: ShopPurchaseBundleFacts,
) -> (ShopPurchaseBundleVerdict, &'static str) {
    if facts.kind == ShopPurchaseBundleKind::LeaveWithGold {
        return (
            ShopPurchaseBundleVerdict::PreserveGoldPreferred,
            "LeaveWithGoldPreservesOptions",
        );
    }
    if facts.solves_next_fight {
        return (
            ShopPurchaseBundleVerdict::HardSurvivalBuy,
            "HardSurvivalPurchase",
        );
    }
    if facts.solves_boss_gap {
        return (
            ShopPurchaseBundleVerdict::HardBossAnswerBuy,
            "HardBossAnswerPurchase",
        );
    }
    if facts.breaks_maw_bank {
        return (
            ShopPurchaseBundleVerdict::Reject,
            "BreaksMawBankWithoutHardNeed",
        );
    }
    if facts.kind == ShopPurchaseBundleKind::RemoveOnly {
        return (
            ShopPurchaseBundleVerdict::EfficientBundleBuy,
            "EfficientRemoveBundle",
        );
    }
    if candidate.lane == CandidateLane::Mainline {
        return (ShopPurchaseBundleVerdict::ContextBuy, "ContextPurchase");
    }
    (
        ShopPurchaseBundleVerdict::Reject,
        "NoShopBundleJustification",
    )
}

fn decision_score(decision: &ShopPurchaseBundleDecision) -> i32 {
    match decision.verdict {
        ShopPurchaseBundleVerdict::HardSurvivalBuy => 400 + decision.candidate_score,
        ShopPurchaseBundleVerdict::HardBossAnswerBuy => 360 + decision.candidate_score,
        ShopPurchaseBundleVerdict::EfficientBundleBuy => 260 + decision.candidate_score,
        ShopPurchaseBundleVerdict::ContextBuy => 160 + decision.candidate_score,
        ShopPurchaseBundleVerdict::PreserveGoldPreferred => {
            120 + decision.facts.future_gold_lost_if_breaks_maw_bank
        }
        ShopPurchaseBundleVerdict::Reject => 0,
    }
}

fn is_hard_survival_potion(potion: PotionId) -> bool {
    matches!(
        potion,
        PotionId::FirePotion
            | PotionId::ExplosivePotion
            | PotionId::FearPotion
            | PotionId::BlockPotion
            | PotionId::DexterityPotion
            | PotionId::SwiftPotion
            | PotionId::EnergyPotion
            | PotionId::GamblersBrew
            | PotionId::LiquidMemories
    )
}

fn is_boss_answer_relic(relic: RelicId) -> bool {
    matches!(
        relic,
        RelicId::Waffle | RelicId::MedicalKit | RelicId::OrangePellets
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::strategy::decision_pipeline::{
        CandidateEvaluation, CandidateLane, DecisionCandidateIr, ExpansionPlan, ScoreComponent,
    };

    fn evaluation(kind: DecisionCandidateKind, score: i32) -> CandidateEvaluation {
        CandidateEvaluation {
            candidate: DecisionCandidateIr { kind },
            lane: CandidateLane::Mainline,
            expansion: ExpansionPlan::Auto,
            scores: vec![ScoreComponent {
                by: "test",
                value: score,
            }],
        }
    }

    fn maw_bank_opportunity(gold: i32) -> ShopGoldOpportunity {
        ShopGoldOpportunity {
            current_gold: gold,
            active_maw_bank: true,
            future_rooms_before_next_shop: 5,
            survival_purchase_needed: false,
            boss_answer_needed: false,
        }
    }

    fn maw_bank_survival_opportunity(gold: i32) -> ShopGoldOpportunity {
        ShopGoldOpportunity {
            survival_purchase_needed: true,
            ..maw_bank_opportunity(gold)
        }
    }

    #[test]
    fn maw_bank_preserves_gold_over_generic_relic_purchase() {
        let leave = evaluation(DecisionCandidateKind::ShopLeave, 0);
        let clockwork = evaluation(
            DecisionCandidateKind::ShopBuyRelic {
                relic: RelicId::ClockworkSouvenir,
                price: 149,
            },
            115,
        );

        let leave_bundle = evaluate_shop_purchase_bundle(maw_bank_opportunity(224), &leave);
        let clockwork_bundle = evaluate_shop_purchase_bundle(maw_bank_opportunity(224), &clockwork);

        assert_eq!(
            leave_bundle.verdict,
            ShopPurchaseBundleVerdict::PreserveGoldPreferred
        );
        assert_ne!(
            clockwork_bundle.verdict,
            ShopPurchaseBundleVerdict::ContextBuy,
            "generic relic purchase should not be context-buy eligible while breaking Maw Bank: {:?}",
            clockwork_bundle
        );
        assert!(
            shop_purchase_bundle_order_key(&leave_bundle)
                < shop_purchase_bundle_order_key(&clockwork_bundle),
            "LeaveWithGold should outrank generic Clockwork while active Maw Bank is valuable: leave={:?} clockwork={:?}",
            leave_bundle,
            clockwork_bundle
        );
    }

    #[test]
    fn hard_survival_potion_can_break_maw_bank() {
        let leave = evaluation(DecisionCandidateKind::ShopLeave, 0);
        let fire_potion = evaluation(
            DecisionCandidateKind::ShopBuyPotion {
                potion: PotionId::FirePotion,
                price: 50,
            },
            70,
        );

        let leave_bundle = evaluate_shop_purchase_bundle(maw_bank_opportunity(80), &leave);
        let potion_bundle =
            evaluate_shop_purchase_bundle(maw_bank_survival_opportunity(80), &fire_potion);

        assert_eq!(
            potion_bundle.verdict,
            ShopPurchaseBundleVerdict::HardSurvivalBuy
        );
        assert!(
            shop_purchase_bundle_order_key(&potion_bundle)
                < shop_purchase_bundle_order_key(&leave_bundle),
            "hard survival potion should be allowed to break Maw Bank: potion={:?} leave={:?}",
            potion_bundle,
            leave_bundle
        );
    }

    #[test]
    fn ordinary_potion_does_not_break_maw_bank_without_survival_need() {
        let fire_potion = evaluation(
            DecisionCandidateKind::ShopBuyPotion {
                potion: PotionId::FirePotion,
                price: 50,
            },
            70,
        );

        let potion_bundle = evaluate_shop_purchase_bundle(maw_bank_opportunity(80), &fire_potion);

        assert_eq!(potion_bundle.verdict, ShopPurchaseBundleVerdict::Reject);
        assert_eq!(potion_bundle.reason, "BreaksMawBankWithoutHardNeed");
    }
}
