use crate::ai::analysis::card_semantics::Mechanic;
use crate::ai::strategy::boss_scaling_evidence::assess_boss_scaling_evidence;
use crate::ai::strategy::deck_plan::DeckPlanSnapshot;
use crate::ai::strategy::deck_strategic_deficit::StrategicDeficitLevel;
use crate::ai::strategy::reward_admission::{RewardAdmission, RewardAdmissionReason};
use crate::content::cards::CardId;

const CHEAP_SHOP_CARD_PRICE: i32 = 35;
const SHOP_PURGE_RESERVE: i32 = 75;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AcquisitionContext {
    deck_plan: DeckPlanSnapshot,
    source: AcquisitionContextSource,
}

impl AcquisitionContext {
    pub fn reward(deck_plan: DeckPlanSnapshot) -> Self {
        Self {
            deck_plan,
            source: AcquisitionContextSource::Reward,
        }
    }

    pub fn shop(deck_plan: DeckPlanSnapshot, gold: i32, price: i32) -> Self {
        Self {
            deck_plan,
            source: AcquisitionContextSource::Shop { gold, price },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AcquisitionContextSource {
    Reward,
    Shop { gold: i32, price: i32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcquisitionSource {
    Reward,
    Shop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcquisitionCost {
    Free,
    Gold {
        price: i32,
        gold_before: i32,
        gold_after: i32,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcquisitionOpportunityCost {
    None,
    Cheap,
    PreservesPurgeReserve,
    SpendsPurgeReserve,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarginalAcquisitionQuality {
    Premium,
    Ordinary,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AcquisitionStrategicDelta {
    pub improves_hard_gap: bool,
    pub improves_any_gap: bool,
    pub adds_card_without_gap_improvement: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcquisitionPolicyVerdict {
    AutoAcquire,
    ContextTake,
    Speculative,
    SkipPreferred,
    Reject,
}

impl AcquisitionPolicyVerdict {
    pub fn allows_acquisition(self) -> bool {
        matches!(self, Self::AutoAcquire | Self::ContextTake)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcquisitionPolicyReason {
    RewardSurfaceChoice,
    PremiumCard,
    UpgradedShopCard,
    HardGapWithAcceptableOpportunityCost,
    LowMarginLacksHardGap,
    PurgeReserveBlocksHardGap,
    NoPolicySupport,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AcquisitionPolicyDecision {
    pub verdict: AcquisitionPolicyVerdict,
    pub reason: AcquisitionPolicyReason,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CardAcquisitionReport {
    pub card: CardId,
    pub upgrades: u8,
    pub source: AcquisitionSource,
    pub cost: AcquisitionCost,
    pub opportunity_cost: AcquisitionOpportunityCost,
    pub quality: MarginalAcquisitionQuality,
    pub strategic_delta: AcquisitionStrategicDelta,
    pub low_margin_filler: bool,
}

pub fn assess_card_acquisition(
    context: AcquisitionContext,
    card: CardId,
    upgrades: u8,
    admission: &RewardAdmission,
) -> CardAcquisitionReport {
    let source = acquisition_source(context.source);
    let cost = acquisition_cost(context.source);
    let opportunity_cost = acquisition_opportunity_cost(context.source);
    let quality = if premium_card(card) {
        MarginalAcquisitionQuality::Premium
    } else {
        MarginalAcquisitionQuality::Ordinary
    };
    let improves_hard_gap = improves_hard_gap(context.deck_plan, admission);
    let improves_any_gap = improves_any_gap(context.deck_plan, admission);
    let strategic_delta = AcquisitionStrategicDelta {
        improves_hard_gap,
        improves_any_gap,
        adds_card_without_gap_improvement: admission.card.is_some() && !improves_any_gap,
    };
    let low_margin_filler = admission.card.is_some_and(low_margin_filler_card);
    CardAcquisitionReport {
        card,
        upgrades,
        source,
        cost,
        opportunity_cost,
        quality,
        strategic_delta,
        low_margin_filler,
    }
}

pub fn evaluate_card_acquisition_policy_v0(
    report: &CardAcquisitionReport,
) -> AcquisitionPolicyDecision {
    acquisition_policy_decision(report)
}

fn acquisition_source(source: AcquisitionContextSource) -> AcquisitionSource {
    match source {
        AcquisitionContextSource::Reward => AcquisitionSource::Reward,
        AcquisitionContextSource::Shop { .. } => AcquisitionSource::Shop,
    }
}

fn acquisition_cost(source: AcquisitionContextSource) -> AcquisitionCost {
    match source {
        AcquisitionContextSource::Reward => AcquisitionCost::Free,
        AcquisitionContextSource::Shop { gold, price } => AcquisitionCost::Gold {
            price,
            gold_before: gold,
            gold_after: gold.saturating_sub(price),
        },
    }
}

fn acquisition_opportunity_cost(source: AcquisitionContextSource) -> AcquisitionOpportunityCost {
    match source {
        AcquisitionContextSource::Reward => AcquisitionOpportunityCost::None,
        AcquisitionContextSource::Shop { price, .. } if price <= CHEAP_SHOP_CARD_PRICE => {
            AcquisitionOpportunityCost::Cheap
        }
        AcquisitionContextSource::Shop { gold, price }
            if gold.saturating_sub(price) >= SHOP_PURGE_RESERVE =>
        {
            AcquisitionOpportunityCost::PreservesPurgeReserve
        }
        AcquisitionContextSource::Shop { .. } => AcquisitionOpportunityCost::SpendsPurgeReserve,
    }
}

impl AcquisitionPolicyDecision {
    pub fn allows_acquisition(self) -> bool {
        self.verdict.allows_acquisition()
    }

    pub fn inspect_only_reason(self) -> Option<&'static str> {
        if self.allows_acquisition() {
            None
        } else {
            Some(acquisition_policy_reason_label(self.reason))
        }
    }
}

fn acquisition_policy_decision(report: &CardAcquisitionReport) -> AcquisitionPolicyDecision {
    match report.source {
        AcquisitionSource::Reward if report.quality == MarginalAcquisitionQuality::Premium => {
            acquisition_policy(
                AcquisitionPolicyVerdict::AutoAcquire,
                AcquisitionPolicyReason::PremiumCard,
            )
        }
        AcquisitionSource::Reward
            if report.low_margin_filler && !report.strategic_delta.improves_hard_gap =>
        {
            acquisition_policy(
                AcquisitionPolicyVerdict::Speculative,
                AcquisitionPolicyReason::LowMarginLacksHardGap,
            )
        }
        AcquisitionSource::Reward if report.strategic_delta.improves_any_gap => acquisition_policy(
            AcquisitionPolicyVerdict::ContextTake,
            AcquisitionPolicyReason::RewardSurfaceChoice,
        ),
        AcquisitionSource::Reward
            if report.low_margin_filler
                && report.strategic_delta.adds_card_without_gap_improvement =>
        {
            acquisition_policy(
                AcquisitionPolicyVerdict::Speculative,
                AcquisitionPolicyReason::LowMarginLacksHardGap,
            )
        }
        AcquisitionSource::Reward => acquisition_policy(
            AcquisitionPolicyVerdict::ContextTake,
            AcquisitionPolicyReason::RewardSurfaceChoice,
        ),
        AcquisitionSource::Shop if report.quality == MarginalAcquisitionQuality::Premium => {
            acquisition_policy(
                AcquisitionPolicyVerdict::AutoAcquire,
                AcquisitionPolicyReason::PremiumCard,
            )
        }
        AcquisitionSource::Shop if report.upgrades > 0 => acquisition_policy(
            AcquisitionPolicyVerdict::AutoAcquire,
            AcquisitionPolicyReason::UpgradedShopCard,
        ),
        AcquisitionSource::Shop
            if report.strategic_delta.improves_hard_gap
                && report.opportunity_cost != AcquisitionOpportunityCost::SpendsPurgeReserve =>
        {
            acquisition_policy(
                AcquisitionPolicyVerdict::ContextTake,
                AcquisitionPolicyReason::HardGapWithAcceptableOpportunityCost,
            )
        }
        AcquisitionSource::Shop if report.strategic_delta.improves_hard_gap => acquisition_policy(
            AcquisitionPolicyVerdict::SkipPreferred,
            AcquisitionPolicyReason::PurgeReserveBlocksHardGap,
        ),
        AcquisitionSource::Shop => acquisition_policy(
            AcquisitionPolicyVerdict::Reject,
            AcquisitionPolicyReason::NoPolicySupport,
        ),
    }
}

fn acquisition_policy(
    verdict: AcquisitionPolicyVerdict,
    reason: AcquisitionPolicyReason,
) -> AcquisitionPolicyDecision {
    AcquisitionPolicyDecision { verdict, reason }
}

fn acquisition_policy_reason_label(reason: AcquisitionPolicyReason) -> &'static str {
    match reason {
        AcquisitionPolicyReason::PurgeReserveBlocksHardGap => {
            "shop card would spend purge reserve despite hard gap"
        }
        AcquisitionPolicyReason::NoPolicySupport => "shop card has no acquisition policy support",
        AcquisitionPolicyReason::LowMarginLacksHardGap => {
            "low-margin card does not improve a hard strategic gap"
        }
        AcquisitionPolicyReason::RewardSurfaceChoice
        | AcquisitionPolicyReason::PremiumCard
        | AcquisitionPolicyReason::UpgradedShopCard
        | AcquisitionPolicyReason::HardGapWithAcceptableOpportunityCost => {
            "shop card fails acquisition discipline"
        }
    }
}

fn premium_card(card: CardId) -> bool {
    matches!(
        card,
        CardId::MasterOfStrategy | CardId::Offering | CardId::Apotheosis
    )
}

fn improves_hard_gap(deck_plan: DeckPlanSnapshot, admission: &RewardAdmission) -> bool {
    let deficit = deck_plan.strategic_deficit;
    (deficit.deck_access == StrategicDeficitLevel::Missing
        && (admission_provides(admission, Mechanic::CardDraw)
            || admission
                .reasons
                .contains(&RewardAdmissionReason::CombatUpgrade)))
        || (needs(deficit.energy_or_playability) && admission_provides(admission, Mechanic::Energy))
        || (deficit.aoe_or_minion_control == StrategicDeficitLevel::Missing
            && admission_aoe(admission))
        || (deficit.boss_scaling_plan == StrategicDeficitLevel::Missing
            && assess_boss_scaling_evidence(deck_plan, None, admission).relevant_to_boss_plan
            && !fragile_supported_payoff(deck_plan, admission))
        || (deficit.frontload_damage == StrategicDeficitLevel::Missing
            && admission_frontloads(admission))
}

fn improves_any_gap(deck_plan: DeckPlanSnapshot, admission: &RewardAdmission) -> bool {
    let deficit = deck_plan.strategic_deficit;
    (needs(deficit.deck_access)
        && (admission_provides(admission, Mechanic::CardDraw)
            || admission
                .reasons
                .contains(&RewardAdmissionReason::CombatUpgrade)))
        || (needs(deficit.energy_or_playability) && admission_provides(admission, Mechanic::Energy))
        || (needs(deficit.aoe_or_minion_control) && admission_aoe(admission))
        || (needs(deficit.block_or_mitigation) && admission_survival_tool(admission))
        || (needs(deficit.boss_scaling_plan)
            && assess_boss_scaling_evidence(deck_plan, None, admission).relevant_to_boss_plan
            && !fragile_supported_payoff(deck_plan, admission))
        || (needs(deficit.frontload_damage) && admission_frontloads(admission))
}

fn needs(level: StrategicDeficitLevel) -> bool {
    matches!(
        level,
        StrategicDeficitLevel::Missing | StrategicDeficitLevel::Thin
    )
}

fn low_margin_filler_card(card: CardId) -> bool {
    matches!(
        card,
        CardId::TwinStrike
            | CardId::SwordBoomerang
            | CardId::WildStrike
            | CardId::RecklessCharge
            | CardId::Rampage
            | CardId::IronWave
            | CardId::Clothesline
            | CardId::ThunderClap
            | CardId::Anger
            | CardId::SwiftStrike
    )
}

fn fragile_supported_payoff(deck_plan: DeckPlanSnapshot, admission: &RewardAdmission) -> bool {
    if !admission
        .reasons
        .iter()
        .any(|reason| matches!(reason, RewardAdmissionReason::Supports(_)))
    {
        return false;
    }
    if admission_damage_uses(admission, Mechanic::Strength) {
        return deck_plan.roles.strength_source_units < 2;
    }
    if admission_damage_uses(admission, Mechanic::Block) {
        let roles = deck_plan.roles;
        return roles.block_units < 4 && roles.cycle_block_units < 2;
    }
    false
}

fn admission_provides(admission: &RewardAdmission, mechanic: Mechanic) -> bool {
    admission
        .reasons
        .contains(&RewardAdmissionReason::Provides(mechanic))
}

fn admission_frontloads(admission: &RewardAdmission) -> bool {
    admission
        .reasons
        .contains(&RewardAdmissionReason::FrontloadDamage)
}

fn admission_aoe(admission: &RewardAdmission) -> bool {
    admission
        .reasons
        .contains(&RewardAdmissionReason::AreaDamage)
}

fn admission_survival_tool(admission: &RewardAdmission) -> bool {
    admission_provides(admission, Mechanic::Block)
        || admission_provides(admission, Mechanic::Weak)
        || admission_provides(admission, Mechanic::EnemyStrengthDown)
}

fn admission_damage_uses(admission: &RewardAdmission, mechanic: Mechanic) -> bool {
    admission
        .reasons
        .contains(&RewardAdmissionReason::DamageUses(mechanic))
}

#[cfg(test)]
mod tests {
    use crate::ai::strategy::deck_admission::DeckAdmissionContext;
    use crate::ai::strategy::deck_plan::DeckPlanSnapshot;
    use crate::ai::strategy::reward_admission::assess_reward_admission_from_master_deck;
    use crate::ai::strategy::run_strategic_facts::RunStrategicFacts;
    use crate::content::cards::CardId;
    use crate::runtime::combat::CombatCard;

    use super::{
        assess_card_acquisition, evaluate_card_acquisition_policy_v0, AcquisitionContext,
        AcquisitionCost, AcquisitionOpportunityCost, AcquisitionPolicyReason,
        AcquisitionPolicyVerdict, AcquisitionSource, MarginalAcquisitionQuality,
    };

    fn deck(cards: &[CardId]) -> Vec<CombatCard> {
        cards
            .iter()
            .enumerate()
            .map(|(index, card)| CombatCard::new(*card, index as u32 + 1))
            .collect()
    }

    fn act1_shop_plan(cards: &[CardId]) -> DeckPlanSnapshot {
        DeckPlanSnapshot::from_deck(
            &deck(cards),
            DeckAdmissionContext {
                act: 1,
                current_hp: 74,
                max_hp: 85,
            },
            RunStrategicFacts {
                entering_act: 2,
                starter_basic_count: 0,
                curse_count: 0,
                has_energy_relic: false,
            },
        )
    }

    fn act1_missing_access_deck() -> Vec<CardId> {
        vec![
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
            CardId::Armaments,
            CardId::Cleave,
            CardId::Inflame,
            CardId::Uppercut,
            CardId::Whirlwind,
        ]
    }

    #[test]
    fn shop_card_acquisition_exposes_gold_opportunity_cost() {
        let cards = act1_missing_access_deck();
        let deck = deck(&cards);
        let admission = assess_reward_admission_from_master_deck(&deck, CardId::ShrugItOff, 0);
        let report = assess_card_acquisition(
            AcquisitionContext::shop(act1_shop_plan(&cards), 72, 51),
            CardId::ShrugItOff,
            0,
            &admission,
        );
        let policy = evaluate_card_acquisition_policy_v0(&report);

        assert_eq!(report.source, AcquisitionSource::Shop);
        assert_eq!(
            report.cost,
            AcquisitionCost::Gold {
                price: 51,
                gold_before: 72,
                gold_after: 21
            }
        );
        assert_eq!(
            report.opportunity_cost,
            AcquisitionOpportunityCost::SpendsPurgeReserve
        );
        assert!(report.strategic_delta.improves_hard_gap);
        assert_eq!(policy.verdict, AcquisitionPolicyVerdict::SkipPreferred);
        assert_eq!(
            policy.reason,
            AcquisitionPolicyReason::PurgeReserveBlocksHardGap
        );
        assert!(!policy.allows_acquisition());
    }

    #[test]
    fn reward_card_acquisition_has_no_gold_opportunity_cost() {
        let cards = act1_missing_access_deck();
        let deck = deck(&cards);
        let admission = assess_reward_admission_from_master_deck(&deck, CardId::ShrugItOff, 0);
        let report = assess_card_acquisition(
            AcquisitionContext::reward(act1_shop_plan(&cards)),
            CardId::ShrugItOff,
            0,
            &admission,
        );

        assert_eq!(report.source, AcquisitionSource::Reward);
        assert_eq!(report.cost, AcquisitionCost::Free);
        assert_eq!(report.opportunity_cost, AcquisitionOpportunityCost::None);

        let policy = evaluate_card_acquisition_policy_v0(&report);
        assert_eq!(policy.verdict, AcquisitionPolicyVerdict::ContextTake);
        assert_eq!(policy.reason, AcquisitionPolicyReason::RewardSurfaceChoice);
        assert!(policy.allows_acquisition());
    }

    #[test]
    fn premium_shop_card_records_premium_quality() {
        let cards = act1_missing_access_deck();
        let deck = deck(&cards);
        let admission =
            assess_reward_admission_from_master_deck(&deck, CardId::MasterOfStrategy, 0);
        let report = assess_card_acquisition(
            AcquisitionContext::shop(act1_shop_plan(&cards), 72, 51),
            CardId::MasterOfStrategy,
            0,
            &admission,
        );
        let policy = evaluate_card_acquisition_policy_v0(&report);

        assert_eq!(report.quality, MarginalAcquisitionQuality::Premium);
        assert_eq!(policy.verdict, AcquisitionPolicyVerdict::AutoAcquire);
        assert_eq!(policy.reason, AcquisitionPolicyReason::PremiumCard);
        assert!(policy.allows_acquisition());
    }
}
