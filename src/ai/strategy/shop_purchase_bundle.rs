use crate::ai::strategy::boss_survival_evidence::BossSurvivalRepairKind;
use crate::ai::strategy::decision_pipeline::{
    CandidateEvaluation, CandidateLane, CleanupTarget, DecisionCandidateIr, DecisionCandidateKind,
};
use crate::content::cards::CardId;
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShopGoldOpportunity {
    pub current_gold: i32,
    pub current_hp: i32,
    pub max_hp: i32,
    pub active_maw_bank: bool,
    pub future_rooms_before_next_shop: u8,
    pub hard_checkpoint_imminent: bool,
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
    StrategicBossRepairBuy,
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
    pub repairs_boss_scaling_plan: bool,
    pub boss_survival_repair: Option<BossSurvivalRepairKind>,
    pub adds_deck_burden: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ShopPurchaseCandidateEvidence {
    pub repairs_boss_scaling_plan: bool,
    pub boss_survival_repair: Option<BossSurvivalRepairKind>,
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
    evaluate_shop_purchase_bundle_with_evidence(
        opportunity,
        candidate,
        ShopPurchaseCandidateEvidence::default(),
    )
}

pub fn evaluate_shop_purchase_bundle_with_evidence(
    opportunity: ShopGoldOpportunity,
    candidate: &CandidateEvaluation,
    evidence: ShopPurchaseCandidateEvidence,
) -> ShopPurchaseBundleDecision {
    let facts = bundle_facts(opportunity, candidate.candidate.kind, evidence);
    let (verdict, reason) = bundle_verdict(opportunity, candidate, facts);
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
            ShopPurchaseBundleVerdict::StrategicBossRepairBuy => 2,
            ShopPurchaseBundleVerdict::EfficientBundleBuy => 3,
            ShopPurchaseBundleVerdict::ContextBuy => 4,
            ShopPurchaseBundleVerdict::PreserveGoldPreferred => 5,
            ShopPurchaseBundleVerdict::Reject => 6,
        },
        -decision_score(decision),
    )
}

fn bundle_facts(
    opportunity: ShopGoldOpportunity,
    kind: DecisionCandidateKind,
    evidence: ShopPurchaseCandidateEvidence,
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
        DecisionCandidateKind::ShopPurge { target } => (
            ShopPurchaseBundleKind::RemoveOnly,
            75,
            false,
            false,
            opportunity.hard_checkpoint_imminent && is_checkpoint_cleanup_target(target),
        ),
        DecisionCandidateKind::ShopBuyCard { price, .. } => (
            ShopPurchaseBundleKind::BuyOneCard,
            price,
            true,
            false,
            opportunity.hard_checkpoint_imminent
                && opportunity.boss_answer_needed
                && is_boss_answer_card(kind),
        ),
        DecisionCandidateKind::ShopBuyRelic { relic, price } => (
            ShopPurchaseBundleKind::BuyOneRelic,
            price,
            false,
            is_hard_survival_relic(opportunity, relic),
            opportunity.hard_checkpoint_imminent
                && opportunity.boss_answer_needed
                && is_boss_answer_relic(relic),
        ),
        DecisionCandidateKind::ShopBuyPotion { potion, price } => (
            ShopPurchaseBundleKind::BuyOnePotion,
            price,
            false,
            opportunity.survival_purchase_needed && is_hard_survival_potion(potion),
            opportunity.hard_checkpoint_imminent
                && opportunity.boss_answer_needed
                && is_boss_answer_potion(potion),
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
        repairs_boss_scaling_plan: opportunity.boss_answer_needed
            && !opportunity.survival_purchase_needed
            && evidence.repairs_boss_scaling_plan,
        boss_survival_repair: if opportunity.boss_answer_needed
            && !opportunity.survival_purchase_needed
        {
            evidence.boss_survival_repair
        } else {
            None
        },
        adds_deck_burden,
    }
}

fn bundle_verdict(
    opportunity: ShopGoldOpportunity,
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
        if facts.kind == ShopPurchaseBundleKind::RemoveOnly {
            return (
                ShopPurchaseBundleVerdict::HardBossAnswerBuy,
                "HardCheckpointCleanupPurchase",
            );
        }
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
    if facts.repairs_boss_scaling_plan {
        return (
            ShopPurchaseBundleVerdict::StrategicBossRepairBuy,
            "StrategicBossScalingRepair",
        );
    }
    if let Some(repair) = facts.boss_survival_repair {
        return (
            ShopPurchaseBundleVerdict::StrategicBossRepairBuy,
            match repair {
                BossSurvivalRepairKind::PlanRepair => "StrategicBossSurvivalPlanRepair",
                BossSurvivalRepairKind::TimedBridge => "StrategicBossSurvivalTimedBridge",
            },
        );
    }
    if spends_future_shop_liquidity_without_hard_need(opportunity, facts, candidate) {
        return (
            ShopPurchaseBundleVerdict::Reject,
            "SpendsFutureShopLiquidityWithoutHardNeed",
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

fn spends_future_shop_liquidity_without_hard_need(
    opportunity: ShopGoldOpportunity,
    facts: ShopPurchaseBundleFacts,
    candidate: &CandidateEvaluation,
) -> bool {
    !opportunity.hard_checkpoint_imminent
        && opportunity.future_rooms_before_next_shop <= 2
        && facts.total_cost > 0
        && facts.future_gold_lost_if_breaks_maw_bank == 0
        && !facts.preserves_next_shop_option
        && !facts.solves_next_fight
        && !facts.solves_boss_gap
        && matches!(
            candidate.candidate.kind,
            DecisionCandidateKind::ShopBuyCard { .. }
                | DecisionCandidateKind::ShopBuyRelic { .. }
                | DecisionCandidateKind::ShopBuyPotion { .. }
                | DecisionCandidateKind::ShopPurge { .. }
        )
}

fn decision_score(decision: &ShopPurchaseBundleDecision) -> i32 {
    match decision.verdict {
        ShopPurchaseBundleVerdict::HardSurvivalBuy => 400 + decision.candidate_score,
        ShopPurchaseBundleVerdict::HardBossAnswerBuy => 360 + decision.candidate_score,
        ShopPurchaseBundleVerdict::StrategicBossRepairBuy => 320 + decision.candidate_score,
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

fn is_hard_survival_relic(opportunity: ShopGoldOpportunity, relic: RelicId) -> bool {
    match relic {
        RelicId::Waffle => {
            opportunity.survival_purchase_needed || low_hp_for_full_heal_relic(opportunity)
        }
        _ => false,
    }
}

fn low_hp_for_full_heal_relic(opportunity: ShopGoldOpportunity) -> bool {
    opportunity.max_hp > 0 && opportunity.current_hp * 2 <= opportunity.max_hp
}

fn is_boss_answer_relic(relic: RelicId) -> bool {
    matches!(
        relic,
        RelicId::Waffle | RelicId::MedicalKit | RelicId::OrangePellets
    )
}

fn is_boss_answer_card(kind: DecisionCandidateKind) -> bool {
    matches!(
        kind,
        DecisionCandidateKind::ShopBuyCard {
            card: CardId::FiendFire | CardId::Bludgeon | CardId::Immolate | CardId::Reaper,
            ..
        }
    )
}

fn is_boss_answer_potion(potion: PotionId) -> bool {
    matches!(
        potion,
        PotionId::FirePotion
            | PotionId::ExplosivePotion
            | PotionId::FearPotion
            | PotionId::PowerPotion
    )
}

fn is_checkpoint_cleanup_target(target: CleanupTarget) -> bool {
    matches!(
        target,
        CleanupTarget::Curse | CleanupTarget::Status | CleanupTarget::StarterStrike
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::strategy::boss_survival_evidence::BossSurvivalRepairKind;
    use crate::ai::strategy::decision_pipeline::{
        CandidateEvaluation, CandidateLane, CandidateLaneAdjudication, DecisionCandidateIr,
        ExpansionPlan, ScoreComponent,
    };

    fn evaluation(kind: DecisionCandidateKind, score: i32) -> CandidateEvaluation {
        CandidateEvaluation {
            candidate: DecisionCandidateIr { kind },
            lane: CandidateLane::Mainline,
            adjudication: CandidateLaneAdjudication::uncapped(CandidateLane::Mainline),
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
            current_hp: 70,
            max_hp: 80,
            active_maw_bank: true,
            future_rooms_before_next_shop: 5,
            hard_checkpoint_imminent: false,
            survival_purchase_needed: false,
            boss_answer_needed: false,
        }
    }

    fn maw_bank_survival_opportunity(gold: i32) -> ShopGoldOpportunity {
        ShopGoldOpportunity {
            current_hp: 41,
            max_hp: 85,
            survival_purchase_needed: true,
            ..maw_bank_opportunity(gold)
        }
    }

    fn maw_bank_boss_gap_opportunity(gold: i32) -> ShopGoldOpportunity {
        ShopGoldOpportunity {
            boss_answer_needed: true,
            hard_checkpoint_imminent: true,
            ..maw_bank_opportunity(gold)
        }
    }

    fn visible_future_shop_opportunity(gold: i32) -> ShopGoldOpportunity {
        ShopGoldOpportunity {
            active_maw_bank: false,
            future_rooms_before_next_shop: 2,
            hard_checkpoint_imminent: false,
            survival_purchase_needed: false,
            boss_answer_needed: false,
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
    fn low_hp_waffle_is_hard_survival_buy() {
        let leave = evaluation(DecisionCandidateKind::ShopLeave, 0);
        let remove = evaluation(
            DecisionCandidateKind::ShopPurge {
                target: crate::ai::strategy::decision_pipeline::CleanupTarget::StarterStrike,
            },
            300,
        );
        let waffle = evaluation(
            DecisionCandidateKind::ShopBuyRelic {
                relic: RelicId::Waffle,
                price: 155,
            },
            220,
        );

        let opportunity = ShopGoldOpportunity {
            current_gold: 335,
            current_hp: 41,
            max_hp: 85,
            active_maw_bank: false,
            future_rooms_before_next_shop: 2,
            hard_checkpoint_imminent: false,
            survival_purchase_needed: false,
            boss_answer_needed: false,
        };
        let leave_bundle = evaluate_shop_purchase_bundle(opportunity, &leave);
        let remove_bundle = evaluate_shop_purchase_bundle(opportunity, &remove);
        let waffle_bundle = evaluate_shop_purchase_bundle(opportunity, &waffle);

        assert_eq!(
            waffle_bundle.verdict,
            ShopPurchaseBundleVerdict::HardSurvivalBuy
        );
        assert_eq!(waffle_bundle.reason, "HardSurvivalPurchase");
        assert!(
            shop_purchase_bundle_order_key(&waffle_bundle)
                < shop_purchase_bundle_order_key(&remove_bundle),
            "low HP Waffle should outrank cleanup: waffle={:?} remove={:?}",
            waffle_bundle,
            remove_bundle
        );
        assert!(
            shop_purchase_bundle_order_key(&waffle_bundle)
                < shop_purchase_bundle_order_key(&leave_bundle),
            "low HP Waffle should outrank leaving with gold: waffle={:?} leave={:?}",
            waffle_bundle,
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

    #[test]
    fn deterministic_boss_repair_can_break_maw_bank_when_boss_gap_is_open() {
        let leave = evaluation(DecisionCandidateKind::ShopLeave, 0);
        let fiend_fire = evaluation(
            DecisionCandidateKind::ShopBuyCard {
                card: crate::content::cards::CardId::FiendFire,
                upgrades: 0,
                price: 152,
            },
            70,
        );
        let fire_potion = evaluation(
            DecisionCandidateKind::ShopBuyPotion {
                potion: PotionId::FirePotion,
                price: 51,
            },
            70,
        );

        let opportunity = maw_bank_boss_gap_opportunity(288);
        let leave_bundle = evaluate_shop_purchase_bundle(opportunity, &leave);
        let fiend_fire_bundle = evaluate_shop_purchase_bundle(opportunity, &fiend_fire);
        let fire_potion_bundle = evaluate_shop_purchase_bundle(opportunity, &fire_potion);

        assert_eq!(
            fiend_fire_bundle.verdict,
            ShopPurchaseBundleVerdict::HardBossAnswerBuy
        );
        assert_eq!(
            fire_potion_bundle.verdict,
            ShopPurchaseBundleVerdict::HardBossAnswerBuy
        );
        assert_ne!(
            fiend_fire_bundle.reason, "BreaksMawBankWithoutHardNeed",
            "boss repair card should not be hard-rejected by Maw Bank when boss plan is open"
        );
        assert!(
            shop_purchase_bundle_order_key(&fiend_fire_bundle)
                < shop_purchase_bundle_order_key(&leave_bundle),
            "deterministic boss repair should outrank pure Maw Bank preservation: repair={:?} leave={:?}",
            fiend_fire_bundle,
            leave_bundle
        );
    }

    #[test]
    fn power_potion_can_break_maw_bank_as_high_ceiling_boss_answer() {
        let leave = evaluation(DecisionCandidateKind::ShopLeave, 0);
        let power_potion = evaluation(
            DecisionCandidateKind::ShopBuyPotion {
                potion: PotionId::PowerPotion,
                price: 78,
            },
            100,
        );

        let opportunity = maw_bank_boss_gap_opportunity(288);
        let leave_bundle = evaluate_shop_purchase_bundle(opportunity, &leave);
        let power_bundle = evaluate_shop_purchase_bundle(opportunity, &power_potion);

        assert_eq!(
            power_bundle.verdict,
            ShopPurchaseBundleVerdict::HardBossAnswerBuy
        );
        assert_ne!(
            power_bundle.reason, "BreaksMawBankWithoutHardNeed",
            "Power Potion should be allowed as a high-ceiling boss answer at a hard checkpoint"
        );
        assert!(
            shop_purchase_bundle_order_key(&power_bundle)
                < shop_purchase_bundle_order_key(&leave_bundle),
            "Power Potion should outrank pure Maw Bank preservation when boss answer pressure is open: power={:?} leave={:?}",
            power_bundle,
            leave_bundle
        );
    }

    #[test]
    fn hard_checkpoint_cleanup_can_break_maw_bank() {
        let leave = evaluation(DecisionCandidateKind::ShopLeave, 0);
        let remove_strike = evaluation(
            DecisionCandidateKind::ShopPurge {
                target: crate::ai::strategy::decision_pipeline::CleanupTarget::StarterStrike,
            },
            180,
        );

        let opportunity = ShopGoldOpportunity {
            current_hp: 70,
            max_hp: 80,
            hard_checkpoint_imminent: true,
            boss_answer_needed: false,
            ..maw_bank_opportunity(249)
        };
        let leave_bundle = evaluate_shop_purchase_bundle(opportunity, &leave);
        let remove_bundle = evaluate_shop_purchase_bundle(opportunity, &remove_strike);

        assert_eq!(
            remove_bundle.verdict,
            ShopPurchaseBundleVerdict::HardBossAnswerBuy
        );
        assert_ne!(
            remove_bundle.reason, "BreaksMawBankWithoutHardNeed",
            "last checkpoint cleanup should not be hard-rejected by Maw Bank"
        );
        assert!(
            shop_purchase_bundle_order_key(&remove_bundle)
                < shop_purchase_bundle_order_key(&leave_bundle),
            "checkpoint cleanup should outrank pure Maw Bank preservation: remove={:?} leave={:?}",
            remove_bundle,
            leave_bundle
        );
    }

    #[test]
    fn boss_repair_does_not_break_maw_bank_before_hard_checkpoint_window() {
        let fiend_fire = evaluation(
            DecisionCandidateKind::ShopBuyCard {
                card: crate::content::cards::CardId::FiendFire,
                upgrades: 0,
                price: 152,
            },
            70,
        );

        let early_boss_gap = ShopGoldOpportunity {
            current_hp: 70,
            max_hp: 80,
            boss_answer_needed: true,
            ..maw_bank_opportunity(230)
        };
        let fiend_fire_bundle = evaluate_shop_purchase_bundle(early_boss_gap, &fiend_fire);

        assert_eq!(fiend_fire_bundle.verdict, ShopPurchaseBundleVerdict::Reject);
        assert_eq!(fiend_fire_bundle.reason, "BreaksMawBankWithoutHardNeed");
    }

    #[test]
    fn future_shop_liquidity_rejects_generic_relic_that_spends_below_shop_option() {
        let clockwork = evaluation(
            DecisionCandidateKind::ShopBuyRelic {
                relic: RelicId::ClockworkSouvenir,
                price: 149,
            },
            115,
        );

        let bundle =
            evaluate_shop_purchase_bundle(visible_future_shop_opportunity(151), &clockwork);

        assert_eq!(bundle.verdict, ShopPurchaseBundleVerdict::Reject);
        assert_eq!(bundle.reason, "SpendsFutureShopLiquidityWithoutHardNeed");
    }

    #[test]
    fn semantic_boss_scaling_repair_can_spend_future_shop_liquidity() {
        let demon_form = evaluation(
            DecisionCandidateKind::ShopBuyCard {
                card: CardId::DemonForm,
                upgrades: 0,
                price: 139,
            },
            160,
        );
        let opportunity = ShopGoldOpportunity {
            boss_answer_needed: true,
            ..visible_future_shop_opportunity(213)
        };

        let ordinary = evaluate_shop_purchase_bundle(opportunity, &demon_form);
        let repair = evaluate_shop_purchase_bundle_with_evidence(
            opportunity,
            &demon_form,
            ShopPurchaseCandidateEvidence {
                repairs_boss_scaling_plan: true,
                boss_survival_repair: None,
            },
        );

        assert_eq!(ordinary.verdict, ShopPurchaseBundleVerdict::Reject);
        assert_eq!(ordinary.reason, "SpendsFutureShopLiquidityWithoutHardNeed");
        assert_eq!(
            repair.verdict,
            ShopPurchaseBundleVerdict::StrategicBossRepairBuy
        );
        assert_eq!(repair.reason, "StrategicBossScalingRepair");
    }

    #[test]
    fn timed_boss_survival_bridge_can_spend_future_shop_liquidity() {
        let shackles = evaluation(
            DecisionCandidateKind::ShopBuyCard {
                card: CardId::DarkShackles,
                upgrades: 1,
                price: 78,
            },
            120,
        );
        let opportunity = ShopGoldOpportunity {
            boss_answer_needed: true,
            ..visible_future_shop_opportunity(180)
        };

        let decision = evaluate_shop_purchase_bundle_with_evidence(
            opportunity,
            &shackles,
            ShopPurchaseCandidateEvidence {
                repairs_boss_scaling_plan: false,
                boss_survival_repair: Some(BossSurvivalRepairKind::TimedBridge),
            },
        );

        assert_eq!(
            decision.verdict,
            ShopPurchaseBundleVerdict::StrategicBossRepairBuy
        );
        assert_eq!(decision.reason, "StrategicBossSurvivalTimedBridge");
        assert_eq!(
            decision.facts.boss_survival_repair,
            Some(BossSurvivalRepairKind::TimedBridge)
        );
    }

    #[test]
    fn boss_survival_plan_repair_uses_distinct_bundle_reason() {
        let disarm = evaluation(
            DecisionCandidateKind::ShopBuyCard {
                card: CardId::Disarm,
                upgrades: 0,
                price: 75,
            },
            120,
        );
        let decision = evaluate_shop_purchase_bundle_with_evidence(
            ShopGoldOpportunity {
                boss_answer_needed: true,
                ..visible_future_shop_opportunity(180)
            },
            &disarm,
            ShopPurchaseCandidateEvidence {
                repairs_boss_scaling_plan: false,
                boss_survival_repair: Some(BossSurvivalRepairKind::PlanRepair),
            },
        );

        assert_eq!(
            decision.verdict,
            ShopPurchaseBundleVerdict::StrategicBossRepairBuy
        );
        assert_eq!(decision.reason, "StrategicBossSurvivalPlanRepair");
    }

    #[test]
    fn timed_bridge_does_not_override_maw_bank_or_survival_emergency() {
        let shackles = evaluation(
            DecisionCandidateKind::ShopBuyCard {
                card: CardId::DarkShackles,
                upgrades: 1,
                price: 78,
            },
            120,
        );
        let evidence = ShopPurchaseCandidateEvidence {
            repairs_boss_scaling_plan: false,
            boss_survival_repair: Some(BossSurvivalRepairKind::TimedBridge),
        };

        let maw = evaluate_shop_purchase_bundle_with_evidence(
            ShopGoldOpportunity {
                boss_answer_needed: true,
                ..maw_bank_opportunity(180)
            },
            &shackles,
            evidence,
        );
        let emergency = evaluate_shop_purchase_bundle_with_evidence(
            ShopGoldOpportunity {
                boss_answer_needed: true,
                survival_purchase_needed: true,
                ..visible_future_shop_opportunity(180)
            },
            &shackles,
            evidence,
        );

        assert_eq!(maw.reason, "BreaksMawBankWithoutHardNeed");
        assert_eq!(emergency.reason, "SpendsFutureShopLiquidityWithoutHardNeed");
    }

    #[test]
    fn strategic_boss_scaling_repair_does_not_override_maw_bank_or_survival_emergency() {
        let demon_form = evaluation(
            DecisionCandidateKind::ShopBuyCard {
                card: CardId::DemonForm,
                upgrades: 0,
                price: 139,
            },
            160,
        );
        let evidence = ShopPurchaseCandidateEvidence {
            repairs_boss_scaling_plan: true,
            boss_survival_repair: None,
        };
        let maw_bank = ShopGoldOpportunity {
            boss_answer_needed: true,
            ..maw_bank_opportunity(213)
        };
        let survival_emergency = ShopGoldOpportunity {
            boss_answer_needed: true,
            survival_purchase_needed: true,
            ..visible_future_shop_opportunity(213)
        };

        let maw_bank_repair =
            evaluate_shop_purchase_bundle_with_evidence(maw_bank, &demon_form, evidence);
        let survival_repair =
            evaluate_shop_purchase_bundle_with_evidence(survival_emergency, &demon_form, evidence);

        assert_eq!(maw_bank_repair.verdict, ShopPurchaseBundleVerdict::Reject);
        assert_eq!(maw_bank_repair.reason, "BreaksMawBankWithoutHardNeed");
        assert_eq!(survival_repair.verdict, ShopPurchaseBundleVerdict::Reject);
        assert_eq!(
            survival_repair.reason,
            "SpendsFutureShopLiquidityWithoutHardNeed"
        );
    }

    #[test]
    fn future_shop_liquidity_allows_remove_that_preserves_next_shop_option() {
        let remove = evaluation(
            DecisionCandidateKind::ShopPurge {
                target: crate::ai::strategy::decision_pipeline::CleanupTarget::StarterStrike,
            },
            180,
        );

        let bundle = evaluate_shop_purchase_bundle(visible_future_shop_opportunity(224), &remove);

        assert_eq!(
            bundle.verdict,
            ShopPurchaseBundleVerdict::EfficientBundleBuy
        );
    }
}
